"""
UI Panels for HDiff Tool Extension
"""

import bpy
import os
from .core import load_metadata, get_metadata_filepath
from .utils import resolve_blend_filepath_for_metadata
from .preferences import get_preferences
from .operators import HDIFF_OT_CreatePatch, HDIFF_OT_go_to_version


class HDIFF_PT_PatchPanel(bpy.types.Panel):
	bl_label = "Differential Patching"
	bl_idname = "VIEW3D_PT_hdiff_patcher"
	bl_space_type = 'VIEW_3D'
	bl_region_type = 'UI'
	bl_category = "HDiff Tool"

	def draw(self, context):
		layout = self.layout
		scene = context.scene
		prefs = get_preferences()

		if not bpy.data.filepath:
			layout.label(text="Save the .blend file first to enable patching features.")
			return
		# Tool Configuration
		box_config = layout.box()
		box_config.label(text="Tool Configuration:")
		
		try:
			prefs = get_preferences()
			hdiffz_path = prefs.hdiffz_path if prefs else ""
			hpatchz_path = prefs.hpatchz_path if prefs else ""
		except Exception as e:
			box_config.label(text="Error accessing preferences", icon='ERROR')
			box_config.label(text="Please restart Blender or reinstall extension")
			print(f"HDiff Tool: Preferences error: {e}")
			return
		
		if not hdiffz_path or not os.path.exists(hdiffz_path):
			box_config.label(text="hdiffz.exe not found!", icon='ERROR')
			box_config.label(text="Configure path in addon preferences")
		else:
			box_config.label(text="hdiffz.exe: Found", icon='CHECKMARK')
			
		if not hpatchz_path or not os.path.exists(hpatchz_path):
			box_config.label(text="hpatchz.exe not found!", icon='ERROR')
			box_config.label(text="Configure path in addon preferences")
		else:
			box_config.label(text="hpatchz.exe: Found", icon='CHECKMARK')

		col = layout.column(align=True)
		# Patch Creation Section
		box_create = col.box()
		# Check patch status using core functions
		# Resolve to original file if we're viewing a preview
		resolved_filepath = resolve_blend_filepath_for_metadata(bpy.data.filepath)
		metadata_filepath = get_metadata_filepath(resolved_filepath)
		metadata = load_metadata(metadata_filepath)
		idx = scene.hdiff_current_version_index
		# Handle header and comment field based on current version
		if metadata and 0 <= idx < len(metadata):
			latest_version_index = len(metadata) - 1
			is_on_latest_version = (idx == latest_version_index)
			
			if is_on_latest_version:
				# On latest version - show normal patch creation UI
				box_create.label(text="Create New Patch:")
				box_create.prop(scene, "hdiff_patch_comment", text="Comment")
			else:
				# On previous version - show previous patch info
				box_create.label(text="Previous Patch Open:")
				current_entry = metadata[idx]
				current_comment = current_entry.get('comment', '(No comment)')
				if not current_comment and idx == 0:
					current_comment = "(Initial Version)"
				
				# Show the comment as a label instead of trying to modify the property
				box_create.label(text=f"Comment: {current_comment}")
		else:
			# No metadata - show initial version creation UI
			box_create.label(text="Create Initial Version:")
			box_create.prop(scene, "hdiff_patch_comment", text="Comment")

		if not metadata:
			op_text = "Create Initial Version"
		else:
			op_text = "Create Patch"
			if 0 <= idx < len(metadata) - 1:
				box_create.label(text="Cannot create patch: not on latest version", icon='ERROR')
		
		box_create.operator(HDIFF_OT_CreatePatch.bl_idname, text=op_text, icon='ADD')
		
		# Current Version Info
		box_current = col.box()
		if metadata and 0 <= idx < len(metadata):
			box_current.label(text="Current Version Info:")
			current_entry = metadata[idx]
			box_current.label(text=f"Version: {idx}")
			box_current.label(text=f"Timestamp: {current_entry.get('timestamp', 'N/A')}")
			box_current.label(text=f"Comment: {current_entry.get('comment', 'N/A')}")
			signature_text = current_entry.get('to_signature', 'N/A')
			box_current.label(text=f"Signature: {signature_text}")
		else:
			box_current.label(text="Current Version Info:")
			box_current.label(text="No version information available.")

		# Version History
		box_history = col.box()
		box_history.label(text="Version History (Newest First):")
		if metadata:
			for i in range(len(metadata) - 1, -1, -1):
				entry = metadata[i]
				row = box_history.row(align=True)
				entry_idx = entry.get('version_index', i)
				
				timestamp = entry.get('timestamp', '').split('T')[0] if entry.get('timestamp') else 'No Date'
				comment = entry.get('comment', '')
				if not comment and entry_idx == 0:
					comment = "(Initial Version)"
				elif not comment:
					comment = "(No Comment)"
				version_label = f"V{entry_idx}: {timestamp} - {comment[:30]}"
				if len(comment) > 30:
					version_label += "..."
				
				# Create operator and set target_version_index property
				op = row.operator(HDIFF_OT_go_to_version.bl_idname, text=version_label)
				op.target_version_index = entry_idx
				
				if entry_idx == scene.hdiff_current_version_index:
					row.enabled = False
		else:
			box_history.label(text="No patch history found.")


# Future panels for preview thumbnails can be added here
class HDIFF_PT_PreviewPanel(bpy.types.Panel):
	"""Panel for preview thumbnail support (future implementation)"""
	bl_label = "Preview Thumbnails"
	bl_idname = "VIEW3D_PT_hdiff_preview"
	bl_space_type = 'VIEW_3D'
	bl_region_type = 'UI'
	bl_category = "HDiff Tool"
	bl_parent_id = "VIEW3D_PT_hdiff_patcher"
	bl_options = {'DEFAULT_CLOSED'}

	def draw(self, context):
		layout = self.layout
		layout.label(text="Preview thumbnail support coming soon...")


# Register classes
classes = (
	HDIFF_PT_PatchPanel,
	HDIFF_PT_PreviewPanel,
)

def register():
	for cls in classes:
		bpy.utils.register_class(cls)

def unregister():
	for cls in reversed(classes):
		bpy.utils.unregister_class(cls)
