"""
Obsidian URI Handler

This module provides core functionality to interact with Obsidian through its URI protocol.
Contains URI building functions and related operators.
"""

import bpy
import os
import webbrowser
from urllib.parse import quote
from .. import LOG_COLORS, SIDECAR_EXTENSION
from ..core import log_info, log_warning, log_error, log_success, log_debug
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


def register():
	# Register classes
	bpy.utils.register_class(BV_OT_RefreshVaultDetection)
	bpy.utils.register_class(BV_OT_OpenSidecarInObsidian)
	log_success("URI handler operators registered.", module_name='ObsidianIntegration')


def unregister():
	bpy.utils.unregister_class(BV_OT_OpenSidecarInObsidian)
	bpy.utils.unregister_class(BV_OT_RefreshVaultDetection)
	log_warning("URI handler operators unregistered.", module_name='ObsidianIntegration')
