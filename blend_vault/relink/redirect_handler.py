import bpy
import os
import atexit
import re  # For check_file_relocation
from .. import LOG_COLORS, REDIRECT_EXTENSION, format_primary_link, MD_EMBED_WIKILINK, get_obsidian_vault_root, PRIMARY_LINK_REGEX
from ..core import log_info, log_warning, log_error, log_success, log_debug

# Store last known working directory per .blend file
t_last_working_dirs = {}

# Track if a relocation dialog is currently shown to prevent duplicates
_relocation_dialog_shown = False

# Track files where user chose to ignore relocation for this session
_ignored_relocations = set()

# Track pending relocations that user can click on
_pending_relocations = {}

_current_blend_file_for_cleanup = None

log_info("redirect_handler.py module loaded.", module_name='RedirectHandler')


def _format_display_path(target_path_abs, current_file_dir_abs, vault_root_abs=None):
    norm_target_path = os.path.normpath(target_path_abs)
    norm_current_file_dir = os.path.normpath(current_file_dir_abs)
    
    display_path_str = ""

    if vault_root_abs:
        norm_vault_root = os.path.normpath(vault_root_abs)
        # Check if target_path is inside vault_root (or is vault_root itself)
        if os.path.commonprefix([norm_target_path, norm_vault_root]) == norm_vault_root:
            # Path is within the vault
            display_path_str = os.path.relpath(norm_target_path, norm_vault_root)
            # If display_path_str is '.', it means target_path_abs is the vault_root_abs.
            # os.path.relpath handles this (e.g. "file.txt" or "subdir/file.txt")
        else:
            # Path is outside the vault, use path relative to current_file_dir
            display_path_str = os.path.relpath(norm_target_path, norm_current_file_dir)
            if not display_path_str.startswith('..' + os.sep) and \
               not display_path_str.startswith('.' + os.sep) and \
               not os.path.isabs(display_path_str):
                display_path_str = '.' + os.sep + display_path_str
    else:
        # Vault root not set, use path relative to current_file_dir
        display_path_str = os.path.relpath(norm_target_path, norm_current_file_dir)
        if not display_path_str.startswith('..' + os.sep) and \
           not display_path_str.startswith('.' + os.sep) and \
           not os.path.isabs(display_path_str):
            display_path_str = '.' + os.sep + display_path_str
            
    return display_path_str.replace(os.sep, '/')


def create_redirect_file(blend_path: str):
    """Creates or overwrites a .redirect.md file for the current blend file."""
    log_debug(f"create_redirect_file called with: {blend_path}", module_name='RedirectHandler')
    
    vault_root = None
    if get_obsidian_vault_root is not None: # Check before calling
        vault_root = get_obsidian_vault_root()

    if vault_root:
        log_debug(f"Obsidian vault root: {vault_root}", module_name='RedirectHandler')
    else:
        log_warning("No Obsidian vault root set in preferences", module_name='RedirectHandler')
    
    if not blend_path:
        log_warning("create_redirect_file called with empty blend_path", module_name='RedirectHandler')
        return

    filename = os.path.basename(blend_path)
    redirect_path = blend_path + REDIRECT_EXTENSION

    redirect_content = f"""{MD_EMBED_WIKILINK['format'].format(name='bv-autogen#^bv-autogen-redirect')}
{format_primary_link(f'./{filename}', filename)}
"""    
    try:
        with open(redirect_path, 'w', encoding='utf-8') as f:
            f.write(redirect_content)

        log_success(f"Created redirect file: {redirect_path}", module_name='RedirectHandler')

        # Store the current working directory
        t_last_working_dirs[blend_path] = os.path.dirname(blend_path)
        # Update the global variable for cleanup on quit
        global _current_blend_file_for_cleanup
        _current_blend_file_for_cleanup = blend_path

    except Exception as e:
        log_error(f"Failed to create redirect file {redirect_path}: {e}", module_name='RedirectHandler')


def cleanup_redirect_file(blend_path: str):
    """Removes the redirect file for the given blend file."""
    if not blend_path:
        log_warning("cleanup_redirect_file called with no blend_path.", module_name='RedirectHandler')
        return

    redirect_path = blend_path + REDIRECT_EXTENSION

    try:
        if os.path.exists(redirect_path):
            os.remove(redirect_path)
            log_success(f"Cleaned up redirect file: {redirect_path}", module_name='RedirectHandler')
    except Exception as e:
        log_error(f"Failed to cleanup redirect file {redirect_path}: {e}", module_name='RedirectHandler')


def check_file_relocation():
    """Checks redirect files to detect if the blend file has been moved by Obsidian."""
    global _relocation_dialog_shown

    # Don't check if dialog is already shown
    if _relocation_dialog_shown:
        return

    blend_path = bpy.data.filepath
    if not blend_path:
        return

    # Don't check if user already chose to ignore relocation for this file
    if blend_path in _ignored_relocations:
        return

    redirect_path = blend_path + REDIRECT_EXTENSION
    if not os.path.exists(redirect_path):
        return

    try:
        with open(redirect_path, 'r', encoding='utf-8') as f:
            content = f.read()

        # Extract the markdown link path using the correct PRIMARY_LINK_REGEX
        # PRIMARY_LINK_REGEX for wikilink: r'\\[\\[([^\\]|]+)\\|([^\\]]+)\\]\\]'
        # Group 1 is the path, Group 2 is the alias/name
        link_match = PRIMARY_LINK_REGEX.search(content)
        
        if not link_match:
            log_debug(f"No primary link match found in redirect file content: {content}", module_name='RedirectHandler')
            return

        # For wikilink [[path|name]], path is group 1
        linked_path = link_match.group(1) 

        current_filename = os.path.basename(blend_path)
        # If path is still relative (./ prefix), file hasn't been moved
        if linked_path.startswith(f'./{current_filename}'):
            # Clear any pending relocations for this file
            _pending_relocations.pop(blend_path, None)
            return
        if linked_path.startswith('../') or linked_path.startswith('./'):
            # Calculate the new absolute path based on the relative path
            # Use the directory where the redirect file is located as the base
            redirect_dir = os.path.dirname(redirect_path)
            new_path = os.path.normpath(os.path.join(redirect_dir, linked_path))
        else:
            # Absolute path or other format
            new_path = linked_path
        
        # Check if the new path exists
        if os.path.exists(new_path):
            # Only proceed if this is a new relocation (not already pending)
            if blend_path not in _pending_relocations:
                # Store pending relocation for status clicking
                _pending_relocations[blend_path] = new_path
                # Don't clean up redirect file yet - keep it for potential future moves
                # Refresh UI to show updated panel
                for area in bpy.context.screen.areas:
                    if area.type == 'VIEW_3D':
                        area.tag_redraw()
                # Show status message
                _show_relocation_status_message(blend_path, new_path)
                # Show modal dialog only if not already shown
                if not _relocation_dialog_shown:
                    _prompt_file_relocation(blend_path, new_path)
            else:
                # Update the path if it's different (file moved again)
                if _pending_relocations[blend_path] != new_path:
                    _pending_relocations[blend_path] = new_path
                    # Refresh UI to show updated path
                    for area in bpy.context.screen.areas:
                        if area.type == 'VIEW_3D':
                            area.tag_redraw()
                    log_info(f"Updated relocation path: {os.path.basename(blend_path)} -> {os.path.basename(new_path)}", module_name='RedirectHandler')
                    # Show new modal dialog for the different location
                    if not _relocation_dialog_shown:
                        _prompt_file_relocation(blend_path, new_path)

    except Exception as e:
        log_error(f"Error checking redirect file {redirect_path}: {e}", module_name='RedirectHandler')


def _show_relocation_status_message(current_path: str, new_path: str):
    """Shows a status message for file relocation."""
    filename = os.path.basename(current_path)
    new_filename = os.path.basename(new_path)
    log_warning(f"File relocation detected: {filename} -> {new_filename}. Check N-panel 'Blend Vault' tab to handle.", module_name='RedirectHandler')


def _prompt_file_relocation(current_path: str, new_path: str):
    """Prompts user and handles file relocation."""
    global _relocation_dialog_shown

    if _relocation_dialog_shown:
        return

    try:
        _relocation_dialog_shown = True
        # Create a modal dialog operator
        # The type checker may not know about dynamically registered operators.
        bpy.ops.blend_vault.confirm_file_relocation( # type: ignore
            'INVOKE_DEFAULT', current_path=current_path, new_path=new_path
        )
    except Exception as e:
        _relocation_dialog_shown = False
        log_error(f"Failed to invoke relocation dialog: {e}", module_name='RedirectHandler')


@bpy.app.handlers.persistent
def create_redirect_on_save(*args, **kwargs):
    """Handler to create redirect file when blend file is saved."""
    blend_path = bpy.data.filepath
    log_debug(f"create_redirect_on_save called, blend_path: {blend_path}", module_name='RedirectHandler')
    if blend_path:
        create_redirect_file(blend_path)
    else:
        log_debug("No blend_path - skipping redirect file creation", module_name='RedirectHandler')


@bpy.app.handlers.persistent
def create_redirect_on_load(*args, **kwargs):
    """Handler to create redirect file and check for relocation when blend file is loaded."""
    global _ignored_relocations
    _ignored_relocations.clear()
    blend_path = bpy.data.filepath
    if blend_path:
        create_redirect_file(blend_path)
        check_file_relocation()


@bpy.app.handlers.persistent
def clear_session_flags_on_new(*args, **kwargs):
    """Handler to clear session flags when a new file is created."""
    global _ignored_relocations, _current_blend_file_for_cleanup
    _ignored_relocations.clear()
    _current_blend_file_for_cleanup = None


# This is the function that will be called by atexit.
# It should not be decorated with @bpy.app.handlers.persistent.
# It should not take *args, **kwargs if atexit calls it directly.
def cleanup_on_blender_quit():
    global _current_blend_file_for_cleanup
    if _current_blend_file_for_cleanup:
        cleanup_redirect_file(_current_blend_file_for_cleanup)


@bpy.app.handlers.persistent
def cleanup_redirect_on_load_pre(*args, **kwargs):
    """Handler to cleanup redirect files before loading a new file."""
    # Clean up redirect file for the current file before loading new one
    current_blend_path = bpy.data.filepath
    if current_blend_path:
        cleanup_redirect_file(current_blend_path)


class BV_PT_FileRelocationPanel(bpy.types.Panel):
    """Panel in N-panel to show file relocation status"""
    bl_label = "File Relocation"
    bl_idname = "BV_PT_file_relocation"
    bl_space_type = 'VIEW_3D'
    bl_region_type = 'UI'
    bl_category = "Blend Vault"

    @classmethod
    def poll(cls, context):
        # Always show panel for debugging - change back to only show when pending relocations exist
        return True
        # return bool(_pending_relocations)

    def draw(self, context):
        layout = self.layout
        
        current_blend = bpy.data.filepath
        
        if current_blend in _pending_relocations:
            # Always read the current path from the redirect file instead of memory
            redirect_path = current_blend + REDIRECT_EXTENSION
            new_path_abs = None # This is determined by the logic below
            
            if os.path.exists(redirect_path):
                try:
                    with open(redirect_path, 'r', encoding='utf-8') as f:
                        content = f.read()
                    
                    # Extract the markdown link path using the correct PRIMARY_LINK_REGEX
                    # PRIMARY_LINK_REGEX for wikilink: r'\\[\\[([^\\]|]+)\\|([^\\]]+)\\]\\]'
                    # Group 1 is the path, Group 2 is the alias/name
                    link_match = PRIMARY_LINK_REGEX.search(content)
                    
                    if link_match:
                        linked_path = link_match.group(1)
                        current_filename_for_check = os.path.basename(current_blend)
                        
                        if not linked_path.startswith(f'./{current_filename_for_check}'):
                            if linked_path.startswith('../') or linked_path.startswith('./'):
                                redirect_dir = os.path.dirname(redirect_path)
                                new_path_abs = os.path.normpath(os.path.join(redirect_dir, linked_path))
                            else:
                                new_path_abs = os.path.normpath(linked_path) 
                except Exception as e:
                    log_error(f"Error reading redirect file in panel: {e}", module_name='RedirectHandler')
            
            if new_path_abs and os.path.exists(new_path_abs):
                current_dir_abs = os.path.dirname(current_blend)
                
                vault_root_abs = None
                if get_obsidian_vault_root is not None: # Check before calling
                    vault_root_abs = get_obsidian_vault_root()

                current_path_to_display = _format_display_path(current_blend, current_dir_abs, vault_root_abs)
                new_path_to_display = _format_display_path(new_path_abs, current_dir_abs, vault_root_abs)

                # Warning icon and title
                row = layout.row()
                row.alert = True
                row.label(text="File relocation detected.", icon='ERROR')
                
                layout.separator()
                
                row = layout.row()
                row.label(text="The file has been moved or renamed externally.")
                
                # Current and new location info
                box = layout.box()
                box.label(text=f"Current: {current_path_to_display}")
                box.label(text=f"New: {new_path_to_display}")
                
                layout.separator()
                
                # Action buttons
                col = layout.column(align=True)
                col.scale_y = 1.2
                
                # Save to new location button
                op_props = col.operator("blend_vault.confirm_file_relocation", text="Save to New Location", icon='FILE_TICK')
                # It's expected that BV_OT_ConfirmFileRelocation has these properties defined.
                # Pylance might not infer this perfectly from layout.operator.
                setattr(op_props, 'current_path', current_blend) # Use setattr to be more explicit for type checker
                setattr(op_props, 'new_path', new_path_abs) # Use setattr
                
                # Ignore button
                col.operator("blend_vault.ignore_relocation", text="Ignore for This Session", icon='CANCEL')
            else:
                # No valid relocation found, remove from pending
                _pending_relocations.pop(current_blend, None)
                layout.label(text="No pending relocations.", icon='CHECKMARK')
        else:
            layout.label(text="No pending relocations.", icon='CHECKMARK')


class BV_OT_IgnoreRelocation(bpy.types.Operator):
    """Operator to ignore relocation for this session"""
    bl_idname = "blend_vault.ignore_relocation"
    bl_label = "Ignore Relocation"
    bl_options = {'REGISTER', 'INTERNAL'}

    def execute(self, context):
        current_blend = bpy.data.filepath
        if current_blend in _pending_relocations:
            # Add to ignored relocations
            _ignored_relocations.add(current_blend)
            # Clean up redirect file
            cleanup_redirect_file(current_blend)
            # Remove from pending
            _pending_relocations.pop(current_blend, None)
            
            # Refresh UI to hide the panel
            for area in bpy.context.screen.areas:
                if area.type == 'VIEW_3D':
                    area.tag_redraw()
            
            self.report({'INFO'}, "File relocation ignored for this session")
            log_info("File relocation ignored for this session", module_name='RedirectHandler')
        
        return {'FINISHED'}


class BV_OT_ConfirmFileRelocation(bpy.types.Operator):
    """Operator to confirm and handle file relocation"""
    bl_idname = "blend_vault.confirm_file_relocation"
    bl_label = "File Relocation"
    bl_options = {'REGISTER', 'INTERNAL', 'BLOCKING'}

    # Define properties as class annotations
    current_path: bpy.props.StringProperty(name="Current Path", description="Current path of the blend file") # type: ignore
    new_path: bpy.props.StringProperty(name="New Path", description="New detected path of the blend file") # type: ignore

    def execute(self, context):
        global _relocation_dialog_shown
        _relocation_dialog_shown = False
        try:
            # Save to new location
            bpy.ops.wm.save_as_mainfile(filepath=self.new_path)
            # Cleanup redirect file and remove from pending
            cleanup_redirect_file(self.current_path)
            _pending_relocations.pop(self.current_path, None)
            
            # Refresh UI to update panel
            for area in bpy.context.screen.areas:
                if area.type == 'VIEW_3D':
                    area.tag_redraw()
            
            log_success(f"File relocated to {self.new_path}", module_name='RedirectHandler')
        except Exception as e:
            log_error(f"Error during relocation: {e}", module_name='RedirectHandler')
        return {'FINISHED'}

    def cancel(self, context):
        global _relocation_dialog_shown
        _relocation_dialog_shown = False
        # Don't clean up redirect file or remove from pending relocations on cancel
        # Keep the relocation pending so user can handle it through the N-panel
        log_info("File relocation dialog dismissed. Use N-panel to handle.", module_name='RedirectHandler')
    
    def invoke(self, context, event):
        return context.window_manager.invoke_props_dialog(self, width=450)

    def draw(self, context):
        layout = self.layout
        current_dir_abs = os.path.dirname(self.current_path)
        
        vault_root_abs = None
        if get_obsidian_vault_root is not None: # Check before calling
            vault_root_abs = get_obsidian_vault_root()

        current_display_path = _format_display_path(self.current_path, current_dir_abs, vault_root_abs)
        new_display_path = _format_display_path(self.new_path, current_dir_abs, vault_root_abs)
        layout.label(text="The file has been moved or renamed externally.")
        layout.separator()
        layout.label(text=f"Current location: {current_display_path}")
        layout.label(text=f"New location: {new_display_path}")
        layout.separator()
        layout.label(text="Save the current file to the new location?")


def register():
    atexit.register(cleanup_on_blender_quit)

    bpy.utils.register_class(BV_OT_ConfirmFileRelocation)
    bpy.utils.register_class(BV_PT_FileRelocationPanel)
    bpy.utils.register_class(BV_OT_IgnoreRelocation)

    log_info("Panel registered with category 'Blend Vault'", module_name='RedirectHandler')

    # Register handlers
    bpy.app.handlers.save_post.append(create_redirect_on_save)
    bpy.app.handlers.load_post.append(create_redirect_on_load)
    bpy.app.handlers.load_factory_startup_post.append(clear_session_flags_on_new)
    bpy.app.handlers.load_pre.append(cleanup_redirect_on_load_pre)

    log_success("Redirect handler module registered.", module_name='RedirectHandler')


def unregister():
    # Note: We intentionally do NOT unregister the atexit callback here
    # because we want it to run when Blender actually quits, not when the addon is disabled

    # Remove handlers
    if create_redirect_on_save in bpy.app.handlers.save_post:
        bpy.app.handlers.save_post.remove(create_redirect_on_save)
    if create_redirect_on_load in bpy.app.handlers.load_post:
        bpy.app.handlers.load_post.remove(create_redirect_on_load)
    if clear_session_flags_on_new in bpy.app.handlers.load_factory_startup_post:
        bpy.app.handlers.load_factory_startup_post.remove(clear_session_flags_on_new)
    if cleanup_redirect_on_load_pre in bpy.app.handlers.load_pre:
        bpy.app.handlers.load_pre.remove(cleanup_redirect_on_load_pre)

    bpy.utils.unregister_class(BV_OT_ConfirmFileRelocation)
    bpy.utils.unregister_class(BV_PT_FileRelocationPanel)
    bpy.utils.unregister_class(BV_OT_IgnoreRelocation)

    log_warning("Redirect handler module unregistered.", module_name='RedirectHandler')


if __name__ == "__main__":
    register()
