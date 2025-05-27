"""
Smart paste functionality and main entry points.
Provides the main SmartPasteOperator and clipboard monitoring.
"""

import bpy  # type: ignore
from .file_validation import is_valid_blend_or_sidecar_path, normalize_path_from_clipboard


class SmartPasteOperator(bpy.types.Operator):
    bl_idname = "wm.smart_paste"
    bl_label = "Smart Paste"
    bl_description = "Paste with .blend file interception, falls back to default paste"
    
    def execute(self, context):
        try:
            clipboard_text = context.window_manager.clipboard
            if is_valid_blend_or_sidecar_path(clipboard_text):
                path = normalize_path_from_clipboard(clipboard_text)
                
                # Always show the new choose action dialog
                bpy.ops.blend_vault.choose_action_before_open('INVOKE_DEFAULT', file_path=path)
                return {'FINISHED'}
        except Exception as e:
            # Log or report error if needed, e.g., self.report({'ERROR'}, f"SmartPaste error: {e}")
            pass  # Fall through to default paste
        
        # Fallback to default paste behavior
        try:
            if context.space_data and context.space_data.type == 'VIEW_3D':
                return bpy.ops.view3d.pastebuffer('INVOKE_DEFAULT')
            # Attempt to find a generic paste operator if not in 3D View
            # This part might need more robust handling for different contexts
            elif hasattr(bpy.ops.ui, 'paste'):  # Check for generic UI paste
                 return bpy.ops.ui.paste('INVOKE_DEFAULT')
            elif hasattr(bpy.ops.text, 'paste'):  # Check for text editor paste
                 return bpy.ops.text.paste('INVOKE_DEFAULT')
            else:
                self.report({'WARNING'}, "No valid paste operation for this context")
                return {'CANCELLED'}
        except Exception as e:
            self.report({'WARNING'}, f"Default paste operation failed: {e}")
            return {'CANCELLED'}


class OpenBlendFromClipboardOperator(bpy.types.Operator):
    bl_idname = "wm.open_blend_from_clipboard"
    bl_label = "Open Blend File from Clipboard"
    bl_description = "Opens the .blend file at the path in the clipboard"

    def execute(self, context):
        try:
            clipboard_text = context.window_manager.clipboard
            if not is_valid_blend_or_sidecar_path(clipboard_text):
                self.report({'ERROR'}, f"Not a valid .blend file: {clipboard_text}")
                return {'CANCELLED'}
            
            path = normalize_path_from_clipboard(clipboard_text)
        except Exception as e:
            self.report({'ERROR'}, f"Clipboard error: {e}")
            return {'CANCELLED'}

        # Always show the new choose action dialog
        bpy.ops.blend_vault.choose_action_before_open('INVOKE_DEFAULT', file_path=path)
        return {'FINISHED'}


def register():
    """Register smart paste operators and keybindings."""
    bpy.utils.register_class(OpenBlendFromClipboardOperator)
    bpy.utils.register_class(SmartPasteOperator)
    
    # Register keybindings
    wm = bpy.context.window_manager
    kc = wm.keyconfigs.addon
    if kc:
        km = kc.keymaps.new(name='3D View', space_type='VIEW_3D')
        kmi = km.keymap_items.new(SmartPasteOperator.bl_idname, 'V', 'PRESS', ctrl=True)
        
        km_window = kc.keymaps.new(name='Window', space_type='EMPTY')
        kmi_window = km_window.keymap_items.new(SmartPasteOperator.bl_idname, 'V', 'PRESS', ctrl=True)


def unregister():
    """Unregister smart paste operators and keybindings."""
    # Remove keybindings
    wm = bpy.context.window_manager
    kc = wm.keyconfigs.addon
    if kc:
        km = kc.keymaps.get('3D View')
        if km:
            for kmi in km.keymap_items:
                if kmi.idname == SmartPasteOperator.bl_idname:
                    km.keymap_items.remove(kmi)
        
        km_window = kc.keymaps.get('Window')
        if km_window:
            for kmi in km_window.keymap_items:
                if kmi.idname == SmartPasteOperator.bl_idname:
                    km_window.keymap_items.remove(kmi)
    
    # Unregister classes
    try:
        bpy.utils.unregister_class(SmartPasteOperator)
    except:
        pass
    
    try:
        bpy.utils.unregister_class(OpenBlendFromClipboardOperator)
    except:
        pass
