"""
HDiff Tool - Patch Creation Operations
"""

import os
import time
from datetime import datetime
from .core import (
	get_meta_directory, get_patch_directory, get_versions_directory, get_patches_directory,
	get_metadata_filepath, get_latest_version_filepath, load_metadata, save_metadata, get_file_signature
)
from .utils import execute_hdiffz, safe_copy_file, resolve_blend_filepath_for_metadata
from .preferences import get_preferences


def create_initial_version(current_blend_filepath, resolved_filepath, patch_comment=""):
	"""Create the initial version when no patches exist yet
	
	Args:
		current_blend_filepath (str): Path to current blend file
		resolved_filepath (str): Resolved filepath for metadata operations
		patch_comment (str): Comment for the initial version
	
	
		
	Returns:
		tuple: (success: bool, message: str, elapsed_time: float)
	"""
	try:
		init_start = time.perf_counter()
		
		# Get paths
		previous_version_filepath = get_latest_version_filepath(resolved_filepath)
		metadata_filepath = get_metadata_filepath(resolved_filepath)
		blend_basename = os.path.basename(current_blend_filepath)
		
		# Simply copy current file to _LATEST.blend - only copy needed
		success, error = safe_copy_file(current_blend_filepath, previous_version_filepath)
		if not success:
			return False, f"Error copying initial version: {error}", 0
			
		current_signature = get_file_signature(current_blend_filepath)
		
		entry = {
			"timestamp": datetime.now().isoformat(),
			"comment": patch_comment.strip() or "Initial Version",
			"reverse_patch_file": None,
			"source_blend_file": blend_basename,
			"from_signature": None,
			"to_signature": current_signature,
			"version_index": 0
		}
		
		metadata = [entry]
		save_metadata(metadata_filepath, metadata)
		
		init_elapsed = time.perf_counter() - init_start
		return True, f"Initial version recorded. (took {init_elapsed:.2f}s)", init_elapsed
		
	except Exception as e:
		return False, f"Error creating initial version: {e}", 0


def create_incremental_patch(current_blend_filepath, resolved_filepath, metadata, patch_comment=""):
	"""Create an incremental patch from existing versions
	
	Args:
		current_blend_filepath (str): Path to current blend file
		resolved_filepath (str): Resolved filepath for metadata operations
		metadata (list): Existing metadata entries
		patch_comment (str): Comment for the new patch
	
		
	Returns:
		tuple: (success: bool, message: str, elapsed_time: float, version_index: int)
	"""
	try:
		start_time = time.perf_counter()
		
		# Get paths
		previous_version_filepath = get_latest_version_filepath(resolved_filepath)
		patches_dir_path = get_patches_directory(resolved_filepath)
		metadata_filepath = get_metadata_filepath(resolved_filepath)
		blend_basename = os.path.basename(current_blend_filepath)
		
		# Check if _LATEST.blend exists
		if not os.path.exists(previous_version_filepath):
			return False, f"Previous version file missing: {previous_version_filepath}", 0, -1

		# Check for changes using file signature
		current_signature = get_file_signature(current_blend_filepath)
		previous_signature = get_file_signature(previous_version_filepath)
		
		if not current_signature or not previous_signature:
			return False, "Cannot calculate file signatures.", 0, -1

		if current_signature == previous_signature:
			return True, "No changes detected. No patch created.", 0, len(metadata) - 1

		# Generate patch filename
		version_index = len(metadata)
		forward_patch_filename = f"patch_{version_index:04d}_forward.hdiff"
		reverse_patch_filename = f"patch_{version_index:04d}_reverse.hdiff"
		
		forward_patch_path = os.path.join(patches_dir_path, forward_patch_filename)
		reverse_patch_path = os.path.join(patches_dir_path, reverse_patch_filename)

		# Get preferences
		prefs = get_preferences()
		compression_level = prefs.default_compression_level if prefs else 16
		timeout = prefs.patch_timeout if prefs else 300

		# Create forward patch (previous_version -> current)
		success, stdout, stderr, forward_elapsed = execute_hdiffz(
			previous_version_filepath, current_blend_filepath, forward_patch_path, 
			compression_level, timeout
		)
		
		if not success:
			return False, f"Forward patch creation failed: {stderr} (took {forward_elapsed:.2f}s)", forward_elapsed, -1

		# Create reverse patch (current -> previous_version)  
		success, stdout, stderr, reverse_elapsed = execute_hdiffz(
			current_blend_filepath, previous_version_filepath, reverse_patch_path,
			compression_level, timeout
		)
		
		if not success:
			# Clean up forward patch if reverse patch fails
			try:
				os.remove(forward_patch_path)
			except:
				pass
			return False, f"Reverse patch creation failed: {stderr} (took {reverse_elapsed:.2f}s)", reverse_elapsed, -1

		# Update latest version file - now current file becomes the latest
		success, error = safe_copy_file(current_blend_filepath, previous_version_filepath)
		if not success:
			# Clean up patch files if copy fails
			try:
				os.remove(forward_patch_path)
				os.remove(reverse_patch_path)
			except:
				pass
			return False, f"Error updating latest version file: {error}", 0, -1

		# Create metadata entry
		entry = {
			"timestamp": datetime.now().isoformat(),
			"comment": patch_comment.strip() or f"Patch {version_index}",
			"forward_patch_file": forward_patch_filename,
			"reverse_patch_file": reverse_patch_filename,
			"source_blend_file": blend_basename,
			"from_signature": previous_signature,
			"to_signature": current_signature,
			"version_index": version_index
		}
		
		metadata.append(entry)
		save_metadata(metadata_filepath, metadata)
		
		total_elapsed = time.perf_counter() - start_time
		return True, f"Patch {version_index} created successfully! Total time: {total_elapsed:.2f}s", total_elapsed, version_index

	except Exception as e:
		return False, f"Error creating patch: {e}", 0, -1


def create_patch_directories(resolved_filepath):
	"""Create all necessary directories for patch operations
	
	Args:
		resolved_filepath (str): Resolved filepath for metadata operations
		
	Returns:
		tuple: (success: bool, error_message: str)
	"""
	try:
		meta_dir_path = get_meta_directory(resolved_filepath)
		patch_dir_path = get_patch_directory(resolved_filepath)
		versions_dir_path = get_versions_directory(resolved_filepath)
		patches_dir_path = get_patches_directory(resolved_filepath)
		
		os.makedirs(meta_dir_path, exist_ok=True)
		os.makedirs(patch_dir_path, exist_ok=True)
		os.makedirs(versions_dir_path, exist_ok=True)
		os.makedirs(patches_dir_path, exist_ok=True)
		
		return True, ""
	except OSError as e:
		return False, f"Error creating patch directories: {e}"


def validate_patch_creation_requirements(current_blend_filepath, resolved_filepath, current_version_index):
	"""Validate that patch creation can proceed
	
	Args:
		current_blend_filepath (str): Path to current blend file
		resolved_filepath (str): Resolved filepath for metadata operations
		current_version_index (int): Current version index from scene
		
	Returns:
		tuple: (can_create: bool, is_initial: bool, metadata: list, message: str)
	"""
	# Load existing metadata
	metadata_filepath = get_metadata_filepath(resolved_filepath)
	metadata = load_metadata(metadata_filepath)
	
	# Check if this is initial version creation
	if not metadata:
		return True, True, [], "Ready to create initial version"
	
	# Check if on latest version
	latest_version_index = len(metadata) - 1
	if current_version_index != latest_version_index:
		return False, False, metadata, "Cannot create patch: not on latest version"
	
	return True, False, metadata, "Ready to create incremental patch"


# --- Registration ---
def register():
	"""Register patch creation module"""
	print("HDiff Tool: Patch creation module registered")

def unregister():
	"""Unregister patch creation module"""
	print("HDiff Tool: Patch creation module unregistered")
