"""
Asset and resource collection utilities for Blend Vault.
Handles collecting assets and external resources from Blender data.
"""

import bpy  # type: ignore
import os
from typing import Dict, List, Tuple
from utils import get_asset_sources_map, ensure_library_hash, BV_UUID_PROP, SIDECAR_EXTENSION
from .uuid_manager import read_sidecar_uuid


def _resolve_linked_asset_uuids(
	linked_assets_by_library: Dict, 
	blend_path: str
) -> None:
	"""Resolve UUIDs for linked assets by reading from library sidecars."""
	print(f"🔥🔥🔥 [Blend Vault][UUID Resolution] FUNCTION CALLED! 🔥🔥🔥")
	blend_dir = os.path.dirname(blend_path)
	print(f"[Blend Vault][UUID Resolution] Starting UUID resolution for {len(linked_assets_by_library)} libraries")
	
	for lib, assets in linked_assets_by_library.items():
		if not lib or not hasattr(lib, 'filepath') or not lib.filepath:
			print(f"[Blend Vault][UUID Resolution] Skipping library with invalid filepath")
			continue
			
		# Get library sidecar path
		lib_path = lib.filepath.lstrip('//').replace('\\', '/')
		lib_sidecar_path = os.path.normpath(
			os.path.join(blend_dir, lib_path)
		) + SIDECAR_EXTENSION
		
		print(f"[Blend Vault][UUID Resolution] Processing library: {lib.name}")
		print(f"[Blend Vault][UUID Resolution]   Library filepath: {lib.filepath}")
		print(f"[Blend Vault][UUID Resolution]   Cleaned path: {lib_path}")
		print(f"[Blend Vault][UUID Resolution]   Sidecar path: {lib_sidecar_path}")
		print(f"[Blend Vault][UUID Resolution]   Assets to resolve: {len(assets)}")
		
		for asset in assets:
			print(f"[Blend Vault][UUID Resolution]     Asset: {asset['name']} ({asset['type']}) UUID={asset['uuid']}")
		
		if not os.path.exists(lib_sidecar_path):
			print(f"[Blend Vault][UUID Resolution] ERROR: Library sidecar not found: {lib_sidecar_path}")
			continue
			
		# Read library's assets from its sidecar using Asset Relinker's approach
		try:
			print(f"[Blend Vault][UUID Resolution] Reading sidecar file...")
			
			# Use the same parsing approach as Asset Relinker
			lib_assets = _get_current_file_assets_from_sidecar(lib_sidecar_path)
			
			print(f"[Blend Vault][UUID Resolution] Parsed {len(lib_assets)} assets from library sidecar")
			
			if not lib_assets:
				print(f"[Blend Vault][UUID Resolution] WARNING: No assets found in library sidecar")
				continue
			
			# Debug: Show what we found in the library
			for lib_asset in lib_assets:
				print(f"[Blend Vault][UUID Resolution]   Library asset: {lib_asset.get('name')} ({lib_asset.get('type')}) UUID={lib_asset.get('uuid')}")
			
			# Create lookup by name and type
			lib_asset_lookup = {}
			for lib_asset in lib_assets:
				name = lib_asset.get("name")
				asset_type = lib_asset.get("type")
				uuid = lib_asset.get("uuid")
				if name and asset_type and uuid:
					lib_asset_lookup[(name, asset_type)] = uuid
					print(f"[Blend Vault][UUID Resolution]   Added to lookup: {name} ({asset_type}) -> {uuid}")
				else:
					print(f"[Blend Vault][UUID Resolution]   Skipped incomplete asset: name={name}, type={asset_type}, uuid={uuid}")
			
			print(f"[Blend Vault][UUID Resolution] Created lookup table with {len(lib_asset_lookup)} entries")
			
			# Resolve UUIDs for linked assets
			resolved_count = 0
			for asset in assets:
				if asset["uuid"] is None:  # Only resolve missing UUIDs
					key = (asset["name"], asset["type"])
					if key in lib_asset_lookup:
						old_uuid = asset["uuid"]
						asset["uuid"] = lib_asset_lookup[key]
						resolved_count += 1
						print(f"[Blend Vault][UUID Resolution] ✓ RESOLVED: '{asset['name']}' ({asset['type']}) {old_uuid} -> {asset['uuid']}")
					else:
						print(f"[Blend Vault][UUID Resolution] ✗ NOT FOUND: '{asset['name']}' ({asset['type']}) - no matching asset in library")
				else:
					print(f"[Blend Vault][UUID Resolution] SKIPPED: '{asset['name']}' already has UUID: {asset['uuid']}")
			
			print(f"[Blend Vault][UUID Resolution] Resolved {resolved_count} UUIDs from library '{lib.name}'")
				
		except Exception as e:
			print(f"[Blend Vault][UUID Resolution] ERROR reading library sidecar {lib_sidecar_path}: {e}")
			import traceback
			print(f"[Blend Vault][UUID Resolution] Traceback: {traceback.format_exc()}")
			continue


def _get_current_file_assets_from_sidecar(sidecar_path: str) -> List[dict]:
	"""Parse Current File assets from sidecar using Asset Relinker's approach."""
	import json
	
	try:
		with open(sidecar_path, 'r', encoding='utf-8') as f:
			lines = f.readlines()
		
		print(f"[Blend Vault][Sidecar Parser] Read {len(lines)} lines from {sidecar_path}")
		
		# Find "### Current File" section
		current_file_start = None
		for i, line in enumerate(lines):
			if line.strip() == "### Current File":
				current_file_start = i
				print(f"[Blend Vault][Sidecar Parser] Found '### Current File' at line {i}")
				break
		
		if current_file_start is None:
			print(f"[Blend Vault][Sidecar Parser] ERROR: No '### Current File' section found")
			return []
		
		# Find the next ```json block after "### Current File"
		json_start = None
		json_end = None
		for i in range(current_file_start + 1, len(lines)):
			line = lines[i].strip()
			if line == "```json":
				json_start = i + 1
				print(f"[Blend Vault][Sidecar Parser] Found '```json' at line {i}")
				break
		
		if json_start is None:
			print(f"[Blend Vault][Sidecar Parser] ERROR: No '```json' block found after '### Current File'")
			return []
		
		# Find the closing ```
		for i in range(json_start, len(lines)):
			if lines[i].strip() == "```":
				json_end = i
				print(f"[Blend Vault][Sidecar Parser] Found closing '```' at line {i}")
				break
		
		if json_end is None:
			print(f"[Blend Vault][Sidecar Parser] ERROR: No closing '```' found")
			return []
		
		# Extract and parse JSON
		json_lines = lines[json_start:json_end]
		json_content = ''.join(json_lines)
		
		print(f"[Blend Vault][Sidecar Parser] Extracted JSON content ({len(json_content)} chars):")
		print(f"[Blend Vault][Sidecar Parser] JSON preview: {json_content[:200]}...")
		
		try:
			data = json.loads(json_content)
			assets = data.get("assets", [])
			print(f"[Blend Vault][Sidecar Parser] Successfully parsed JSON with {len(assets)} assets")
			
			# Validate each asset has required fields
			valid_assets = []
			for asset in assets:
				name = asset.get("name")
				asset_type = asset.get("type") 
				uuid = asset.get("uuid")
				
				if name and asset_type and uuid:
					valid_assets.append(asset)
					print(f"[Blend Vault][Sidecar Parser]   Valid asset: {name} ({asset_type}) {uuid}")
				else:
					print(f"[Blend Vault][Sidecar Parser]   Invalid asset (missing fields): {asset}")
			
			print(f"[Blend Vault][Sidecar Parser] Returning {len(valid_assets)} valid assets")
			return valid_assets
			
		except json.JSONDecodeError as e:
			print(f"[Blend Vault][Sidecar Parser] JSON decode error: {e}")
			print(f"[Blend Vault][Sidecar Parser] Failed JSON content: {json_content}")
			return []
			
	except Exception as e:
		print(f"[Blend Vault][Sidecar Parser] Error reading sidecar: {e}")
		import traceback
		print(f"[Blend Vault][Sidecar Parser] Traceback: {traceback.format_exc()}")
		return []


def collect_assets() -> Tuple[Dict[str, dict], Dict]:
	"""Collect assets from Blender data, separating local and linked assets."""
	print(f"🚨🚨🚨 [Blend Vault][Collectors] STARTING COLLECT_ASSETS() - UUID RESOLUTION FUNCTION 🚨🚨🚨")
	print(f"[Blend Vault][Collectors] Starting collect_assets()")
	
	local_assets = {}
	linked_assets_by_library = {}
	
	print(f"[Blend Vault][Collectors] Getting asset sources map...")
	asset_sources_map = get_asset_sources_map()
	print(f"[Blend Vault][Collectors] Asset sources map has {len(asset_sources_map)} types: {list(asset_sources_map.keys())}")
	
	for asset_type, collection in asset_sources_map.items():
		if not collection:
			print(f"[Blend Vault][Collectors] No collection for {asset_type}")
			continue
			
		print(f"[Blend Vault][Collectors] Processing {asset_type} collection with {len(collection)} items")
		
		for item in collection:
			if not item:
				continue
						# Check if item is an asset or scene
			is_asset = asset_type == "Scene" or getattr(item, 'asset_data', None) is not None
			if not is_asset:
				print(f"[Blend Vault][Collectors]   Skipping {getattr(item, 'name', 'unnamed')} - not an asset")
				continue
				
			library = getattr(item, 'library', None)
			item_name = getattr(item, 'name', f'Unnamed{asset_type}')
			
			print(f"[Blend Vault][Collectors]   Processing {asset_type} asset: {item_name} (library: {library.name if library else 'None'})")
					# Collect asset info
			if library is None:
				# For local assets, ensure they have UUIDs and use them
				ensure_library_hash(item)
				asset_info = {
					"name": item_name,
					"type": asset_type,
					"uuid": ensure_library_hash(item)
				}
				local_assets[asset_info["uuid"]] = asset_info
				print(f"[Blend Vault][Collectors]     Local asset: {item_name} -> UUID: {asset_info['uuid']}")
			else:
				# For linked assets, try to get existing UUID from their properties
				# If they don't have one, we'll adopt it from the library sidecar later
				existing_uuid = None
				if hasattr(item, 'id_properties_ensure'):
					props = item.id_properties_ensure()
					existing_uuid = props.get(BV_UUID_PROP)
				
				# Don't generate new UUIDs for linked assets - they should adopt from library
				# The UUID will be set when the library sidecar is processed
				asset_info = {
					"name": item_name,
					"type": asset_type,
					"uuid": existing_uuid  # May be None, will be set from library sidecar
				}
				linked_assets_by_library.setdefault(library, []).append(asset_info)
				print(f"[Blend Vault][Collectors]     Linked asset: {item_name} -> UUID: {existing_uuid} (library: {library.name})")
		print(f"[Blend Vault][Collectors] Collection complete:")
	print(f"[Blend Vault][Collectors]   Local assets: {len(local_assets)}")
	print(f"[Blend Vault][Collectors]   Linked libraries: {len(linked_assets_by_library)}")
	
	for lib, assets in linked_assets_by_library.items():
		print(f"[Blend Vault][Collectors]     Library {lib.name}: {len(assets)} assets")
		for asset in assets:
			print(f"[Blend Vault][Collectors]       - {asset['name']} ({asset['type']}) UUID={asset['uuid']}")
	
	# Resolve UUIDs for linked assets from library sidecars
	blend_filepath = bpy.data.filepath
	print(f"[Blend Vault][Collectors] Current blend file: {blend_filepath}")
	
	if blend_filepath:
		print(f"[Blend Vault][Collectors] Calling UUID resolution...")
		_resolve_linked_asset_uuids(linked_assets_by_library, blend_filepath)
		
		print(f"[Blend Vault][Collectors] After UUID resolution:")
		for lib, assets in linked_assets_by_library.items():
			print(f"[Blend Vault][Collectors]   Library {lib.name}:")
			for asset in assets:
				print(f"[Blend Vault][Collectors]     - {asset['name']} ({asset['type']}) UUID={asset['uuid']}")
		
		# Preserve existing UUIDs from current file's sidecar for local assets
		print(f"[Blend Vault][Collectors] Preserving existing UUIDs for local assets...")
		_preserve_existing_uuids_from_current_sidecar(local_assets, blend_filepath)
		
		print(f"[Blend Vault][Collectors] After UUID preservation:")
		for uuid_key, asset in local_assets.items():
			print(f"[Blend Vault][Collectors]   Local asset: {asset['name']} ({asset['type']}) UUID={asset['uuid']}")
	else:
		print(f"[Blend Vault][Collectors] No blend filepath - skipping UUID resolution and preservation")
	
	print(f"[Blend Vault][Collectors] collect_assets() returning {len(local_assets)} local, {len(linked_assets_by_library)} linked libraries")
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
		print(f"[Blend Vault][UUID Preservation] No blend filepath - skipping preservation")
		return
		
	sidecar_path = blend_path + SIDECAR_EXTENSION
	if not os.path.exists(sidecar_path):
		print(f"[Blend Vault][UUID Preservation] No existing sidecar found at: {sidecar_path}")
		return
		
	print(f"[Blend Vault][UUID Preservation] Reading existing UUIDs from current file's sidecar: {sidecar_path}")
	
	try:
		# Read existing assets from the current file's sidecar
		existing_assets = _get_current_file_assets_from_sidecar(sidecar_path)
		print(f"[Blend Vault][UUID Preservation] Found {len(existing_assets)} existing assets in sidecar")
		
		# Create lookup by (name, type) for existing UUIDs
		existing_uuid_lookup = {}
		for existing_asset in existing_assets:
			name = existing_asset.get("name")
			asset_type = existing_asset.get("type")
			existing_uuid = existing_asset.get("uuid")
			if name and asset_type and existing_uuid:
				existing_uuid_lookup[(name, asset_type)] = existing_uuid
				print(f"[Blend Vault][UUID Preservation]   Existing: {name} ({asset_type}) -> {existing_uuid}")
		
		# Update local assets with existing UUIDs where they match
		preserved_count = 0
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
					print(f"[Blend Vault][UUID Preservation] ✓ PRESERVED: '{asset['name']}' ({asset['type']}) {old_uuid} -> {preserved_uuid}")
					preserved_count += 1
				else:
					print(f"[Blend Vault][UUID Preservation] ✓ SAME: '{asset['name']}' ({asset['type']}) already has correct UUID: {preserved_uuid}")
					preserved_count += 1
		
		print(f"[Blend Vault][UUID Preservation] Preserved {preserved_count} existing UUIDs")
		
	except Exception as e:
		print(f"[Blend Vault][UUID Preservation] ERROR reading current sidecar {sidecar_path}: {e}")
		import traceback
		print(f"[Blend Vault][UUID Preservation] Traceback: {traceback.format_exc()}")
