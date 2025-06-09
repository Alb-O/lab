"""
Main sidecar writing orchestration for Blend Vault.
Coordinates asset collection, content building, and file operations.
"""

import bpy
import os
from ..core import SIDECAR_EXTENSION, LOG_COLORS, log_info, log_warning, log_error, log_success, log_debug
from ..utils.constants import PREVIEW_EXTENSION
from ..preferences import get_obsidian_vault_root
from .collectors import collect_assets, collect_resources
from .content_builder import build_sidecar_content
from .file_operations import write_sidecar_with_content_preservation, push_uuid_to_sidecar


def generate_preview_if_needed(blend_path: str) -> bool:
	"""Generate preview image for the blend file if it doesn't exist.
	
	Args:
		blend_path: absolute path to the blend file
		
	Returns:
		True if preview exists (was created or already existed), False otherwise
	"""
	# Use splitext to properly remove .blend extension before adding preview extension
	base, _ = os.path.splitext(blend_path)
	preview_path = base + PREVIEW_EXTENSION
	
	# Check if preview already exists
	if os.path.exists(preview_path):
		return True
	
	# Import here to avoid circular imports
	try:
		from ..obsidian_integration.preview_image import save_blend_preview_to_png
		log_info(f"Generating preview image: {preview_path}", module_name="SidecarWriter")
		if save_blend_preview_to_png(blend_path, preview_path):
			log_success(f"Preview image generated: {preview_path}", module_name="SidecarWriter")
			return True
		else:
			log_warning(f"Failed to generate preview image for: {blend_path}", module_name="SidecarWriter")
			return False
	except ImportError as e:
		log_error(f"Could not import preview image module: {e}", module_name="SidecarWriter")
		return False
	except Exception as e:
		log_error(f"Error generating preview image: {e}", module_name="SidecarWriter")
		return False


def generate_preview_on_save(blend_path: str) -> bool:
	"""Always generate/update preview image for the blend file on save.
	
	This function is optimized for the save workflow and always regenerates
	the preview since the operation is now very fast with the new implementation.
	
	Args:
		blend_path: absolute path to the blend file
		
	Returns:
		True if preview was successfully generated, False otherwise
	"""
	# Use splitext to properly remove .blend extension before adding preview extension
	base, _ = os.path.splitext(blend_path)
	preview_path = base + PREVIEW_EXTENSION
	
	# Import here to avoid circular imports
	try:
		from ..obsidian_integration.preview_image import save_blend_preview_to_png
		log_debug(f"Updating preview image on save: {preview_path}", module_name="SidecarWriter")
		if save_blend_preview_to_png(blend_path, preview_path):
			log_debug(f"Preview image updated: {preview_path}", module_name="SidecarWriter")
			return True
		else:
			log_warning(f"Failed to update preview image on save: {blend_path}", module_name="SidecarWriter")
			return False
	except ImportError as e:
		log_warning(f"Could not import preview image module for save update: {e}", module_name="SidecarWriter")
		return False
	except Exception as e:
		log_warning(f"Error updating preview image on save: {e}", module_name="SidecarWriter")
		return False


@bpy.app.handlers.persistent
def write_library_info(*args, **kwargs):
	"""Main handler to write sidecar file."""
	blend_path = bpy.data.filepath
	if not blend_path:
		log_warning("No blend file path found, skipping write", module_name="SidecarWriter")
		return

	# Use absolute path - content builder will compute vault-relative paths
	abs_blend_path = os.path.abspath(blend_path)
	
	log_info(f"Writing sidecar for: {abs_blend_path}", module_name="SidecarWriter")
	# Note: Asset relinking is now handled by the missing links dialog for user confirmation
	# No automatic relinking during sidecar write operations
	# Collect data
	local_assets, linked_assets_by_library = collect_assets()
	resources = collect_resources()
	
	# Always generate/update preview image on save (operation is now very fast)
	generate_preview_on_save(abs_blend_path)
	
	# Build content
	sidecar_content, uuid_pushes = build_sidecar_content(
		abs_blend_path,
		local_assets,
		linked_assets_by_library,
		resources
	)
	# Generate preview image link if preview exists
	preview_link = None
	base, _ = os.path.splitext(abs_blend_path)
	preview_path = base + PREVIEW_EXTENSION
	if os.path.exists(preview_path):
		# Get vault root to create vault-relative preview link
		vault_root = get_obsidian_vault_root()
		if vault_root:
			try:
				vault_rel_preview_path = os.path.relpath(preview_path, vault_root).replace(os.sep, '/')
				preview_link = vault_rel_preview_path
			except ValueError:
				# Path is not relative to vault root, skip preview link
				log_warning(f"Preview image is outside vault, skipping preview link: {preview_path}", module_name="SidecarWriter")
		else:
			log_warning("No vault root configured, skipping preview link", module_name="SidecarWriter")

	# Write main sidecar
	md_path = abs_blend_path + SIDECAR_EXTENSION
	try:
		write_sidecar_with_content_preservation(md_path, sidecar_content, preview_link)
		log_success(f"Sidecar written: {md_path}", module_name="SidecarWriter")
	except Exception as e:
		log_error(f"Failed to write sidecar {md_path}: {e}", module_name="SidecarWriter")
		return
	# Push UUIDs to linked library sidecars
	for lib_sidecar_path, (file_uuid, asset_updates) in uuid_pushes.items():
		# Validate linked blend file exists
		linked_blend_path = lib_sidecar_path[:-len(SIDECAR_EXTENSION)]
		if os.path.exists(linked_blend_path) and (asset_updates or file_uuid):
			push_uuid_to_sidecar(lib_sidecar_path, file_uuid, asset_updates)
		elif not os.path.exists(linked_blend_path):
			log_warning(f"Skipping push to {lib_sidecar_path} - linked blend file missing", module_name="SidecarWriter")

	# Now that library sidecars exist, resolve UUIDs for linked assets and update main sidecar
	from .collectors import _resolve_linked_asset_uuids
	# Re-resolve UUIDs now that library sidecars exist
	_resolve_linked_asset_uuids(linked_assets_by_library, abs_blend_path)

	# Check if any UUIDs were resolved
	uuids_resolved = False
	for lib, assets in linked_assets_by_library.items():
		for asset in assets:
			if asset["uuid"] is not None:
				uuids_resolved = True
				break
		if uuids_resolved:
			break	# If UUIDs were resolved, rebuild and rewrite the main sidecar
	if uuids_resolved:
		updated_sidecar_content, _ = build_sidecar_content(
			abs_blend_path,
			local_assets,
			linked_assets_by_library,
			resources
		)
		try:
			write_sidecar_with_content_preservation(md_path, updated_sidecar_content, preview_link)
			log_success(f"Main sidecar updated with resolved UUIDs", module_name="SidecarWriter")
		except Exception as e:
			log_error(f"Failed to update main sidecar: {e}", module_name="SidecarWriter")
