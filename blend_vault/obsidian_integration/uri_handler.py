"""
Obsidian URI Handler

This module provides functionality to interact with Obsidian through its URI protocol.
Supports opening sidecar files and other Obsidian actions.
"""

import bpy  # type: ignore
import os
import webbrowser
from urllib.parse import quote
from .. import LOG_COLORS, SIDECAR_EXTENSION


def _log(level: str, message: str) -> None:
	"""Simplified logging function"""
	print(f"{LOG_COLORS.get(level, '')}{message}{LOG_COLORS['RESET']}")


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
		_log('ERROR', f"[Blend Vault][Obsidian] File does not exist: {file_path}")
		return False
		# Use the path parameter to open by absolute path
	uri = build_obsidian_uri("open", path=file_path)
	
	_log('DEBUG', f"[Blend Vault][Obsidian] Opening URI: {uri}")
	
	# Check for internet access permission (required for extensions)
	if not bpy.app.online_access:
		_log('ERROR', "[Blend Vault][Obsidian] Online access is disabled. Cannot open URIs.")
		return False
	
	try:
		# Use webbrowser to open the URI, which should be handled by the OS
		webbrowser.open(uri)
		_log('SUCCESS', f"[Blend Vault][Obsidian] Successfully opened file: {file_path}")
		return True
	except Exception as e:
		_log('ERROR', f"[Blend Vault][Obsidian] Failed to open file in Obsidian: {e}")
		return False


def open_current_sidecar_in_obsidian() -> bool:
	"""
	Open the sidecar file for the currently loaded blend file in Obsidian.
	
	Returns:
		True if successful, False otherwise
	"""
	if not bpy.data.is_saved:
		_log('WARN', "[Blend Vault][Obsidian] Current .blend file is not saved. Cannot open sidecar.")
		return False
	
	blend_path = bpy.data.filepath
	sidecar_path = blend_path + SIDECAR_EXTENSION
	
	if not os.path.exists(sidecar_path):
		_log('WARN', f"[Blend Vault][Obsidian] Sidecar file not found: {sidecar_path}")
		return False
	
	return open_file_in_obsidian(sidecar_path)


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
		_log('DEBUG', f"[Blend Vault][Obsidian] Unregistered class: {cls.__name__}")
	except RuntimeError:  # This typically means it wasn't registered or already unregistered.
		_log('DEBUG', f"[Blend Vault][Obsidian] Class {cls.__name__} was not registered or already unregistered.")
	except Exception as e:
		_log('ERROR', f"[Blend Vault][Obsidian] Unexpected error unregistering class {cls.__name__}: {e}")


def register():
	# Register classes
	bpy.utils.register_class(BV_OT_OpenSidecarInObsidian)
	bpy.utils.register_class(BV_PT_ObsidianIntegrationPanel)
	_log('SUCCESS', "[Blend Vault][Obsidian] URI handler module registered.")


def unregister():
    # --- BEGIN LINGERING HANDLER CLEANUP ---
    # This section is to clean up a potentially lingering depsgraph handler
    # named '_ui_refresh_handler' from previous versions of this script.
    _log('INFO', "Obsidian Integration: Attempting to clean up potential lingering UI handlers...")
    
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
                    _log('INFO', f"Obsidian Integration: Successfully removed lingering depsgraph handler: {handler_name_to_remove} ({handler_to_remove})")
                except Exception as e:
                    _log('WARN', f"Obsidian Integration: Could not remove lingering depsgraph handler {handler_name_to_remove} ({handler_to_remove}): {e}")
        else:
            _log('INFO', f"Obsidian Integration: No lingering depsgraph handler named '{handler_name_to_remove}' found in depsgraph_update_post.")
    else:
        _log('WARN', "Obsidian Integration: bpy.app.handlers.depsgraph_update_post not available for cleanup.")
    
    # Add similar cleanup for timers if one was suspected by name, e.g.:
    # timer_callback_name_to_remove = "_delayed_refresh_callback" 
    # However, timer removal without the exact function object is difficult.
    # For now, focusing on the depsgraph handler.
    
    _log('INFO', "Obsidian Integration: Lingering handler cleanup attempt finished.")
    # --- END LINGERING HANDLER CLEANUP ---

    _safe_unregister_class(BV_PT_ObsidianIntegrationPanel)
    _safe_unregister_class(BV_OT_OpenSidecarInObsidian)
    _log('WARN', "[Blend Vault][Obsidian] URI handler module unregistered.")
