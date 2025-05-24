import bpy  # type: ignore
import os
import atexit
import re  # For check_file_relocation
from utils import LOG_COLORS, REDIRECT_EXTENSION, MD_PRIMARY_FORMAT, MD_EMBED_WIKILINK

# Store last known working directory per .blend file
t_last_working_dirs = {}

# Track if a relocation dialog is currently shown to prevent duplicates
_relocation_dialog_shown = False

# Track files where user chose to ignore relocation for this session
_ignored_relocations = set()

# Track pending relocations that user can click on
_pending_relocations = {}

_current_blend_file_for_cleanup = None

print("[Blend Vault][Redirect] redirect_handler.py module loaded.")


def create_redirect_file(blend_path: str):
	"""Creates or overwrites a .redirect.md file for the current blend file."""
	if not blend_path:
		return

	filename = os.path.basename(blend_path)
	redirect_path = blend_path + REDIRECT_EXTENSION

	redirect_content = f"""{MD_EMBED_WIKILINK['format'].format(name='BV_MSG_REDIR')}
{MD_PRIMARY_FORMAT['format'].format(name=filename, path=f'./{filename}')}
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
					print(f"{LOG_COLORS['INFO']}[Blend Vault][Redirect] Updated relocation path: {os.path.basename(blend_path)} -> {os.path.basename(new_path)}{LOG_COLORS['RESET']}")
					# Show new modal dialog for the different location
					if not _relocation_dialog_shown:
						_prompt_file_relocation(blend_path, new_path)

	except Exception as e:
		print(
			f"{LOG_COLORS['ERROR']}[Blend Vault][Redirect] Error checking redirect file {redirect_path}: {e}{LOG_COLORS['RESET']}"		)


def _show_relocation_status_message(current_path: str, new_path: str):
	"""Shows a status message for file relocation."""
	filename = os.path.basename(current_path)
	new_filename = os.path.basename(new_path)
	print(f"{LOG_COLORS['WARN']}[Blend Vault][Redirect] File relocation detected: {filename} -> {new_filename}. Check N-panel 'Blend Vault' tab to handle.{LOG_COLORS['RESET']}")


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
			new_path = None
			
			if os.path.exists(redirect_path):
				try:
					with open(redirect_path, 'r', encoding='utf-8') as f:
						content = f.read()
					
					# Extract the markdown link path
					link_match = re.search(r'\[([^\]]+)\]\(<([^>]+)>\)', content)
					if link_match:
						linked_path = link_match.group(2)
						current_filename = os.path.basename(current_blend)
						
						# Check if file has been moved
						if not linked_path.startswith(f'./{current_filename}'):
							# Calculate the new absolute path
							if linked_path.startswith('../') or linked_path.startswith('./'):
								redirect_dir = os.path.dirname(redirect_path)
								new_path = os.path.normpath(os.path.join(redirect_dir, linked_path))
							else:
								new_path = linked_path
				except Exception as e:
					print(f"{LOG_COLORS['ERROR']}[Blend Vault][Redirect] Error reading redirect file in panel: {e}{LOG_COLORS['RESET']}")
			
			if new_path and os.path.exists(new_path):
				# Get common base directory to show relative paths like markdown
				current_dir = os.path.dirname(current_blend)
				current_filename = os.path.basename(current_blend)
				new_rel_path = os.path.relpath(new_path, current_dir)

				# Convert backslashes to forward slashes for consistent markdown-style paths
				new_rel_path = new_rel_path.replace('\\', '/')

				# Add ./ prefix if the path doesn't start with ../ (same level or subdirectory)
				if not new_rel_path.startswith('../') and not os.path.isabs(new_rel_path):
					new_rel_path = './' + new_rel_path

				# Warning icon and title
				row = layout.row()
				row.alert = True
				row.label(text="File Moved Detected!", icon='ERROR')
				
				layout.separator()
				
				# Current and new location info
				box = layout.box()
				box.label(text=f"Current: ./{current_filename}")
				box.label(text=f"New: {new_rel_path}")
				
				layout.separator()
				
				# Action buttons
				col = layout.column(align=True)
				col.scale_y = 1.2
				
				# Save to new location button
				op = col.operator("blend_vault.confirm_file_relocation", text="Save to New Location", icon='FILE_TICK')
				op.current_path = current_blend
				op.new_path = new_path
				
				# Ignore button
				col.operator("blend_vault.ignore_relocation", text="Ignore This Session", icon='CANCEL')
			else:
				# No valid relocation found, remove from pending
				_pending_relocations.pop(current_blend, None)
				layout.label(text="No pending relocations", icon='CHECKMARK')
		else:
			layout.label(text="No pending relocations", icon='CHECKMARK')


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
			print(f"{LOG_COLORS['INFO']}[Blend Vault][Redirect] File relocation ignored for this session{LOG_COLORS['RESET']}")
		
		return {'FINISHED'}


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
			# Cleanup redirect file and remove from pending
			cleanup_redirect_file(self.current_path)
			_pending_relocations.pop(self.current_path, None)
			
			# Refresh UI to update panel
			for area in bpy.context.screen.areas:
				if area.type == 'VIEW_3D':
					area.tag_redraw()
			
			print(f"{LOG_COLORS['SUCCESS']}[Blend Vault][Redirect] File relocated to {self.new_path}{LOG_COLORS['RESET']}")
		except Exception as e:
			print(
				f"{LOG_COLORS['ERROR']}[Blend Vault][Redirect] Error during relocation: {e}{LOG_COLORS['RESET']}"
			)
		return {'FINISHED'}
	def cancel(self, context):
		global _relocation_dialog_shown
		_relocation_dialog_shown = False
		# Don't clean up redirect file or remove from pending relocations on cancel
		# Keep the relocation pending so user can handle it through the N-panel
		print(f"{LOG_COLORS['INFO']}[Blend Vault][Redirect] File relocation dialog dismissed. Use N-panel to handle.{LOG_COLORS['RESET']}")
	
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
	bpy.utils.register_class(BV_PT_FileRelocationPanel)
	bpy.utils.register_class(BV_OT_IgnoreRelocation)

	print(f"{LOG_COLORS['INFO']}[Blend Vault][Redirect] Panel registered with category 'Blend Vault'{LOG_COLORS['RESET']}")

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
	bpy.utils.unregister_class(BV_PT_FileRelocationPanel)
	bpy.utils.unregister_class(BV_OT_IgnoreRelocation)

	print(
		f"{LOG_COLORS['WARN']}[Blend Vault] Redirect handler module unregistered.{LOG_COLORS['RESET']}"
	)


if __name__ == "__main__":
	register()
