"""
Resource relinking module for Blend Vault.
Handles relinking external resources (textures, videos, audio, scripts, caches) based on vault-root-relative paths from sidecar files.
"""

import bpy
import os
import re
import traceback
from typing import Dict, Optional, Any

from .. import SIDECAR_EXTENSION, parse_primary_link, RESOURCE_WARNING_PREFIX
from ..preferences import get_obsidian_vault_root
from ..core import log_info, log_warning, log_error, log_success, log_debug
from .shared_utils import (
	SidecarParser,
	PathResolver,
	ResourceManager,
	ensure_saved_file
)


class ResourceRelinkProcessor:
	"""Handles resource relinking using vault-root-relative paths."""
	
	def __init__(self, main_blend_path: str):
		# Get vault root and fail fast if not available
		vault_root = get_obsidian_vault_root()
		if not vault_root:
			raise ValueError("Obsidian vault root is not configured. Resource relinking requires a configured vault root.")
		self.vault_root = vault_root
		
		# Store original main blend path for relative path computation
		self.original_main_blend_path = main_blend_path
		
		# Check if this is a relocated file using redirect handler
		from .redirect_handler import _pending_relocations
		for old_path, new_path in _pending_relocations.items():
			if new_path == main_blend_path:
				self.original_main_blend_path = old_path
				log_info(f"Detected relocation: using original path {old_path} for relative computations", module_name='ResourceRelink')
				break
		
		# All paths computed relative to vault root
		self.main_blend_path = main_blend_path
		self.main_vault_rel = os.path.relpath(self.main_blend_path, self.vault_root).replace(os.sep, '/')
		self.sidecar_path = os.path.normpath(os.path.join(self.vault_root, self.main_vault_rel + SIDECAR_EXTENSION))
		
		log_debug(f"ResourceRelinkProcessor initialized with vault_root={self.vault_root}, main_vault_rel={self.main_vault_rel}", module_name='ResourceRelink')
	
	def process_relink(self) -> None:
		"""Main entry point for resource relinking process."""
		# Skip if sidecar doesn't exist
		if not os.path.exists(self.sidecar_path):
			log_warning(f"Main sidecar file not found: {self.sidecar_path}", module_name='ResourceRelink')
			return
		
		log_info(f"Processing main sidecar for resource relinking: {self.sidecar_path}", module_name='ResourceRelink')
		
		try:
			parser = SidecarParser(self.sidecar_path)
			
			# Process the Resources section - check for subsections
			resource_subsections = [
				"Textures",
				"Videos", 
				"Audio",
				"Text Files",
				"Cache Files"
			]
			
			relink_count = 0
			for subsection in resource_subsections:
				if self._process_resource_subsection(parser, subsection):
					relink_count += 1
			
			if relink_count > 0:
				log_success(f"Successfully processed {relink_count} resource subsections", module_name='ResourceRelink')
			else:
				log_info("No resource entries found in sidecar.", module_name='ResourceRelink')
			
		except Exception as e:
			log_error(f"Error during resource relinking: {e}", module_name='ResourceRelink')
			traceback.print_exc()
	
	def _process_resource_subsection(self, parser: SidecarParser, subsection_name: str) -> bool:
		"""Process a single resource subsection from the Resources section."""
		log_debug(f"Processing subsection: {subsection_name}", module_name='ResourceRelinker')
		
		# Extract resource type from subsection name
		resource_type = self._extract_resource_type_from_subsection(subsection_name)
		if not resource_type:
			log_warning(f"Unknown resource subsection: {subsection_name}", module_name='ResourceRelinker')
			return False
		
		# First, check if we have a Resources section at all
		resources_section_start = parser.find_section_start("Resources")
		if resources_section_start == -1:
			log_debug("No Resources section found", module_name='ResourceRelinker')
			return False
		
		# Look for the specific subsection within Resources
		resource_data = self._extract_resources_from_subsection(parser, resources_section_start, subsection_name)
		
		if not resource_data:
			log_debug(f"No resources found in {subsection_name}", module_name='ResourceRelinker')
			return False
		
		found_any = False
		for resource_path, resource_info in resource_data.items():
			if self._process_single_resource(resource_info, resource_type):
				found_any = True
		
		return found_any
	
	def _extract_resource_type_from_subsection(self, subsection_name: str) -> Optional[str]:
		"""Extract the resource type from subsection name."""
		type_mapping = {
			"Textures": "Image",
			"Videos": "Video",
			"Audio": "Audio", 
			"Text Files": "Text",
			"Cache Files": "Cache"
		}
		return type_mapping.get(subsection_name)
	
	def _extract_resources_from_subsection(self, parser: SidecarParser, resources_start: int, subsection_name: str) -> Dict[str, Dict[str, Any]]:
		"""Extract resources from a specific subsection within the Resources section."""
		# Look for the subsection heading (#### {subsection_name})
		subsection_target = f"#### {subsection_name}"
		subsection_start = -1
		
		# Search from the Resources section start
		for i in range(resources_start + 1, len(parser.lines)):
			line = parser.lines[i].strip()
			if line == subsection_target:
				subsection_start = i
				break
			# Stop if we hit another ### section
			elif line.startswith("### "):
				break
		
		if subsection_start == -1:
			return {}
		
		# Extract markdown links from this subsection
		results = {}
		current_line_idx = subsection_start + 1
		
		while current_line_idx < len(parser.lines):
			line_stripped = parser.lines[current_line_idx].strip()
			
			# Stop if we hit another heading
			if re.match(r"^(###|####)", line_stripped):
				break
			
			# Look for markdown links using parse_primary_link
			md_link_match = parse_primary_link(line_stripped)
			if md_link_match:
				link_path = md_link_match.group(1)
				link_name = md_link_match.group(2) or link_path
				
				# For resources without JSON blocks, create a simple entry
				results[link_path] = {
					"link_name": link_name,
					"link_path": link_path,
					"json_data": {"name": link_name, "path": link_path}
				}
			
			current_line_idx += 1
		
		return results
	
	def _process_single_resource(self, resource_info: Dict[str, Any], resource_type: str) -> bool:
		"""Process a single resource entry using vault-root-relative paths."""
		link_name = resource_info["link_name"]
		link_path = resource_info["link_path"]
		json_data = resource_info["json_data"]
		
		# Extract resource details from JSON
		stored_path = json_data.get("path", "")
		display_name = json_data.get("name", link_name)
		
		# Handle warning prefixes in the name
		clean_name = self._remove_warning_prefix(display_name)
		
		# Unescape markdown characters (specifically underscores)
		clean_name = self._unescape_markdown_name(clean_name)

		log_info(f"Processing {resource_type}: '{clean_name}' -> '{link_path}'", module_name='ResourceRelink')

		# Find the resource in Blender
		resource = ResourceManager.find_resource_by_name(clean_name, resource_type)
		if not resource:
			log_warning(f"{resource_type} '{clean_name}' not found in session", module_name='ResourceRelink')
			return False
		
		# Use the markdown link path preferentially
		target_vault_rel_path = link_path or stored_path
		if not target_vault_rel_path:
			log_warning(f"No path found for {resource_type} '{clean_name}'", module_name='ResourceRelink')
			return False

		# Convert vault-relative path to absolute
		target_abs_path = os.path.normpath(os.path.join(self.vault_root, target_vault_rel_path))
		if not os.path.exists(target_abs_path):
			log_warning(f"Resource file does not exist: {target_abs_path} (vault-relative: {target_vault_rel_path})", module_name='ResourceRelink')
			return False

		# Compute Blender-relative path from original main blend location for resource path
		original_main_dir = os.path.dirname(self.original_main_blend_path)
		blender_rel_path = os.path.relpath(target_abs_path, original_main_dir).replace(os.sep, '/')
		
		log_debug(f"Resource '{clean_name}' paths:", module_name='ResourceRelink')
		log_debug(f"  Vault-relative: {target_vault_rel_path}", module_name='ResourceRelink')
		log_debug(f"  Absolute: {target_abs_path}", module_name='ResourceRelink')
		log_debug(f"  Blender-relative: {blender_rel_path}", module_name='ResourceRelink')

		# Update the resource path with Blender-relative path
		success = ResourceManager.update_resource_filepath(resource, blender_rel_path, resource_type)
		if success:
			log_success(f"Successfully relinked {resource_type} '{clean_name}' to {target_vault_rel_path}", module_name='ResourceRelink')

		return success
	
	def _remove_warning_prefix(self, text: str) -> str:
		"""Remove warning prefix from a text line if present."""
		if text.startswith(RESOURCE_WARNING_PREFIX):
			return text[len(RESOURCE_WARNING_PREFIX):].strip()
		return text
	
	def _unescape_markdown_name(self, name: str) -> str:
		"""Unescape markdown characters in resource names."""
		# Unescape common markdown characters
		unescaped = name.replace('\\_', '_')  # Unescape underscores
		unescaped = unescaped.replace('\\*', '*')  # Unescape asterisks
		unescaped = unescaped.replace('\\-', '-')  # Unescape hyphens
		unescaped = unescaped.replace('\\#', '#')  # Unescape hash symbols
		return unescaped


@bpy.app.handlers.persistent
def relink_resources(*args, **kwargs):
	"""Main entry point for resource relinking. Called by Blender handlers."""
	blend_path = ensure_saved_file()
	if not blend_path:
		return
	
	try:
		processor = ResourceRelinkProcessor(blend_path)
		processor.process_relink()
	except Exception as e:
		log_error(f"Unexpected error: {e}", module_name='ResourceRelinker')
		traceback.print_exc()


# Make the handler persistent
relink_resources.persistent = True


def relink_resources_for_blend(main_blend_path: str) -> None:
	"""
	Main entry point for resource relinking.
	
	Args:
		main_blend_path: Absolute path to the main blend file
	"""
	try:
		processor = ResourceRelinkProcessor(main_blend_path)
		processor.process_relink()
	except Exception as e:
		log_error(f"Resource relinking failed: {e}", module_name='ResourceRelink')


def register():
	log_success("Resource relinking module loaded.", module_name='ResourceRelink')


def unregister():
	log_warning("Resource relinking module unloaded.", module_name='ResourceRelink')


if __name__ == "__main__":
	register()
