"""
HDiff Tool - Handlers for file operations
"""

import bpy

def hdiff_save_pre_handler(dummy):
	"""Handler to warn user before saving when viewing a previous version"""
	try:
		if not bpy.data.filepath:
			return
		
		# Check if we're viewing a previous version
		scene = bpy.context.scene
		if not hasattr(scene, 'hdiff_current_version_index'):
			return
			
		from .core import get_metadata_filepath, load_metadata
		
		metadata_filepath = get_metadata_filepath(bpy.data.filepath)
		metadata = load_metadata(metadata_filepath)
		
		if not metadata:
			return
			
		idx = scene.hdiff_current_version_index
		latest_version_index = len(metadata) - 1
		
		if 0 <= idx < latest_version_index:
			# User is trying to save while viewing a previous version
			# Show a warning (this is just a notification, we don't prevent the save)
			print(f"HDiff Tool WARNING: Saving while viewing version {idx} (not latest version {latest_version_index})")
			print("HDiff Tool: This will overwrite your working file with the previous version content!")
			
	except Exception as e:
		print(f"HDiff Tool: Error in save pre-handler: {e}")

# --- Registration ---
def register():
	"""Register handlers"""
	if hdiff_save_pre_handler not in bpy.app.handlers.save_pre:
		bpy.app.handlers.save_pre.append(hdiff_save_pre_handler)
	print("HDiff Tool: Handlers module registered")

def unregister():
	"""Unregister handlers"""
	if hdiff_save_pre_handler in bpy.app.handlers.save_pre:
		bpy.app.handlers.save_pre.remove(hdiff_save_pre_handler)
	print("HDiff Tool: Handlers module unregistered")
