import bpy # type: ignore
import os
import functools # Added import
import preferences  # Use absolute import for Blender add-on compatibility
from utils import SIDECAR_EXTENSION

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

    file_path: bpy.props.StringProperty() # type: ignore

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

    file_path: bpy.props.StringProperty() # type: ignore

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

    file_path: bpy.props.StringProperty() # type: ignore

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

    file_path: bpy.props.StringProperty() # type: ignore

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
                
                if path.lower().endswith(SIDECAR_EXTENSION):
                    path = path[: -len(SIDECAR_EXTENSION)]
                
                # Always show the new choose action dialog
                bpy.ops.blend_vault.choose_action_before_open('INVOKE_DEFAULT', file_path=path)
                return {'FINISHED'}
        except Exception as e:
            # Log or report error if needed, e.g., self.report({'ERROR'}, f"SmartPaste error: {e}")
            pass # Fall through to default paste
        
        # Fallback to default paste behavior
        try:
            if context.space_data and context.space_data.type == 'VIEW_3D':
                return bpy.ops.view3d.pastebuffer('INVOKE_DEFAULT')
            # Attempt to find a generic paste operator if not in 3D View
            # This part might need more robust handling for different contexts
            elif hasattr(bpy.ops.ui, 'paste'): # Check for generic UI paste
                 return bpy.ops.ui.paste('INVOKE_DEFAULT')
            elif hasattr(bpy.ops.text, 'paste'): # Check for text editor paste
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
        
        # Always show the new choose action dialog
        bpy.ops.blend_vault.choose_action_before_open('INVOKE_DEFAULT', file_path=path)
        return {'FINISHED'}

def register():
    bpy.utils.register_class(BV_OT_ProcessOpenAction)
    bpy.utils.register_class(BV_OT_ChooseActionBeforeOpen)
    bpy.utils.register_class(BV_OT_ConfirmSaveBeforeOpen)
    bpy.utils.register_class(BV_OT_SaveAndOpenFile)
    bpy.utils.register_class(BV_OT_OpenFileWithoutSave)
    bpy.utils.register_class(BV_OT_LinkFromFile)
    bpy.utils.register_class(BV_OT_AppendFromFile)
    bpy.utils.register_class(BV_OT_LinkFirstAsset)
    bpy.utils.register_class(BV_OT_AppendFirstAsset)
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
    bpy.utils.unregister_class(BV_OT_ProcessOpenAction)
    bpy.utils.unregister_class(BV_OT_ChooseActionBeforeOpen)
    bpy.utils.unregister_class(BV_OT_ConfirmSaveBeforeOpen)
    bpy.utils.unregister_class(BV_OT_SaveAndOpenFile)
    bpy.utils.unregister_class(BV_OT_OpenFileWithoutSave)
    bpy.utils.unregister_class(BV_OT_LinkFromFile)
    bpy.utils.unregister_class(BV_OT_AppendFromFile)
    bpy.utils.unregister_class(BV_OT_LinkFirstAsset)
    bpy.utils.unregister_class(BV_OT_AppendFirstAsset)
    bpy.utils.unregister_class(OpenBlendFromClipboardOperator)
    bpy.utils.unregister_class(SmartPasteOperator)

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
