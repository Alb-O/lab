# Preview image generation utilities for Blend Vault.

import bpy
import os
from typing import Optional, Tuple
from ..utils.constants import PREVIEW_EXTENSION

try:
	from PIL import Image
	PIL_AVAILABLE = True
except ImportError:
	PIL_AVAILABLE = False
	print("Blend Vault ERROR: Pillow (PIL) library not found even after wheel extraction.")
	print("Blend Vault ERROR: This should not happen if the extension was built properly.")


def save_blend_preview_to_png(blend_filepath: str, output_png_path: str) -> bool:
	import bpy # Keep local import as per user findings
	"""
	Extracts the preview image of the specified .blend file
	and saves it as a PNG image to the output_png_path.

	Args:
		blend_filepath (str): Path to the .blend file.
		output_png_path (str): Path to save the PNG image.

	Returns:
		bool: True if successful, False otherwise.
	"""
	if not PIL_AVAILABLE:
		print("Error: Pillow (PIL) library is required to save images to disk but is not installed.")
		return False

	if not blend_filepath or not os.path.exists(blend_filepath):
		print(f"Error: Blend file not found at {blend_filepath}")
		return False
	try:
		import bpy.utils.previews
	except ImportError:
		print("Error: Could not import bpy.utils.previews.")
		return False
	
	pcoll = bpy.utils.previews.new()
	preview_name = os.path.basename(blend_filepath) + "_preview"
	
	try:
		preview = pcoll.load(preview_name, blend_filepath, 'BLEND', force_reload=True)
		
		if not preview:
			print(f"Could not load preview for {blend_filepath}.")
			return False # pcoll removal is in finally
		
		if preview.image_size[0] <= 0 or preview.image_size[1] <= 0:
			print("No preview image data found (dimensions are 0).")
			return False # pcoll removal is in finally
		
		width, height = preview.image_size
		pixel_data_bytes = None

		# Try to get float pixel data first (RGBA format, values 0.0-1.0)
		if hasattr(preview, 'image_pixels_float') and len(preview.image_pixels_float) == width * height * 4:
			print("Using image_pixels_float data.")
			
			# Fastest approach: Use numpy-style vectorized operations if available
			try:
				# Try to import numpy for fast operations (might not be available)
				import numpy as np
				float_array = np.array(preview.image_pixels_float, dtype=np.float32)
				# Clamp to 0-1 range and scale to 0-255 in one operation
				byte_array = np.clip(float_array * 255.0, 0, 255).astype(np.uint8)
				pixel_data_bytes = bytearray(byte_array.tobytes())
				print(f"NumPy conversion: {len(preview.image_pixels_float)} floats -> {len(pixel_data_bytes)} bytes")
			except ImportError:
				# NumPy not available, use optimized pure Python approach
				print("NumPy not available, using optimized Python conversion")
				# Use memoryview for faster access
				float_data = memoryview(preview.image_pixels_float)
				# Pre-allocate the bytearray for better performance
				pixel_data_bytes = bytearray(len(float_data))
				
				# Convert in chunks for better cache performance
				chunk_size = 4096  # Process 4KB at a time
				for i in range(0, len(float_data), chunk_size):
					end_idx = min(i + chunk_size, len(float_data))
					for j in range(i, end_idx):
						pixel_data_bytes[j] = min(255, max(0, int(float_data[j] * 255.999)))
				
				print(f"Optimized Python conversion: {len(float_data)} floats -> {len(pixel_data_bytes)} bytes")
		# Fallback to integer pixel data (values 0-255)
		elif hasattr(preview, 'image_pixels') and len(preview.image_pixels) == width * height:
			print("Using image_pixels data (already bytes).")
			pixel_data_bytes = bytearray()
			for p_obj in preview.image_pixels: # p_obj is a an object with r,g,b,a attributes
				pixel_data_bytes.extend([p_obj.r, p_obj.g, p_obj.b, p_obj.a])
		else:
			print("Error: Could not find suitable pixel data in preview.")
			return False # pcoll removal is in finally

		if not pixel_data_bytes:
			print("Error: Pixel data processing failed.")
			return False

		# Create PIL Image
		pil_image = Image.frombytes('RGBA', (width, height), bytes(pixel_data_bytes))
		
		# Fix upside-down issue: Blender stores images bottom-to-top, PIL expects top-to-bottom
		pil_image = pil_image.transpose(Image.FLIP_TOP_BOTTOM)
		
		# Save to disk
		pil_image.save(output_png_path)
		print(f"Successfully saved preview to {output_png_path}")
		return True
		
	except RuntimeError as re:
		print(f"RuntimeError processing preview for {blend_filepath}: {re}")
		return False
	except Exception as e:
		print(f"Error processing preview for {blend_filepath}: {e}")
		return False
	finally:
		if 'pcoll' in locals() and pcoll: # Ensure pcoll exists before trying to remove
			bpy.utils.previews.remove(pcoll)


# Operator class for UI integration
class BLENDVAULT_OT_save_preview_to_file(bpy.types.Operator):
	"""Save current file's preview image to a PNG file"""
	bl_idname = "blendvault.save_preview_to_file"
	bl_label = "Save Preview to PNG"
	bl_description = "Extract and save the current .blend file's preview image as a PNG file next to it"
	bl_options = {'REGISTER', 'UNDO'}
	
	@classmethod
	def poll(cls, context):
		return bpy.data.filepath != "" and PIL_AVAILABLE # Only if file is saved and PIL is available
	
	def execute(self, context):
		current_blend_filepath = bpy.data.filepath
		if not current_blend_filepath:
			self.report({'ERROR'}, "Current file is not saved.")
			return {'CANCELLED'}		# Construct output path: e.g., /path/to/file.blend -> /path/to/file.blend.preview.png
		base, ext = os.path.splitext(current_blend_filepath)
		output_png_path = base + PREVIEW_EXTENSION
		
		success = save_blend_preview_to_png(current_blend_filepath, output_png_path)
		
		if success:
			self.report({'INFO'}, f"Preview image saved to: {output_png_path}")
			return {'FINISHED'}
		else:
			self.report({'ERROR'}, f"Failed to save preview image to {output_png_path}")
			return {'CANCELLED'}


class BLENDVAULT_OT_remove_preview_image(bpy.types.Operator):
	"""Remove the preview image PNG file from disk"""
	bl_idname = "blendvault.remove_preview_image"
	bl_label = "Remove Preview Image"
	bl_description = "Deletes the .blend.preview.png file associated with the current .blend file"
	bl_options = {'REGISTER', 'UNDO'}
	@classmethod
	def poll(cls, context):
		if not bpy.data.is_saved:
			return False
		blend_filepath = bpy.data.filepath
		base, _ = os.path.splitext(blend_filepath)
		preview_png_path = base + PREVIEW_EXTENSION
		return os.path.exists(preview_png_path)

	def execute(self, context):
		current_blend_filepath = bpy.data.filepath
		base, _ = os.path.splitext(current_blend_filepath)
		preview_png_path = base + PREVIEW_EXTENSION

		if os.path.exists(preview_png_path):
			try:
				os.remove(preview_png_path)
				self.report({'INFO'}, f"Preview image removed: {preview_png_path}")
				return {'FINISHED'}
			except OSError as e:
				self.report({'ERROR'}, f"Could not remove preview image: {e}")
				return {'CANCELLED'}
		else:
			self.report({'WARNING'}, "Preview image not found, nothing to remove.")
			return {'CANCELLED'}

# Registration
classes = [
	BLENDVAULT_OT_save_preview_to_file,
	BLENDVAULT_OT_remove_preview_image, # Added back
]


def register():
	# global bpy # Not needed here
	for cls in classes:
		bpy.utils.register_class(cls)


def unregister():
	# global bpy # Not needed here
	for cls in classes:
		bpy.utils.unregister_class(cls)


if __name__ == "__main__":
	# This block is for testing, ensure PIL is available or handle appropriately
	if PIL_AVAILABLE:
		register()
		# Add a test call here if needed, e.g.
		# if bpy.data.filepath:
		#     base, ext = os.path.splitext(bpy.data.filepath)
		#     test_output_path = base + "_test_preview.png"
		#     save_blend_preview_to_png(bpy.data.filepath, test_output_path)
		# else:
		#     print("Save a .blend file to test preview saving.")
	else:
		print("Cannot register preview saving operator: Pillow (PIL) library not found.")