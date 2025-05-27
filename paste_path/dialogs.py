"""
Dialog operators for user interaction.
Provides popup menus for choosing actions and confirming save operations.
"""

import bpy  # type: ignore
import os


class BV_OT_ChooseActionBeforeOpen(bpy.types.Operator):
    """Modal operator to choose action (Open, Link, Append) before opening/linking a new file."""
    bl_idname = "blend_vault.choose_action_before_open"
    bl_label = ""  # Title will be set in popup_menu
    bl_description = "Choose action for the selected .blend file"
    bl_options = {'REGISTER', 'INTERNAL'}

    file_path: bpy.props.StringProperty()  # type: ignore

    def _draw_action_dialog_content(self, menu_self, context):
        layout = menu_self.layout

        # Primary action
        open_op = layout.operator("blend_vault.process_open_action", text="Open", icon='FILE_FOLDER')
        open_op.file_path = self.file_path

        layout.separator()

        # Standard link/append operations (opens file browser)
        link_op = layout.operator("blend_vault.link_from_file", text="Link...", icon='LINKED')
        link_op.file_path = self.file_path

        append_op = layout.operator("blend_vault.append_from_file", text="Append...", icon='APPEND_BLEND')
        append_op.file_path = self.file_path

        layout.separator()

        # Quick operations (automatically finds first asset)
        link_first_op = layout.operator("blend_vault.link_first_asset", text="Quick Link", icon='LINKED')
        link_first_op.file_path = self.file_path

        append_first_op = layout.operator("blend_vault.append_first_asset", text="Quick Append", icon='APPEND_BLEND')
        append_first_op.file_path = self.file_path

    def invoke(self, context, event):
        filename = os.path.basename(self.file_path)
        context.window_manager.popup_menu(
            lambda menu_s, ctx: self._draw_action_dialog_content(menu_s, ctx),
            title=filename,
            icon='QUESTION'
        )
        return {'FINISHED'}

    def execute(self, context):
        # This operator should be invoked as a popup.
        self.report({'WARNING'}, "Dialog operator executed directly; invoking popup instead.")
        # Fallback to ensure it still tries to show the popup if called directly, though INVOKE_DEFAULT is preferred.
        return self.invoke(context, None)


class BV_OT_ConfirmSaveBeforeOpen(bpy.types.Operator):
    """Modal operator to confirm save before opening new file"""
    bl_idname = "blend_vault.confirm_save_before_open"
    bl_label = ""
    bl_description = "Confirm save before opening new file"
    bl_options = {'REGISTER', 'INTERNAL'}

    file_path: bpy.props.StringProperty()  # type: ignore

    def _draw_confirmation_dialog_content(self, menu_self, context):
        layout = menu_self.layout
        layout.label(text=f"Save changes before opening {os.path.basename(self.file_path)}?", icon='QUESTION')
        
        layout.separator()
        
        save_op = layout.operator("blend_vault.save_and_open_file", text="Save", icon='FILE_TICK')
        save_op.file_path = self.file_path
        
        dont_save_op = layout.operator("blend_vault.open_file_without_save", text="Don't Save")
        dont_save_op.file_path = self.file_path

    def invoke(self, context, event):
        context.window_manager.popup_menu(
            lambda menu_s, ctx: self._draw_confirmation_dialog_content(menu_s, ctx),
            title=self.bl_label, 
            icon='QUESTION'
        )
        return {'FINISHED'}

    def execute(self, context):
        self.report({'WARNING'}, "Dialog operator executed directly; invoking popup instead.")
        return {'CANCELLED'}


def register():
    """Register dialog operators."""
    bpy.utils.register_class(BV_OT_ChooseActionBeforeOpen)
    bpy.utils.register_class(BV_OT_ConfirmSaveBeforeOpen)


def unregister():
    """Unregister dialog operators."""
    bpy.utils.unregister_class(BV_OT_ChooseActionBeforeOpen)
    bpy.utils.unregister_class(BV_OT_ConfirmSaveBeforeOpen)
