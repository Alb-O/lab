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
	"""Handles the asset relinking logic."""
	
	def __init__(self, main_blend_path: str):
		self.main_blend_path = main_blend_path
		self.main_blend_dir = os.path.dirname(main_blend_path)
		self.sidecar_path = get_sidecar_path(main_blend_path)
		self.asset_sources_map = get_asset_sources_map()
	
	def process_relink(self) -> None:
		"""Main entry point for asset relinking process."""
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
		Get authoritative asset information from each library's own sidecar.
		
		Returns:
			Dict mapping library paths to asset UUID -> asset info mappings:
			{
				"path/to/lib.blend": {
					"asset_uuid_1": {"name": "AssetName", "type": "AssetType"},
					...
				}
			}
		"""
		authoritative_data = {}
		
		for lib_rel_path, lib_data in main_linked_data.items():
			# Determine absolute library path. lib_rel_path is expected to be
			# vault-relative (e.g., "data/library.blend.side.md").
			# Resolve it from the vault root.
			lib_abs_path = PathResolver.resolve_from_vault(lib_rel_path) # MODIFIED LINE
			
			# Parse the library's own sidecar (sidecar file is authoritative without needing to reload)
			lib_sidecar_path = get_sidecar_path(lib_abs_path)
			try:
				lib_parser = SidecarParser(lib_sidecar_path)
				_, assets_in_lib = lib_parser.extract_current_file_section()
				
				if assets_in_lib:
					authoritative_data[lib_rel_path] = {
						asset["uuid"]: {
							"name": asset["name"],
							"type": asset["type"]
						}
						for asset in assets_in_lib
					}
					log_debug(f"Authoritative data for '{lib_rel_path}': {len(assets_in_lib)} assets", module_name='AssetRelink')
				else:
					log_info(f"No authoritative data found for library '{lib_rel_path}'", module_name='AssetRelink')
					
			except Exception as e:
				log_warning(f"Could not read library sidecar for '{lib_rel_path}': {e}", module_name='AssetRelink')
		
		return authoritative_data
	
	def get_missing_assets(self) -> List[Dict[str, str]]:
		"""
		Identify missing linked assets by checking Blender session data for broken library references.
		Returns a list of dicts with keys: uuid, name, type, lib_rel_path.
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
				# Asset is missing: report it
				try:
					lib_rel_path = os.path.relpath(abs_path, self.main_blend_dir).replace('\\', '/')
				except ValueError:
					lib_rel_path = item.library.filepath
				# Get UUID if available
				uuid = getattr(item, BV_UUID_PROP, None) if hasattr(item, BV_UUID_PROP) else None
				if not uuid and hasattr(item, 'id_properties') and item.id_properties:
					uuid = item.id_properties.get(BV_UUID_PROP)
				missing.append({
					"uuid": uuid or "unknown",
					"name": item.name,
					"type": asset_type,
					"lib_rel_path": lib_rel_path,
				})
				log_debug(f"Found missing linked asset: {item.name} ({asset_type}) from library {lib_rel_path}", module_name='AssetRelink')
		
		log_debug(f"Total missing linked assets detected: {len(missing)}", module_name='AssetRelink')
		return missing
	def _identify_relink_operations(
		self,
		main_linked_data: Dict[str, Dict[str, Any]],
		authoritative_data: Dict[str, Dict[str, Dict[str, str]]]
	) -> List[Dict[str, Any]]:
		"""Identify assets that need relinking by comparing names."""
		relink_operations = []
		
		# Check assets recorded in main sidecar against authoritative data
		for lib_rel_path, lib_link_info in main_linked_data.items():
			assets_in_main = lib_link_info["json_data"].get("assets", [])
			authoritative_assets = authoritative_data.get(lib_rel_path, {})
			
			if not authoritative_assets:
				continue
			
			for asset_info in assets_in_main:
				asset_uuid = asset_info.get("uuid")
				old_name = asset_info.get("name")
				asset_type = asset_info.get("type")
				
				if not all([asset_uuid, old_name, asset_type]):
					continue
				
				current_asset_info = authoritative_assets.get(asset_uuid)
				if not current_asset_info:
					continue
				
				current_name = current_asset_info["name"]
				if old_name != current_name:
					log_info(f"Name change detected: '{old_name}' -> '{current_name}' (UUID: {asset_uuid})", module_name='AssetRelink')
					
					# Find the session item and prepare relink operation
					session_item = self._find_and_prepare_session_item(lib_rel_path, asset_uuid, asset_type, old_name)
					if session_item:
						relink_operations.append({
							"session_uid": getattr(session_item, 'session_uid', None),
							"library_path": lib_rel_path,
							"asset_type": asset_type,
							"new_name": current_name,
							"old_name": old_name,
							"uuid": asset_uuid
						})
		
		return relink_operations
	def _find_and_prepare_session_item(self, lib_rel_path: str, asset_uuid: str, asset_type: str, old_name: str):
		"""Find the session item for an asset and ensure it has the correct UUID."""
		bpy_collection = self.asset_sources_map.get(asset_type)
		if not bpy_collection:
			return None
		
		# Convert vault-relative sidecar path to absolute .blend file path
		raw_abs_sidecar_path = PathResolver.resolve_from_vault(lib_rel_path)
		if not os.path.exists(raw_abs_sidecar_path):
			return None

		lib_blend_abs_path = get_blend_file_path_from_sidecar(raw_abs_sidecar_path)
		expected_lib_path = PathResolver.normalize_path(lib_blend_abs_path)
		
		# Find session item by library path and name
		for item in bpy_collection:
			item_lib_path = self._get_item_library_path(item)
			if (item_lib_path and 
				PathResolver.normalize_path(item_lib_path) == expected_lib_path and 
				item.name == old_name):
				# Ensure the session item has the correct UUID
				try:
					item.id_properties_ensure()[BV_UUID_PROP] = asset_uuid
				except Exception:
					pass
				return item
		
		return None
	
	def _get_item_library_path(self, item) -> Optional[str]:
		"""Get the library path for a session item, ensuring it points to the actual .blend file."""
		raw_path = None
		
		if getattr(item, 'library', None) and item.library.filepath:
			raw_path = PathResolver.resolve_blender_path(item.library.filepath)
		elif hasattr(item, 'library_weak_reference') and item.library_weak_reference and item.library_weak_reference.filepath:
			raw_path = PathResolver.resolve_blender_path(item.library_weak_reference.filepath)
		
		if raw_path:
			# Ensure we return the actual .blend file path, not any sidecar path
			cleaned_path = get_blend_file_path_from_sidecar(raw_path)
			if cleaned_path != raw_path:
				log_debug(f"Converted sidecar path '{raw_path}' to blend path '{cleaned_path}'", module_name='AssetRelink')
			return cleaned_path
		return None
	
	def _execute_relink_operations(self, relink_operations: List[Dict[str, Any]]) -> None:
		"""Execute the identified relink operations."""
		if not relink_operations:
			log_info("No relink operations to perform.", module_name='AssetRelink')
			return
		
		log_info(f"Found {len(relink_operations)} operations. Attempting to relocate...", module_name='AssetRelink')
		
		for op in relink_operations:
			log_debug(f"Executing relink operation: {op}", module_name='AssetRelink')
			self._execute_single_relink(op)
	
	def _execute_single_relink(self, op_data: Dict[str, Any]) -> None:
		"""Execute a single relink operation."""
		session_uid = op_data["session_uid"]
		if not session_uid:
			log_warning(f"No session_uid for asset {op_data['uuid']}", module_name='AssetRelink')
			return
		
		# Find the session item
		bpy_collection = self.asset_sources_map.get(op_data["asset_type"])
		session_item = None
		if bpy_collection:
			session_item = next(
				(item for item in bpy_collection if getattr(item, 'session_uid', None) == session_uid),
				None
			)
		
		if not session_item:
			log_warning(f"Could not find session item for session_uid {session_uid}", module_name='AssetRelink')
			return
		
		# Get library filepath
		lib_filepath = self._get_item_library_path(session_item)
		if not lib_filepath:
			log_warning(f"Session item {session_item.name} has no library filepath", module_name='AssetRelink')
			return
		
		log_debug(f"Using library filepath for relink: '{lib_filepath}'", module_name='AssetRelink')
		
		# Prepare relink parameters
		try:
			self._perform_blender_relink(session_item, lib_filepath, op_data)
		except Exception as e:
			log_error(f"Exception during relink: {e}", module_name='AssetRelink')
			traceback.print_exc()
	
	def _perform_blender_relink(self, session_item, lib_filepath: str, op_data: Dict[str, Any]) -> None:
		"""Perform the actual Blender relink operation."""
		asset_type = op_data["asset_type"]
		new_name = op_data["new_name"]
		
		# Determine if path is relative
		is_relative = lib_filepath.startswith("//")
		# Construct file paths
		if is_relative:
			abs_lib_path = PathResolver.resolve_blender_path(lib_filepath)
			filepath_arg = f"{lib_filepath}/{asset_type}/{new_name}"
		else:
			abs_lib_path = PathResolver.normalize_path(lib_filepath)
			filepath_arg = f"{abs_lib_path}/{asset_type}/{new_name}"
			directory_arg = f"{abs_lib_path}/{asset_type}/"
		
		# Store references for post-relink verification
		original_session_uid = op_data["session_uid"]
		backup_uuid = getattr(session_item, BV_UUID_PROP, None)
		
		log_debug(f"Relink parameters: session_uid={original_session_uid}, filepath={filepath_arg}, directory={directory_arg}, filename={new_name}, relative_path={is_relative}", module_name='AssetRelink')
		
		try:
			result = bpy.ops.wm.id_linked_relocate(
				id_session_uid=original_session_uid,
				filepath=filepath_arg,
				directory=directory_arg,
				filename=new_name,
				relative_path=is_relative
			)
			log_success(f"Relink operation for '{op_data['old_name']}' returned: {result}", module_name='AssetRelink')
			
			# Verify the relink was successful
			self._verify_relink_success(op_data, original_session_uid, backup_uuid, new_name, abs_lib_path)
			
		except RuntimeError as e:
			log_error(f"RuntimeError during relink: {e}", module_name='AssetRelink')
		except Exception as e:
			log_error(f"Exception during relink: {e}", module_name='AssetRelink')
	def _verify_relink_success(
		self,
		op_data: Dict[str, Any],
		original_session_uid: str,
		backup_uuid: Optional[str],
		expected_name: str,
		expected_lib_path: str
	) -> None:
		"""Verify that the relink operation was successful."""
		bpy_collection = self.asset_sources_map.get(op_data["asset_type"])
		if not bpy_collection:
			return
		
		# Find the renamed item using session_uid, UUID, or name+library match
		refetched_item = (
			next((item for item in bpy_collection if getattr(item, 'session_uid', None) == original_session_uid), None)
			or (backup_uuid and next((item for item in bpy_collection if getattr(item, BV_UUID_PROP, None) == backup_uuid), None))
			or next(
				(
					item for item in bpy_collection
					if (
						self._get_item_library_path(item) is not None
						and PathResolver.normalize_path(self._get_item_library_path(item) or "") == expected_lib_path
						and item.name == expected_name
					)
				),
				None
			)
		)
		
		if refetched_item:
			log_success(f"Relink verified: '{op_data['old_name']}' -> '{refetched_item.name}'", module_name='AssetRelink')
			# Update the main sidecar to reflect the successful relink
			self._update_main_sidecar_after_relink(op_data, expected_name)

	def _update_main_sidecar_after_relink(self, op_data: Dict[str, Any], new_name: str) -> None:
		"""Update the main sidecar file to reflect the successful asset relink."""
		try:
			# Import sidecar writing functionality
			from ..sidecar_io.writer import write_library_info
			
			log_debug(f"Updating main sidecar after successful relink: {op_data['old_name']} -> {new_name}", module_name='AssetRelink')
			
			# Trigger a sidecar update - this will collect current session data and update the sidecar
			write_library_info()
			
			log_success(f"Main sidecar updated after relink: {op_data['old_name']} -> {new_name}", module_name='AssetRelink')
			
		except Exception as e:
			log_warning(f"Failed to update main sidecar after relink: {e}", module_name='AssetRelink')


def relink_renamed_assets(*args, **kwargs):
	"""Main entry point for asset relinking. Called by Blender handlers."""
	blend_path = ensure_saved_file()
	if not blend_path:
		return
	
	log_info("Starting asset relink process", module_name='AssetRelink')
	
	try:
		processor = AssetRelinkProcessor(blend_path)
		processor.process_relink()
	except Exception as e:
		log_error(f"Unexpected error in relink process: {e}", module_name='AssetRelink')
		traceback.print_exc()
	
	log_info("Finished asset relinking attempt", module_name='AssetRelink')


# After relink_renamed_assets definition, remove lingering persistent handler
import bpy as _bpy
# Ensure automatic asset relink is not called at startup
to_remove = relink_renamed_assets
for _list in (_bpy.app.handlers.load_post, _bpy.app.handlers.save_post):
	if to_remove in _list:
		_list.remove(to_remove)


def register():
	log_success("[Blend Vault] Asset relinking module loaded.")


def unregister():
	log_warning("[Blend Vault] Asset relinking module unloaded.")


if __name__ == "__main__":
	register()
