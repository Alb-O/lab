"""
HDiff Tool - Core functionality and configuration
"""

import os
import json
import time
from datetime import datetime
from pathlib import Path
import bpy
from bpy.app.handlers import persistent

# --- Configuration ---
# TODO: These paths will be made configurable through preferences
# For now, using default paths from the bin directory
DEFAULT_HDIFFZ_PATH = r"c:\Users\Albert\_\obsidian vaults\blend-vault-obsidian\.bin\hdiffz.exe"
DEFAULT_HPATCHZ_PATH = r"c:\Users\Albert\_\obsidian vaults\blend-vault-obsidian\.bin\hpatchz.exe"

# File naming constants
META_FOLDER_SUFFIX = ".meta"
HDIFF_ADDON_FOLDER = "hdiff"
LATEST_VERSION_FILENAME = "_LATEST.blend"
METADATA_FILENAME = "patch_metadata.json"
VERSIONS_SUBFOLDER = "versions"
PATCHES_SUBFOLDER = "patches"

# --- Load Post Handler for UI Refresh ---
@persistent
def hdiff_on_load_post_handler(dummy):
	"""Handler to refresh UI after blend file loads"""
	def force_ui_redraw():
		try:
			if hasattr(bpy, 'context') and bpy.context and hasattr(bpy.context, 'window_manager') and bpy.context.window_manager:
				for window in bpy.context.window_manager.windows:
					for area in window.screen.areas:
						area.tag_redraw()
			else:
				print("HDiff Tool: bpy.context not fully available for UI redraw.")
		except Exception as e:
			print(f"HDiff Tool: Error in force_ui_redraw timer: {e}")
		return None

	if hdiff_on_load_post_handler in bpy.app.handlers.load_post:
		bpy.app.timers.register(force_ui_redraw, first_interval=0.1)

# --- Helper Functions ---
def get_file_signature(file_path):
	"""Get a lightweight file signature using size + modification time"""
	try:
		stat = os.stat(file_path)
		return f"{stat.st_size}_{int(stat.st_mtime)}"
	except (FileNotFoundError, OSError) as e:
		print(f"HDiff Tool: Could not get file signature for {file_path}: {e}")
		return None

def load_metadata(metadata_filepath):
	"""Load patch metadata from JSON file"""
	if not os.path.exists(metadata_filepath):
		return []
	try:
		with open(metadata_filepath, 'r') as f:
			return json.load(f)
	except json.JSONDecodeError:
		print(f"HDiff Tool: Metadata file {metadata_filepath} is corrupted. Returning empty list.")
		return []
	except IOError:
		print(f"HDiff Tool: Could not read metadata file {metadata_filepath}. Returning empty list.")
		return []

def save_metadata(metadata_filepath, data):
	"""Save patch metadata to JSON file"""
	try:
		with open(metadata_filepath, 'w') as f:
			json.dump(data, f, indent=4)
	except IOError:
		print(f"HDiff Tool: Could not write metadata to {metadata_filepath}")

def get_hdiffz_path():
	"""Get the path to hdiffz.exe from preferences or default"""
	try:
		from .preferences import get_preferences
		prefs = get_preferences()
		if prefs and prefs.hdiffz_path:
			return prefs.hdiffz_path
	except Exception:
		pass
	return DEFAULT_HDIFFZ_PATH

def get_hpatchz_path():
	"""Get the path to hpatchz.exe from preferences or default"""
	try:
		from .preferences import get_preferences
		prefs = get_preferences()
		if prefs and prefs.hpatchz_path:
			return prefs.hpatchz_path
	except Exception:
		pass
	return DEFAULT_HPATCHZ_PATH

def get_meta_directory(blend_filepath):
	"""Get the meta directory for a given blend file"""
	blend_dir = os.path.dirname(blend_filepath)
	blend_filename = os.path.basename(blend_filepath)
	meta_dir_name = f"{blend_filename}{META_FOLDER_SUFFIX}"
	return os.path.join(blend_dir, meta_dir_name)

def get_patch_directory(blend_filepath):
	"""Get the hdiff addon directory inside the meta folder"""
	meta_dir = get_meta_directory(blend_filepath)
	return os.path.join(meta_dir, HDIFF_ADDON_FOLDER)

def get_versions_directory(blend_filepath):
	"""Get the versions subdirectory for storing blend files"""
	patch_dir = get_patch_directory(blend_filepath)
	return os.path.join(patch_dir, VERSIONS_SUBFOLDER)

def get_patches_directory(blend_filepath):
	"""Get the patches subdirectory for storing hdiff files"""
	patch_dir = get_patch_directory(blend_filepath)
	return os.path.join(patch_dir, PATCHES_SUBFOLDER)

def get_metadata_filepath(blend_filepath):
	"""Get the metadata file path for a given blend file"""
	patch_dir = get_patch_directory(blend_filepath)
	return os.path.join(patch_dir, METADATA_FILENAME)

def get_latest_version_filepath(blend_filepath):
	"""Get the latest version file path for a given blend file"""
	versions_dir = get_versions_directory(blend_filepath)
	return os.path.join(versions_dir, LATEST_VERSION_FILENAME)

def get_preview_version_filepath(blend_filepath, version_index):
	"""Get the preview version file path for a specific version"""
	versions_dir = get_versions_directory(blend_filepath)
	return os.path.join(versions_dir, f"preview_v{version_index:04d}.blend")

def cleanup_old_preview_files(blend_filepath, keep_recent=5):
	"""Clean up old preview files to prevent clutter in versions folder
	
	Args:
		blend_filepath (str): Path to the current blend file
		keep_recent (int): Number of recent preview files to keep
	"""
	try:
		versions_dir = get_versions_directory(blend_filepath)
		if not os.path.exists(versions_dir):
			return
		# Find all preview files (preview_vXXXX.blend)
		preview_files = []
		for filename in os.listdir(versions_dir):
			if filename.startswith("preview_v") and filename.endswith(".blend"):
				filepath = os.path.join(versions_dir, filename)
				mtime = os.path.getmtime(filepath)
				preview_files.append((filepath, mtime))
		
		# Sort by modification time (newest first) and remove old ones
		preview_files.sort(key=lambda x: x[1], reverse=True)
		for filepath, _ in preview_files[keep_recent:]:
			try:
				os.remove(filepath)
				print(f"HDiff Tool: Cleaned up old preview file: {os.path.basename(filepath)}")
			except OSError as e:
				print(f"HDiff Tool: Failed to remove preview file {filepath}: {e}")
				
	except Exception as e:
		print(f"HDiff Tool: Error during preview cleanup: {e}")

# --- Registration ---
def register_properties():
	"""Register scene properties for HDiff Tool"""
	bpy.types.Scene.hdiff_patch_comment = bpy.props.StringProperty(
		name="Patch Comment",
		description="A comment to store with the next patch (cleared after use)",
		default=""
	)
	bpy.types.Scene.hdiff_current_version_index = bpy.props.IntProperty(
		name="Current Version Index",
		description="The index in the metadata that the current .blend file corresponds to",
		default=-1
	)

def unregister_properties():
	"""Unregister scene properties for HDiff Tool"""
	try:
		del bpy.types.Scene.hdiff_patch_comment
	except AttributeError:
		pass
	try:
		del bpy.types.Scene.hdiff_current_version_index
	except AttributeError:
		pass

def register():
	"""Register core functionality"""
	# Register properties first
	register_properties()
	
	# Add load post handler
	if hdiff_on_load_post_handler not in bpy.app.handlers.load_post:
		bpy.app.handlers.load_post.append(hdiff_on_load_post_handler)
		print("HDiff Tool: Core module registered")

def unregister():
	"""Unregister core functionality"""
	# Remove load post handler
	if hdiff_on_load_post_handler in bpy.app.handlers.load_post:
		bpy.app.handlers.load_post.remove(hdiff_on_load_post_handler)
	
	# Unregister properties
	unregister_properties()
	print("HDiff Tool: Core module unregistered")
