"""
Core operators for file operations: Open, Link, Append.
These operators handle the basic file operations with proper error handling.
"""

import bpy  # type: ignore
import os


class BV_OT_ProcessOpenAction(bpy.types.Operator):
    """Open the file, checking for unsaved changes in the current file first."""
    bl_idname = "blend_vault.process_open_action"
    bl_label = "Process Open Action"
    bl_options = {'REGISTER', 'INTERNAL'}

    file_path: bpy.props.StringProperty()  # type: ignore

    def execute(self, context):
        if bpy.data.is_dirty:
            bpy.ops.blend_vault.confirm_save_before_open('INVOKE_DEFAULT', file_path=self.file_path)
        else:
            try:
                bpy.ops.wm.open_mainfile(filepath=self.file_path)
                self.report({'INFO'}, f"Opened: {os.path.basename(self.file_path)}")
            except Exception as e:
                self.report({'ERROR'}, f"Failed to open file: {e}")
                return {'CANCELLED'}
        return {'FINISHED'}


class BV_OT_LinkFromFile(bpy.types.Operator):
    """Link from a Library .blend file."""
    bl_idname = "blend_vault.link_from_file"
    bl_label = "Link From File"
    bl_options = {'REGISTER', 'INTERNAL'}

    file_path: bpy.props.StringProperty()  # type: ignore

    def execute(self, context):
        if not self.file_path or not os.path.isfile(self.file_path):
            self.report({'ERROR'}, "File path is invalid or not set for linking.")
            return {'CANCELLED'}
        try:
            bpy.ops.wm.append('INVOKE_DEFAULT', filepath=self.file_path, link=True)
        except Exception as e:
            self.report({'ERROR'}, f"Failed to invoke link operation: {e}")
            return {'CANCELLED'}
        return {'FINISHED'}


class BV_OT_AppendFromFile(bpy.types.Operator):
    """Append from a Library .blend file."""
    bl_idname = "blend_vault.append_from_file"
    bl_label = "Append From File"
    bl_options = {'REGISTER', 'INTERNAL'}

    file_path: bpy.props.StringProperty()  # type: ignore

    def execute(self, context):
        if not self.file_path or not os.path.isfile(self.file_path):
            self.report({'ERROR'}, "File path is invalid or not set for appending.")
            return {'CANCELLED'}
        try:
            bpy.ops.wm.append('INVOKE_DEFAULT', filepath=self.file_path, link=False)
        except Exception as e:
            self.report({'ERROR'}, f"Failed to invoke append operation: {e}")
            return {'CANCELLED'}
        return {'FINISHED'}


class BV_OT_OpenFileWithoutSave(bpy.types.Operator):
    """Open new file without saving current file"""
    bl_idname = "blend_vault.open_file_without_save"
    bl_label = "Open Without Save"
    bl_options = {'REGISTER', 'INTERNAL'}

    file_path: bpy.props.StringProperty()  # type: ignore

    def execute(self, context):
        try:
            bpy.ops.wm.open_mainfile(filepath=self.file_path)
            self.report({'INFO'}, f"Opened: {os.path.basename(self.file_path)}")
        except Exception as e:
            self.report({'ERROR'}, f"Failed to open file: {e}")
            return {'CANCELLED'}
        
        return {'FINISHED'}


def register():
    """Register core operators."""
    bpy.utils.register_class(BV_OT_ProcessOpenAction)
    bpy.utils.register_class(BV_OT_LinkFromFile)
    bpy.utils.register_class(BV_OT_AppendFromFile)
    bpy.utils.register_class(BV_OT_OpenFileWithoutSave)


def unregister():
    """Unregister core operators."""
    bpy.utils.unregister_class(BV_OT_ProcessOpenAction)
    bpy.utils.unregister_class(BV_OT_LinkFromFile)
    bpy.utils.unregister_class(BV_OT_AppendFromFile)
    bpy.utils.unregister_class(BV_OT_OpenFileWithoutSave)
