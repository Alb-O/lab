"""
Asset and resource collection utilities for Blend Vault.
Handles collecting assets and external resources from Blender data.
"""

import bpy  # type: ignore
from typing import Dict, List, Tuple
from utils import get_asset_sources_map, ensure_library_hash


def collect_assets() -> Tuple[Dict[str, dict], Dict[object, List[dict]]]:
	"""Collect assets from Blender data, separating local and linked assets."""
	local_assets = {}
	linked_assets_by_library = {}
	
	for asset_type, collection in get_asset_sources_map().items():
		if not collection:
			continue
			
		for item in collection:
			if not item:
				continue
				
			# Check if item is an asset or scene
			is_asset = asset_type == "Scene" or getattr(item, 'asset_data', None) is not None
			if not is_asset:
				continue
				
			library = getattr(item, 'library', None)
			
			# Only generate UUIDs for local assets
			if library is None:
				ensure_library_hash(item)
			
			# Collect asset info
			asset_info = {
				"name": getattr(item, 'name', f'Unnamed{asset_type}'),
				"type": asset_type,
				"uuid": ensure_library_hash(item)
			}
			
			if library is None:
				local_assets[asset_info["uuid"]] = asset_info
			else:
				linked_assets_by_library.setdefault(library, []).append(asset_info)
	
	return local_assets, linked_assets_by_library


def collect_resources() -> List[dict]:
	"""Collect external resource files (textures, videos, sounds, etc.) from the current file only."""
	resources = []
	
	# Helper function to add a resource if valid
	def add_resource(filepath: str, name: str, resource_type: str):
		if not filepath or filepath.startswith('<') or filepath == '':
			return
		clean_path = filepath.lstrip('//').replace('\\', '/')
		if clean_path:  # Only add if we have a valid path
			resources.append({
				"name": name,
				"path": clean_path,
				"type": resource_type
			})
	
	# Collect Images (textures)
	for image in bpy.data.images:
		if not image:
			continue
		# Skip if belongs to linked library or is packed
		if (getattr(image, 'library', None) is not None or 
			image.packed_file is not None):
			continue
		add_resource(getattr(image, 'filepath', ''), image.name, "Image")
	
	# Collect Movie Clips (video files used in video editor or motion tracking)
	for movieclip in bpy.data.movieclips:
		if not movieclip:
			continue
		# Skip if belongs to linked library
		if getattr(movieclip, 'library', None) is not None:
			continue
		add_resource(getattr(movieclip, 'filepath', ''), movieclip.name, "Video")
	
	# Collect Sounds (audio files)
	for sound in bpy.data.sounds:
		if not sound:
			continue
		# Skip if belongs to linked library or is packed
		if (getattr(sound, 'library', None) is not None or 
			getattr(sound, 'packed_file', None) is not None):
			continue
		add_resource(getattr(sound, 'filepath', ''), sound.name, "Audio")
	
	# Collect Text files (external scripts)
	for text in bpy.data.texts:
		if not text:
			continue
		# Skip if belongs to linked library or is not from external file
		if (getattr(text, 'library', None) is not None or 
			not getattr(text, 'is_saved', False)):
			continue
		add_resource(getattr(text, 'filepath', ''), text.name, "Text")
	
	# Collect Cache Files (Alembic, USD, etc.)
	for cache_file in bpy.data.cache_files:
		if not cache_file:
			continue
		# Skip if belongs to linked library
		if getattr(cache_file, 'library', None) is not None:
			continue
		add_resource(getattr(cache_file, 'filepath', ''), cache_file.name, "Cache")
		
	return resources
