import bpy
import os
import atexit
import re  # For check_file_relocation
from .. import LOG_COLORS, REDIRECT_EXTENSION, format_primary_link, MD_EMBED_WIKILINK, get_obsidian_vault_root, PRIMARY_LINK_REGEX
from ..core import log_info, log_warning, log_error, log_success, log_debug
from . import redirect_panel

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


def _absolute_to_vault_relative(abs_path: str) -> str:
	"""Convert an absolute path to a vault-relative path."""
	vault_root = None
	if get_obsidian_vault_root is not None:
		vault_root = get_obsidian_vault_root()
	
	if not vault_root:
		log_warning("No vault root available for path conversion", module_name='RedirectHandler')
		return abs_path
	
	if not os.path.isabs(abs_path):
		abs_path = os.path.abspath(abs_path)
	
	try:
		vault_rel_path = os.path.relpath(abs_path, vault_root).replace(os.sep, '/')
		return vault_rel_path
	except ValueError:
		# This can happen if paths are on different drives on Windows
		log_warning(f"Cannot convert path to vault-relative: {abs_path}", module_name='RedirectHandler')
		return abs_path


def _vault_relative_to_absolute(vault_rel_path: str) -> str:
	"""Convert a vault-relative path to an absolute path."""
	vault_root = None
	if get_obsidian_vault_root is not None:
		vault_root = get_obsidian_vault_root()
	
	if not vault_root:
		log_warning("No vault root available for path conversion", module_name='RedirectHandler')
		return vault_rel_path
	
	abs_path = os.path.normpath(os.path.join(vault_root, vault_rel_path.lstrip('/\\')))
	return abs_path


def _format_display_path(target_path_abs, current_file_dir_abs=None, vault_root_abs=None) -> str:
	"""Format a path for display using vault-relative format."""
	return _absolute_to_vault_relative(target_path_abs)


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
		return
	
	if not blend_path:
		log_warning("create_redirect_file called with empty blend_path", module_name='RedirectHandler')
		return

	# Convert to vault-relative path for the link
	vault_rel_path = _absolute_to_vault_relative(blend_path)
	filename = os.path.basename(blend_path)
	redirect_path = blend_path + REDIRECT_EXTENSION

	redirect_content = f"""{MD_EMBED_WIKILINK['format'].format(name='bv-autogen#^bv-autogen-redirect')}
{format_primary_link(vault_rel_path, filename)}
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
		linked_vault_rel_path = link_match.group(1) 

		# Get current file's vault-relative path
		current_vault_rel_path = _absolute_to_vault_relative(blend_path)
		
		# If the linked path matches the current path, no relocation detected
		if linked_vault_rel_path == current_vault_rel_path:
			# Clear any pending relocations for this file
			_pending_relocations.pop(blend_path, None)
			return
		
		# Convert linked vault-relative path to absolute path
		new_path = _vault_relative_to_absolute(linked_vault_rel_path)
		
		# Check if the new path exists
		if os.path.exists(new_path):
			# Always add to pending relocations if not already there
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
				# Show modal dialog only if not ignored and not already shown
				if blend_path not in _ignored_relocations and not _relocation_dialog_shown:
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
					# Show new modal dialog for the different location only if not ignored
					if blend_path not in _ignored_relocations and not _relocation_dialog_shown:
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


def register():
	atexit.register(cleanup_on_blender_quit)

	# Register UI components
	redirect_panel.register()

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
	# Unregister UI components
	redirect_panel.unregister()

	log_warning("Redirect handler module unregistered.", module_name='RedirectHandler')


# Accessor functions for UI components
def set_relocation_dialog_shown(value: bool):
	"""Set the relocation dialog shown flag"""
	global _relocation_dialog_shown
	_relocation_dialog_shown = value


def get_pending_relocations():
	"""Get the pending relocations dictionary"""
	return _pending_relocations


def get_ignored_relocations():
	"""Get the ignored relocations set"""
	return _ignored_relocations


def add_to_ignored_relocations(path: str):
	"""Add a path to ignored relocations"""
	_ignored_relocations.add(path)


def remove_from_ignored_relocations(path: str):
	"""Remove a path from ignored relocations"""
	_ignored_relocations.discard(path)


def remove_from_pending_relocations(path: str):
	"""Remove a path from pending relocations"""
	_pending_relocations.pop(path, None)


def add_to_pending_relocations(current_path: str, new_path: str):
	"""Add a relocation to pending relocations"""
	_pending_relocations[current_path] = new_path
