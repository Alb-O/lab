"""
File operation utilities for Blend Vault.
Handles reading, writing, and managing sidecar files.
"""

import os
import json
import re
from typing import Dict, Optional
from ..core import (
	LOG_COLORS,
	SIDECAR_EXTENSION,
	BV_FILE_UUID_KEY,
	format_primary_link,
	log_info, log_warning, log_error, log_success, log_debug
)
from ..utils.constants import FRONTMATTER_TAGS
from ..utils.templates import build_template_heading, build_template_heading_regex, HEADING_LEVEL_3
from .frontmatter import generate_frontmatter_string
from .content_builder import build_simple_current_file_content


def _build_current_file_section_regex() -> str:
	"""Build a regex pattern to match the Current File section in both plain and markdown link formats."""
	# Use the template system to build the regex pattern for the current_file section
	pattern = build_template_heading_regex("current_file")
	# Add the JSON block pattern
	return rf"{pattern}\s*\n```json\s*\n(.*?)\n```"

def _log(level: str, message: str) -> None:
	if level == 'INFO':
		log_info(message, module_name='SidecarFileOps')
	elif level == 'WARN':
		log_warning(message, module_name='SidecarFileOps')
	elif level == 'ERROR':
		log_error(message, module_name='SidecarFileOps')
	elif level == 'SUCCESS':
		log_success(message, module_name='SidecarFileOps')
	elif level == 'DEBUG':
		log_debug(message, module_name='SidecarFileOps')
	else:
		print(f"{message}")

def write_sidecar_with_content_preservation(md_path: str, new_data_content: str, preview_link: Optional[str] = None) -> None:
	"""Write sidecar while preserving user content."""
	original_lines = []
	if os.path.exists(md_path):
		with open(md_path, 'r', encoding='utf-8') as f:
			original_lines = f.readlines()
	
	# Generate frontmatter and extract user content
	frontmatter, fm_end_idx = generate_frontmatter_string(original_lines, list(FRONTMATTER_TAGS), preview_link)
	user_content = ""
	
	if original_lines:
		user_lines = original_lines[fm_end_idx + 1:] if fm_end_idx != -1 else original_lines
		# Find and remove existing BV Data section
		blend_vault_heading = build_template_heading("main_heading")
		for i, line in enumerate(user_lines):
			if line.strip() == blend_vault_heading:
				user_lines = user_lines[:i]
				break
		
		user_content = "".join(user_lines).strip()
	
	# Assemble final content
	content_parts = [frontmatter]
	if user_content:
		content_parts.extend([user_content, "\n\n"])
	elif frontmatter:
		content_parts.append("\n")
	
	content_parts.append(new_data_content)
	
	with open(md_path, 'w', encoding='utf-8') as f:
		f.write("".join(content_parts))

def push_uuid_to_sidecar(sidecar_path: str, file_uuid: str, asset_updates: Dict) -> None:
	"""Push UUID and asset updates to a sidecar file."""
	try:
		# Read existing content
		original_lines = []
		if os.path.exists(sidecar_path):
			with open(sidecar_path, 'r', encoding='utf-8') as f:
				original_lines = f.readlines()		
		# Extract existing assets
		existing_assets = []
		blend_vault_heading = build_template_heading("main_heading")
		
		for i, line in enumerate(original_lines):
			if line.strip() == blend_vault_heading:
				content_after = ''.join(original_lines[i:])
				# Use dynamic regex pattern for Current File section
				current_file_pattern = _build_current_file_section_regex()
				json_match = re.search(current_file_pattern, content_after, re.DOTALL)
				if json_match:
					try:
						data = json.loads(json_match.group(1))
						existing_assets = data.get('assets', [])
					except json.JSONDecodeError:
						pass
				break
				# Merge assets
		if asset_updates:
			asset_dict = {asset.get('uuid'): asset for asset in existing_assets if asset.get('uuid')}
			asset_dict.update(asset_updates)
			existing_assets = list(asset_dict.values())
			
		# Build new content using consolidated function
		new_content = build_simple_current_file_content(
			sidecar_path.replace(SIDECAR_EXTENSION, ''),
			file_uuid,
			existing_assets
		)
		
		write_sidecar_with_content_preservation(sidecar_path, new_content)
		_log('SUCCESS', f"[Blend Vault] Pushed UUIDs to sidecar: {sidecar_path}")
		
	except Exception as e:
		_log('ERROR', f"[Blend Vault] Failed to push UUIDs to sidecar {sidecar_path}: {e}")
