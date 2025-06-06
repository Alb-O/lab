"""
Missing links dialog for Blend Vault.

Provides an automatic, user-friendly workflow for detecting and relinking missing assets and library files.
Features automatic dialog re-showing, progress tracking, and safety mechanisms to prevent infinite loops.

- Automatic detection of missing assets and library files
- Modal dialog for user-guided relinking
- Auto-retry mechanism for seamless batch processing
- Informational panel showing current missing links status
- Safety limits to prevent runaway dialog sequences
"""

import bpy
import os
from ..core import log_info, log_warning, log_error, log_success, log_debug, ensure_saved_file
from .asset_relinker import AssetRelinkProcessor
from .library_relinker import LibraryRelinkProcessor
from .shared_utils import get_sidecar_path, SidecarParser


# === Global State Management ===
# These variables manage the dialog's lifecycle and auto-retry behavior

# Flag to prevent multiple dialogs from being shown simultaneously
_missing_links_dialog_shown = False

# Cache for detected relink items (used by the dialog and panel)
_pending_relink_items = []

# Counter to track consecutive automatic dialog showings (prevents infinite loops)
_consecutive_dialog_count = 0
_max_consecutive_dialogs = 5


class RelinkItem:
	"""Represents an item that needs to be relinked."""
	def __init__(self, item_type: str, description: str, details: str = ""):
		self.item_type = item_type  # "asset", "library", "resource"
		self.description = description
		self.details = details


def check_missing_links():
	"""
	Check what items need to be relinked.
	Returns a list of RelinkItem objects.
	"""
	blend_path = ensure_saved_file()
	if not blend_path:
		log_debug("No saved file, cannot check missing links", module_name='MissingLinks')
		return []
	
	log_debug("Checking for missing links...", module_name='MissingLinks')
	
	# Create a fresh list for this check
	all_items = []
	
	# Check for missing asset datablocks via sidecars
	asset_items = _check_asset_relinks(blend_path)
	# Check for missing library files via main sidecar
	library_items = _check_library_relinks(blend_path)
	# If any libraries are missing, report only libraries (skip asset datablocks)
	if library_items:
		all_items.extend(library_items)
		log_debug(f"After library check (libraries missing): {len(all_items)} items", module_name='MissingLinks')
	else:
		all_items.extend(asset_items)
		log_debug(f"After asset check: {len(all_items)} items", module_name='MissingLinks')
	
	# Update global state for backward compatibility with other parts of the system
	global _pending_relink_items
	_pending_relink_items.clear()
	_pending_relink_items.extend(all_items)
	
	# Debug: report number of missing items detected
	if all_items:
		log_debug(f"MissingLinks detected {len(all_items)} items:", module_name='MissingLinks')
		for i, item in enumerate(all_items):
			log_debug(f"  {i+1}. {item.item_type}: {item.description}", module_name='MissingLinks')
	else:
		log_debug("MissingLinks detected 0 items", module_name='MissingLinks')
	
	return all_items

def _check_asset_relinks(blend_path: str):
	"""Detect missing asset datablocks via AssetRelinkProcessor."""
	items = []
	try:
		# Use AssetRelinkProcessor to detect missing assets without relinking
		processor = AssetRelinkProcessor(main_blend_path=blend_path)
		missing_assets = processor.get_missing_assets()
		for asset in missing_assets:
			name = asset.get('name', 'Unknown')
			typ = asset.get('type', 'Unknown')
			lib_rel = asset.get('lib_rel_path')
			lib_name = os.path.basename(lib_rel) if lib_rel else 'unknown'
			description = f"'{name}' ({typ}) from library: {lib_name}"
			items.append(RelinkItem('asset', description))
	except Exception as e:
		log_debug(f"Error checking asset relinks via AssetRelinkProcessor: {e}", module_name='MissingLinks')
	return items

def _check_library_relinks(blend_path: str):
	"""Detect missing library files referenced in the main sidecar."""
	items = []
	# Check for broken libraries currently linked in session
	log_debug(f"Checking {len(bpy.data.libraries)} libraries for missing files", module_name='MissingLinks')
	
	for lib in bpy.data.libraries:
		if lib.filepath and not lib.filepath.startswith('<builtin>'):
			# Resolve and normalize library path
			raw_path = bpy.path.abspath(lib.filepath)
			abs_path = os.path.normpath(raw_path)
			log_debug(f"Checking library: {lib.filepath} -> {abs_path}", module_name='MissingLinks')
			
			if not os.path.exists(abs_path):
				lib_name = os.path.basename(abs_path)
				desc = f"Missing library: {lib_name}"
				det = f"Linked path broken: {lib.filepath}"
				log_debug(f"Found missing library: {desc}", module_name='MissingLinks')
				items.append(RelinkItem('library', desc, det))
			else:
				log_debug(f"Library exists: {abs_path}", module_name='MissingLinks')
	
	return items

def perform_all_relinks():
	"""Perform asset relinking operations and return counts."""
	blend_path = ensure_saved_file()
	if not blend_path:
		return 0, 0
		
	# Gather initial missing links
	initial_items = check_missing_links()
	initial_count = len(initial_items)
	
	# Count initial items by type for detailed logging
	initial_asset_count = sum(1 for it in initial_items if it.item_type == 'asset')
	initial_lib_count = sum(1 for it in initial_items if it.item_type == 'library')
	
	log_debug(f"Initial counts: {initial_asset_count} assets, {initial_lib_count} libraries, {initial_count} total", module_name='MissingLinks')

	# Track what we attempt to fix for accurate success counting
	assets_attempted = initial_asset_count
	libraries_attempted = initial_lib_count
	
	# Perform asset datablock relinking
	log_info("Performing asset datablock relinking...", module_name='MissingLinks')
	asset_processor = AssetRelinkProcessor(main_blend_path=blend_path)
	asset_processor.process_relink()
	
	# Check asset progress after asset relinking (before library relinking)
	after_assets = check_missing_links()
	assets_remaining_after_asset_relink = sum(1 for it in after_assets if it.item_type == 'asset')
	assets_fixed = max(0, initial_asset_count - assets_remaining_after_asset_relink)
		# Perform library file relinking
	log_info("Performing library file relinking...", module_name='MissingLinks')
	lib_processor = LibraryRelinkProcessor(main_blend_path=blend_path)
	lib_processor.process_relink()
	# After fixing libraries, try asset relinking again in case new assets can now be relinked
	log_info("Re-attempting asset datablock relinking after library fixes...", module_name='MissingLinks')
	asset_processor.process_relink()
	
	# Re-check missing links after all relinking
	pending_after = check_missing_links()
	final_count = len(pending_after)
	
	# Count remaining items by type for detailed logging
	remaining_asset = sum(1 for it in pending_after if it.item_type == 'asset')
	remaining_lib = sum(1 for it in pending_after if it.item_type == 'library')
	
	# Calculate libraries fixed (this is straightforward)
	libraries_fixed = max(0, initial_lib_count - remaining_lib)
	
	# Calculate total success based on what we actually attempted to fix
	# Assets fixed is already calculated above
	# Libraries fixed is calculated above
	total_fixed = assets_fixed + libraries_fixed
	
	# Log detailed results
	log_debug(f"Asset relink results: {assets_fixed}/{assets_attempted} fixed", module_name='MissingLinks')
	log_debug(f"Library relink results: {libraries_fixed}/{libraries_attempted} fixed", module_name='MissingLinks')
	log_debug(f"Final counts: {remaining_asset} assets, {remaining_lib} libraries, {final_count} total", module_name='MissingLinks')
	
	# If libraries were fixed but new assets appeared, provide informative logging
	if libraries_fixed > 0 and remaining_asset > initial_asset_count:
		new_assets_revealed = remaining_asset - initial_asset_count
		log_info(f"Fixed {libraries_fixed} library files, but {new_assets_revealed} new missing assets were revealed", module_name='MissingLinks')
	
	log_debug(f"Total success: {total_fixed} items resolved (assets: {assets_fixed}, libraries: {libraries_fixed})", module_name='MissingLinks')
	
	return total_fixed, initial_count


def show_missing_links_dialog():
	"""Show the missing links dialog if items need relinking."""
	global _missing_links_dialog_shown, _consecutive_dialog_count
	
	if _missing_links_dialog_shown:
		return
	
	relink_items = check_missing_links()
	
	if not relink_items:
		log_debug("No missing links found.", module_name='MissingLinks')
		return
	
	try:
		# Reset counter for manual invocations
		_consecutive_dialog_count = 0
		_missing_links_dialog_shown = True
		bpy.ops.bv.missing_links_dialog('INVOKE_DEFAULT')  # type: ignore
	except Exception as e:
		_missing_links_dialog_shown = False
		log_error(f"Failed to invoke missing links dialog: {e}", module_name='MissingLinks')


class BV_OT_MissingLinksDialog(bpy.types.Operator):
	"""Modal dialog to show missing links that require relinking."""
	bl_idname = "bv.missing_links_dialog"
	bl_label = "Missing Links"
	bl_options = {'REGISTER', 'INTERNAL', 'BLOCKING'}
	
	def execute(self, context):
		global _missing_links_dialog_shown, _consecutive_dialog_count
		_missing_links_dialog_shown = False
		
		try:
			success_count, total_count = perform_all_relinks()
			
			# Check current state after relinking to provide better feedback
			remaining_items = check_missing_links()
			remaining_count = len(remaining_items)
			
			# Determine if we should automatically show the dialog again
			should_show_dialog_again = False
			
			if success_count is not None and success_count > 0:
				if remaining_count == 0:
					# All resolved successfully
					if success_count == 1:
						self.report({'INFO'}, "Successfully relinked 1 item")
					else:
						self.report({'INFO'}, f"Successfully relinked all {success_count} items")
				elif remaining_count > total_count:
					# Special case: new missing items appeared (library relinking revealed new assets)
					new_items = remaining_count - total_count
					should_show_dialog_again = True
					if success_count == 1:
						self.report({'INFO'}, f"Relinked 1 item, but {new_items} new missing assets were revealed. Showing dialog for next batch...")
					else:
						self.report({'INFO'}, f"Relinked {success_count} items, but {new_items} new missing assets were revealed. Showing dialog for next batch...")
				else:
					# Some items successfully relinked, some still need attention
					should_show_dialog_again = True
					item_word = "item" if total_count == 1 else "items"
					if success_count == 1:
						self.report({'INFO'}, f"Relinked 1 out of {total_count} {item_word}. Showing dialog for remaining {remaining_count}...")
					else:
						self.report({'INFO'}, f"Relinked {success_count} out of {total_count} {item_word}. Showing dialog for remaining {remaining_count}...")
			else:
				if remaining_count == 0:
					self.report({'INFO'}, "All items were already properly linked")
				elif remaining_count == total_count:
					# Nothing was fixed, don't auto-show dialog again to prevent infinite loop
					self.report({'WARNING'}, f"No items were successfully relinked. {remaining_count} items still need manual attention. Check console for details.")
				else:
					# Some progress was made even though success_count is 0, show dialog again
					should_show_dialog_again = True
					self.report({'INFO'}, f"Some progress made. Showing dialog for remaining {remaining_count} items...")
			
			# Force immediate UI refresh to update the properties panel
			for area in bpy.context.screen.areas:
				if area.type == 'VIEW_3D':
					area.tag_redraw()
			
			# Clear pending items after relinking attempt
			global _pending_relink_items
			_pending_relink_items.clear()
			  # Automatically show dialog again if there are remaining items and we made progress
			if should_show_dialog_again and remaining_count > 0:
				_consecutive_dialog_count += 1
				if _consecutive_dialog_count <= _max_consecutive_dialogs:
					# Use a timer to show the dialog again after a brief delay to allow UI to update
					bpy.app.timers.register(lambda: self._show_dialog_again(), first_interval=0.1)
				else:
					# Reset counter and warn user
					_consecutive_dialog_count = 0
					self.report({'WARNING'}, f"Reached maximum of {_max_consecutive_dialogs} consecutive relink attempts. Check the Blend Vault panel for remaining {remaining_count} items.")
				
		except Exception as e:
			self.report({'ERROR'}, f"Error during relinking: {e}")
			log_error(f"Error during missing links relinking: {e}", module_name='MissingLinks')
		
		return {'FINISHED'}
	
	def _show_dialog_again(self):
		"""Helper method to show the dialog again after a brief delay."""
		global _missing_links_dialog_shown, _consecutive_dialog_count
		
		try:
			# Double-check that items still need relinking
			remaining_items = check_missing_links()
			if remaining_items:
				if not _missing_links_dialog_shown:  # Prevent multiple dialogs
					_missing_links_dialog_shown = True
					bpy.ops.bv.missing_links_dialog('INVOKE_DEFAULT')  # type: ignore
				else:
					# Reset counter if we couldn't show dialog
					_consecutive_dialog_count = max(0, _consecutive_dialog_count - 1)
			else:
				# No more items, reset counter
				_consecutive_dialog_count = 0
		except Exception as e:
			# Reset counter on error
			_consecutive_dialog_count = 0
			log_error(f"Failed to show missing links dialog again: {e}", module_name='MissingLinks')
		return None  # Don't repeat the timer
	def cancel(self, context):
		global _missing_links_dialog_shown, _pending_relink_items, _consecutive_dialog_count
		_missing_links_dialog_shown = False
		_pending_relink_items.clear()
		_consecutive_dialog_count = 0  # Reset counter when user cancels
		log_info("Missing links dialog dismissed. Items may need manual relinking.", module_name='MissingLinks')
		
		# Force immediate UI refresh to update the properties panel
		for area in bpy.context.screen.areas:
			if area.type == 'VIEW_3D':
				area.tag_redraw()
		
		# No return value to satisfy Blender's cancel signature
		return None
	
	def invoke(self, context, event):
		# If this is a fresh invocation (not part of auto-sequence), reset counter
		global _consecutive_dialog_count
		if _consecutive_dialog_count == 0:
			log_debug("Fresh dialog invocation, resetting consecutive counter", module_name='MissingLinks')
		
		# Get missing items and store them for this dialog instance
		items = check_missing_links()
		self._dialog_items = items.copy()
		
		# Don't show dialog if no items need relinking
		if not self._dialog_items:
			global _missing_links_dialog_shown
			_missing_links_dialog_shown = False
			log_debug("No missing links found during invoke, cancelling dialog", module_name='MissingLinks')
			return {'CANCELLED'}
		
		log_debug(f"Dialog invoke storing {len(self._dialog_items)} items for display", module_name='MissingLinks')
		return context.window_manager.invoke_props_dialog(self, width=500)
	
	def draw(self, context):
		layout = self.layout
		
		# Use the items stored during invoke to ensure consistency
		items = getattr(self, '_dialog_items', [])
		
		if not items:
			layout.label(text="All links appear to be valid!")
			log_debug("Dialog draw: no items to display", module_name='MissingLinks')
			return
		
		log_debug(f"Dialog draw: displaying {len(items)} items", module_name='MissingLinks')
		
		# Show summary and first few items
		if len(items) == 1:
			layout.label(text="1 item needs relinking:", icon='ERROR')
		else:
			layout.label(text=f"{len(items)} items need relinking:", icon='ERROR')
		layout.separator()
		# Show the first 10 items
		for item in items[:10]:
			row = layout.row()
			# Show icon based on type
			icon = 'ASSET_MANAGER' if item.item_type == 'asset' else \
				   'LIBRARY_DATA_DIRECT' if item.item_type == 'library' else \
				   'FILE_IMAGE'
			
			col = row.column()
			col.label(text=item.description, icon=icon)
			  # Show details if available
			if item.details:
				sub_row = col.row()
				sub_row.scale_y = 0.8
				sub_row.label(text=item.details)
		
		if len(items) > 10:
			row = layout.row()
			row.label(text=f"... and {len(items) - 10} more items")
		
		layout.separator()
		layout.label(text="Continue with automatic relinking?")


class BV_OT_ManualRelink(bpy.types.Operator):
	"""Manual relink operator that can be triggered from the panel."""
	bl_idname = "bv.manual_relink"
	bl_label = "Relink Missing Items"
	bl_description = "Manually trigger relinking of missing assets and libraries"
	bl_options = {'REGISTER', 'UNDO'}
	
	def execute(self, context):
		global _missing_links_dialog_shown

		# Automatically handle any pending file relocations before relinking
		from .redirect_handler import _pending_relocations
		for old_path, new_path in list(_pending_relocations.items()):
			try:
				# Perform relocation to sync Blender's filepath and sidecar
				bpy.ops.blend_vault.confirm_file_relocation('EXEC_DEFAULT', current_path=old_path, new_path=new_path)
			except Exception:
				pass

		# Check if there are items that need relinking
		items = check_missing_links()
		if not items:
			self.report({'INFO'}, "No missing links detected")
			return {'FINISHED'}
		
		# If dialog is already shown, don't do anything
		if _missing_links_dialog_shown:
			self.report({'WARNING'}, "Missing links dialog is already active")
			return {'CANCELLED'}
		
		# Show the missing links dialog
		try:
			_missing_links_dialog_shown = True
			bpy.ops.bv.missing_links_dialog('INVOKE_DEFAULT')  # type: ignore
			return {'FINISHED'}
		except Exception as e:
			_missing_links_dialog_shown = False
			self.report({'ERROR'}, f"Failed to show missing links dialog: {e}")
			log_error(f"Failed to show missing links dialog from manual trigger: {e}", module_name='MissingLinks')
			return {'CANCELLED'}


class BV_PT_MissingLinksPanel(bpy.types.Panel):
	"""Panel to display missing links status."""
	bl_label = "Missing Links"
	bl_idname = "BV_PT_missing_links"
	bl_space_type = 'VIEW_3D'
	bl_region_type = 'UI'
	bl_category = "Blend Vault"
	
	def draw(self, context):
		global _missing_links_dialog_shown
		layout = self.layout
		# Refresh missing items on each draw
		items = check_missing_links()
		
		if items:
			# Warning icon and title
			row = layout.row()
			row.alert = True
			if len(items) == 1:
				row.label(text="1 item needs relinking:", icon='ERROR')
			else:
				row.label(text=f"{len(items)} items need relinking:", icon='ERROR')

			# Show first few items for reference
			if len(items) > 0:
				box = layout.box()             
				# Show up to 3 items
				for item in items[:3]:
					row = box.row()
					# Icon based on type
					icon = 'ASSET_MANAGER' if item.item_type == 'asset' else \
						   'LIBRARY_DATA_DIRECT' if item.item_type == 'library' else \
						   'FILE_IMAGE'
					row.label(text=item.description, icon=icon)
				
				if len(items) > 3:
					row = box.row()
					row.label(text=f"... and {len(items) - 3} more")
					# Show manual relink button only if dialog is not active
			if not _missing_links_dialog_shown:
				layout.separator()
				row = layout.row()
				row.scale_y = 1.2
				row.operator("bv.manual_relink", text="Relink Missing Items", icon='FILE_REFRESH')
			else:
				row = layout.row()
				row.label(text="Relink dialog is active", icon='INFO')
		else:
			# No relinks needed - clean status
			row = layout.row()
			row.label(text="All links valid", icon='CHECKMARK')

def register():
	"""Register the missing links dialog."""
	bpy.utils.register_class(BV_OT_MissingLinksDialog)
	bpy.utils.register_class(BV_OT_ManualRelink)
	bpy.utils.register_class(BV_PT_MissingLinksPanel)
	log_success("Missing links dialog registered.", module_name='MissingLinks')


def unregister():
	"""Unregister the missing links dialog."""
	bpy.utils.unregister_class(BV_OT_MissingLinksDialog)
	bpy.utils.unregister_class(BV_OT_ManualRelink)
	bpy.utils.unregister_class(BV_PT_MissingLinksPanel)
	log_success("Missing links dialog unregistered.", module_name='MissingLinks')
