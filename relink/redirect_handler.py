import bpy  # type: ignore
import os
import atexit
import re  # For check_file_relocation
from utils import LOG_COLORS, REDIRECT_EXTENSION

# Store last known working directory per .blend file
t_last_working_dirs = {}

# Track if a relocation dialog is currently shown to prevent duplicates
_relocation_dialog_shown = False

# Track files where user chose to ignore relocation for this session
_ignored_relocations = set()

_current_blend_file_for_cleanup = None

print("[Blend Vault][Redirect] redirect_handler.py module loaded.")


def create_redirect_file(blend_path: str):
	"""Creates or overwrites a .redirect.md file for the current blend file."""
	if not blend_path:
		return

	filename = os.path.basename(blend_path)
	redirect_path = blend_path + REDIRECT_EXTENSION

	redirect_content = f"""![[BV_MSG_REDIR]]
[{filename}](<./{filename}>)
"""

	try:
		with open(redirect_path, 'w', encoding='utf-8') as f:
			f.write(redirect_content)

		# Store the current working directory
		t_last_working_dirs[blend_path] = os.path.dirname(blend_path)        # Update the global variable for cleanup on quit
		global _current_blend_file_for_cleanup
		_current_blend_file_for_cleanup = blend_path

	except Exception as e:
		print(
			f"{LOG_COLORS['ERROR']}[Blend Vault][Redirect] Failed to create redirect file {redirect_path}: {e}{LOG_COLORS['RESET']}"
		)


def cleanup_redirect_file(blend_path: str):
	"""Removes the redirect file for the given blend file."""
	if not blend_path:
		print(
			f"{LOG_COLORS['WARN']}[Blend Vault][Redirect] cleanup_redirect_file called with no blend_path.{LOG_COLORS['RESET']}"
		)
		return

	redirect_path = blend_path + REDIRECT_EXTENSION

	try:
		if os.path.exists(redirect_path):
			os.remove(redirect_path)
			print(
				f"{LOG_COLORS['SUCCESS']}[Blend Vault][Redirect] Cleaned up redirect file: {redirect_path}{LOG_COLORS['RESET']}"
			)
	except Exception as e:
		print(
			f"{LOG_COLORS['ERROR']}[Blend Vault][Redirect] Failed to cleanup redirect file {redirect_path}: {e}{LOG_COLORS['RESET']}"
		)


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

		# Extract the markdown link path
		link_match = re.search(r'\[([^\]]+)\]\(<([^>]+)>\)', content)
		if not link_match:
			return

		linked_path = link_match.group(2)

		current_filename = os.path.basename(blend_path)

		# If path is still relative (./ prefix), file hasn't been moved
		if linked_path.startswith(f'./{current_filename}'):
			return

		# File has been moved - extract new path
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
			# Prompt user for action
			_prompt_file_relocation(blend_path, new_path)

	except Exception as e:
		print(
			f"{LOG_COLORS['ERROR']}[Blend Vault][Redirect] Error checking redirect file {redirect_path}: {e}{LOG_COLORS['RESET']}"
		)


def _prompt_file_relocation(current_path: str, new_path: str):
	"""Prompts user and handles file relocation."""
	global _relocation_dialog_shown

	if _relocation_dialog_shown:
		return

	try:
		_relocation_dialog_shown = True
		# Create a modal dialog operator
		bpy.ops.blend_vault.confirm_file_relocation(
			'INVOKE_DEFAULT', current_path=current_path, new_path=new_path
		)
	except Exception as e:
		_relocation_dialog_shown = False
		print(
			f"{LOG_COLORS['ERROR']}[Blend Vault][Redirect] Failed to invoke relocation dialog: {e}{LOG_COLORS['RESET']}"
		)


@bpy.app.handlers.persistent
def create_redirect_on_save(*args, **kwargs):
	"""Handler to create redirect file when blend file is saved."""
	blend_path = bpy.data.filepath
	if blend_path:
		create_redirect_file(blend_path)


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


class BV_OT_ConfirmFileRelocation(bpy.types.Operator):
	"""Operator to confirm and handle file relocation"""
	bl_idname = "blend_vault.confirm_file_relocation"
	bl_label = "File Relocation Detected"
	bl_options = {'REGISTER', 'INTERNAL', 'BLOCKING'}

	current_path: bpy.props.StringProperty()  # type: ignore
	new_path: bpy.props.StringProperty()  # type: ignore

	def execute(self, context):
		global _relocation_dialog_shown
		_relocation_dialog_shown = False
		try:
			# Save to new location
			bpy.ops.wm.save_as_mainfile(filepath=self.new_path)
			# Cleanup redirect file
			cleanup_redirect_file(self.current_path)
		except Exception as e:
			print(
				f"{LOG_COLORS['ERROR']}[Blend Vault][Redirect] Error during relocation: {e}{LOG_COLORS['RESET']}"
			)
		return {'FINISHED'}

	def cancel(self, context):
		global _relocation_dialog_shown
		_relocation_dialog_shown = False
		# Add current file to ignored relocations for this session
		_ignored_relocations.add(self.current_path)
		# Clean up the redirect file since user chose to ignore
		cleanup_redirect_file(self.current_path)
		return {'CANCELLED'}
	
	def invoke(self, context, event):
		return context.window_manager.invoke_props_dialog(self, width=450)

	def draw(self, context):
		layout = self.layout

		# Get common base directory to show relative paths like markdown
		current_dir = os.path.dirname(self.current_path)
		current_filename = os.path.basename(self.current_path)
		new_rel_path = os.path.relpath(self.new_path, current_dir)

		# Convert backslashes to forward slashes for consistent markdown-style paths
		new_rel_path = new_rel_path.replace('\\', '/')

		# Add ./ prefix if the path doesn't start with ../ (same level or subdirectory)
		if not new_rel_path.startswith('../') and not os.path.isabs(new_rel_path):
			new_rel_path = './' + new_rel_path

		layout.label(text=f"Current location: ./{current_filename}")
		layout.label(text=f"New location: {new_rel_path}")
		layout.separator()
		layout.label(text="Save the current file to the new location?")


def register():
	atexit.register(cleanup_on_blender_quit)

	bpy.utils.register_class(BV_OT_ConfirmFileRelocation)

	# Register handlers
	bpy.app.handlers.save_post.append(create_redirect_on_save)
	bpy.app.handlers.load_post.append(create_redirect_on_load)
	bpy.app.handlers.load_factory_startup_post.append(clear_session_flags_on_new)
	bpy.app.handlers.load_pre.append(cleanup_redirect_on_load_pre)

	print(
		f"{LOG_COLORS['SUCCESS']}[Blend Vault][Redirect] Redirect handler module registered.{LOG_COLORS['RESET']}"
	)


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

	print(
		f"{LOG_COLORS['WARN']}[Blend Vault] Redirect handler module unregistered.{LOG_COLORS['RESET']}"
	)


if __name__ == "__main__":
	register()
