"""
HDiff Tool - Utility functions
"""

import os
import subprocess
import tempfile
import shutil
import time
from .core import get_hdiffz_path, get_hpatchz_path

def execute_hdiffz(old_file, new_file, patch_file, compression_level=None, timeout=300):
	"""
	Execute hdiffz to create a patch
	
	Args:
		old_file: Path to the old file
		new_file: Path to the new file  
		patch_file: Path to save the patch
		compression_level: Compression level (None for default)
		timeout: Timeout in seconds
		
	Returns:
		tuple: (success, stdout, stderr, elapsed_time)
	"""
	hdiffz_path = get_hdiffz_path()
	
	if not os.path.exists(hdiffz_path):
		return False, "", f"hdiffz.exe not found at {hdiffz_path}", 0

	# Build command
	cmd = [hdiffz_path, "-f"]
	
	# Add compression level if specified
	# For binary files like .blend, use memory mode (-m) with match score
	if compression_level is not None:
		# Convert preferences compression level (1-64) to match score (0-9 for binary)
		# Map 1-64 range to 0-4 range (recommended for binary files)
		match_score = min(4, max(0, int(compression_level / 16)))
		cmd.append(f"-m-{match_score}")
	else:
		cmd.append("-m-2")  # Default for binary files
	
	cmd.extend([old_file, new_file, patch_file])
	
	start_time = time.perf_counter()
	
	try:
		process = subprocess.Popen(
			cmd,
			stdout=subprocess.PIPE,
			stderr=subprocess.PIPE,
			text=True,
			creationflags=subprocess.CREATE_NO_WINDOW if os.name == 'nt' else 0
		)
		stdout, stderr = process.communicate(timeout=timeout)
		elapsed_time = time.perf_counter() - start_time
		
		success = process.returncode == 0
		return success, stdout, stderr, elapsed_time
		
	except subprocess.TimeoutExpired:
		elapsed_time = time.perf_counter() - start_time
		return False, "", f"hdiffz timed out after {timeout} seconds", elapsed_time
	except Exception as e:
		elapsed_time = time.perf_counter() - start_time
		return False, "", str(e), elapsed_time

def execute_hpatchz(old_file, patch_file, output_file, timeout=60):
	"""
	Execute hpatchz to apply a patch
	
	Args:
		old_file: Path to the old file
		patch_file: Path to the patch file
		output_file: Path to save the patched file
		timeout: Timeout in seconds
		
	Returns:
		tuple: (success, stdout, stderr, elapsed_time)
	"""
	hpatchz_path = get_hpatchz_path()
	
	if not os.path.exists(hpatchz_path):
		return False, "", f"hpatchz.exe not found at {hpatchz_path}", 0
	
	# Build command  
	cmd = [hpatchz_path, "-f", old_file, patch_file, output_file]
	
	start_time = time.perf_counter()
	
	try:
		process = subprocess.Popen(
			cmd,
			stdout=subprocess.PIPE,
			stderr=subprocess.PIPE,
			text=True,
			creationflags=subprocess.CREATE_NO_WINDOW if os.name == 'nt' else 0
		)
		stdout, stderr = process.communicate(timeout=timeout)
		elapsed_time = time.perf_counter() - start_time
		
		success = process.returncode == 0
		return success, stdout, stderr, elapsed_time
		
	except subprocess.TimeoutExpired:
		elapsed_time = time.perf_counter() - start_time
		return False, "", f"hpatchz timed out after {timeout} seconds", elapsed_time
	except Exception as e:
		elapsed_time = time.perf_counter() - start_time
		return False, "", str(e), elapsed_time

def get_file_size_mb(filepath):
	"""Get file size in megabytes"""
	try:
		size_bytes = os.path.getsize(filepath)
		return size_bytes / (1024 * 1024)
	except OSError:
		return 0

def safe_copy_file(src, dst):
	"""Safely copy a file with error handling"""
	try:
		shutil.copy2(src, dst)
		return True, ""
	except Exception as e:
		return False, str(e)

def create_temp_file(suffix=".blend", prefix="hdiff_"):
	"""Create a temporary file and return the file descriptor and path"""
	try:
		fd, path = tempfile.mkstemp(suffix=suffix, prefix=prefix)
		return fd, path
	except Exception as e:
		return None, str(e)

def cleanup_temp_file(filepath):
	"""Safely remove a temporary file"""
	if filepath and os.path.exists(filepath):
		try:
			os.remove(filepath)
			return True
		except OSError:
			return False
	return True

def is_preview_file(filepath):
	"""Check if the current filepath is a preview file
	
	Args:
		filepath (str): Path to check
		
	Returns:
		bool: True if this is a preview file
	"""
	if not filepath:
		return False
	
	filename = os.path.basename(filepath)
	# Preview files follow pattern: preview_vXXXX.blend
	return filename.startswith("preview_v") and filename.endswith(".blend")

def get_original_blend_file(preview_filepath):
	"""Get the original blend file path from a preview file path
	
	Args:
		preview_filepath (str): Path to a preview file
		
	Returns:
		str or None: Path to original blend file, or None if not found
	"""
	if not preview_filepath or not is_preview_file(preview_filepath):
		return preview_filepath  # Not a preview file, return as-is
	
	try:
		# Preview file is in: {original}.blend.meta/hdiff/versions/preview_vXXXX.blend
		# We need to get back to: {original}.blend
		
		# Navigate up from preview file to get to meta directory
		preview_dir = os.path.dirname(preview_filepath)  # versions/
		hdiff_dir = os.path.dirname(preview_dir)         # hdiff/
		meta_dir = os.path.dirname(hdiff_dir)            # {original}.blend.meta/
		
		# The original file should be adjacent to the meta directory
		blend_dir = os.path.dirname(meta_dir)
		
		# Extract original filename from meta directory name
		meta_dirname = os.path.basename(meta_dir)
		
		if meta_dirname.endswith(".meta"):
			original_filename = meta_dirname[:-5]  # Remove ".meta" suffix
			original_filepath = os.path.join(blend_dir, original_filename)
			
			if os.path.exists(original_filepath):
				return original_filepath
			else:
				print(f"HDiff Tool: Warning - Original file not found: {original_filepath}")
		else:
			print(f"HDiff Tool: Warning - Unexpected meta directory name: {meta_dirname}")
		
		return None
		
	except Exception as e:
		print(f"HDiff Tool: Error resolving original file from preview: {e}")
		return None

def resolve_blend_filepath_for_metadata(current_filepath):
	"""Resolve the correct blend file path for accessing metadata
	
	This function handles the case where we're currently viewing a preview file
	but need to access the patch metadata from the original blend file.
	
	Args:
		current_filepath (str): Current blend file path (might be preview)
		
	Returns:
		str: Path to use for metadata operations (original blend file)
	"""
	if not current_filepath:
		return current_filepath
	
	if is_preview_file(current_filepath):
		original = get_original_blend_file(current_filepath)
		if original:
			return original
		else:
			print(f"HDiff Tool: Warning - Could not resolve original file for preview: {os.path.basename(current_filepath)}")
			return current_filepath
	
	return current_filepath

# --- Registration ---
def register():
	"""Register utils module"""
	print("HDiff Tool: Utils module registered")

def unregister():
	"""Unregister utils module"""
	print("HDiff Tool: Utils module unregistered")
