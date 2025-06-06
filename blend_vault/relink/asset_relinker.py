"""
Asset relinking module for Blend Vault.
Handles relinking renamed individual asset datablocks by comparing sidecar states.
"""

import bpy
import os
import traceback
from typing import Dict, List, Optional, Any
from .. import (
	get_asset_sources_map,
	BV_UUID_PROP,
	SIDECAR_EXTENSION
)
from ..preferences import get_obsidian_vault_root
from .shared_utils import (
	SidecarParser,
	PathResolver,
	LibraryManager,
	log_info,
	log_warning,
	log_error,
	log_success,
	log_debug,
	get_sidecar_path,
	get_blend_file_path_from_sidecar,
	ensure_saved_file
)


class AssetRelinkProcessor:
	"""Handles the asset relinking logic using vault-root-relative paths."""
	def __init__(self, main_blend_path: str):
		# Get vault root and fail fast if not available
		vault_root = get_obsidian_vault_root()
		if not vault_root:
			raise ValueError("Obsidian vault root is not configured. Asset relinking requires a configured vault root.")
		self.vault_root = vault_root
		
		# Store original main blend path for Blender relative path computation
		self.original_main_blend_path = main_blend_path
		
		# Check if this is a relocated file using redirect handler
		from .redirect_handler import _pending_relocations
		for old_path, new_path in _pending_relocations.items():
			if new_path == main_blend_path:
				self.original_main_blend_path = old_path
				log_info(f"Detected relocation: using original path {old_path} for relative computations", module_name='AssetRelink')
				break
		
		# All paths computed relative to vault root
		self.main_blend_path = main_blend_path
		self.main_vault_rel = os.path.relpath(self.main_blend_path, self.vault_root).replace(os.sep, '/')
		self.sidecar_path = os.path.normpath(os.path.join(self.vault_root, self.main_vault_rel + SIDECAR_EXTENSION))
		
		# Asset sources for missing asset detection
		self.asset_sources_map = get_asset_sources_map()
		
		log_debug(f"AssetRelinkProcessor initialized with vault_root={self.vault_root}, main_vault_rel={self.main_vault_rel}", module_name='AssetRelink')
	
	def process_relink(self) -> None:
		"""Main entry point for asset relinking process."""
		# Ensure libraries are reloaded to pick up renamed assets
		for lib in bpy.data.libraries:
			if lib.filepath and not lib.filepath.startswith('<builtin>'):
				LibraryManager.reload_library(lib.filepath)

		# Skip if sidecar doesn't exist
		if not os.path.exists(self.sidecar_path):
			log_warning(f"Main sidecar file not found: {self.sidecar_path}", module_name='AssetRelink')
			return
		
		log_info(f"Processing main sidecar for asset relinking: {self.sidecar_path}", module_name='AssetRelink')
		
		try:
			parser = SidecarParser(self.sidecar_path)
			
			# Parse linked libraries from main sidecar
			main_linked_data = parser.extract_json_blocks_with_links("Linked Libraries")
			if not main_linked_data:
				log_info("No linked library data found in main sidecar.", module_name='AssetRelink')
				return
			
			# Get authoritative data from library sidecars
			authoritative_data = self._get_authoritative_library_data(main_linked_data)
			if not authoritative_data:
				log_info("No authoritative data found from any library sidecars.", module_name='AssetRelink')
				return
			
			# Identify and execute relink operations
			relink_ops = self._identify_relink_operations(main_linked_data, authoritative_data)
			self._execute_relink_operations(relink_ops)
			
		except Exception as e:
			log_error(f"Error during asset relinking process: {e}", module_name='AssetRelink')
			traceback.print_exc()
	
	def _get_authoritative_library_data(self, main_linked_data: Dict[str, Dict[str, Any]]) -> Dict[str, Dict[str, Dict[str, str]]]:
		"""
		Get authoritative asset information from each library's own sidecar using vault-root-relative paths.
		
		Returns:
			Dict mapping vault-relative library paths to asset UUID -> asset info mappings:
			{
				"Media/libs/lib.blend": {
					"asset_uuid_1": {"name": "AssetName", "type": "AssetType"},
					...
				}
			}
		"""
		authoritative_data = {}
		
		for lib_vault_rel, lib_data in main_linked_data.items():
			# Convert vault-relative path to absolute for sidecar lookup
			lib_abs_path = os.path.normpath(os.path.join(self.vault_root, lib_vault_rel))
			lib_sidecar_path = os.path.normpath(os.path.join(self.vault_root, lib_vault_rel + SIDECAR_EXTENSION))
			
			try:
				lib_parser = SidecarParser(lib_sidecar_path)
				_, assets_in_lib = lib_parser.extract_current_file_section()
				
				if assets_in_lib:
					authoritative_data[lib_vault_rel] = {
						asset["uuid"]: {
							"name": asset["name"],
							"type": asset["type"]
						}
						for asset in assets_in_lib
					}
					log_debug(f"Authoritative data for '{lib_vault_rel}': {len(assets_in_lib)} assets", module_name='AssetRelink')
				else:
					log_info(f"No authoritative data found for library '{lib_vault_rel}'", module_name='AssetRelink')
					
			except Exception as e:
				log_warning(f"Could not read library sidecar for '{lib_vault_rel}': {e}", module_name='AssetRelink')
		
		return authoritative_data
	
	def get_missing_assets(self) -> List[Dict[str, str]]:
		"""
		Identify missing linked assets by checking Blender session data for broken library references.
		Returns a list of dicts with keys: uuid, name, type, lib_vault_rel.
		"""
		missing = []
		
		# Check all linked assets in the current Blender session
		for asset_type, bpy_collection in self.asset_sources_map.items():
			for item in bpy_collection:
				# Skip non-linked items
				if not item.library or not item.library.filepath:
					continue
				
				# Resolve absolute library path
				abs_path = PathResolver.resolve_blender_path(item.library.filepath)
				
				# If library file is missing, skip asset items (library-level missing handled separately)
				if (hasattr(item.library, 'is_missing') and item.library.is_missing) or not os.path.exists(abs_path):
					continue
				
				# Only report items Blender marks as missing
				if not hasattr(item, 'is_missing') or not item.is_missing:
					continue
				
				# Asset is missing: compute vault-relative path for reporting
				try:
					lib_vault_rel = os.path.relpath(abs_path, self.vault_root).replace(os.sep, '/')
				except ValueError:
					# If relpath fails (different drives on Windows), use absolute path as fallback
					lib_vault_rel = abs_path
				
				# Get UUID if available
				uuid = getattr(item, BV_UUID_PROP, None) if hasattr(item, BV_UUID_PROP) else None
				if not uuid and hasattr(item, 'id_properties') and item.id_properties:
					uuid = item.id_properties.get(BV_UUID_PROP)
				
				missing.append({
					"uuid": uuid or "unknown",
					"name": item.name,
					"type": asset_type,
					"lib_vault_rel": lib_vault_rel,
				})
				log_debug(f"Found missing linked asset: {item.name} ({asset_type}) from library {lib_vault_rel}", module_name='AssetRelink')
		
		log_debug(f"Total missing linked assets detected: {len(missing)}", module_name='AssetRelink')
		return missing

	def _identify_relink_operations(
		self,
		main_linked_data: Dict[str, Dict[str, Any]],
		authoritative_data: Dict[str, Dict[str, Dict[str, str]]]
	) -> List[Dict[str, Any]]:
		"""Identify assets that need relinking by comparing names."""
		relink_operations = []
		
		log_debug(f"Starting relink identification with {len(main_linked_data)} libraries", module_name='AssetRelink')
		log_debug(f"Main linked data keys: {list(main_linked_data.keys())}", module_name='AssetRelink')
		log_debug(f"Authoritative data keys: {list(authoritative_data.keys())}", module_name='AssetRelink')
		
		for lib_vault_rel, lib_link_info in main_linked_data.items():
			assets_in_main = lib_link_info["json_data"].get("assets", [])
			authoritative_assets = authoritative_data.get(lib_vault_rel, {})
			
			log_debug(f"Processing library '{lib_vault_rel}': {len(assets_in_main)} assets in main, {len(authoritative_assets)} in authoritative", module_name='AssetRelink')
			
			if not authoritative_assets:
				log_debug(f"Skipping library '{lib_vault_rel}' - no authoritative data", module_name='AssetRelink')
				continue
			
			for asset_info in assets_in_main:
				asset_uuid = asset_info.get("uuid")
				old_name = asset_info.get("name")
				asset_type = asset_info.get("type")
				
				if not asset_uuid or not old_name or not asset_type:
					log_debug(f"Skipping asset with incomplete info: uuid={asset_uuid}, name={old_name}, type={asset_type}", module_name='AssetRelink')
					continue
				
				# Check if we have authoritative data for this asset
				auth_info = authoritative_assets.get(asset_uuid)
				if not auth_info:
					log_debug(f"No authoritative data found for asset UUID {asset_uuid}", module_name='AssetRelink')
					continue
				
				new_name = auth_info["name"]
				
				# Compare names to detect rename
				if old_name != new_name:
					# Convert vault-relative library path to absolute for Blender operations
					lib_abs_path = os.path.normpath(os.path.join(self.vault_root, lib_vault_rel))
					
					relink_operations.append({
						"uuid": asset_uuid,
						"old_name": old_name,
						"new_name": new_name,
						"type": asset_type,
						"lib_abs_path": lib_abs_path,
						"lib_vault_rel": lib_vault_rel
					})
					log_info(f"Asset rename detected: '{old_name}' -> '{new_name}' ({asset_type}) in {lib_vault_rel}", module_name='AssetRelink')
		
		log_info(f"Identified {len(relink_operations)} relink operations", module_name='AssetRelink')
		return relink_operations

	def _execute_relink_operations(self, relink_ops: List[Dict[str, Any]]) -> None:
		"""Execute the identified relink operations."""
		if not relink_ops:
			log_info("No relink operations to execute.", module_name='AssetRelink')
			return
		
		success_count = 0
		
		for op in relink_ops:
			try:
				# Compute Blender relative path from original main blend location
				original_main_dir = os.path.dirname(self.original_main_blend_path)
				blender_rel_path = os.path.relpath(op["lib_abs_path"], original_main_dir).replace(os.sep, '/')
				blender_lib_path = PathResolver.blender_relative_path(blender_rel_path)
				
				success = self._relink_single_asset(
					op["uuid"], op["old_name"], op["new_name"], 
					op["type"], blender_lib_path, op["lib_vault_rel"]
				)
				if success:
					success_count += 1
			except Exception as e:
				log_error(f"Error executing relink operation for {op['old_name']} -> {op['new_name']}: {e}", module_name='AssetRelink')
				log_success(f"Successfully relinked {success_count}/{len(relink_ops)} assets", module_name='AssetRelink')

	def _relink_single_asset(
		self,
		asset_uuid: str,
		old_name: str,
		new_name: str,
		asset_type: str,
		blender_lib_path: str,
		lib_vault_rel: str
	) -> bool:
		"""
		Relink a single asset by updating its name in Blender's data structures.
		
		Args:
			asset_uuid: UUID of the asset to relink
			old_name: Current name of the asset in Blender
			new_name: New name from the authoritative library sidecar
			asset_type: Type of the asset (Collection, Object, etc.)
			blender_lib_path: Blender-relative path to the library file
			lib_vault_rel: Vault-relative path to the library (for logging)
		
		Returns:
			True if relinking was successful, False otherwise
		"""
		# Get the appropriate Blender data collection for this asset type
		asset_sources = self.asset_sources_map.get(asset_type)
		if not asset_sources:
			log_error(f"Unknown asset type: {asset_type}", module_name='AssetRelink')
			return False
		
		# Find the asset by UUID and old name
		target_asset = None
		for asset in asset_sources:
			# Check if this asset is from the target library
			if not asset.library or asset.library.filepath != blender_lib_path:
				continue
			
			# Check UUID match
			asset_uuid_prop = getattr(asset, BV_UUID_PROP, None) if hasattr(asset, BV_UUID_PROP) else None
			if not asset_uuid_prop and hasattr(asset, 'id_properties') and asset.id_properties:
				asset_uuid_prop = asset.id_properties.get(BV_UUID_PROP)
			
			if asset_uuid_prop == asset_uuid and asset.name == old_name:
				target_asset = asset
				break
		
		if not target_asset:
			log_warning(f"Could not find asset with UUID {asset_uuid} and name '{old_name}' in library {lib_vault_rel}", module_name='AssetRelink')
			return False
		
		try:
			# Update the asset name to match the authoritative name
			target_asset.name = new_name
			log_success(f"Relinked asset: '{old_name}' -> '{new_name}' ({asset_type}) in {lib_vault_rel}", module_name='AssetRelink')
			return True
			
		except Exception as e:
			log_error(f"Failed to relink asset '{old_name}' -> '{new_name}': {e}", module_name='AssetRelink')
			return False


def relink_assets(main_blend_path: str) -> None:
	"""
	Main entry point for asset relinking.
	
	Args:
		main_blend_path: Absolute path to the main blend file
	"""
	try:
		processor = AssetRelinkProcessor(main_blend_path)
		processor.process_relink()
	except Exception as e:
		log_error(f"Asset relinking failed: {e}", module_name='AssetRelink')


def get_missing_assets(main_blend_path: str) -> List[Dict[str, str]]:
	"""
	Get a list of missing linked assets.
	
	Args:
		main_blend_path: Absolute path to the main blend file
		
	Returns:
		List of missing asset dictionaries with keys: uuid, name, type, lib_vault_rel
	"""
	try:
		processor = AssetRelinkProcessor(main_blend_path)
		return processor.get_missing_assets()
	except Exception as e:
		log_error(f"Failed to get missing assets: {e}", module_name='AssetRelink')
		return []
