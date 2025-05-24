import bpy  # type: ignore
import os
import json
import re
from utils import (
    ensure_library_hash, get_asset_sources_map, LOG_COLORS, SIDECAR_EXTENSION, 
    FRONTMATTER_TAGS, MD_LINK_FORMATS, BV_UUID_PROP, 
    BV_UUID_KEY, BV_FILE_UUID_KEY
)
from .frontmatter import generate_frontmatter_string

def _ensure_all_asset_uuids_are_set():
    """
    Iterates through all relevant local and linked assets to ensure they have
    the BLEND_VAULT_HASH_PROP custom property set.
    """
    print(f"{LOG_COLORS['INFO']}[Blend Vault] Ensuring all asset UUIDs are set on datablocks...{LOG_COLORS['RESET']}")
    ASSET_SOURCES_MAP = get_asset_sources_map()

    # Process local assets only (preserve original UUIDs on linked assets)
    for asset_type_name, datablock_collection in ASSET_SOURCES_MAP.items():
        if datablock_collection is None:
            continue
        
        try:
            items_list = list(datablock_collection)
        except Exception as e_list:
            print(f"{LOG_COLORS['ERROR']}[Blend Vault][EnsureUUIDs] Failed to list local items for '{asset_type_name}': {e_list}{LOG_COLORS['RESET']}")
            continue
            
        for item in items_list:
            if not item:
                continue
            
            item_library = getattr(item, 'library', None)
            is_scene = asset_type_name == "Scene"
            item_asset_data = getattr(item, 'asset_data', None)
            is_asset = item_asset_data is not None

            # Only process local assets (library is None)
            if item_library is None and (is_scene or is_asset):
                ensure_library_hash(item)

    print(f"{LOG_COLORS['INFO']}[Blend Vault] Finished ensuring all asset UUIDs.{LOG_COLORS['RESET']}")


def _collect_assets_by_type():
    """
    Collect all local and linked assets by type.
    Returns tuple of (local_assets_dict, linked_assets_by_library_dict)
    """
    local_assets = {}
    linked_assets_by_library = {}
    
    ASSET_SOURCES_MAP = get_asset_sources_map()
    
    for asset_type_name, datablock_collection in ASSET_SOURCES_MAP.items():
        if datablock_collection is None:
            continue
            
        try:
            items_list = list(datablock_collection)
        except Exception as e_list:
            print(f"{LOG_COLORS['ERROR']}[Blend Vault] Failed to list items for '{asset_type_name}': {e_list}{LOG_COLORS['RESET']}")
            continue
            
        for item_idx, item in enumerate(items_list):
            if not item:
                continue
                
            try:
                item_name = getattr(item, 'name', f'Unnamed{asset_type_name}')
                item_library = getattr(item, 'library', None)
                is_scene = asset_type_name == "Scene"
                item_asset_data = getattr(item, 'asset_data', None)
                is_asset = item_asset_data is not None
                
                if is_scene or is_asset:
                    item_uuid = ensure_library_hash(item)
                    asset_info = {
                        "name": item_name,
                        "type": asset_type_name,
                        "uuid": item_uuid
                    }
                    
                    if item_library is None:
                        # Local asset
                        local_assets[item_uuid] = asset_info
                    else:
                        # Linked asset
                        if item_library not in linked_assets_by_library:
                            linked_assets_by_library[item_library] = []
                        linked_assets_by_library[item_library].append(asset_info)
                        
            except Exception as e_item:
                print(f"{LOG_COLORS['ERROR']}[Blend Vault] Error processing item {item_idx} in {asset_type_name}: {e_item}{LOG_COLORS['RESET']}")
                continue
                
    return local_assets, linked_assets_by_library


def _get_or_create_blendfile_uuid(blend_path):
    """
    Gets existing blendfile UUID from its own sidecar file or creates a new one if not found.
    The sidecar file is the source of truth for the blendfile's UUID.
    Returns the UUID string.
    """
    md_path = blend_path + SIDECAR_EXTENSION
    
    if os.path.exists(md_path):
        try:
            with open(md_path, 'r', encoding='utf-8') as f_sidecar:
                content = f_sidecar.read()
            
            # Try to find BV_FILE_UUID_KEY first
            uuid_match = re.search(rf'"{BV_FILE_UUID_KEY}"\s*:\s*"([^"]+)"', content)
            if uuid_match:
                existing_uuid = uuid_match.group(1)
                print(f"{LOG_COLORS['INFO']}[Blend Vault] Using existing blendfile UUID from sidecar '{md_path}': {existing_uuid}{LOG_COLORS['RESET']}")
                return existing_uuid
            
            # Fallback to BV_UUID_KEY for older sidecars or different structures
            uuid_match_generic = re.search(rf'"{BV_UUID_KEY}"\s*:\s*"([^"]+)"', content)
            if uuid_match_generic:
                # This assumes that if BV_UUID_KEY is present at the top level of a blend file's sidecar,
                # it refers to the blend file's UUID.
                existing_uuid = uuid_match_generic.group(1)
                print(f"{LOG_COLORS['INFO']}[Blend Vault] Using existing generic UUID (as blendfile UUID) from sidecar '{md_path}': {existing_uuid}{LOG_COLORS['RESET']}")
                return existing_uuid
            
            print(f"{LOG_COLORS['WARN']}[Blend Vault] UUID key not found in existing sidecar '{md_path}'. A new UUID will be generated.{LOG_COLORS['RESET']}")

        except Exception as e:
            print(f"{LOG_COLORS['WARN']}[Blend Vault] Could not read UUID from sidecar '{md_path}': {e}. A new UUID will be generated.{LOG_COLORS['RESET']}")
    else:
        print(f"{LOG_COLORS['INFO']}[Blend Vault] Sidecar file '{md_path}' not found. A new UUID will be generated for the blendfile.{LOG_COLORS['RESET']}")
    
    # If UUID not found in sidecar or sidecar doesn't exist, generate a new one
    new_uuid = ensure_library_hash(blend_path) # Use blend_path for hashing, as bpy.data.filepath is the same
    print(f"{LOG_COLORS['INFO']}[Blend Vault] Generated new blendfile UUID: {new_uuid}{LOG_COLORS['RESET']}")
    return new_uuid


@bpy.app.handlers.persistent
def write_library_info(*args, **kwargs):
    """Main handler to write sidecar file."""
    print(f"{LOG_COLORS['INFO']}[Blend Vault] Preparing to write sidecar for: {bpy.data.filepath}{LOG_COLORS['RESET']}")
    
    blend_path = bpy.data.filepath
    if not blend_path:
        print(f"{LOG_COLORS['WARN']}[Blend Vault] No blend file path found, skipping write.{LOG_COLORS['RESET']}")
        return

    blend_file_basename = os.path.basename(blend_path)
    md_path = blend_path + SIDECAR_EXTENSION

    original_lines = []
    if os.path.exists(md_path):
        with open(md_path, 'r', encoding='utf-8') as f_read:
            original_lines = f_read.readlines()

    new_frontmatter_string, original_fm_end_idx = generate_frontmatter_string(original_lines, FRONTMATTER_TAGS)

    user_content_lines = original_lines[original_fm_end_idx + 1:] if original_fm_end_idx != -1 else original_lines
    processed_user_content = "".join(user_content_lines).strip()

    try:
        _ensure_all_asset_uuids_are_set()
    except Exception as e_ensure_uuids:
        print(f"{LOG_COLORS['ERROR']}[Blend Vault] Failed to ensure all asset UUIDs: {e_ensure_uuids}{LOG_COLORS['RESET']}")

    try:
        from relink.asset_relinker import relink_renamed_assets
        print(f"{LOG_COLORS['INFO']}[Blend Vault] Attempting asset datablock relink before writing sidecar...{LOG_COLORS['RESET']}")
        relink_renamed_assets()
    except Exception as e:
        print(f"{LOG_COLORS['ERROR']}[Blend Vault] Failed to run asset datablock relink before writing sidecar: {e}{LOG_COLORS['RESET']}")

    local_assets, linked_assets_by_library = _collect_assets_by_type()
    
    # Get or create blendfile UUID from its own sidecar or generate new
    blendfile_uuid = _get_or_create_blendfile_uuid(blend_path)
    
    blend_vault_data_lines = []
    blend_vault_data_lines.append("## %% Blend Vault Data")
    blend_vault_data_lines.append(
        "This section is auto-generated by the Blend Vault plugin and will be overwritten on save. "
        "User content can be written above this heading."
    )
    
    blend_vault_data_lines.append("### Current File")
    current_file_data = {
        "path": os.path.basename(blend_path),
        BV_FILE_UUID_KEY: blendfile_uuid,
        "assets": list(local_assets.values())
    }
    
    blend_vault_data_lines.append("```json")
    for line in json.dumps(current_file_data, indent=2, ensure_ascii=False).splitlines():
        blend_vault_data_lines.append(line)
    blend_vault_data_lines.append("```")
    
    blend_vault_data_lines.append("### Linked Libraries")
    libraries = list(bpy.data.libraries)
    
    if libraries:
        for lib in libraries:
            processed_lib_path = lib.filepath
            if processed_lib_path.startswith('//'):
                processed_lib_path = processed_lib_path[2:]
            processed_lib_path = processed_lib_path.replace('\\', '/')
            
            lib_sidecar_path = os.path.normpath(
                os.path.join(os.path.dirname(blend_path), processed_lib_path)
            ) + SIDECAR_EXTENSION
            
            library_uuid = "MISSING_HASH"
            if os.path.exists(lib_sidecar_path):
                try:
                    with open(lib_sidecar_path, 'r', encoding='utf-8') as f_lib_sc:
                        sc_content = f_lib_sc.read()
                    uuid_match = re.search(rf'"{BV_FILE_UUID_KEY}"\s*:\s*"([^"]+)"', sc_content)
                    if uuid_match:
                        library_uuid = uuid_match.group(1)
                    else:
                        uuid_match_generic = re.search(rf'"{BV_UUID_KEY}"\s*:\s*"([^"]+)"', sc_content)
                        if uuid_match_generic:
                            library_uuid = uuid_match_generic.group(1)
                            print(f"{LOG_COLORS['WARN']}[Blend Vault] Found generic UUID for library {lib_sidecar_path}, using it. Consider re-saving library to update to {BV_FILE_UUID_KEY}.{LOG_COLORS['RESET']}")
                except Exception as e:
                    print(f"{LOG_COLORS['WARN']}[Blend Vault] Could not read library sidecar {lib_sidecar_path}: {e}{LOG_COLORS['RESET']}")
            
            if library_uuid == "MISSING_HASH":
                library_uuid = ensure_library_hash(lib.filepath)
            
            lib.id_properties_ensure()[BV_UUID_PROP] = library_uuid
            
            markdown_link_path = os.path.basename(processed_lib_path)
            markdown_link_target = processed_lib_path
            blend_vault_data_lines.append(
                MD_LINK_FORMATS['MD_ANGLE_BRACKETS']['format'].format(
                    name=markdown_link_path, 
                    path=markdown_link_target
                )
            )
            
            linked_assets = linked_assets_by_library.get(lib, [])
            
            library_data = {
                "path": processed_lib_path,
                "uuid": library_uuid,
                "assets": linked_assets
            }
            
            blend_vault_data_lines.append("```json")
            for line in json.dumps(library_data, indent=2, ensure_ascii=False).splitlines():
                blend_vault_data_lines.append(line)
            blend_vault_data_lines.append("```")
            blend_vault_data_lines.append("")
    else:
        blend_vault_data_lines.append("- None")

    blend_vault_data_block_string = "\n".join(blend_vault_data_lines) + "\n"

    final_content_parts = [new_frontmatter_string]
    if processed_user_content:
        final_content_parts.append(processed_user_content)
        final_content_parts.append("\n\n")
    elif new_frontmatter_string:
        final_content_parts.append("\n")

    blend_vault_heading = '## %% Blend Vault Data'
    if original_lines:
        heading_idx = -1
        for idx, line in enumerate(original_lines):
            if line.strip() == blend_vault_heading:
                heading_idx = idx
                break
        
        if heading_idx != -1:
            preserved_content = ''.join(original_lines[:heading_idx])
            if not preserved_content.endswith('\n'):
                preserved_content += '\n'
            output_content = preserved_content + blend_vault_data_block_string
        else:
            output_content = ''.join(final_content_parts) + blend_vault_data_block_string
    else:
        output_content = ''.join(final_content_parts) + blend_vault_data_block_string

    try:
        with open(md_path, 'w', encoding='utf-8') as f_write:
            f_write.write(output_content)
        print(f"{LOG_COLORS['SUCCESS']}[Blend Vault] Sidecar file written to: {md_path}{LOG_COLORS['RESET']}")
        
    except Exception as e:
        print(f"{LOG_COLORS['ERROR']}[Blend Vault] Failed to write sidecar file {md_path}: {e}{LOG_COLORS['RESET']}")


write_library_info.persistent = True
