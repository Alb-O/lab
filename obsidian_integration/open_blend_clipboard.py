import bpy # type: ignore
import os
import functools # Added import
import preferences  # Use absolute import for Blender add-on compatibility
from utils import SIDECAR_EXTENSION

class BV_OT_ConfirmSaveBeforeOpen(bpy.types.Operator):
    """Modal operator to confirm save before opening new file"""
    bl_idname = "blend_vault.confirm_save_before_open"
    bl_label = ""
    bl_description = "Confirm save before opening new file"
    bl_options = {'REGISTER', 'INTERNAL'}

    file_path: bpy.props.StringProperty()  # type: ignore

    def _draw_confirmation_dialog_content(self, menu_self, context):
        layout = menu_self.layout
        layout.label(text="Save changes before opening", icon='QUESTION')

        # Get vault root from preferences
        vault_root = preferences.get_obsidian_vault_root(context)
        display_name = os.path.basename(self.file_path)
        if vault_root:
            try:
                abs_vault = os.path.abspath(os.path.normpath(vault_root))
                abs_file = os.path.abspath(os.path.normpath(self.file_path))
                if abs_file.lower().startswith(abs_vault.lower() + os.sep.lower()):
                    rel_path = os.path.relpath(abs_file, abs_vault)
                    display_name = rel_path  # Only show the path relative to the vault root
            except Exception:
                pass
        layout.label(text=f"{display_name}?")
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

class BV_OT_SaveAndOpenFile(bpy.types.Operator):
    """Save current file then open new file"""
    bl_idname = "blend_vault.save_and_open_file"
    bl_label = "Save and Open"
    bl_options = {'REGISTER', 'INTERNAL'}

    file_path: bpy.props.StringProperty()  # type: ignore

    def execute(self, context):
        try:
            # Always set the target file path for the post_save_handler
            # This key will be consumed by blend_vault_post_save_handler
            context.window_manager['blend_vault_open_after_save'] = self.file_path
            
            if bpy.data.is_saved: # File has a path, might be dirty (e.g. existing file with unsaved changes)
                bpy.ops.wm.save_mainfile()
                # The save_post_handler will now pick this up and schedule the open.
            else: # File is new or not yet saved (no path)
                bpy.ops.wm.save_as_mainfile('INVOKE_DEFAULT')
            
            return {'FINISHED'}
        except Exception as e:
            # Clean up the key if an error occurs before save op is invoked or if setup fails
            if 'blend_vault_open_after_save' in context.window_manager:
                del context.window_manager['blend_vault_open_after_save']
            self.report({'ERROR'}, f"Failed to initiate save and open: {e}")
            return {'CANCELLED'}

# New helper function to be called by the timer
def _actual_open_file_operation(file_path):
    try:
        print(f"[Blend Vault] Timer executing: Attempting to open {file_path}")
        bpy.ops.wm.open_mainfile(filepath=file_path)
        print(f"[Blend Vault] Successfully initiated open for: {os.path.basename(file_path)}")
    except Exception as e:
        print(f"[Blend Vault] Timer failed to open file '{file_path}': {e}")
    return None # Returning None unregisters the timer after it runs once

# Add a handler to check after saving if a file should be opened
@bpy.app.handlers.persistent
def blend_vault_post_save_handler(dummy):
    wm = bpy.context.window_manager
    key = 'blend_vault_open_after_save'
    if key in wm:
        file_path_to_open = wm[key]
        # It's crucial to remove the key immediately after retrieving it
        # to prevent re-processing or issues with stale data.
        del wm[key] 
        try:
            if bpy.data.is_saved:  # Double check the file is indeed saved
                print(f"[Blend Vault] Post-save: File is saved. Scheduling open for {file_path_to_open}")
                
                # Use functools.partial to pass the file_path to the timer callback
                open_action = functools.partial(_actual_open_file_operation, file_path_to_open)
                
                # Register the open_action to run after a short delay (e.g., 0.1 seconds)
                # This delay might help Blender stabilize before opening the new file.
                if not bpy.app.timers.is_registered(open_action): # Avoid duplicate timers if somehow possible
                    bpy.app.timers.register(open_action, first_interval=0.1)
            else:
                # This case (file not saved after a save operation) should be rare but is logged.
                current_file_display = bpy.data.filepath if bpy.data.filepath else "Untitled"
                print(f"[Blend Vault] Post-save: Current file '{current_file_display}' is NOT marked as saved. Aborting open of '{file_path_to_open}'.")
        except Exception as e:
            # Log any error during the handler's logic, especially before timer registration
            print(f"[Blend Vault] Error in post_save_handler for '{file_path_to_open}': {e}")

# Register the handler if not already present
if blend_vault_post_save_handler not in bpy.app.handlers.save_post:
    bpy.app.handlers.save_post.append(blend_vault_post_save_handler)

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

def is_valid_blend_file_path(clipboard_text):
    """Check if clipboard contains a valid .blend file path."""
    try:
        path = clipboard_text.strip()
        if path.startswith('"') and path.endswith('"'):
            path = path[1:-1]
        elif path.startswith("'") and path.endswith("'"):
            path = path[1:-1]
        
        return path and os.path.isfile(path) and path.lower().endswith('.blend')
    except:
        return False

def is_valid_blend_or_sidecar_path(clipboard_text):
    """Check if clipboard contains a valid .blend or sidecar file path."""
    try:
        path = clipboard_text.strip()
        if path.startswith('"') and path.endswith('"'):
            path = path[1:-1]
        elif path.startswith("'") and path.endswith("'"):
            path = path[1:-1]
        
        if not path:
            return False
        if os.path.isfile(path):
            if path.lower().endswith('.blend'):
                return True
            if path.lower().endswith(SIDECAR_EXTENSION):
                # Check if the corresponding .blend file exists next to it
                blend_path = path[: -len(SIDECAR_EXTENSION)]
                if os.path.isfile(blend_path) and blend_path.lower().endswith('.blend'):
                    return True
        return False
    except:
        return False

class SmartPasteOperator(bpy.types.Operator):
    bl_idname = "wm.smart_paste"
    bl_label = "Smart Paste"
    bl_description = "Paste with .blend file interception, falls back to default paste"
    
    def execute(self, context):
        try:
            clipboard_text = context.window_manager.clipboard
            if is_valid_blend_or_sidecar_path(clipboard_text):
                path = clipboard_text.strip()
                if path.startswith('"') and path.endswith('"'):
                    path = path[1:-1]
                elif path.startswith("'") and path.endswith("'"):
                    path = path[1:-1]
                # If it's a sidecar, open the corresponding .blend file
                if path.lower().endswith(SIDECAR_EXTENSION):
                    path = path[: -len(SIDECAR_EXTENSION)]
                if bpy.data.is_dirty:
                    bpy.ops.blend_vault.confirm_save_before_open('INVOKE_DEFAULT', file_path=path)
                else:
                    bpy.ops.wm.open_mainfile(filepath=path)
                    self.report({'INFO'}, f"Opened blend file: {os.path.basename(path)}")
                
                return {'FINISHED'}
        except:
            pass
        
        try:
            if context.space_data and context.space_data.type == 'VIEW_3D':
                return bpy.ops.view3d.pastebuffer('INVOKE_DEFAULT')
            else:
                self.report({'WARNING'}, "No valid paste operation for this context")
                return {'CANCELLED'}
        except Exception as e:
            self.report({'WARNING'}, f"Paste operation failed: {e}")
            return {'CANCELLED'}

class OpenBlendFromClipboardOperator(bpy.types.Operator):
    bl_idname = "wm.open_blend_from_clipboard"
    bl_label = "Open Blend File from Clipboard"
    bl_description = "Opens the .blend file at the path in the clipboard"

    def execute(self, context):
        try:
            path = context.window_manager.clipboard.strip()
            if path.startswith('"') and path.endswith('"'):
                path = path[1:-1]
            elif path.startswith("'") and path.endswith("'"):
                path = path[1:-1]
        except Exception as e:
            self.report({'ERROR'}, f"Clipboard error: {e}")
            return {'CANCELLED'}

        if not path or not os.path.isfile(path) or not path.lower().endswith('.blend'):
            self.report({'ERROR'}, f"Not a valid .blend file: {path}")
            return {'CANCELLED'}
        if bpy.data.is_dirty:
            bpy.ops.blend_vault.confirm_save_before_open('INVOKE_DEFAULT', file_path=path)
        else:
            bpy.ops.wm.open_mainfile(filepath=path)

        return {'FINISHED'}

def register():
    bpy.utils.register_class(BV_OT_ConfirmSaveBeforeOpen)
    bpy.utils.register_class(BV_OT_SaveAndOpenFile)
    bpy.utils.register_class(BV_OT_OpenFileWithoutSave)
    bpy.utils.register_class(OpenBlendFromClipboardOperator)
    bpy.utils.register_class(SmartPasteOperator)
    
    wm = bpy.context.window_manager
    kc = wm.keyconfigs.addon
    if kc:
        km = kc.keymaps.new(name='3D View', space_type='VIEW_3D')
        kmi = km.keymap_items.new(SmartPasteOperator.bl_idname, 'V', 'PRESS', ctrl=True)
        
        km_window = kc.keymaps.new(name='Window', space_type='EMPTY')
        kmi_window = km_window.keymap_items.new(SmartPasteOperator.bl_idname, 'V', 'PRESS', ctrl=True)

def unregister():
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
    
    try:
        bpy.utils.unregister_class(SmartPasteOperator)
    except:
        pass
    
    try:
        bpy.utils.unregister_class(OpenBlendFromClipboardOperator)
    except:
        pass

if __name__ == "__main__":
    register()
