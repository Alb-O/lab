"""
Save workflow management for handling unsaved changes before opening new files.
Provides save-and-open functionality with proper timer-based file opening.
"""

import bpy
import os
import functools


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
            
            if bpy.data.is_saved:  # File has a path, might be dirty (e.g. existing file with unsaved changes)
                bpy.ops.wm.save_mainfile()
                # The save_post_handler will now pick this up and schedule the open.
            else:  # File is new or not yet saved (no path)
                bpy.ops.wm.save_as_mainfile('INVOKE_DEFAULT')
            
            return {'FINISHED'}
        except Exception as e:
            # Clean up the key if an error occurs before save op is invoked or if setup fails
            if context.window_manager and 'blend_vault_open_after_save' in context.window_manager: # Check context.window_manager
                del context.window_manager['blend_vault_open_after_save']
            self.report({'ERROR'}, f"Failed to initiate save and open: {e}")
            return {'CANCELLED'}


def _actual_open_file_operation(file_path):
    """Helper function to be called by the timer"""
    try:
        print(f"[Blend Vault] Timer executing: Attempting to open {file_path}")
        bpy.ops.wm.open_mainfile(filepath=file_path)
        print(f"[Blend Vault] Successfully initiated open for: {os.path.basename(file_path)}")
    except Exception as e:
        print(f"[Blend Vault] Timer failed to open file '{file_path}': {e}")
    return None  # Returning None unregisters the timer after it runs once


@bpy.app.handlers.persistent
def blend_vault_post_save_handler(dummy):
    """Handler to check after saving if a file should be opened"""
    wm = bpy.context.window_manager
    key = 'blend_vault_open_after_save'
    if wm and key in wm: # Check wm before using 'in'
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
                if not bpy.app.timers.is_registered(open_action):  # Avoid duplicate timers if somehow possible
                    bpy.app.timers.register(open_action, first_interval=0.1)
            else:
                # This case (file not saved after a save operation) should be rare but is logged.
                current_file_display = bpy.data.filepath if bpy.data.filepath else "Untitled"
                print(f"[Blend Vault] Post-save: Current file '{current_file_display}' is NOT marked as saved. Aborting open of '{file_path_to_open}'.")
        except Exception as e:
            # Log any error during the handler's logic, especially before timer registration
            print(f"[Blend Vault] Error in post_save_handler for '{file_path_to_open}': {e}")


def register():
    """Register save workflow components."""
    bpy.utils.register_class(BV_OT_SaveAndOpenFile)
    
    # Register the handler if not already present
    if blend_vault_post_save_handler not in bpy.app.handlers.save_post:
        bpy.app.handlers.save_post.append(blend_vault_post_save_handler)


def unregister():
    """Unregister save workflow components."""
    bpy.utils.unregister_class(BV_OT_SaveAndOpenFile)
    
    # Remove handler if present
    if blend_vault_post_save_handler in bpy.app.handlers.save_post:
        bpy.app.handlers.save_post.remove(blend_vault_post_save_handler)
