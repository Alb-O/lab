"""
Asset and resource collection utilities for Blend Vault.
Handles collecting assets and external resources from Blender data.
"""

import bpy  # type: ignore
import os
import re
from typing import Dict, List, Tuple
from ..utils import get_asset_sources_map, get_or_create_datablock_uuid, BV_UUID_PROP, SIDECAR_EXTENSION, format_primary_link
from .uuid_manager import read_sidecar_uuid


def _matches_current_file_heading(line: str) -> bool:
	"""Check if a line matches the Current File section heading in any format."""
	line_stripped = line.strip()
	
	# Check plain format
	if line_stripped == "### Current File":
		return True
	
	# Check markdown link format using the current MD_PRIMARY_FORMAT
	if line_stripped.startswith("### "):
		# Use helper to format primary link and compare strings
		expected = format_primary_link(path="", name="Current File").replace("[]|", "").strip()
		# Fallback to regex match on text content
		link_match = re.search(r"\[\[([^\]|]+)\|([^\]]+)\]\]", line_stripped[4:])
		if link_match and link_match.group(1) == "Current File":
			return True
	
	return False


def _resolve_linked_asset_uuids(
	linked_assets_by_library: Dict, 
	blend_path: str
) -> None:
	"""Resolve UUIDs for linked assets by reading from library sidecars."""
	blend_dir = os.path.dirname(blend_path)
	
	for lib, assets in linked_assets_by_library.items():
		if not lib or not hasattr(lib, 'filepath') or not lib.filepath:
			continue
			
		# Get library sidecar path
		lib_path = lib.filepath.lstrip('//').replace('\\', '/')
		lib_sidecar_path = os.path.normpath(
			os.path.join(blend_dir, lib_path)
		) + SIDECAR_EXTENSION
		
		if not os.path.exists(lib_sidecar_path):
			continue
			
		# Read library's assets from its sidecar
		try:
			lib_assets = _get_current_file_assets_from_sidecar(lib_sidecar_path)
			if not lib_assets:
				continue
			
			# Create lookup by name and type
			lib_asset_lookup = {}
			for lib_asset in lib_assets:
				name = lib_asset.get("name")
				asset_type = lib_asset.get("type")
				uuid = lib_asset.get("uuid")
				if name and asset_type and uuid:
					lib_asset_lookup[(name, asset_type)] = uuid
			
			# Resolve UUIDs for linked assets
			for asset in assets:
				if asset["uuid"] is None:  # Only resolve missing UUIDs
					key = (asset["name"], asset["type"])
					if key in lib_asset_lookup:
						asset["uuid"] = lib_asset_lookup[key]
				
		except Exception as e:
			print(f"[Blend Vault] Error reading library sidecar {lib_sidecar_path}: {e}")
			continue


def _get_current_file_assets_from_sidecar(sidecar_path: str) -> List[dict]:
	"""Parse Current File assets from sidecar using Asset Relinker's approach."""
	import json
	
	try:
		with open(sidecar_path, 'r', encoding='utf-8') as f:
			lines = f.readlines()
		# Find "### Current File" section (handle both old and new markdown link format)
		current_file_start = None
		for i, line in enumerate(lines):
			if _matches_current_file_heading(line):
				current_file_start = i
				break
		
		if current_file_start is None:
			return []
		
		# Find the next ```json block after "### Current File"
		json_start = None
		json_end = None
		for i in range(current_file_start + 1, len(lines)):
			line = lines[i].strip()
			if line == "```json":
				json_start = i + 1
				break
		
		if json_start is None:
			return []
		
		# Find the closing ```
		for i in range(json_start, len(lines)):
			if lines[i].strip() == "```":
				json_end = i
				break
		
		if json_end is None:
			return []
		
		# Extract and parse JSON
		json_lines = lines[json_start:json_end]
		json_content = ''.join(json_lines)
		
		try:
			data = json.loads(json_content)
			assets = data.get("assets", [])
			
			# Validate each asset has required fields
			valid_assets = []
			for asset in assets:
				name = asset.get("name")
				asset_type = asset.get("type") 
				uuid = asset.get("uuid")
				
				if name and asset_type and uuid:
					valid_assets.append(asset)
			
			return valid_assets
			
		except json.JSONDecodeError:
			return []
			
	except Exception:
		return []


def collect_assets() -> Tuple[Dict[str, dict], Dict]:
	"""Collect assets from Blender data, separating local and linked assets."""
	local_assets = {}
	linked_assets_by_library = {}
	
	asset_sources_map = get_asset_sources_map()
	
	for asset_type, collection in asset_sources_map.items():
		if not collection:
			continue
			
		for item in collection: # item is a Blender datablock, e.g. an Object, Material, Scene
			if not item:
				continue
				
			# Check if item is an asset or scene
			is_asset_or_scene = asset_type == "Scene" or getattr(item, 'asset_data', None) is not None
			if not is_asset_or_scene:
				continue
				
			library = getattr(item, 'library', None) # library is a bpy.types.Library, representing the source .blend file
			item_name = getattr(item, 'name', f'Unnamed{asset_type}')
			
			# Collect asset info
			if library is None:
				# For local assets (defined in the current .blend file), ensure they have UUIDs and use them
				asset_uuid = get_or_create_datablock_uuid(item)
				asset_info = {
					"name": item_name,
					"type": asset_type,
					"uuid": asset_uuid
				}
				local_assets[asset_uuid] = asset_info
			else:
				 # Skip assets linked from "copybuffer.blend" files
				if library.filepath.endswith("copybuffer.blend"):
					continue

				# For linked assets, try to get existing UUID from their custom properties.
				# This UUID would have been assigned by Blend Vault in the source library file.
				# If it doesn't have one, it will be None for now and resolved later
				# by reading the library's sidecar file.
				existing_uuid = None
				if hasattr(item, 'id_properties_ensure'): # Should always be true for datablocks
					props = item.id_properties_ensure()
					existing_uuid = props.get(BV_UUID_PROP) 
				
				asset_info = {
					"name": item_name,
					"type": asset_type,
					"uuid": existing_uuid  # May be None, will be resolved from library's sidecar if possible
				}
				# Group linked assets by their source library object
				linked_assets_by_library.setdefault(library, []).append(asset_info)
	
	# Resolve UUIDs for linked assets by reading their respective library's sidecar files
	blend_filepath = bpy.data.filepath
	
	if blend_filepath:
		_resolve_linked_asset_uuids(linked_assets_by_library, blend_filepath)
		
		# Preserve existing UUIDs from current file's sidecar for local assets
		_preserve_existing_uuids_from_current_sidecar(local_assets, blend_filepath)
	
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


def _preserve_existing_uuids_from_current_sidecar(
	local_assets: Dict[str, dict], 
	blend_path: str
) -> None:
	"""Preserve existing UUIDs from the current file's sidecar before generating new ones."""
	if not blend_path:
		return
		
	sidecar_path = blend_path + SIDECAR_EXTENSION
	if not os.path.exists(sidecar_path):
		return
		
	try:
		# Read existing assets from the current file's sidecar
		existing_assets = _get_current_file_assets_from_sidecar(sidecar_path)
		
		# Create lookup by (name, type) for existing UUIDs
		existing_uuid_lookup = {}
		for existing_asset in existing_assets:
			name = existing_asset.get("name")
			asset_type = existing_asset.get("type")
			existing_uuid = existing_asset.get("uuid")
			if name and asset_type and existing_uuid:
				existing_uuid_lookup[(name, asset_type)] = existing_uuid
		
		# Update local assets with existing UUIDs where they match
		for uuid_key, asset in local_assets.items():
			key = (asset["name"], asset["type"])
			if key in existing_uuid_lookup:
				old_uuid = asset["uuid"]
				preserved_uuid = existing_uuid_lookup[key]
				
				# Update both the asset and the dictionary key
				asset["uuid"] = preserved_uuid
				
				# If the UUID changed, we need to update the dictionary
				if old_uuid != preserved_uuid:
					# Remove old entry and add with new UUID
					del local_assets[uuid_key]
					local_assets[preserved_uuid] = asset
		
	except Exception as e:
		print(f"[Blend Vault] Error preserving UUIDs from sidecar: {e}")
