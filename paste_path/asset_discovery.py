"""
Asset discovery utilities for automatically finding and linking/appending assets.
Provides quick operations that find the first asset in a .blend file.
"""

import bpy  # type: ignore
import os


def _find_first_asset_in_blend_file(file_path: str):
    """Find the first asset datablock in a .blend file, prioritizing collections."""
    try:
        # Priority order: Collections first, then other asset types
        asset_types_priority = ["Collection", "Object", "Material", "World", "NodeTree", "Brush", "Action", "Scene"]
        
        with bpy.data.libraries.load(file_path, link=False, relative=False) as (data_from, data_to):
            # Check each asset type in priority order
            for asset_type in asset_types_priority:
                collection_name = asset_type.lower() + "s"  # Convert to collection name (e.g., "Collection" -> "collections")
                if collection_name == "brushs":  # Handle irregular plural
                    collection_name = "brushes"
                elif collection_name == "node_trees":  # Handle NodeTree special case
                    collection_name = "node_groups"
                
                # Get the collection from data_from
                if hasattr(data_from, collection_name):
                    items = getattr(data_from, collection_name)
                    if items:
                        # Return the first item found
                        return {
                            "name": items[0],
                            "type": asset_type,
                            "directory": f"{file_path}\\{asset_type}\\",
                            "filename": items[0]
                        }
        
        return None
    except Exception as e:
        print(f"Error scanning blend file {file_path}: {e}")
        return None


class BV_OT_LinkFirstAsset(bpy.types.Operator):
    """Link the first asset found in a Library .blend file (prioritizing collections)."""
    bl_idname = "blend_vault.link_first_asset"
    bl_label = "Quick Link"
    bl_options = {'REGISTER', 'INTERNAL'}

    file_path: bpy.props.StringProperty()  # type: ignore

    def execute(self, context):
        if not self.file_path or not os.path.isfile(self.file_path):
            self.report({'ERROR'}, "File path is invalid or not set for linking.")
            return {'CANCELLED'}
        
        # Find the first asset in the file
        asset_info = _find_first_asset_in_blend_file(self.file_path)
        if not asset_info:
            self.report({'WARNING'}, f"No assets found in {os.path.basename(self.file_path)}")
            return {'CANCELLED'}
        
        try:
            # Link the specific asset directly
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
    """Append the first asset found in a Library .blend file (prioritizing collections)."""
    bl_idname = "blend_vault.append_first_asset"
    bl_label = "Quick Append"
    bl_options = {'REGISTER', 'INTERNAL'}

    file_path: bpy.props.StringProperty()  # type: ignore

    def execute(self, context):
        if not self.file_path or not os.path.isfile(self.file_path):
            self.report({'ERROR'}, "File path is invalid or not set for appending.")
            return {'CANCELLED'}
        
        # Find the first asset in the file
        asset_info = _find_first_asset_in_blend_file(self.file_path)
        if not asset_info:
            self.report({'WARNING'}, f"No assets found in {os.path.basename(self.file_path)}")
            return {'CANCELLED'}
        
        try:
            # Append the specific asset directly
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
