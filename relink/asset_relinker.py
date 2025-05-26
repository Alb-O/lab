"""
Asset relinking module for Blend Vault.
Handles relinking renamed individual asset datablocks by comparing sidecar states.
"""

import bpy  # type: ignore
import os
import traceback
from typing import Dict, List, Optional, Any
from utils import (
    get_asset_sources_map,
    BV_UUID_PROP
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
            log_warning(f"[Blend Vault][AssetRelink] Main sidecar file not found: {self.sidecar_path}")
            return
        
        log_info(f"[Blend Vault][AssetRelink] Processing main sidecar for asset relinking: {self.sidecar_path}")
        
        try:
            parser = SidecarParser(self.sidecar_path)
            
            # Parse linked libraries from main sidecar
            main_linked_data = parser.extract_json_blocks_with_links("Linked Libraries")
            if not main_linked_data:
                log_info("[Blend Vault][AssetRelink] No linked library data found in main sidecar.")
                return
            
            # Get authoritative data from library sidecars
            authoritative_data = self._get_authoritative_library_data(main_linked_data)
            if not authoritative_data:
                log_info("[Blend Vault][AssetRelink] No authoritative data found from any library sidecars.")
                return
            
            # Identify and execute relink operations
            relink_ops = self._identify_relink_operations(main_linked_data, authoritative_data)
            self._execute_relink_operations(relink_ops)
            
        except Exception as e:
            log_error(f"[Blend Vault][AssetRelink] Error during asset relinking process: {e}")
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
            # Reload the library to ensure it's up to date
            lib_abs_path = PathResolver.resolve_relative_to_absolute(lib_rel_path, self.main_blend_dir)
            LibraryManager.reload_library(lib_abs_path)
            
            # Parse the library's own sidecar
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
                    log_debug(f"[Blend Vault][AssetRelink] Authoritative data for '{lib_rel_path}': {len(assets_in_lib)} assets")
                else:
                    log_info(f"[Blend Vault][AssetRelink] No authoritative data found for library '{lib_rel_path}'")
                    
            except Exception as e:
                log_warning(f"[Blend Vault][AssetRelink] Could not read library sidecar for '{lib_rel_path}': {e}")
        
        return authoritative_data
    
    def _identify_relink_operations(
        self,
        main_linked_data: Dict[str, Dict[str, Any]],
        authoritative_data: Dict[str, Dict[str, Dict[str, str]]]
    ) -> List[Dict[str, Any]]:
        """Identify assets that need relinking by comparing names."""
        relink_operations = []
        
        for lib_rel_path, lib_link_info in main_linked_data.items():
            assets_in_main = lib_link_info["json_data"].get("assets", [])
            authoritative_assets = authoritative_data.get(lib_rel_path, {})
            
            if not authoritative_assets:
                log_debug(f"[Blend Vault][AssetRelink] Skipping library '{lib_rel_path}' - no authoritative data")
                continue
            
            for asset_info in assets_in_main:
                asset_uuid = asset_info.get("uuid")
                old_name = asset_info.get("name")
                asset_type = asset_info.get("type")
                
                if not all([asset_uuid, old_name, asset_type]):
                    log_warning(f"[Blend Vault][AssetRelink] Incomplete asset data: {asset_info}")
                    continue
                
                current_asset_info = authoritative_assets.get(asset_uuid)
                if not current_asset_info:
                    log_info(f"[Blend Vault][AssetRelink] Asset '{old_name}' (UUID: {asset_uuid}) no longer exists in library")
                    continue
                
                current_name = current_asset_info["name"]
                if old_name != current_name:
                    log_info(f"[Blend Vault][AssetRelink] Name change detected: '{old_name}' -> '{current_name}' (UUID: {asset_uuid})")
                    
                    # Find the session item and prepare relink operation
                    session_item = self._find_and_prepare_session_item(lib_rel_path, asset_uuid, asset_type)
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
    
    def _find_and_prepare_session_item(self, lib_rel_path: str, asset_uuid: str, asset_type: str):
        """Find the session item for an asset and ensure it has the correct UUID."""
        bpy_collection = self.asset_sources_map.get(asset_type)
        if not bpy_collection:
            log_warning(f"[Blend Vault][AssetRelink] Unknown asset type '{asset_type}'")
            return None
        
        expected_lib_path = PathResolver.normalize_path(
            PathResolver.resolve_relative_to_absolute(lib_rel_path, self.main_blend_dir)
        )
        
        for item in bpy_collection:
            item_lib_path = self._get_item_library_path(item)
            if item_lib_path and PathResolver.normalize_path(item_lib_path) == expected_lib_path:
                # Assign the correct UUID to the session item
                try:
                    item.id_properties_ensure()[BV_UUID_PROP] = asset_uuid
                    log_debug(f"[Blend Vault][AssetRelink] Assigned UUID '{asset_uuid}' to session item '{item.name}'")
                except Exception:
                    pass
                return item
        
        log_warning(f"[Blend Vault][AssetRelink] Could not find session item for UUID {asset_uuid}")
        return None
    
    def _get_item_library_path(self, item) -> Optional[str]:
        """Get the library path for a session item."""
        if getattr(item, 'library', None) and item.library.filepath:
            return PathResolver.resolve_blender_path(item.library.filepath)
        elif hasattr(item, 'library_weak_reference') and item.library_weak_reference and item.library_weak_reference.filepath:
            return PathResolver.resolve_blender_path(item.library_weak_reference.filepath)
        return None
    
    def _execute_relink_operations(self, relink_operations: List[Dict[str, Any]]) -> None:
        """Execute the identified relink operations."""
        if not relink_operations:
            log_info("[Blend Vault][AssetRelink] No relink operations to perform.")
            return
        
        log_info(f"[Blend Vault][AssetRelink] Found {len(relink_operations)} operations. Attempting to relocate...")
        
        for op in relink_operations:
            self._execute_single_relink(op)
    
    def _execute_single_relink(self, op_data: Dict[str, Any]) -> None:
        """Execute a single relink operation."""
        session_uid = op_data["session_uid"]
        if not session_uid:
            log_warning(f"[Blend Vault][AssetRelink] No session_uid for asset {op_data['uuid']}")
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
            log_warning(f"[Blend Vault][AssetRelink] Could not find session item for session_uid {session_uid}")
            return
        
        # Get library filepath
        lib_filepath = self._get_item_library_path(session_item)
        if not lib_filepath:
            log_warning(f"[Blend Vault][AssetRelink] Session item {session_item.name} has no library filepath")
            return
        
        # Prepare relink parameters
        try:
            self._perform_blender_relink(session_item, lib_filepath, op_data)
        except Exception as e:
            log_error(f"[Blend Vault][AssetRelink] Exception during relink: {e}")
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
            filepath_arg = f"{lib_filepath}\\{asset_type}\\{new_name}"
        else:
            abs_lib_path = PathResolver.normalize_path(lib_filepath)
            filepath_arg = f"{abs_lib_path}\\{asset_type}\\{new_name}"
        
        directory_arg = f"{abs_lib_path}\\{asset_type}\\"
        
        # Store references for post-relink verification
        original_session_uid = op_data["session_uid"]
        backup_uuid = getattr(session_item, BV_UUID_PROP, None)
        
        try:
            result = bpy.ops.wm.id_linked_relocate(
                id_session_uid=original_session_uid,
                filepath=filepath_arg,
                directory=directory_arg,
                filename=new_name,
                relative_path=is_relative
            )
            log_success(f"[Blend Vault][AssetRelink] Relink operation for '{op_data['old_name']}' returned: {result}")
            
            # Verify the relink was successful
            self._verify_relink_success(op_data, original_session_uid, backup_uuid, new_name, abs_lib_path)
            
        except RuntimeError as e:
            log_error(f"[Blend Vault][AssetRelink] RuntimeError during relink: {e}")
        except Exception as e:
            log_error(f"[Blend Vault][AssetRelink] Exception during relink: {e}")
    
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
        
        # Try multiple strategies to find the renamed item
        refetched_item = None
        
        # Strategy 1: Find by original session_uid
        refetched_item = next(
            (item for item in bpy_collection if getattr(item, 'session_uid', None) == original_session_uid),
            None
        )
        
        # Strategy 2: Find by UUID if session_uid failed
        if not refetched_item and backup_uuid:
            log_debug(f"[Blend Vault][AssetRelink] Session UID lookup failed, trying UUID backup for {backup_uuid}")
            refetched_item = next(
                (item for item in bpy_collection if getattr(item, BV_UUID_PROP, None) == backup_uuid),
                None
            )
        
        # Strategy 3: Find by name and library match
        if not refetched_item:
            log_debug(f"[Blend Vault][AssetRelink] UUID lookup failed, trying name-based lookup for '{expected_name}'")
            for item in bpy_collection:
                item_lib_path = self._get_item_library_path(item)
                if (item_lib_path and 
                    PathResolver.normalize_path(item_lib_path) == expected_lib_path and 
                    item.name == expected_name):
                    refetched_item = item
                    log_debug(f"[Blend Vault][AssetRelink] Found renamed item by name and library match: '{item.name}'")
                    break
        
        if refetched_item:
            log_success(f"[Blend Vault][AssetRelink] Successfully verified relinked item '{refetched_item.name}'")
        else:
            log_warning(f"[Blend Vault][AssetRelink] Could not verify relink success for UUID {op_data['uuid']}")


@bpy.app.handlers.persistent
def relink_renamed_assets(*args, **kwargs):
    """Main entry point for asset relinking. Called by Blender handlers."""
    blend_path = ensure_saved_file()
    if not blend_path:
        return
    
    log_info("[Blend Vault][AssetRelink] Starting asset relink process")
    
    try:
        processor = AssetRelinkProcessor(blend_path)
        processor.process_relink()
    except Exception as e:
        log_error(f"[Blend Vault][AssetRelink] Unexpected error in relink process: {e}")
        traceback.print_exc()
    
    log_info("[Blend Vault][AssetRelink] Finished asset relinking attempt")


# Make the handler persistent
relink_renamed_assets.persistent = True


def register():
    log_success("[Blend Vault] Asset relinking module loaded.")


def unregister():
    log_warning("[Blend Vault] Asset relinking module unloaded.")


if __name__ == "__main__":
    register()
