"""
HDiff Tool - Patch Application Operations
"""

import os
import time
from .core import (
	get_patch_directory, get_versions_directory, get_patches_directory,
	get_metadata_filepath, get_latest_version_filepath, get_preview_version_filepath,
	load_metadata, cleanup_old_preview_files
)
from .utils import execute_hpatchz, safe_copy_file, resolve_blend_filepath_for_metadata


def navigate_to_latest_version(resolved_filepath, target_version_index):
	"""Navigate to the latest version by copying _LATEST.blend
	
	Args:
		resolved_filepath (str): Resolved filepath for metadata operations
		target_version_index (int): Target version index
		
	Returns:
		tuple: (success: bool, preview_filepath: str, message: str, elapsed_time: float)
	"""
	try:
		start_time = time.perf_counter()
		
		previous_version_filepath = get_latest_version_filepath(resolved_filepath)
		preview_filepath = get_preview_version_filepath(resolved_filepath, target_version_index)
		
		if not os.path.exists(previous_version_filepath):
			return False, "", f"Latest version file not found: {previous_version_filepath}", 0

		success, error = safe_copy_file(previous_version_filepath, preview_filepath)
		if not success:
			return False, "", f"Error copying latest version: {error}", 0
			
		elapsed = time.perf_counter() - start_time
		return True, preview_filepath, f"Prepared latest version (took {elapsed:.2f}s)", elapsed
		
	except Exception as e:
		return False, "", f"Error preparing latest version: {e}", 0


def navigate_backward(resolved_filepath, target_version_index, metadata):
	"""Navigate backward by applying reverse patches
	
	Args:
		resolved_filepath (str): Resolved filepath for metadata operations
		target_version_index (int): Target version index
		metadata (list): Patch metadata
		
	Returns:
		tuple: (success: bool, preview_filepath: str, message: str, elapsed_time: float)
	"""
	try:
		start_time = time.perf_counter()
		
		previous_version_filepath = get_latest_version_filepath(resolved_filepath)
		preview_filepath = get_preview_version_filepath(resolved_filepath, target_version_index)
		patches_dir_path = get_patches_directory(resolved_filepath)
		
		latest_version_idx = len(metadata) - 1
		steps_to_go_back = latest_version_idx - target_version_index
		
		if not os.path.exists(previous_version_filepath):
			return False, "", f"Latest version file not found: {previous_version_filepath}", 0

		# Copy _LATEST.blend to preview file as starting point
		success, error = safe_copy_file(previous_version_filepath, preview_filepath)
		if not success:
			return False, "", f"Error copying latest version: {error}", 0
		
		messages = [f"Starting from latest version"]
		
		# Apply reverse patches directly to the preview file
		for step in range(steps_to_go_back):
			patch_version_idx = latest_version_idx - step
			if patch_version_idx <= 0:
				break
			
			patch_entry = metadata[patch_version_idx]
			reverse_patch_file = patch_entry.get("reverse_patch_file")
			
			if not reverse_patch_file:
				return False, "", f"No reverse patch found for version {patch_version_idx}", 0
			
			reverse_patch_path = os.path.join(patches_dir_path, reverse_patch_file)
			if not os.path.exists(reverse_patch_path):
				return False, "", f"Reverse patch file not found: {reverse_patch_path}", 0

			# Apply reverse patch to preview file
			success, stdout, stderr, patch_elapsed = execute_hpatchz(
				preview_filepath, reverse_patch_path, preview_filepath, timeout=60
			)

			if not success:
				return False, "", f"Failed to apply reverse patch {patch_version_idx}: {stderr} (took {patch_elapsed:.2f}s)", 0

			messages.append(f"Applied reverse patch {patch_version_idx} (took {patch_elapsed:.2f}s)")

		total_elapsed = time.perf_counter() - start_time
		return True, preview_filepath, "; ".join(messages), total_elapsed
		
	except Exception as e:
		return False, "", f"Error during backward navigation: {e}", 0


def navigate_forward(current_blend_filepath, resolved_filepath, target_version_index, current_version_idx, metadata):
	"""Navigate forward by applying forward patches
	
	Args:
		current_blend_filepath (str): Current blend file path
		resolved_filepath (str): Resolved filepath for metadata operations
		target_version_index (int): Target version index
		current_version_idx (int): Current version index
		metadata (list): Patch metadata
		
	Returns:
		tuple: (success: bool, preview_filepath: str, message: str, elapsed_time: float)
	"""
	try:
		start_time = time.perf_counter()
		
		preview_filepath = get_preview_version_filepath(resolved_filepath, target_version_index)
		patches_dir_path = get_patches_directory(resolved_filepath)
		
		# First, copy current file to preview file as starting point
		success, error = safe_copy_file(current_blend_filepath, preview_filepath)
		if not success:
			return False, "", f"Error copying current file: {error}", 0
		
		messages = [f"Starting from current version"]
		steps_to_go_forward = target_version_index - current_version_idx
		
		# Apply forward patches to preview file
		for step in range(steps_to_go_forward):
			patch_version_idx = current_version_idx + step + 1
			if patch_version_idx >= len(metadata):
				break
			
			patch_entry = metadata[patch_version_idx]
			forward_patch_file = patch_entry.get("forward_patch_file")
			
			if not forward_patch_file:
				return False, "", f"No forward patch found for version {patch_version_idx}", 0
			
			forward_patch_path = os.path.join(patches_dir_path, forward_patch_file)
			if not os.path.exists(forward_patch_path):
				return False, "", f"Forward patch file not found: {forward_patch_path}", 0

			# Apply forward patch to preview file
			success, stdout, stderr, patch_elapsed = execute_hpatchz(
				preview_filepath, forward_patch_path, preview_filepath, timeout=60
			)

			if not success:
				return False, "", f"Failed to apply forward patch {patch_version_idx}: {stderr} (took {patch_elapsed:.2f}s)", 0

			messages.append(f"Applied forward patch {patch_version_idx} (took {patch_elapsed:.2f}s)")

		total_elapsed = time.perf_counter() - start_time
		return True, preview_filepath, "; ".join(messages), total_elapsed
		
	except Exception as e:
		return False, "", f"Error during forward navigation: {e}", 0


def apply_patches_to_version(current_blend_filepath, resolved_filepath, target_version_index, current_version_idx, metadata):
	"""Apply patches to navigate to a specific version
	
	Args:
		current_blend_filepath (str): Current blend file path
		resolved_filepath (str): Resolved filepath for metadata operations
		target_version_index (int): Target version index to navigate to
		current_version_idx (int): Current version index
		metadata (list): Patch metadata
		
	Returns:
		tuple: (success: bool, preview_filepath: str, messages: list, elapsed_time: float)
	"""
	if not metadata or not (0 <= target_version_index < len(metadata)):
		return False, "", [f"Invalid target version: {target_version_index}"], 0

	if current_version_idx == target_version_index:
		return True, "", [f"Already at version {target_version_index}"], 0

	latest_version_idx = len(metadata) - 1
	
	# Determine navigation strategy  
	if target_version_index == latest_version_idx:
		# Going to latest version - just copy _LATEST.blend (most efficient)
		success, preview_filepath, message, elapsed = navigate_to_latest_version(resolved_filepath, target_version_index)
		return success, preview_filepath, [message], elapsed
		
	elif target_version_index < current_version_idx:
		# Going backwards - use reverse patches
		success, preview_filepath, message, elapsed = navigate_backward(resolved_filepath, target_version_index, metadata)
		return success, preview_filepath, [message], elapsed

	else:
		# Going forwards (but not to latest) - use forward patches  
		success, preview_filepath, message, elapsed = navigate_forward(
			current_blend_filepath, resolved_filepath, target_version_index, current_version_idx, metadata
		)
		return success, preview_filepath, [message], elapsed


def validate_navigation_requirements(current_blend_filepath, resolved_filepath):
	"""Validate that navigation can proceed
	
	Args:
		current_blend_filepath (str): Current blend file path
		resolved_filepath (str): Resolved filepath for metadata operations
		
	Returns:
		tuple: (can_navigate: bool, metadata: list, message: str)
	"""
	if not current_blend_filepath:
		return False, [], "Current file is not saved."
	
	metadata_filepath = get_metadata_filepath(resolved_filepath)
	metadata = load_metadata(metadata_filepath)
	
	if not metadata:
		return False, [], "No patch history found."
	
	return True, metadata, "Ready for navigation"


# --- Registration ---
def register():
	"""Register patch application module"""
	print("HDiff Tool: Patch application module registered")

def unregister():
	"""Unregister patch application module"""
	print("HDiff Tool: Patch application module unregistered")
