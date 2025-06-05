"""
Missing links dialog for Blend Vault.
Shows a modal dialog when items need to be relinked.
"""

import bpy
import os
from typing import List, Dict, Any, Tuple
from ..core import log_info, log_warning, log_error, log_success, log_debug, SIDECAR_EXTENSION
from . import asset_relinker
from .library_relinker import LibraryRelinkProcessor
from .resource_relinker import ResourceRelinkProcessor
from .shared_utils import ensure_saved_file, get_sidecar_path, SidecarParser, PathResolver, get_blend_file_path_from_sidecar


# Global flag to track if dialog is currently shown
_missing_links_dialog_shown = False

# Store detected relink items for the dialog
_pending_relink_items = []


class RelinkItem:
    """Represents an item that needs to be relinked."""
    def __init__(self, item_type: str, description: str, details: str = ""):
        self.item_type = item_type  # "asset", "library", "resource"
        self.description = description
        self.details = details


def check_missing_links():
    """
    Check what items need to be relinked.
    Returns a list of RelinkItem objects.
    """
    global _pending_relink_items
    _pending_relink_items.clear()
    
    blend_path = ensure_saved_file()
    if not blend_path:
        return []
    
    log_info("Checking for missing links...", module_name='MissingLinks')
    
    # Check for asset relinks needed
    _check_asset_relinks(blend_path)
    
    # Check for library relinks needed
    _check_library_relinks(blend_path)
    
    # Check for resource relinks needed
    _check_resource_relinks(blend_path)
    
    return _pending_relink_items


def _check_asset_relinks(blend_path: str):
    """Check if any assets need to be relinked."""
    try:
        sidecar_path = get_sidecar_path(blend_path)
        if not os.path.exists(sidecar_path):
            return
        
        parser = SidecarParser(sidecar_path)
        main_linked_data = parser.extract_json_blocks_with_links("Linked Libraries")
        if not main_linked_data:
            return
        
        # Check each library for potential asset mismatches
        for lib_rel_path, lib_data in main_linked_data.items():
            blend_dir = os.path.dirname(blend_path)
            lib_abs_path = PathResolver.resolve_relative_to_absolute(lib_rel_path, blend_dir)
            lib_sidecar_path = get_sidecar_path(lib_abs_path)
            
            if not os.path.exists(lib_sidecar_path):
                continue
                
            try:
                lib_parser = SidecarParser(lib_sidecar_path)
                _, assets_in_lib = lib_parser.extract_current_file_section()
                
                if not assets_in_lib:
                    continue
                
                # Create a mapping of authoritative UUID -> asset info
                authoritative_map = {
                    asset["uuid"]: {"name": asset["name"], "type": asset["type"]}
                    for asset in assets_in_lib if asset.get("uuid")
                }
                
                # Check for mismatches
                for asset_data in lib_data.get("assets", []):
                    asset_uuid = asset_data.get("uuid")
                    if not asset_uuid:
                        continue
                        
                    if asset_uuid in authoritative_map:
                        auth_info = authoritative_map[asset_uuid]
                        current_name = asset_data.get("name", "")
                        current_type = asset_data.get("type", "")
                        
                        if (auth_info["name"] != current_name or 
                            auth_info["type"] != current_type):
                            
                            description = f"Asset datablock '{current_name}' renamed to '{auth_info['name']}'"
                            details = f"In library: {os.path.basename(lib_rel_path)}"
                            if auth_info["type"] != current_type:
                                details += f" | Type changed: {current_type} → {auth_info['type']}"
                            _pending_relink_items.append(RelinkItem("asset", description, details))                            
            except Exception as e:
                log_debug(f"Error checking library sidecar {lib_sidecar_path}: {e}", module_name='MissingLinks')
                
    except Exception as e:
        log_debug(f"Error checking asset relinks: {e}", module_name='MissingLinks')


def _check_library_relinks(blend_path: str):
    """Check if any library paths need to be relinked."""
    try:
        processor = LibraryRelinkProcessor(blend_file_path=blend_path)
        sidecar_path = get_sidecar_path(blend_path)
        
        if not os.path.exists(sidecar_path):
            return
        parser = SidecarParser(sidecar_path)
        library_data = parser.extract_json_blocks_with_links("Linked Libraries")
        
        if not library_data:
            return
        # Debug: Log what library data was found
        log_debug(f"Found library data: {list(library_data.keys())}", module_name='MissingLinks')
        
        blend_dir = os.path.dirname(blend_path)
        
        # Also check for missing libraries in the current Blender file
        missing_libraries = []
        for lib in bpy.data.libraries:
            if processor._is_library_missing(lib):
                missing_libraries.append(lib)
                log_debug(f"Found missing library in Blender: {lib.name} (filepath: {lib.filepath})", module_name='MissingLinks')
        
        for lib_rel_path, lib_info in library_data.items():
            log_debug(f"Processing library path: {lib_rel_path}", module_name='MissingLinks')
            
            # Convert any sidecar path to its corresponding .blend path
            # This handles cases where the sidecar stores .side or .side.md paths instead of .blend paths
            converted_path = get_blend_file_path_from_sidecar(lib_rel_path)
              # If the path was converted, it was a sidecar reference, not a library reference
            if converted_path != lib_rel_path:
                log_debug(f"Converted sidecar reference {lib_rel_path} to {converted_path}", module_name='MissingLinks')
                lib_rel_path = converted_path
                
            lib_abs_path = PathResolver.resolve_relative_to_absolute(lib_rel_path, blend_dir)
            
            # Check if the library file exists at the expected path
            if not os.path.exists(lib_abs_path):
                # Try to find if the library exists elsewhere
                lib_name = os.path.basename(lib_abs_path)
                description = f"Missing library: {lib_name}"
                details = f"Expected at: {lib_rel_path}"
                _pending_relink_items.append(RelinkItem("library", description, details))
            else:
                # Even if the file exists, check if any Blender libraries are missing and could be relinked to this path
                lib_name = os.path.basename(lib_abs_path)
                for missing_lib in missing_libraries:
                    missing_lib_name = os.path.basename(missing_lib.name) if missing_lib.name else os.path.basename(missing_lib.filepath)
                    
                    # Handle Blender's automatic .001, .002 etc. suffixes
                    base_missing_name = missing_lib_name
                    if '.' in missing_lib_name:
                        # Remove potential numeric suffix (e.g., .001, .002)
                        parts = missing_lib_name.split('.')
                        if len(parts) > 1 and parts[-1].isdigit():
                            base_missing_name = '.'.join(parts[:-1])
                    
                    if missing_lib_name == lib_name or base_missing_name == lib_name:
                        description = f"Broken link: {missing_lib_name}"
                        details = f"Corrected path: {lib_rel_path}"
                        _pending_relink_items.append(RelinkItem("library", description, details))
                        log_debug(f"Added broken library link to relink items: {missing_lib_name} -> {lib_name}", module_name='MissingLinks')
                        break
                _check_resource_relinks(blend_path)
    except Exception as e:
        log_debug(f"Error checking library relinks: {e}", module_name='MissingLinks')


def _check_resource_relinks(blend_path: str):
    """Check if any resources need to be relinked."""
    try:
        processor = ResourceRelinkProcessor(blend_file_path=blend_path)
        sidecar_path = get_sidecar_path(blend_path)
        
        if not os.path.exists(sidecar_path):
            return
            
        parser = SidecarParser(sidecar_path)
        resource_data = parser.extract_json_blocks_with_links("Resources")
        
        if not resource_data:
            return
        
        blend_dir = os.path.dirname(blend_path)
        
        for resource_rel_path, resource_info in resource_data.items():
            resource_abs_path = PathResolver.resolve_relative_to_absolute(resource_rel_path, blend_dir)
            
            # Check if the resource file exists at the expected path
            if not os.path.exists(resource_abs_path):
                resource_name = os.path.basename(resource_abs_path)
                description = f"Missing resource: {resource_name}"
                details = f"Expected at: {resource_rel_path}"
                _pending_relink_items.append(RelinkItem("resource", description, details))
    except Exception as e:
        log_debug(f"Error checking resource relinks: {e}", module_name='MissingLinks')


def perform_all_relinks():
    """Perform all the relink operations."""
    blend_path = ensure_saved_file()
    if not blend_path:
        return
    
    log_info("Performing relink operations...", module_name='MissingLinks')
    # Store the initial count before any processing
    initial_count = len(_pending_relink_items)
    
    try:
        # Run asset relinking first (before library reloads)
        log_info("Running asset datablock relinking...", module_name='MissingLinks')
        from . import asset_relinker
        asset_processor = asset_relinker.AssetRelinkProcessor(main_blend_path=blend_path)
        asset_processor.process_relink()
          # Run library path relinking
        log_info("Running library path relinking...", module_name='MissingLinks')
        from .library_relinker import LibraryRelinkProcessor
        library_processor = LibraryRelinkProcessor(blend_file_path=blend_path)
        library_processor.process_relink()
          # Run resource relinking
        log_info("Running resource relinking...", module_name='MissingLinks')
        from .resource_relinker import ResourceRelinkProcessor
        resource_processor = ResourceRelinkProcessor(blend_file_path=blend_path)
        resource_processor.process_relink()
        
        # Force update the scene to refresh any dependencies
        log_info("Refreshing Blender scene state...", module_name='MissingLinks')
        try:
            bpy.context.view_layer.update()
            if hasattr(bpy.context.scene, 'update'):
                bpy.context.scene.update()
        except Exception as e:
            log_debug(f"Scene refresh failed (this is usually ok): {e}", module_name='MissingLinks')
          # Verify that relinks were successful by checking again
        log_info("Verifying relink results...", module_name='MissingLinks')
        remaining_items = check_missing_links()
        success_count = initial_count - len(remaining_items)
        
        if success_count > 0:
            log_success(f"Successfully relinked {success_count} out of {initial_count} items.", module_name='MissingLinks')
        else:
            log_warning(f"No items were successfully relinked out of {initial_count} items.", module_name='MissingLinks')
        if remaining_items:
            log_warning(f"{len(remaining_items)} items still require manual attention.", module_name='MissingLinks')
            for item in remaining_items:
                log_warning(f"  - {item.item_type}: {item.description}", module_name='MissingLinks')
        
        return success_count, initial_count
    except Exception as e:
        log_error(f"Error during relinking: {e}", module_name='MissingLinks')


def show_missing_links_dialog():
    """Show the missing links dialog if items need relinking."""
    global _missing_links_dialog_shown
    
    if _missing_links_dialog_shown:
        return
    
    relink_items = check_missing_links()
    
    if not relink_items:
        log_debug("No missing links found.", module_name='MissingLinks')
        return
    
    try:
        _missing_links_dialog_shown = True
        # Type: ignore to suppress Pylance warning about dynamic operator access
        bpy.ops.bv.missing_links_dialog('INVOKE_DEFAULT')  # type: ignore
    except Exception as e:
        _missing_links_dialog_shown = False
        log_error(f"Failed to invoke missing links dialog: {e}", module_name='MissingLinks')


class BV_OT_MissingLinksDialog(bpy.types.Operator):
    """Modal dialog to show missing links that require relinking."""
    bl_idname = "bv.missing_links_dialog"
    bl_label = "Relink Required"
    bl_options = {'REGISTER', 'INTERNAL', 'BLOCKING'}
    
    def execute(self, context):
        global _missing_links_dialog_shown
        _missing_links_dialog_shown = False
        
        try:
            success_count, total_count = perform_all_relinks()
            if success_count is not None and success_count > 0:
                if success_count == total_count:
                    # Use singular/plural appropriately
                    if total_count == 1:
                        self.report({'INFO'}, f"Successfully relinked 1 item")
                    else:
                        self.report({'INFO'}, f"Successfully relinked all {success_count} items")
                else:
                    # Use singular/plural appropriately
                    item_word = "item" if total_count == 1 else "items"
                    if success_count == 1:
                        self.report({'WARNING'}, f"Relinked 1 out of {total_count} {item_word}. Some items may need manual attention.")
                    else:
                        self.report({'WARNING'}, f"Relinked {success_count} out of {total_count} {item_word}. Some items may need manual attention.")
            else:
                self.report({'WARNING'}, f"No items were successfully relinked. Check console for details.")
                
        except Exception as e:
            self.report({'ERROR'}, f"Error during relinking: {e}")
            log_error(f"Error during missing links relinking: {e}", module_name='MissingLinks')
        
        # Force immediate UI refresh to update the properties panel
        for area in bpy.context.screen.areas:
            if area.type == 'VIEW_3D':
                area.tag_redraw()
        
        return {'FINISHED'}
    def cancel(self, context):
        global _missing_links_dialog_shown
        _missing_links_dialog_shown = False
        log_info("Missing links dialog dismissed. Items may need manual relinking.", module_name='MissingLinks')
        return {'CANCELLED'}
    
    def invoke(self, context, event):
        return context.window_manager.invoke_props_dialog(self, width=500)
    
    def draw(self, context):
        layout = self.layout
        
        # Warning icon and title
        row = layout.row()
        row.alert = True
        row.label(text="Items need to be relinked", icon='ERROR')
        
        # Show the items that need relinking
        if _pending_relink_items:
            box = layout.box()
            for item in _pending_relink_items[:10]:  # Limit to first 10 items
                row = box.row()
                
                # Icon based on type
                icon = 'ASSET_MANAGER' if item.item_type == 'asset' else \
                       'LIBRARY_DATA_DIRECT' if item.item_type == 'library' else \
                       'FILE_IMAGE'
                
                col = row.column()
                col.label(text=item.description, icon=icon)
                
                # Show details if available
                if item.details:
                    sub_row = col.row()
                    sub_row.scale_y = 0.8
                    sub_row.label(text=item.details)
            
            if len(_pending_relink_items) > 10:
                row = layout.row()
                row.label(text=f"... and {len(_pending_relink_items) - 10} more items")
        
        layout.separator()
        layout.label(text="Continue with automatic relinking?")


class BV_PT_MissingLinksPanel(bpy.types.Panel):
    """Panel to manually trigger missing links dialog for testing."""
    bl_label = "Missing Links"
    bl_idname = "BV_PT_missing_links"
    bl_space_type = 'VIEW_3D'
    bl_region_type = 'UI'
    bl_category = "Blend Vault"
    def draw(self, context):
        layout = self.layout
        
        if _pending_relink_items:
            # Warning icon and title
            row = layout.row()
            row.alert = True
            row.label(text="Relinks required", icon='ERROR')
            
            layout.separator()
            
            row = layout.row()
            row.label(text=f"{len(_pending_relink_items)} items need relinking.")
            
            # Show first few items in a box
            box = layout.box()
            for item in _pending_relink_items[:3]:  # Show first 3 items
                row = box.row()
                # Icon based on type
                icon = 'ASSET_MANAGER' if item.item_type == 'asset' else \
                       'LIBRARY_DATA_DIRECT' if item.item_type == 'library' else \
                       'FILE_IMAGE'
                row.label(text=item.description, icon=icon)
            
            if len(_pending_relink_items) > 3:
                box.label(text=f"... and {len(_pending_relink_items) - 3} more")
            
            layout.separator()
            
            # Action button
            col = layout.column(align=True)
            col.scale_y = 1.2
            col.operator("bv.missing_links_dialog", text="Fix Relinks", icon='FILE_TICK')
        else:
            # No relinks needed
            layout.label(text="No pending relinks", icon='CHECKMARK')
            layout.separator()
            layout.operator("bv.missing_links_dialog", text="Check for Relinks", icon='LINK_BLEND')


def register():
    """Register the missing links dialog."""
    bpy.utils.register_class(BV_OT_MissingLinksDialog)
    bpy.utils.register_class(BV_PT_MissingLinksPanel)
    log_success("Missing links dialog registered.", module_name='MissingLinks')


def unregister():
    """Unregister the missing links dialog."""
    bpy.utils.unregister_class(BV_OT_MissingLinksDialog)
    bpy.utils.unregister_class(BV_PT_MissingLinksPanel)
    log_success("Missing links dialog unregistered.", module_name='MissingLinks')
