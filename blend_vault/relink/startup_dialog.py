"""
Startup relink dialog for Blend Vault.
Shows a modal dialog when items need to be relinked on file load.
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
_startup_dialog_shown = False

# Store detected relink items for the dialog
_pending_relink_items = []


class RelinkItem:
    """Represents an item that needs to be relinked."""
    def __init__(self, item_type: str, description: str, details: str = ""):
        self.item_type = item_type  # "asset", "library", "resource"
        self.description = description
        self.details = details


def check_startup_relinks():
    """
    Check what items need to be relinked on startup.
    Returns a list of RelinkItem objects.
    """
    global _pending_relink_items
    _pending_relink_items.clear()
    
    blend_path = ensure_saved_file()
    if not blend_path:
        return []
    
    log_info("Checking for startup relink requirements...", module_name='StartupRelink')
    
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
                log_debug(f"Error checking library sidecar {lib_sidecar_path}: {e}", module_name='StartupRelink')
                
    except Exception as e:
        log_debug(f"Error checking asset relinks: {e}", module_name='StartupRelink')


def _check_library_relinks(blend_path: str):
    """Check if any library paths need to be relinked."""
    try:
        processor = LibraryRelinkProcessor(blend_path)
        sidecar_path = get_sidecar_path(blend_path)
        
        if not os.path.exists(sidecar_path):
            return
        parser = SidecarParser(sidecar_path)
        library_data = parser.extract_json_blocks_with_links("Linked Libraries")
        
        if not library_data:
            return
        # Debug: Log what library data was found
        log_debug(f"Found library data: {list(library_data.keys())}", module_name='StartupRelink')
        
        blend_dir = os.path.dirname(blend_path)
        for lib_rel_path, lib_info in library_data.items():
            log_debug(f"Processing library path: {lib_rel_path}", module_name='StartupRelink')
            
            # Convert any sidecar path to its corresponding .blend path
            # This handles cases where the sidecar stores .side or .side.md paths instead of .blend paths
            converted_path = get_blend_file_path_from_sidecar(lib_rel_path)
            
            # If the path was converted, it was a sidecar reference, not a library reference
            if converted_path != lib_rel_path:
                log_debug(f"Converted sidecar reference {lib_rel_path} to {converted_path}", module_name='StartupRelink')
                lib_rel_path = converted_path
                
            lib_abs_path = PathResolver.resolve_relative_to_absolute(lib_rel_path, blend_dir)
            
            # Check if the library file exists at the expected path
            if not os.path.exists(lib_abs_path):
                # Try to find if the library exists elsewhere
                lib_name = os.path.basename(lib_abs_path)
                description = f"Missing library: {lib_name}"
                details = f"Expected at: {lib_rel_path}"
                _pending_relink_items.append(RelinkItem("library", description, details))
                
    except Exception as e:
        log_debug(f"Error checking library relinks: {e}", module_name='StartupRelink')


def _check_resource_relinks(blend_path: str):
    """Check if any resources need to be relinked."""
    try:
        processor = ResourceRelinkProcessor(blend_path)
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
        log_debug(f"Error checking resource relinks: {e}", module_name='StartupRelink')


def perform_all_relinks():
    """Perform all the relink operations."""
    blend_path = ensure_saved_file()
    if not blend_path:
        return
    
    log_info("Performing startup relink operations...", module_name='StartupRelink')
    
    try:
        # Run asset relinking first (before library reloads)
        log_info("Running asset datablock relinking...", module_name='StartupRelink')
        asset_relinker.relink_renamed_assets()
        
        # Run library path relinking
        log_info("Running library path relinking...", module_name='StartupRelink')
        from .library_relinker import relink_library_info
        relink_library_info()
        
        # Run resource relinking
        log_info("Running resource relinking...", module_name='StartupRelink')
        from .resource_relinker import relink_resources
        relink_resources()
        
        log_success("All startup relink operations completed.", module_name='StartupRelink')
        
    except Exception as e:
        log_error(f"Error during startup relinking: {e}", module_name='StartupRelink')


def show_startup_relink_dialog():
    """Show the startup relink dialog if items need relinking."""
    global _startup_dialog_shown
    
    if _startup_dialog_shown:
        return
    
    relink_items = check_startup_relinks()
    
    if not relink_items:
        log_debug("No startup relink items found.", module_name='StartupRelink')
        return
    
    try:
        _startup_dialog_shown = True
        # Type: ignore to suppress Pylance warning about dynamic operator access
        bpy.ops.blend_vault.startup_relink_dialog('INVOKE_DEFAULT')  # type: ignore
    except Exception as e:
        _startup_dialog_shown = False
        log_error(f"Failed to invoke startup relink dialog: {e}", module_name='StartupRelink')


class BV_OT_StartupRelinkDialog(bpy.types.Operator):
    """Modal dialog to show startup relink requirements."""
    bl_idname = "blend_vault.startup_relink_dialog"
    bl_label = "Relink Required"
    bl_options = {'REGISTER', 'INTERNAL', 'BLOCKING'}

    def execute(self, context):
        global _startup_dialog_shown
        _startup_dialog_shown = False
        
        try:
            perform_all_relinks()
            self.report({'INFO'}, f"Relinked {len(_pending_relink_items)} items")
        except Exception as e:
            self.report({'ERROR'}, f"Error during relinking: {e}")
            log_error(f"Error during startup relinking: {e}", module_name='StartupRelink')
        
        return {'FINISHED'}

    def cancel(self, context):
        global _startup_dialog_shown
        _startup_dialog_shown = False
        log_info("Startup relink dialog dismissed. Items may need manual relinking.", module_name='StartupRelink')
        return {'CANCELLED'}
    
    def invoke(self, context, event):
        return context.window_manager.invoke_props_dialog(self, width=500)

    def draw(self, context):
        layout = self.layout
        
        # Header
        layout.label(text="Items need to be relinked:", icon='INFO')
        layout.separator()
        
        # Show the items that need relinking
        if _pending_relink_items:
            box = layout.box()
            for item in _pending_relink_items[:10]:  # Limit to first 10 items
                row = box.row()
                
                # Icon based on type
                icon = 'ASSET_MANAGER' if item.item_type == 'asset' else \
                       'LIBRARY_DATA_DIRECT' if item.item_type == 'library' else \
                       'FILE_IMAGE'
                
                row.label(text=item.description, icon=icon)
                
                # Show details if available
                if item.details:
                    sub_row = box.row()
                    sub_row.scale_y = 0.8
                    sub_row.label(text=f"    {item.details}")
            
            if len(_pending_relink_items) > 10:
                layout.label(text=f"... and {len(_pending_relink_items) - 10} more items")
        
        layout.separator()
        layout.label(text="Continue with automatic relinking?")


class BV_PT_StartupRelinkPanel(bpy.types.Panel):
    """Panel to manually trigger startup relink dialog for testing."""
    bl_label = "Startup Relink"
    bl_idname = "BV_PT_startup_relink"
    bl_space_type = 'VIEW_3D'
    bl_region_type = 'UI'
    bl_category = "Blend Vault"

    def draw(self, context):
        layout = self.layout
        
        layout.label(text="Manual relink check:")
        layout.operator("blend_vault.startup_relink_dialog", text="Check for Relinks", icon='LINK_BLEND')
        
        if _pending_relink_items:
            layout.separator()
            layout.label(text=f"{len(_pending_relink_items)} items need relinking", icon='ERROR')


def register():
    """Register the startup relink dialog."""
    bpy.utils.register_class(BV_OT_StartupRelinkDialog)
    bpy.utils.register_class(BV_PT_StartupRelinkPanel)
    log_success("Startup relink dialog registered.", module_name='StartupRelink')


def unregister():
    """Unregister the startup relink dialog."""
    bpy.utils.unregister_class(BV_OT_StartupRelinkDialog)
    bpy.utils.unregister_class(BV_PT_StartupRelinkPanel)
    log_success("Startup relink dialog unregistered.", module_name='StartupRelink')
