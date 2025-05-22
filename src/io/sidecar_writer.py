import bpy  # type: ignore
import os
from ..utils.hashing import ensure_blendfile_hash, ensure_library_hash
from ..utils.config import LOG_INFO, LOG_ERROR, LOG_SUCCESS, LOG_WARN, LOG_RESET, SIDECAR_EXTENSION, FRONTMATTER_TAGS
from .frontmatter import parse_existing_frontmatter, reconstruct_frontmatter_internal_lines, generate_frontmatter_string
import json
import re

@bpy.app.handlers.persistent
def write_library_info(*args, **kwargs):  # Decorated as persistent handler
    print(f"{LOG_INFO}[Blend Vault] Preparing to write sidecar for: {bpy.data.filepath}{LOG_RESET}")
    blend_path = bpy.data.filepath
    if not blend_path:
        print(f"{LOG_WARN}[Blend Vault][LibraryInfo] No blend file path found, skipping write.{LOG_RESET}")
        return

    md_path = blend_path + SIDECAR_EXTENSION
    blendfile_hash = ensure_blendfile_hash() # Ensure hash for current file
    libraries = list(bpy.data.libraries)

    original_lines = []
    if os.path.exists(md_path):
        with open(md_path, 'r', encoding='utf-8') as f_read:
            original_lines = f_read.readlines()

    # Use functions from frontmatter.py
    existing_frontmatter, user_content_lines = parse_existing_frontmatter(original_lines)
    new_defined_tags = FRONTMATTER_TAGS  # Use from config
    final_tags = sorted(list(existing_frontmatter.get("tags", set()).union(new_defined_tags)))

    reconstructed_fm_content_lines = reconstruct_frontmatter_internal_lines(existing_frontmatter, final_tags)
    new_frontmatter_string = generate_frontmatter_string(reconstructed_fm_content_lines)

    # Prepare user content string (stripped, with appropriate newlines)
    processed_user_content = "".join(user_content_lines).strip()

    # Prepare Blend Vault Data block, formatted as JSON for current file assets
    blend_vault_data_lines = []
    blend_vault_data_lines.append("## %% Blend Vault Data")
    blend_vault_data_lines.append("")
    # Load asset info from Blender text block JSON
    assets = []
    try:
        txt = bpy.data.texts.get('blend_vault_hash')
        if txt:
            data = json.loads(txt.as_string())
            assets = data.get('assets', [])
    except Exception:
        assets = []
    blend_vault_data_lines.append("### Assets")
    blend_vault_data_lines.append("")
    # Dump assets as JSON code block
    blend_vault_data_lines.append("```json")
    for line in json.dumps(assets, indent=2, ensure_ascii=False).splitlines():
        blend_vault_data_lines.append(line)
    blend_vault_data_lines.append("```")
    # Linked Libraries section remains below
    blend_vault_data_lines.append("### Linked Libraries")
    blend_vault_data_lines.append("")
    if libraries:
        for lib in libraries:
            # Determine relative path for the library in the sidecar
            processed_lib_path = lib.filepath
            if processed_lib_path.startswith('//'):
                processed_lib_path = processed_lib_path[2:]
            processed_lib_path = processed_lib_path.replace('\\\\', '/')
            # Try to read the library file's own sidecar to get its UUID
            lib_sidecar_path = os.path.normpath(os.path.join(os.path.dirname(bpy.data.filepath), processed_lib_path)) + SIDECAR_EXTENSION
            if os.path.exists(lib_sidecar_path):
                with open(lib_sidecar_path, 'r', encoding='utf-8') as f_lib_sc:
                    sc_content = f_lib_sc.read()
                m_uuid = re.search(r'"blendfile_uuid"\s*:\s*"([^\"]+)"', sc_content)
                if m_uuid:
                    library_uuid = m_uuid.group(1)
                else:
                    library_uuid = ensure_library_hash(lib)
            else:
                # No sidecar for the library file: generate or reuse a hash
                library_uuid = ensure_library_hash(lib)
            # Store this UUID on the library datablock for relinking matches
            lib.id_properties_ensure()["blend_vault_hash"] = library_uuid
            raw_library_identity_str = library_uuid
            
            # Add the markdown link for the library
            # Ensure forward slashes for markdown link path
            markdown_link_path = os.path.basename(processed_lib_path).replace('\\', '/') if os.path.basename(processed_lib_path) else processed_lib_path.replace('\\', '/')
            markdown_link_target = processed_lib_path.replace('\\', '/')
            blend_vault_data_lines.append(f"[{markdown_link_path}](<{markdown_link_target}>)")
            
            # Prepare JSON data for the library
            parsed_library_identity = None

            if raw_library_identity_str != 'MISSING_HASH':
                try:
                    parsed_library_identity = json.loads(raw_library_identity_str)
                except json.JSONDecodeError:
                    print(f"{LOG_INFO}[Blend Vault][Info] 'blend_vault_hash' for library {lib.filepath} is not valid JSON. Assuming it's a direct UUID or marker. Content: '{raw_library_identity_str}'{LOG_RESET}")
                    # parsed_library_identity remains None

            # Determine the library's own blendfile_uuid to store
            library_blendfile_uuid_to_store = "UNKNOWN_LIBRARY_UUID" # Default
            if raw_library_identity_str == 'MISSING_HASH':
                library_blendfile_uuid_to_store = 'MISSING_HASH'
            elif isinstance(parsed_library_identity, dict) and "blendfile_uuid" in parsed_library_identity:
                library_blendfile_uuid_to_store = parsed_library_identity["blendfile_uuid"]
            elif isinstance(parsed_library_identity, str): # Parsed JSON was just a string e.g. "uuid-value"
                library_blendfile_uuid_to_store = parsed_library_identity
            elif parsed_library_identity is None and raw_library_identity_str != 'MISSING_HASH':
                 # Failed to parse as JSON, but wasn't MISSING_HASH. Use the raw string.
                library_blendfile_uuid_to_store = raw_library_identity_str
            # If parsed_library_identity was some other JSON type (e.g. list, number) and not a dict with blendfile_uuid,
            # it might fall through to UNKNOWN_LIBRARY_UUID unless raw_library_identity_str was used.

            # Collect assets linked from this specific library, using their live names and stored UUIDs
            live_linked_assets = []
            
            asset_sources_map = {
                "Collection": bpy.data.collections,
                "Object": bpy.data.objects,
                "World": bpy.data.worlds,
                "Material": bpy.data.materials,
                "Brush": bpy.data.brushes,
                "Action": bpy.data.actions,
                "Node Group": bpy.data.node_groups,
                "Scene": bpy.data.scenes,
                # Potentially add: Image, Light, Camera, Mesh, Armature, etc.
                # if they are expected to be marked as assets and carry 'blend_vault_uuid'
            }

            for asset_type_name, datablock_collection in asset_sources_map.items():
                if datablock_collection is None: continue
                for item in datablock_collection:
                    if hasattr(item, 'library') and item.library == lib:
                        item_uuid = item.get('blend_vault_uuid')
                        if item_uuid: # Only include if it has our UUID
                            live_linked_assets.append({
                                "name": item.name,       # Live name from current file
                                "type": asset_type_name, # Human-readable type
                                "uuid": item_uuid        # Original UUID from custom prop
                            })
            
            # Construct the data to be stored for this library's "uuid" field in the JSON
            library_content_for_uuid_field = None
            if library_blendfile_uuid_to_store == 'MISSING_HASH' and not live_linked_assets:
                # If there was no hash on the library object and no linked assets found, output simple MISSING_HASH
                library_content_for_uuid_field = "MISSING_HASH"
            else:
                library_content_for_uuid_field = {
                    "blendfile_uuid": library_blendfile_uuid_to_store,
                    "assets": live_linked_assets
                }
            
            library_data_for_json = {
                "path": processed_lib_path,
                "uuid": library_content_for_uuid_field 
            }
            
            # Add JSON block
            blend_vault_data_lines.append("```json")
            # Ensure that if library_content_for_uuid_field ended up as None (should not happen with current logic but defensive)
            # or some other unexpected state, json.dumps handles it or we provide a fallback.
            # For now, assuming library_content_for_uuid_field is either a dict or "MISSING_HASH" string.
            for line in json.dumps(library_data_for_json, indent=2, ensure_ascii=False).splitlines():
                blend_vault_data_lines.append(line)
            blend_vault_data_lines.append("```")
            blend_vault_data_lines.append("") # Add a blank line for separation
    else:
        blend_vault_data_lines.append("- None")
    blend_vault_data_lines.append("")

    # Assemble final content for writing
    final_content_parts = [new_frontmatter_string]
    if processed_user_content:
        final_content_parts.append(processed_user_content)
        final_content_parts.append("\n\n") # Ensure separation after user content
    elif len(reconstructed_fm_content_lines) > 0: # Frontmatter exists
        final_content_parts.append("\n") # Add a newline to separate frontmatter from data block

    blend_vault_data_block_string = "\n".join(blend_vault_data_lines) + "\n"
    final_content_parts.append(blend_vault_data_block_string)

    output_content = "".join(final_content_parts)

    try:
        with open(md_path, 'w', encoding='utf-8') as f_write:
            f_write.write(output_content)
        print(f"{LOG_SUCCESS}[Blend Vault] Sidecar file written to: {md_path}{LOG_RESET}")
    except Exception as e:
        print(f"{LOG_ERROR}[Blend Vault][Error] Failed to write sidecar file {md_path}: {e}{LOG_RESET}")

# Ensure the handler remains persistent
write_library_info.persistent = True
