"""
UI components for file relocation handling.
Contains panels and operators for managing file relocations through the N-panel interface.
"""

import os
import bpy
from ..core import log_info, log_success, log_error


class BV_PT_FileRelocationPanel(bpy.types.Panel):
	"""Panel in N-panel to show file relocation status"""
	bl_label = "File Relocation"
	bl_idname = "BV_PT_file_relocation"
	bl_space_type = 'VIEW_3D'
	bl_region_type = 'UI'
	bl_category = "Blend Vault"
	@classmethod
	def poll(cls, context):
		# Import here to avoid circular imports
		from .redirect_handler import get_pending_relocations
		# Show panel when there are pending relocations
		return bool(get_pending_relocations())
	
	def draw(self, context):
		# Import here to avoid circular imports
		from .redirect_handler import get_pending_relocations, _format_display_path, remove_from_pending_relocations
		
		layout = self.layout
		
		current_blend = bpy.data.filepath
		pending_relocations = get_pending_relocations()
		
		if current_blend and current_blend in pending_relocations:
			new_path_abs = pending_relocations[current_blend]
			
			# Display paths using the simplified format
			current_path_to_display = _format_display_path(current_blend)
			new_path_to_display = _format_display_path(new_path_abs)
			
			if new_path_abs and os.path.exists(new_path_abs):
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
				remove_from_pending_relocations(current_blend)
				layout.label(text="No pending relocations.", icon='CHECKMARK')
		else:
			layout.label(text="No pending relocations.", icon='CHECKMARK')


class BV_OT_IgnoreRelocation(bpy.types.Operator):
	"""Operator to ignore relocation for this session"""
	bl_idname = "blend_vault.ignore_relocation"
	bl_label = "Ignore Relocation"
	bl_options = {'REGISTER', 'INTERNAL'}
	def execute(self, context):
		# Import here to avoid circular imports
		from .redirect_handler import (
			add_to_ignored_relocations, 
			remove_from_pending_relocations, 
			cleanup_redirect_file,
			get_pending_relocations
		)
		
		current_blend = bpy.data.filepath
		pending_relocations = get_pending_relocations()
		
		if current_blend in pending_relocations:
			# Add to ignored relocations
			add_to_ignored_relocations(current_blend)
			# Clean up redirect file
			cleanup_redirect_file(current_blend)
			# Remove from pending
			remove_from_pending_relocations(current_blend)
			
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
		# Import here to avoid circular imports
		from .redirect_handler import (
			set_relocation_dialog_shown,
			remove_from_pending_relocations,
			remove_from_ignored_relocations,
			cleanup_redirect_file
		)
		
		set_relocation_dialog_shown(False)
		try:
			# Save to new location
			bpy.ops.wm.save_as_mainfile(filepath=self.new_path)
			# Cleanup redirect file and remove from pending
			cleanup_redirect_file(self.current_path)
			remove_from_pending_relocations(self.current_path)
			# Remove from ignored list since relocation was handled
			remove_from_ignored_relocations(self.current_path)
			
			# Refresh UI to update panel
			for area in bpy.context.screen.areas:
				if area.type == 'VIEW_3D':
					area.tag_redraw()
			
			log_success(f"File relocated to {self.new_path}", module_name='RedirectHandler')
		except Exception as e:
			log_error(f"Error during relocation: {e}", module_name='RedirectHandler')
		return {'FINISHED'}
	def cancel(self, context):
		# Import here to avoid circular imports
		from .redirect_handler import (
			set_relocation_dialog_shown,
			get_pending_relocations,
			add_to_pending_relocations,
			add_to_ignored_relocations
		)
		
		set_relocation_dialog_shown(False)
		
		# Ensure the relocation is added to pending relocations
		pending_relocations = get_pending_relocations()
		if self.current_path not in pending_relocations:
			add_to_pending_relocations(self.current_path, self.new_path)
		
		# Add to ignored relocations to prevent immediate re-prompt
		add_to_ignored_relocations(self.current_path)
		
		# Refresh UI to show the pending relocation in the N-panel
		for area in bpy.context.screen.areas:
			if area.type == 'VIEW_3D':
				area.tag_redraw()
		
		log_info("File relocation dialog dismissed. Use N-panel to handle.", module_name='RedirectHandler')
	
	def invoke(self, context, event):
		return context.window_manager.invoke_props_dialog(self, width=450)

	def draw(self, context):
		# Import here to avoid circular imports
		from .redirect_handler import _format_display_path
		
		layout = self.layout
		
		current_display_path = _format_display_path(self.current_path)
		new_display_path = _format_display_path(self.new_path)
		layout.label(text="The file has been moved or renamed externally.")
		layout.separator()
		layout.label(text=f"Current location: {current_display_path}")
		layout.label(text=f"New location: {new_display_path}")
		layout.separator()
		layout.label(text="Save the current file to the new location?")


def register():
	"""Register UI classes"""
	bpy.utils.register_class(BV_OT_ConfirmFileRelocation)
	bpy.utils.register_class(BV_PT_FileRelocationPanel)
	bpy.utils.register_class(BV_OT_IgnoreRelocation)
	
	log_info("Redirect panel UI registered with category 'Blend Vault'", module_name='RedirectHandler')


def unregister():
	"""Unregister UI classes"""
	bpy.utils.unregister_class(BV_OT_ConfirmFileRelocation)
	bpy.utils.unregister_class(BV_PT_FileRelocationPanel)
	bpy.utils.unregister_class(BV_OT_IgnoreRelocation)
	
	log_info("Redirect panel UI unregistered", module_name='RedirectHandler')
