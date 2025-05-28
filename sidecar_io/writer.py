"""
Main sidecar writing orchestration for Blend Vault.
Coordinates asset collection, content building, and file operations.
"""

import bpy  # type: ignore
import os
from ..utils import SIDECAR_EXTENSION, LOG_COLORS
from .collectors import collect_assets, collect_resources
from .content_builder import build_sidecar_content
from .file_operations import write_sidecar_with_content_preservation, push_uuid_to_sidecar
from ..utils import log_info, log_warning, log_error, log_success, log_debug


@bpy.app.handlers.persistent
def write_library_info(*args, **kwargs):
    """Main handler to write sidecar file."""
    blend_path = bpy.data.filepath
    if not blend_path:
        log_warning("[Blend Vault] No blend file path found, skipping write")
        return
    
    log_info(f"[Blend Vault] Writing sidecar for: {blend_path}")
    
    # Optional relink step
    try:
        from relink.asset_relinker import relink_renamed_assets
        relink_renamed_assets()
    except Exception as e:
        log_error(f"[Blend Vault] Asset relink failed: {e}")
    
    # Collect data
    local_assets, linked_assets_by_library = collect_assets()
    resources = collect_resources()
    
    # Build content
    sidecar_content, uuid_pushes = build_sidecar_content(
        blend_path, 
        local_assets, 
        linked_assets_by_library,
        resources
    )
    
    # Write main sidecar
    md_path = blend_path + SIDECAR_EXTENSION
    try:
        write_sidecar_with_content_preservation(md_path, sidecar_content)
        log_success(f"[Blend Vault] Sidecar written: {md_path}")
    except Exception as e:
        log_error(f"[Blend Vault] Failed to write sidecar {md_path}: {e}")
        return
    # Push UUIDs to linked library sidecars
    for lib_sidecar_path, (file_uuid, asset_updates) in uuid_pushes.items():
        # Validate linked blend file exists
        linked_blend_path = lib_sidecar_path[:-len(SIDECAR_EXTENSION)]
        if os.path.exists(linked_blend_path) and (asset_updates or file_uuid):
            push_uuid_to_sidecar(lib_sidecar_path, file_uuid, asset_updates)
        elif not os.path.exists(linked_blend_path):
            log_warning(f"[Blend Vault] Skipping push to {lib_sidecar_path} - linked blend file missing")
    
    # Now that library sidecars exist, resolve UUIDs for linked assets and update main sidecar
    from .collectors import _resolve_linked_asset_uuids
    
    # Re-resolve UUIDs now that library sidecars exist
    _resolve_linked_asset_uuids(linked_assets_by_library, blend_path)
    
    # Check if any UUIDs were resolved
    uuids_resolved = False
    for lib, assets in linked_assets_by_library.items():
        for asset in assets:
            if asset["uuid"] is not None:
                uuids_resolved = True
                break
        if uuids_resolved:
            break
    
    # If UUIDs were resolved, rebuild and rewrite the main sidecar
    if uuids_resolved:
        updated_sidecar_content, _ = build_sidecar_content(
            blend_path, 
            local_assets, 
            linked_assets_by_library,
            resources
        )
        
        try:
            write_sidecar_with_content_preservation(md_path, updated_sidecar_content)
            log_success(f"[Blend Vault] Main sidecar updated with resolved UUIDs")
        except Exception as e:
            log_error(f"[Blend Vault] Failed to update main sidecar: {e}")


write_library_info.persistent = True
