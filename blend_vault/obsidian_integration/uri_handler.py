"""
Obsidian URI Handler

This module provides functionality to interact with Obsidian through its URI protocol.
Supports opening sidecar files and other Obsidian actions.
"""

import bpy
import os
import webbrowser
from urllib.parse import quote
from .. import LOG_COLORS, SIDECAR_EXTENSION
from ..utils.helpers import log_info, log_warning, log_error, log_success, log_debug
from ..preferences import get_obsidian_vault_root, detect_obsidian_vault_from_asset_libraries


def build_obsidian_uri(action: str, **params) -> str:
	"""
	Build an Obsidian URI with the given action and parameters.
	
	Args:
		action: The Obsidian action (open, new, search, etc.)
		**params: Additional parameters for the URI
		
	Returns:
		Complete Obsidian URI string
	"""
	uri = f"obsidian://{action}"
	
	if params:
		param_strings = []
		for key, value in params.items():
			if value is not None:
				# URI encode the value
				encoded_value = quote(str(value))
				param_strings.append(f"{key}={encoded_value}")
		
		if param_strings:
			uri += "?" + "&".join(param_strings)
	
	return uri


def open_file_in_obsidian(file_path: str) -> bool:
	"""
	Open a file in Obsidian using the URI protocol.
	
	Args:
		file_path: Absolute path to the file to open
		
	Returns:
		True if the URI was successfully launched, False otherwise
	"""
	if not file_path or not os.path.exists(file_path):
		log_error(f"File does not exist: {file_path}", module_name='ObsidianIntegration')
		return False
		# Use the path parameter to open by absolute path
	uri = build_obsidian_uri("open", path=file_path)
	
	log_debug(f"Opening URI: {uri}", module_name='ObsidianIntegration')
	
	# Check for internet access permission (required for extensions)
	if not bpy.app.online_access:
		log_error("Online access is disabled. Cannot open URIs.", module_name='ObsidianIntegration')
		return False
	
	try:
		# Use webbrowser to open the URI, which should be handled by the OS
		webbrowser.open(uri)
		log_success(f"Successfully opened file: {file_path}", module_name='ObsidianIntegration')
		return True
	except Exception as e:
		log_error(f"Failed to open file in Obsidian: {e}", module_name='ObsidianIntegration')
		return False


def open_current_sidecar_in_obsidian() -> bool:
	"""
	Open the sidecar file for the currently loaded blend file in Obsidian.
	
	Returns:
		True if successful, False otherwise
	"""
	if not bpy.data.is_saved:
		log_warning("Current .blend file is not saved. Cannot open sidecar.", module_name='ObsidianIntegration')
		return False
	
	blend_path = bpy.data.filepath
	sidecar_path = blend_path + SIDECAR_EXTENSION
	
	if not os.path.exists(sidecar_path):
		log_warning(f"Sidecar file not found: {sidecar_path}", module_name='ObsidianIntegration')
		return False
	
	return open_file_in_obsidian(sidecar_path)


class BV_OT_RefreshVaultDetection(bpy.types.Operator):
	"""Refresh Obsidian vault detection from asset libraries"""
	bl_idname = "blend_vault.refresh_vault_detection"
	bl_label = "Refresh Vault Detection"
	bl_description = "Re-check asset libraries for Obsidian vaults"
	bl_options = {'REGISTER'}

	def execute(self, context):
		from ..preferences import refresh_vault_detection
		detected_vault = refresh_vault_detection()
		
		if detected_vault:
			self.report({'INFO'}, f"Found vault at: {detected_vault}")
		else:
			self.report({'WARNING'}, "No vault found in asset libraries")
		
		return {'FINISHED'}


class BV_OT_OpenSidecarInObsidian(bpy.types.Operator):
	"""Open the current blend file's sidecar in Obsidian"""
	bl_idname = "blend_vault.open_sidecar_in_obsidian"
	bl_label = "Open Sidecar in Obsidian"
	bl_description = "Open the sidecar file for the current blend file in Obsidian"
	bl_options = {'REGISTER'}

	@classmethod
	def poll(cls, context):
		"""Only enable if blend file is saved and sidecar exists"""
		if not bpy.data.is_saved:
			return False
		
		blend_path = bpy.data.filepath
		sidecar_path = blend_path + SIDECAR_EXTENSION
		return os.path.exists(sidecar_path)

	def execute(self, context):
		success = open_current_sidecar_in_obsidian()
		
		if success:
			self.report({'INFO'}, "Opened sidecar in Obsidian")
			return {'FINISHED'}
		else:
			self.report({'ERROR'}, "Failed to open sidecar in Obsidian")
			return {'CANCELLED'}


class BV_PT_ObsidianIntegrationPanel(bpy.types.Panel):
	"""Panel for Obsidian integration features"""
	bl_label = "Obsidian Integration"
	bl_idname = "BV_PT_obsidian_integration"
	bl_space_type = 'VIEW_3D'
	bl_region_type = 'UI'
	bl_category = "Blend Vault"
	def draw(self, context):
		layout = self.layout
				# Vault Status Section
		vault_box = layout.box()
		header_row = vault_box.row()
		header_row.label(text="Vault Status:", icon='FILE_FOLDER')
		header_row.operator("blend_vault.refresh_vault_detection", text="", icon='FILE_REFRESH')
		
		vault_root = get_obsidian_vault_root(context)
		detected_vault = detect_obsidian_vault_from_asset_libraries()
		
		if detected_vault:
			if vault_root == detected_vault:
				# Auto-detected vault is being used
				status_row = vault_box.row()
				status_row.label(text="Auto-detected", icon='CHECKMARK')
				vault_box.label(text=f"Path: {detected_vault}")
			else:
				# Manual override is being used
				status_row = vault_box.row()
				status_row.label(text="Manual override", icon='SETTINGS')
				vault_box.label(text=f"Path: {vault_root}")
		elif vault_root:
			# Only manual vault set
			status_row = vault_box.row()
			status_row.label(text="Manual", icon='SETTINGS')
			vault_box.label(text=f"Path: {vault_root}")
		else:
			# No vault configured
			status_row = vault_box.row()
			status_row.label(text="Not configured", icon='ERROR')
			vault_box.label(text="Set vault in preferences or add as asset library")
		
		layout.separator()
		
		# Sidecar Section
		# Check if sidecar exists for current file
		if bpy.data.is_saved:
			blend_path = bpy.data.filepath
			sidecar_path = blend_path + SIDECAR_EXTENSION
			
			if os.path.exists(sidecar_path):
				layout.operator("blend_vault.open_sidecar_in_obsidian", icon='CURRENT_FILE')
			else:
				row = layout.row()
				row.enabled = False
				row.label(text="No sidecar found")
		else:
			row = layout.row()
			row.enabled = False
			row.label(text="Please save the file first.")


def _safe_unregister_class(cls):
	"""Safely unregister a class, handling cases where it might not be registered."""
	try:
		bpy.utils.unregister_class(cls)
		log_debug(f"Unregistered class: {cls.__name__}", module_name='ObsidianIntegration')
	except RuntimeError:  # This typically means it wasn't registered or already unregistered.
		log_debug(f"Class {cls.__name__} was not registered or already unregistered.", module_name='ObsidianIntegration')
	except Exception as e:
		log_error(f"Unexpected error unregistering class {cls.__name__}: {e}", module_name='ObsidianIntegration')


def register():
	# Register classes
	bpy.utils.register_class(BV_OT_RefreshVaultDetection)
	bpy.utils.register_class(BV_OT_OpenSidecarInObsidian)
	bpy.utils.register_class(BV_PT_ObsidianIntegrationPanel)
	log_success("URI handler module registered.", module_name='ObsidianIntegration')


def unregister():
    # --- BEGIN LINGERING HANDLER CLEANUP ---
    # This section is to clean up a potentially lingering depsgraph handler
    # named '_ui_refresh_handler' from previous versions of this script.
    log_info("Obsidian Integration: Attempting to clean up potential lingering UI handlers...", module_name='ObsidianIntegration')
    
    handler_name_to_remove = "_ui_refresh_handler"
    handlers_found_for_removal = []

    # Check depsgraph_update_post handlers
    if hasattr(bpy.app, "handlers") and hasattr(bpy.app.handlers, "depsgraph_update_post"):
        for handler_func in bpy.app.handlers.depsgraph_update_post[:]: # Iterate a copy
            if hasattr(handler_func, '__name__') and handler_func.__name__ == handler_name_to_remove:
                handlers_found_for_removal.append(handler_func)
        
        if handlers_found_for_removal:
            for handler_to_remove in handlers_found_for_removal:
                try:
                    bpy.app.handlers.depsgraph_update_post.remove(handler_to_remove)
                    log_info(f"Obsidian Integration: Successfully removed lingering depsgraph handler: {handler_name_to_remove} ({handler_to_remove})", module_name='ObsidianIntegration')
                except Exception as e:
                    log_warning(f"Obsidian Integration: Could not remove lingering depsgraph handler {handler_name_to_remove} ({handler_to_remove}): {e}", module_name='ObsidianIntegration')
        else:
            log_info(f"Obsidian Integration: No lingering depsgraph handler named '{handler_name_to_remove}' found in depsgraph_update_post.", module_name='ObsidianIntegration')
    else:
        log_warning("Obsidian Integration: bpy.app.handlers.depsgraph_update_post not available for cleanup.", module_name='ObsidianIntegration')
    
    # Add similar cleanup for timers if one was suspected by name, e.g.:
    # timer_callback_name_to_remove = "_delayed_refresh_callback" 
    # However, timer removal without the exact function object is difficult.
    # For now, focusing on the depsgraph handler.
    
    log_info("Obsidian Integration: Lingering handler cleanup attempt finished.", module_name='ObsidianIntegration')
    # --- END LINGERING HANDLER CLEANUP ---

    _safe_unregister_class(BV_PT_ObsidianIntegrationPanel)
    _safe_unregister_class(BV_OT_OpenSidecarInObsidian)
    _safe_unregister_class(BV_OT_RefreshVaultDetection)
    log_warning("URI handler module unregistered.", module_name='ObsidianIntegration')
