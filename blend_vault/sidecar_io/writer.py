"""
Main sidecar writing orchestration for Blend Vault.
Coordinates asset collection, content building, and file operations.
"""

import bpy
import os
from ..core import SIDECAR_EXTENSION, LOG_COLORS, log_info, log_warning, log_error, log_success, log_debug
from .collectors import collect_assets, collect_resources
from .content_builder import build_sidecar_content
from .file_operations import write_sidecar_with_content_preservation, push_uuid_to_sidecar


@bpy.app.handlers.persistent
def write_library_info(*args, **kwargs):
    """Main handler to write sidecar file."""
    blend_path = bpy.data.filepath
    if not blend_path:
        log_warning("No blend file path found, skipping write", module_name="SidecarWriter")
        return

    # Make blend_path relative to the current blend file directory
    blend_dir = os.path.dirname(bpy.data.filepath)
    rel_blend_path = os.path.relpath(blend_path, blend_dir)
    
    log_info(f"Writing sidecar for: {rel_blend_path}", module_name="SidecarWriter")

    # Note: Asset relinking is now handled by the startup dialog for user confirmation
    # No automatic relinking during sidecar write operations

    # Collect data
    local_assets, linked_assets_by_library = collect_assets()
    resources = collect_resources()

    # Build content
    sidecar_content, uuid_pushes = build_sidecar_content(
        rel_blend_path,
        local_assets,
        linked_assets_by_library,
        resources
    )

    # Write main sidecar
    md_path = blend_path + SIDECAR_EXTENSION
    try:
        write_sidecar_with_content_preservation(md_path, sidecar_content)
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
    _resolve_linked_asset_uuids(linked_assets_by_library, rel_blend_path)

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
            rel_blend_path,
            local_assets,
            linked_assets_by_library,
            resources
        )

        try:
            write_sidecar_with_content_preservation(md_path, updated_sidecar_content)
            log_success(f"Main sidecar updated with resolved UUIDs", module_name="SidecarWriter")
        except Exception as e:
            log_error(f"Failed to update main sidecar: {e}", module_name="SidecarWriter")
