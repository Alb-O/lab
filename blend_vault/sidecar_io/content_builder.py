"""
Content building utilities for Blend Vault.
Handles building sidecar markdown content from collected data.
"""

import bpy
import os
import json
import re
from typing import Dict, List, Tuple
from .. import (
	generate_filepath_hash,  # Renamed from ensure_library_hash
	get_resource_warning_prefix,
	SIDECAR_EXTENSION,	BV_UUID_PROP,
	BV_FILE_UUID_KEY,
	format_primary_link
)
from ..utils.constants import (
	SIDECAR_NO_ITEMS,
	SIDECAR_JSON_BLOCK_START,
	SIDECAR_JSON_BLOCK_END,
	RESOURCE_TYPE_ORDER,
	RESOURCE_TYPE_DISPLAY_NAMES
)
from ..utils.templates import (
	build_template_heading,
	SIDECAR_MESSAGE_EMBED,
	HEADING_LEVEL_3,
	HEADING_LEVEL_4,
	get_heading_prefix
)
from .. import get_obsidian_vault_root
from .uuid_manager import read_sidecar_uuid


def build_sidecar_content(
	blend_path: str, 
	local_assets: Dict, 
	linked_assets_by_library: Dict,
	resources: List[dict]
) -> Tuple[str, Dict]:
	"""Build sidecar content and track UUID pushes."""
	file_uuid = read_sidecar_uuid(blend_path + SIDECAR_EXTENSION) or generate_filepath_hash(blend_path)
		# Build content sections
	blend_filename = os.path.basename(blend_path)
	sections = [
		build_template_heading("main_heading"),
		SIDECAR_MESSAGE_EMBED,
		build_template_heading("current_file", "./" + blend_filename),
		SIDECAR_JSON_BLOCK_START,
		json.dumps({
			"path": os.path.basename(blend_path),
			BV_FILE_UUID_KEY: file_uuid,
			"assets": list(local_assets.values())
		}, indent=2, ensure_ascii=False),
		SIDECAR_JSON_BLOCK_END,
		build_template_heading("linked_libraries")
	]
	
	# Add linked libraries section
	uuid_pushes = {}
	libraries = list(bpy.data.libraries)
	
	if not libraries:
		sections.append(SIDECAR_NO_ITEMS)
	else:
		uuid_pushes = _build_linked_libraries_section(sections, libraries, blend_path, linked_assets_by_library)
	
	# Add resources section
	_build_resources_section(sections, resources, blend_path)
	
	return "\n".join(sections) + "\n", uuid_pushes


def build_simple_current_file_content(
	blend_path: str,
	file_uuid: str,
	assets: List[dict]
) -> str:
	"""Build simple sidecar content with just current file section for UUID pushing."""
	blend_filename = os.path.basename(blend_path)
	sections = [
		build_template_heading("main_heading"),
		SIDECAR_MESSAGE_EMBED,
		"",
		build_template_heading("current_file", "./" + blend_filename),
		SIDECAR_JSON_BLOCK_START,
		json.dumps({
			"path": blend_filename,
			BV_FILE_UUID_KEY: file_uuid,
			"assets": assets
		}, indent=2, ensure_ascii=False),
		SIDECAR_JSON_BLOCK_END,
		"",
		build_template_heading("linked_libraries"),
		SIDECAR_NO_ITEMS,
		""
	]
	
	return "\n".join(sections)


def _build_linked_libraries_section(
	sections: List[str], 
	libraries: List, 
	blend_path: str, 
	linked_assets_by_library: Dict
) -> Dict:
	"""Build the linked libraries section and return UUID pushes."""
	uuid_pushes = {}
	for lib in libraries:
		# Process library path
		lib_path = lib.filepath.lstrip('//').replace('\\', '/')
		# Ensure lib_path doesn't already have sidecar extension
		if lib_path.endswith(SIDECAR_EXTENSION):
			lib_path = lib_path[:-len(SIDECAR_EXTENSION)]
		
		# Keep the library sidecar path relative for local path resolution
		# but use absolute path for file existence checking
		lib_sidecar_path = os.path.normpath(
			os.path.join(os.path.dirname(blend_path), lib_path)
		) + SIDECAR_EXTENSION
		
		# Get or generate library UUID
		lib_uuid = read_sidecar_uuid(lib_sidecar_path)
		uuid_was_generated = False
		
		if not lib_uuid:
			lib_uuid = generate_filepath_hash(lib.filepath)  # Use renamed function
			uuid_was_generated = True
		
		# Store UUID on library datablock
		lib.id_properties_ensure()[BV_UUID_PROP] = lib_uuid
		# Only push UUIDs to libraries that don't have sidecars yet
		# If a library already has a sidecar, it should manage its own UUIDs
		linked_assets = linked_assets_by_library.get(lib, [])
		new_assets = {}
		
		if not os.path.exists(lib_sidecar_path):
			# Library has no sidecar - we need to create initial sidecar with proper asset UUIDs
			# Generate UUIDs for each linked asset based on their library and name
			for asset in linked_assets:
				if asset["uuid"] is None:
					# Generate a deterministic UUID for this library asset
					# This ensures the same asset always gets the same UUID
					asset_identifier = f"{lib_uuid}:{asset['name']}:{asset['type']}"
					import hashlib
					asset_uuid = str(hashlib.md5(asset_identifier.encode()).hexdigest())
					# Format as UUID
					asset_uuid = f"{asset_uuid[:8]}-{asset_uuid[8:12]}-{asset_uuid[12:16]}-{asset_uuid[16:20]}-{asset_uuid[20:32]}"
					asset["uuid"] = asset_uuid
				
				new_assets[asset["uuid"]] = asset
		# If library already has a sidecar, don't push any UUIDs - let library manage its own
		# Schedule UUID push only for new libraries without sidecars
		if not os.path.exists(lib_sidecar_path) and (uuid_was_generated or new_assets):
			uuid_pushes[lib_sidecar_path] = (lib_uuid, new_assets)
		
		# Add to sidecar content
		vault_root = None
		# The get_obsidian_vault_root is imported at the top of the file.
		# We need to ensure it's not None before calling.
		if get_obsidian_vault_root is not None:
			vault_root = get_obsidian_vault_root() # Call the function
		
		# If get_obsidian_vault_root was None (e.g. import issue), vault_root remains None.
		# get_resource_warning_prefix is expected to handle vault_root being None.
		warning_prefix = get_resource_warning_prefix(lib_path, blend_path, vault_root)
		library_display_name = warning_prefix + os.path.basename(lib_path)
		sections.extend([
			build_template_heading("library_entry", lib_path + SIDECAR_EXTENSION, library_display_name),
			SIDECAR_JSON_BLOCK_START,
			json.dumps({
				"path": lib_path,
				"uuid": lib_uuid,
				"assets": linked_assets
			}, indent=2, ensure_ascii=False),
			SIDECAR_JSON_BLOCK_END,
			""
		])
	
	return uuid_pushes


def _build_resources_section(sections: List[str], resources: List[dict], blend_path: str) -> None:
	"""Build the resources section with categorized subheadings."""
	sections.extend([
		build_template_heading("resources")
	])
	
	if not resources:
		sections.append(SIDECAR_NO_ITEMS)
		return
	# Group resources by type
	resources_by_type = {}
	for resource in resources:
		resource_type = resource["type"]
		if resource_type not in resources_by_type:
			resources_by_type[resource_type] = []
		resources_by_type[resource_type].append(resource)
	
	# Use constants for type order and display names
	vault_root_for_res_section = None
	# Similar check for the resources section
	if get_obsidian_vault_root is not None:
		vault_root_for_res_section = get_obsidian_vault_root() # Call the function
	
	# Add each resource type as a subheading
	for resource_type in RESOURCE_TYPE_ORDER:
		if resource_type in resources_by_type:
			sections.extend([
				f"{get_heading_prefix(3)}{RESOURCE_TYPE_DISPLAY_NAMES[resource_type]}"
			])
			
			for resource in resources_by_type[resource_type]:
				# Get warning prefix if resource is outside the vault
				warning_prefix = get_resource_warning_prefix(
					resource["path"], 
					blend_path, 
					vault_root_for_res_section # Use the potentially None vault_root_for_res_section
				)
				
				sections.append('- ' + warning_prefix + format_primary_link(resource["path"], resource["name"]))
