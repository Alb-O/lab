"""
Resource relinking utilities for Blend Vault.
Handles relinking external resources (textures, videos, audio, scripts, caches) based on sidecar file information.
"""

import bpy  # type: ignore
import os
import re
import traceback
from typing import Dict, List, Optional
from utils import (
	SIDECAR_EXTENSION,
	LOG_COLORS,
	MD_PRIMARY_FORMAT,
	RESOURCE_WARNING_PREFIX
)


def _log(level: str, message: str) -> None:
	"""Simplified logging function."""
	print(f"{LOG_COLORS.get(level, '')}{message}{LOG_COLORS['RESET']}")


def _remove_warning_prefix(text: str) -> str:
	"""Remove warning prefix from a text line if present."""
	if text.startswith(RESOURCE_WARNING_PREFIX):
		return text[len(RESOURCE_WARNING_PREFIX):].strip()
	return text


def _find_resource_by_name_and_type(name: str, resource_type: str):
	"""Find a Blender resource by name and type."""
	collections_map = {
		"Image": bpy.data.images,
		"Video": bpy.data.movieclips,
		"Audio": bpy.data.sounds,
		"Text": bpy.data.texts,
		"Cache": bpy.data.cache_files
	}
	
	collection = collections_map.get(resource_type)
	if not collection:
		return None
	
	for item in collection:
		if item and getattr(item, 'name', '') == name:
			return item
	return None


def _update_resource_path(resource, new_path: str, resource_type: str) -> bool:
	"""Update the filepath of a resource and reload if necessary."""
	try:
		# Convert to Blender relative path format
		rel_path = '//' + new_path
		
		if resource_type == "Image":
			old_path = getattr(resource, 'filepath', '')
			resource.filepath = rel_path
			try:
				resource.reload()
				_log('SUCCESS', f"[Blend Vault][ResourceRelink] Successfully reloaded image '{getattr(resource, 'name', 'unknown')}' from '{old_path}' to '{rel_path}'")
				return True
			except Exception as e:
				_log('ERROR', f"[Blend Vault][ResourceRelink] Failed to reload image '{getattr(resource, 'name', 'unknown')}': {e}")
				return False
				
		elif resource_type == "Video":
			old_path = getattr(resource, 'filepath', '')
			resource.filepath = rel_path
			_log('SUCCESS', f"[Blend Vault][ResourceRelink] Updated video clip '{getattr(resource, 'name', 'unknown')}' from '{old_path}' to '{rel_path}'")
			return True
			
		elif resource_type == "Audio":
			old_path = getattr(resource, 'filepath', '')
			resource.filepath = rel_path
			_log('SUCCESS', f"[Blend Vault][ResourceRelink] Updated sound '{getattr(resource, 'name', 'unknown')}' from '{old_path}' to '{rel_path}'")
			return True
			
		elif resource_type == "Text":
			# For text files, we need to reload the content
			old_path = getattr(resource, 'filepath', '')
			working_dir = os.path.dirname(bpy.data.filepath)
			abs_path = os.path.normpath(os.path.join(working_dir, new_path))
			
			if os.path.exists(abs_path):
				try:
					with open(abs_path, 'r', encoding='utf-8') as f:
						content = f.read()
					resource.from_string(content)
					resource.filepath = rel_path
					_log('SUCCESS', f"[Blend Vault][ResourceRelink] Reloaded text file '{getattr(resource, 'name', 'unknown')}' from '{old_path}' to '{rel_path}'")
					return True
				except Exception as e:
					_log('ERROR', f"[Blend Vault][ResourceRelink] Failed to reload text file '{getattr(resource, 'name', 'unknown')}': {e}")
					return False
			else:
				_log('WARN', f"[Blend Vault][ResourceRelink] Text file not found at '{abs_path}'")
				return False
				
		elif resource_type == "Cache":
			old_path = getattr(resource, 'filepath', '')
			resource.filepath = rel_path
			_log('SUCCESS', f"[Blend Vault][ResourceRelink] Updated cache file '{getattr(resource, 'name', 'unknown')}' from '{old_path}' to '{rel_path}'")
			return True
			
	except Exception as e:
		_log('ERROR', f"[Blend Vault][ResourceRelink] Error updating {resource_type.lower()} '{getattr(resource, 'name', 'unknown')}': {e}")
		return False
	
	return False


def _process_resource_category(lines: List[str], category_header: str, resource_type: str, start_idx: int) -> int:
	"""Process a specific resource category section and return the next line index."""
	_log('INFO', f"[Blend Vault][ResourceRelink] Processing {category_header} section")
	
	current_idx = start_idx
	resources_processed = 0
	
	while current_idx < len(lines):
		line = lines[current_idx].strip()
		
		# Stop if we hit another section
		if line.startswith('####') or line.startswith('###') or line.startswith('##'):
			break
			
		# Process resource links
		if line.startswith('- '):
			# Remove the "- " prefix and any warning prefix
			link_line = line[2:].strip()
			link_line = _remove_warning_prefix(link_line)
			# Match the markdown link format
			md_link_match = re.search(MD_PRIMARY_FORMAT['regex'], link_line)
			if md_link_match:
				resource_name = md_link_match.group(1)
				resource_path = md_link_match.group(2)
				# Unescape the resource name - remove backslash escapes that may have been added during sidecar writing
				original_resource_name = resource_name
				resource_name = resource_name.replace('\\_', '_').replace('\\-', '-').replace('\\(', '(').replace('\\)', ')')
				
				if original_resource_name != resource_name:
					_log('DEBUG', f"[Blend Vault][ResourceRelink] Unescaped resource name: '{original_resource_name}' -> '{resource_name}'")
				
				_log('INFO', f"[Blend Vault][ResourceRelink] Found {resource_type.lower()}: {resource_name} -> {resource_path}")
				
				# Find the resource in Blender
				resource = _find_resource_by_name_and_type(resource_name, resource_type)
				if resource:
					# Check if the path needs updating
					current_path = getattr(resource, 'filepath', '')
					current_path_clean = current_path.lstrip('//').replace('\\', '/')
					
					if current_path_clean != resource_path:
						working_dir = os.path.dirname(bpy.data.filepath)
						abs_path = os.path.normpath(os.path.join(working_dir, resource_path))
						
						if os.path.exists(abs_path):
							if _update_resource_path(resource, resource_path, resource_type):
								resources_processed += 1
						else:
							_log('WARN', f"[Blend Vault][ResourceRelink] Resource file not found: '{abs_path}'")
					else:
						_log('INFO', f"[Blend Vault][ResourceRelink] Path for {resource_type.lower()} '{resource_name}' already correct")
				else:
					_log('WARN', f"[Blend Vault][ResourceRelink] {resource_type} '{resource_name}' not found in current file")
		
		current_idx += 1
	
	if resources_processed > 0:
		_log('SUCCESS', f"[Blend Vault][ResourceRelink] Successfully processed {resources_processed} {resource_type.lower()}(s)")
	
	return current_idx


@bpy.app.handlers.persistent
def relink_resources(*args, **kwargs):
	"""Relink external resources based on information in the sidecar Markdown file."""
	if not bpy.data.is_saved:
		_log('WARN', "[Blend Vault][ResourceRelink] Current .blend file is not saved. Cannot process sidecar.")
		return

	blend_path = bpy.data.filepath
	md_path = blend_path + SIDECAR_EXTENSION

	if not os.path.exists(md_path):
		_log('WARN', f"[Blend Vault][ResourceRelink] Sidecar file not found: {md_path}")
		return

	_log('INFO', f"[Blend Vault][ResourceRelink] Processing sidecar file: {md_path}")
	
	try:
		with open(md_path, 'r', encoding='utf-8') as f:
			lines = f.readlines()

		# Find the "### Resources" section
		resources_header_idx = -1
		for i, line in enumerate(lines):
			if line.strip() == "### Resources":
				resources_header_idx = i
				break
		
		if resources_header_idx == -1:
			_log('INFO', "[Blend Vault][ResourceRelink] '### Resources' section not found in sidecar file.")
			return

		# Check if resources section is empty
		if (resources_header_idx + 1 < len(lines) and 
			lines[resources_header_idx + 1].strip() == "- None"):
			_log('INFO', "[Blend Vault][ResourceRelink] No resources to process (marked as 'None').")
			return

		# Process each resource category
		resource_categories = [
			("#### Textures", "Image"),
			("#### Videos", "Video"),
			("#### Audio", "Audio"),
			("#### Scripts", "Text"),
			("#### Caches", "Cache")
		]
		
		current_idx = resources_header_idx + 1
		total_processed = 0
		
		while current_idx < len(lines):
			line = lines[current_idx].strip()
			
			# Stop if we hit a new major section
			if line.startswith('##') and not line.startswith('####'):
				break
			
			# Check for resource category headers
			category_found = False
			for category_header, resource_type in resource_categories:
				if line == category_header:
					current_idx += 1  # Move past the header
					next_idx = _process_resource_category(lines, category_header, resource_type, current_idx)
					category_processed = next_idx - current_idx
					total_processed += category_processed
					current_idx = next_idx
					category_found = True
					break
			
			if not category_found:
				current_idx += 1

		if total_processed > 0:
			_log('SUCCESS', f"[Blend Vault][ResourceRelink] Completed resource relinking. Total resources processed: {total_processed}")
		else:
			_log('INFO', "[Blend Vault][ResourceRelink] No resources needed relinking.")

	except Exception as e:
		_log('ERROR', f"[Blend Vault][ResourceRelink] An error occurred during the resource relinking process: {e}")
		traceback.print_exc()

	# Make paths relative at the end
	try:
		bpy.ops.file.make_paths_relative()
		_log('SUCCESS', "[Blend Vault][ResourceRelink] Made all external file paths relative.")
	except RuntimeError as e:
		_log('WARN', f"[Blend Vault][ResourceRelink] Could not make paths relative: {e}")
	except Exception as e:
		_log('ERROR', f"[Blend Vault][ResourceRelink] Error making paths relative: {e}")

	_log('INFO', "[Blend Vault][ResourceRelink] Finished resource relink attempt.")


# Make the function persistent
relink_resources.persistent = True


class BV_OT_RelinkResources(bpy.types.Operator):
	"""Operator to relink resources based on sidecar file"""
	bl_idname = "blend_vault.relink_resources"
	bl_label = "Relink Resources from Sidecar"
	bl_description = "Relink external resources (textures, videos, audio, scripts, caches) based on sidecar file information"
	bl_options = {'REGISTER', 'UNDO'}

	def execute(self, context: bpy.types.Context):
		relink_resources()
		return {'FINISHED'}


def register():
	bpy.utils.register_class(BV_OT_RelinkResources)
	_log('SUCCESS', "[Blend Vault] Resource relinking operator registered.")


def unregister():
	bpy.utils.unregister_class(BV_OT_RelinkResources)
	_log('WARN', "[Blend Vault] Resource relinking operator unregistered.")


if __name__ == "__main__":
	register()
