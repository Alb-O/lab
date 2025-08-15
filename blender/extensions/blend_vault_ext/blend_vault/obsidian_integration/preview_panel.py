# Preview image UI panel for Blend Vault

import bpy
import os
from ..utils.constants import PREVIEW_EXTENSION


def draw_preview_panel_section(layout, context):
	"""
	Draw the preview image section of a panel.
	
	Args:
		layout: The layout to draw into
		context: The Blender context
	"""
	# Preview Image Section
	preview_box = layout.box()
	
	if not bpy.data.is_saved:
		preview_box.label(text="Save .blend file to manage preview.")
	else:
		blend_filepath = bpy.data.filepath
		# Construct preview PNG path using constant
		base, _ = os.path.splitext(blend_filepath)
		preview_png_path = base + PREVIEW_EXTENSION

		if os.path.exists(preview_png_path):
			preview_box.label(text="Preview image exists on disk.", icon='CHECKMARK')
			col = preview_box.column(align=True)
			col.scale_y = 1.1
			# Update button
			col.operator("blendvault.save_preview_to_file", text="Update Image", icon='FILE_REFRESH')
			# Open in default app button
			col.operator("wm.path_open", text="Open Image", icon='IMAGE_DATA').filepath = preview_png_path
			# Remove button
			col.operator("blendvault.remove_preview_image", text="Remove Image", icon='TRASH')
		else:
			preview_box.label(text="No preview image found on disk.", icon='ERROR')

			row = preview_box.row()
			row.scale_y = 1.2
			# Save button
			row.operator("blendvault.save_preview_to_file", text="Save Preview Image", icon='IMAGE_DATA')


class BV_PT_PreviewPanel(bpy.types.Panel):
	"""Standalone preview image panel"""
	bl_label = "Preview Image"
	bl_idname = "BV_PT_preview_panel"
	bl_space_type = 'PROPERTIES'
	bl_region_type = 'WINDOW'
	bl_context = "output"  # Place in Output Properties
	bl_options = {'DEFAULT_CLOSED'}
	
	def draw(self, context):
		layout = self.layout
		draw_preview_panel_section(layout, context)


# Registration
classes = [
	BV_PT_PreviewPanel,
]


def register():
	for cls in classes:
		bpy.utils.register_class(cls)


def unregister():
	for cls in classes:
		bpy.utils.unregister_class(cls)


if __name__ == "__main__":
	register()
