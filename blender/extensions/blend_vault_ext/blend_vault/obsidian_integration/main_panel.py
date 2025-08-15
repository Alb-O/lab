# Main Obsidian integration panel that orchestrates all components

import bpy
from ..core import log_success, log_warning, log_error, log_info, log_debug
from .uri_panel import draw_vault_status_section
from .preview_panel import draw_preview_panel_section


class BV_PT_ObsidianIntegrationPanel(bpy.types.Panel):
	"""Main panel for Obsidian integration features"""
	bl_label = "Obsidian Integration"
	bl_idname = "BV_PT_obsidian_integration"
	bl_space_type = 'VIEW_3D'
	bl_region_type = 'UI'
	bl_category = "Blend Vault"
	
	def draw(self, context):
		layout = self.layout
		
		# Vault Status Section
		draw_vault_status_section(layout, context)
		
		layout.separator()
		
		# Preview Image Section
		draw_preview_panel_section(layout, context)


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
	# Register main panel
	bpy.utils.register_class(BV_PT_ObsidianIntegrationPanel)
	log_success("Main panel registered.", module_name='ObsidianIntegration')


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
	
	log_info("Obsidian Integration: Lingering handler cleanup attempt finished.", module_name='ObsidianIntegration')
	# --- END LINGERING HANDLER CLEANUP ---

	_safe_unregister_class(BV_PT_ObsidianIntegrationPanel)
	log_warning("Main panel unregistered.", module_name='ObsidianIntegration')
