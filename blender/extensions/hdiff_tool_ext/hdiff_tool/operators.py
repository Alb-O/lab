"""
HDiff Tool - Operators for patch management
"""

import bpy
import os
import time
from .core import (
	get_metadata_filepath, load_metadata, cleanup_old_preview_files
)
from .utils import resolve_blend_filepath_for_metadata
from .preferences import get_preferences
from .patch_creation import (
	create_initial_version, create_incremental_patch, create_patch_directories,
	validate_patch_creation_requirements
)
from .patch_application import (
	apply_patches_to_version, validate_navigation_requirements
)

class HDIFF_OT_CreatePatch(bpy.types.Operator):
	"""Create a differential patch for the current blend file"""
	bl_idname = "hdiff.create_patch"
	bl_label = "Create Patch"
	bl_description = "Creates a new differential patch for the current .blend file"
	bl_options = {'REGISTER', 'UNDO'}

	@classmethod
	def poll(cls, context):
		if not bpy.data.filepath:
			return False
			
		prefs = get_preferences()
		if not prefs:
			return False
			
		# Check if tools exist
		if not os.path.exists(prefs.hdiffz_path):
			return False

		# Allow patch creation only if on latest version or creating initial patch
		blend_filepath = bpy.data.filepath
		# Resolve to original file if we're viewing a preview
		resolved_filepath = resolve_blend_filepath_for_metadata(blend_filepath)
		metadata_filepath = get_metadata_filepath(resolved_filepath)
		metadata = load_metadata(metadata_filepath)
		
		if not metadata:
			return True  # Allow if no patches exist yet
		
		idx = context.scene.hdiff_current_version_index
		latest_version_index = len(metadata) - 1
		
		return idx == latest_version_index
	def execute(self, context):
		start_total = time.perf_counter()
		current_blend_filepath = bpy.data.filepath
		
		if not current_blend_filepath:
			# Auto-save if preferences allow it
			prefs = get_preferences()
			if prefs and prefs.auto_save_before_patch:
				try:
					bpy.ops.wm.save_mainfile()
					current_blend_filepath = bpy.data.filepath
				except Exception as e:
					self.report({'WARNING'}, f"Could not auto-save file: {e}")
			
			if not current_blend_filepath:
				self.report({'ERROR'}, "File not saved yet. Cannot create patch.")
				return {'CANCELLED'}

		# Resolve to original file if we're viewing a preview
		resolved_filepath = resolve_blend_filepath_for_metadata(current_blend_filepath)
		
		# Create necessary directories
		success, error = create_patch_directories(resolved_filepath)
		if not success:
			self.report({'ERROR'}, error)
			return {'CANCELLED'}

		# Validate patch creation requirements
		can_create, is_initial, metadata, message = validate_patch_creation_requirements(
			current_blend_filepath, resolved_filepath, context.scene.hdiff_current_version_index
		)
		
		if not can_create:
			self.report({'ERROR'}, message)
			return {'CANCELLED'}

		patch_comment = context.scene.hdiff_patch_comment
		
		if is_initial:
			# Create initial version
			success, message, elapsed = create_initial_version(
				current_blend_filepath, resolved_filepath, patch_comment
			)
			if success:
				context.scene.hdiff_current_version_index = 0
				context.scene.hdiff_patch_comment = ""
				self.report({'INFO'}, message)
				return {'FINISHED'}
			else:
				self.report({'ERROR'}, message)
				return {'CANCELLED'}
		else:
			# Create incremental patch
			success, message, elapsed, version_index = create_incremental_patch(
				current_blend_filepath, resolved_filepath, metadata, patch_comment
			)
			if success:
				context.scene.hdiff_current_version_index = version_index
				context.scene.hdiff_patch_comment = ""
				self.report({'INFO'}, message)
				return {'FINISHED'}
			else:
				self.report({'ERROR'}, message)
				return {'CANCELLED'}


class HDIFF_OT_go_to_version(bpy.types.Operator):
	"""Navigate to a specific version using reverse patches"""
	bl_idname = "hdiff.go_to_version"
	bl_label = "Go to Selected Version"
	bl_description = "Reconstructs and loads the selected version using reverse patches"
	bl_options = {'REGISTER', 'UNDO'}

	target_version_index: bpy.props.IntProperty(name="Target Version Index")

	@classmethod
	def poll(cls, context):
		if not bpy.data.filepath:
			return False
			
		prefs = get_preferences()
		if not prefs:
			return False
			
		return os.path.exists(prefs.hpatchz_path)
	def execute(self, context):
		start_time = time.perf_counter()
		current_blend_filepath = bpy.data.filepath
		
		# Resolve to original file if we're viewing a preview
		resolved_filepath = resolve_blend_filepath_for_metadata(current_blend_filepath)
		
		# Validate navigation requirements
		can_navigate, metadata, message = validate_navigation_requirements(current_blend_filepath, resolved_filepath)
		if not can_navigate:
			self.report({'ERROR'}, message)
			return {'CANCELLED'}
		
		# Check if target version is valid
		if not (0 <= self.target_version_index < len(metadata)):
			self.report({'ERROR'}, f"Invalid target version: {self.target_version_index}")
			return {'CANCELLED'}
		
		current_version_idx = context.scene.hdiff_current_version_index
		
		# Check if already at target version
		if current_version_idx == self.target_version_index:
			self.report({'INFO'}, f"Already at version {self.target_version_index}")
			return {'FINISHED'}
		
		self.report({'INFO'}, f"Navigating from version {current_version_idx} to {self.target_version_index}")
		
		# Apply patches to get to target version
		success, preview_filepath, messages, elapsed = apply_patches_to_version(
			current_blend_filepath, resolved_filepath, self.target_version_index, current_version_idx, metadata
		)
		
		if not success:
			self.report({'ERROR'}, preview_filepath if preview_filepath else "Navigation failed")
			return {'CANCELLED'}
				# Report progress messages
		for msg in messages:
			self.report({'INFO'}, msg)
		
		# Load the reconstructed preview file in Blender
		if preview_filepath and os.path.exists(preview_filepath):
			load_start = time.perf_counter()
			bpy.ops.wm.open_mainfile(filepath=preview_filepath)
			bpy.context.scene.hdiff_current_version_index = self.target_version_index
			load_elapsed = time.perf_counter() - load_start
			
			# Clean up old preview files AFTER loading the current one
			cleanup_old_preview_files(resolved_filepath, keep_recent=3)
			
			total_elapsed = time.perf_counter() - start_time
			self.report({'INFO'}, f"Loaded version {self.target_version_index} (load: {load_elapsed:.2f}s, total: {total_elapsed:.2f}s)")
		else:
			# Update current version index even if no file to load (already at latest)
			context.scene.hdiff_current_version_index = self.target_version_index
			total_elapsed = time.perf_counter() - start_time
			self.report({'INFO'}, f"Navigation completed in {total_elapsed:.2f}s")
			
			# Clean up old preview files even if no file was loaded
			cleanup_old_preview_files(resolved_filepath, keep_recent=3)
		
		return {'FINISHED'}


# --- Registration ---
classes = (
	HDIFF_OT_CreatePatch,
	HDIFF_OT_go_to_version,
)

def register():
	"""Register operators"""
	for cls in classes:
		bpy.utils.register_class(cls)
	print("HDiff Tool: Operators module registered")

def unregister():
	"""Unregister operators"""
	for cls in reversed(classes):
		bpy.utils.unregister_class(cls)
	print("HDiff Tool: Operators module unregistered")
