"""
Asset discovery utilities for automatically finding and linking/appending assets.
Provides quick operations that find the first asset in a .blend file.
"""

import bpy  # type: ignore
import os
from .. import log_debug


def _find_first_asset_in_blend_file(file_path: str):
    """Find the first marked asset datablock in a .blend file."""
    try:
        log_debug(f"[Blend Vault][Asset Discovery] Starting marked asset search in {os.path.basename(file_path)}")
        
        # Try to find actual asset-marked items
        actual_asset = _find_actual_assets_in_blend_file(file_path)
        if actual_asset:
            log_debug(f"[Blend Vault][Asset Discovery] ✓ Found marked asset: {actual_asset['name']} ({actual_asset['type']})")
            return actual_asset
        
        log_debug("[Blend Vault][Asset Discovery] ✗ No marked assets found")
        return None
    except Exception as e:
        log_debug(f"[Blend Vault][Asset Discovery] Error scanning blend file {file_path}: {e}")
        return None


def _find_actual_assets_in_blend_file(file_path: str):
    """PRIORITY METHOD: Find actual asset-marked datablocks using Blender's asset system."""
    try:
        log_debug(f"[Blend Vault][Asset Discovery] PRIORITY: Scanning for marked assets in {os.path.basename(file_path)}")
        
        # Simple approach: temporarily link items to check asset_data, then remove them
        found_assets = []
        
        with bpy.data.libraries.load(file_path, link=True, relative=False) as (data_from, data_to):
            asset_types_priority = ["Collection", "Object", "Material", "World", "NodeTree", "Brush", "Action"]
            
            for asset_type in asset_types_priority:
                collection_name = asset_type.lower() + "s"
                if collection_name == "brushs":
                    collection_name = "brushes"
                elif collection_name == "node_trees":
                    collection_name = "node_groups"
                
                if hasattr(data_from, collection_name):
                    items = getattr(data_from, collection_name)
                    if items:
                        log_debug(f"[Blend Vault][Asset Discovery] PRIORITY: Found {len(items)} {asset_type}(s), linking to check for asset metadata")
                        # Link all items to check them
                        setattr(data_to, collection_name, items[:])
        
        # Now check the linked items for asset_data
        linked_libraries = [lib for lib in bpy.data.libraries if lib.filepath == file_path]
        
        if linked_libraries:
            library = linked_libraries[0]
            
            # Check each datablock type for assets
            for asset_type in ["Collection", "Object", "Material", "World", "NodeTree", "Brush", "Action"]:
                collection_name = asset_type.lower() + "s"
                if collection_name == "brushs":
                    collection_name = "brushes"
                elif collection_name == "node_trees":
                    collection_name = "node_groups"
                
                if hasattr(bpy.data, collection_name):
                    collection = getattr(bpy.data, collection_name)
                    for item in collection:
                        # Check if this item is from our library and has asset_data
                        if (hasattr(item, 'library') and item.library == library and
                            hasattr(item, 'asset_data') and item.asset_data is not None):
                            
                            log_debug(f"[Blend Vault][Asset Discovery] PRIORITY: ✓ Found marked asset: {item.name} ({asset_type})")
                            found_assets.append({
                                "name": item.name,
                                "type": asset_type,
                                "directory": f"{file_path}\\{asset_type}\\",
                                "filename": item.name
                            })
            
            # Clean up: remove the linked library
            try:
                bpy.data.libraries.remove(library)
                log_debug(f"[Blend Vault][Asset Discovery] PRIORITY: Cleaned up temporary library link")
            except Exception as cleanup_error:
                log_debug(f"[Blend Vault][Asset Discovery] PRIORITY: Library cleanup warning: {cleanup_error}")
        
        if found_assets:
            # Return the first asset found (prioritized by the order we checked)
            selected_asset = found_assets[0]
            log_debug(f"[Blend Vault][Asset Discovery] PRIORITY: Selected marked asset: {selected_asset['name']} ({selected_asset['type']})")
            return selected_asset
        
        log_debug(f"[Blend Vault][Asset Discovery] PRIORITY: No marked assets found in {os.path.basename(file_path)}")
        return None
        
    except Exception as e:
        log_debug(f"[Blend Vault][Asset Discovery] PRIORITY: Asset detection failed for {os.path.basename(file_path)}: {e}")
        return None


class BV_OT_LinkFirstAsset(bpy.types.Operator):
    """Link the first marked asset found in a Library .blend file, or open manual dialogue."""
    bl_idname = "blend_vault.link_first_asset"
    bl_label = "Quick Link"
    bl_options = {'REGISTER', 'INTERNAL'}

    file_path: bpy.props.StringProperty()  # type: ignore

    def execute(self, context):
        if not self.file_path or not os.path.isfile(self.file_path):
            self.report({'ERROR'}, "File path is invalid or not set for linking.")
            return {'CANCELLED'}
        # Find the first marked asset in the file
        asset_info = _find_first_asset_in_blend_file(self.file_path)
        if not asset_info:
            # No marked assets found, fall back to manual file dialogue
            self.report({'WARNING'}, f"No marked assets found in {os.path.basename(self.file_path)}, opening standard library selector.")
            log_debug(f"[Blend Vault][Asset Discovery] No marked assets found, opening manual link dialogue for {os.path.basename(self.file_path)}")
            try:
                bpy.ops.wm.append('INVOKE_DEFAULT', filepath=self.file_path, link=True)
                return {'FINISHED'}
            except Exception as e:
                self.report({'ERROR'}, f"Failed to open link dialogue: {e}")
                return {'CANCELLED'}
        
        try:
            # Link the specific marked asset directly
            bpy.ops.wm.append(
                filepath=f"{self.file_path}\\{asset_info['type']}\\{asset_info['name']}",
                directory=asset_info['directory'],
                filename=asset_info['filename'],
                instance_collections=True,
                link=True
            )
            self.report({'INFO'}, f"Linked {asset_info['type']}: {asset_info['name']}")
        except Exception as e:
            self.report({'ERROR'}, f"Failed to link asset: {e}")
            return {'CANCELLED'}
        
        return {'FINISHED'}


class BV_OT_AppendFirstAsset(bpy.types.Operator):
    """Append the first marked asset found in a Library .blend file, or open manual dialogue."""
    bl_idname = "blend_vault.append_first_asset"
    bl_label = "Quick Append"
    bl_options = {'REGISTER', 'INTERNAL'}

    file_path: bpy.props.StringProperty()  # type: ignore

    def execute(self, context):
        if not self.file_path or not os.path.isfile(self.file_path):
            self.report({'ERROR'}, "File path is invalid or not set for appending.")
            return {'CANCELLED'}
          # Find the first marked asset in the file
        asset_info = _find_first_asset_in_blend_file(self.file_path)
        if not asset_info:
            # No marked assets found, fall back to manual file dialogue
            self.report({'WARNING'}, f"No marked assets found in {os.path.basename(self.file_path)}, opening standard library selector.")
            log_debug(f"[Blend Vault][Asset Discovery] No marked assets found, opening manual append dialogue for {os.path.basename(self.file_path)}")
            try:
                bpy.ops.wm.append('INVOKE_DEFAULT', filepath=self.file_path, link=False)
                return {'FINISHED'}
            except Exception as e:
                self.report({'ERROR'}, f"Failed to open append dialogue: {e}")
                return {'CANCELLED'}
        
        try:
            # Append the specific marked asset directly
            bpy.ops.wm.append(
                filepath=f"{self.file_path}\\{asset_info['type']}\\{asset_info['name']}",
                directory=asset_info['directory'],
                filename=asset_info['filename'],
                use_recursive=True,
                link=False
            )
            self.report({'INFO'}, f"Appended {asset_info['type']}: {asset_info['name']}")
        except Exception as e:
            self.report({'ERROR'}, f"Failed to append asset: {e}")
            return {'CANCELLED'}
        
        return {'FINISHED'}


def register():
    """Register asset discovery operators."""
    bpy.utils.register_class(BV_OT_LinkFirstAsset)
    bpy.utils.register_class(BV_OT_AppendFirstAsset)


def unregister():
    """Unregister asset discovery operators."""
    bpy.utils.unregister_class(BV_OT_LinkFirstAsset)
    bpy.utils.unregister_class(BV_OT_AppendFirstAsset)
