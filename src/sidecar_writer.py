import bpy  # type: ignore
import os
from datetime import datetime, timezone  # Added datetime and timezone
from .hashing import ensure_blendfile_hash, ensure_library_hash  # Added ensure_library_hash
from .config import GREEN, RESET, SIDECAR_EXTENSION, FRONTMATTER_TAGS  # Added FRONTMATTER_TAGS
import json  # Added for JSON handling
import re  # For parsing library sidecar JSON

@bpy.app.handlers.persistent
def write_library_info(*args, **kwargs):  # Decorated as persistent handler
    print(f"{GREEN}[Blend Vault] Preparing to write sidecar for: {bpy.data.filepath}{RESET}")
    blend_path = bpy.data.filepath
    if not blend_path:
        print("[Blend Vault][LibraryInfo] No blend file path found, skipping write.")
        return

    md_path = blend_path + SIDECAR_EXTENSION
    blendfile_hash = ensure_blendfile_hash() # Ensure hash for current file
    libraries = list(bpy.data.libraries)

    original_lines = []
    if os.path.exists(md_path):
        with open(md_path, 'r', encoding='utf-8') as f_read:
            original_lines = f_read.readlines()

    existing_tags = set()
    frontmatter_end_line_idx = -1
    if original_lines and original_lines[0].strip() == "---":
        try:
            # Try to find the closing '---' for frontmatter
            for i in range(1, len(original_lines)):
                if original_lines[i].strip() == "---":
                    frontmatter_end_line_idx = i
                    break
            
            if frontmatter_end_line_idx != -1:
                is_tags_section = False
                for i in range(1, frontmatter_end_line_idx):
                    line_content = original_lines[i].strip()
                    if line_content.startswith("tags:"):
                        is_tags_section = True
                        # Handle tags on the same line as "tags:" (e.g., tags: tagA, tagB)
                        tag_value_str = line_content.split("tags:", 1)[1].strip()
                        if tag_value_str:
                            if tag_value_str.startswith('[') and tag_value_str.endswith(']'): # Handle JSON-like array
                                tag_value_str = tag_value_str[1:-1]
                            for t in tag_value_str.split(','):
                                cleaned_tag = t.strip()
                                if cleaned_tag:
                                    existing_tags.add(cleaned_tag)
                    elif is_tags_section and line_content.startswith("- "):
                        existing_tags.add(line_content[2:].strip())
                    elif is_tags_section and not (line_content.startswith("  ") or line_content.startswith("- ")):
                        is_tags_section = False # End of current tags section
        except IndexError: # Should not happen with valid line list
            pass
        except ValueError: # No closing ---, so no valid frontmatter or malformed
            frontmatter_end_line_idx = -1 # Reset if parsing failed

    user_content_lines = []
    content_start_idx = frontmatter_end_line_idx + 1 if frontmatter_end_line_idx != -1 else 0
    old_data_block_found = False # Flag to indicate if any version of the BV data block was found in the original file

    # This loop identifies user content by stopping when it finds the start of any known Blend Vault data block.
    for i in range(content_start_idx, len(original_lines)):
        current_line_raw = original_lines[i] # Preserve original line including newline characters
        current_line_stripped = current_line_raw.strip()

        # Check for the new primary header format
        is_new_format_header = current_line_stripped.startswith("## %% Blend Vault Data")
        
        # Check for old callout-style headers for backward compatibility
        is_old_callout_header_standard = current_line_stripped.startswith("> [!example] Blend Vault Data")
        is_old_callout_header_hyphenated = current_line_stripped.startswith("> [!example]- Blend Vault Data")

        if is_new_format_header or is_old_callout_header_standard or is_old_callout_header_hyphenated:
            old_data_block_found = True # Mark that we found a data block to replace
            # Stop collecting user content; the lines from this point in the original file will be replaced.
            break 
        
        user_content_lines.append(current_line_raw) # Add to user content if not a data block header

    # Prepare new frontmatter
    new_defined_tags = FRONTMATTER_TAGS  # Use from config
    final_tags = sorted(list(existing_tags.union(new_defined_tags)))
    
    current_time_utc = datetime.now(timezone.utc)  # Added for timestamp
    formatted_time = current_time_utc.strftime('%Y-%m-%dT%H:%M:%S.%f')[:-3] + 'Z'  # Added for timestamp

    reconstructed_fm_content_lines = []
    if frontmatter_end_line_idx != -1: # If valid frontmatter was found
        # Process lines between "---" fences
        fm_lines_to_process = original_lines[1:frontmatter_end_line_idx]
        i = 0
        tags_block_written_for_new_fm = False # Flag to ensure tags are written only once

        while i < len(fm_lines_to_process):
            line_raw = fm_lines_to_process[i]
            line_stripped = line_raw.strip()

            if line_stripped.startswith("tags:") and not tags_block_written_for_new_fm:
                # Encountered the original 'tags:' key. Write our new/updated tags block.
                if final_tags:
                    reconstructed_fm_content_lines.append("tags:")
                    for tag in final_tags:
                        reconstructed_fm_content_lines.append(f"  - {tag}")
                tags_block_written_for_new_fm = True
                
                # Advance 'i' past the old "tags:" line itself
                i += 1 
                # Skip subsequent lines if they are part of the old tags list's values
                while i < len(fm_lines_to_process):
                    current_line_in_old_tags_raw = fm_lines_to_process[i]
                    # A line is an old tag item if it's a YAML list item,
                    # typically starting with "- " (possibly indented)
                    if current_line_in_old_tags_raw.lstrip().startswith("- "):
                        i += 1 # Skip this old tag item line
                    else:
                        # This line is not an old tag item, so the old tags block (values) has ended.
                        break 
                continue # Continue the outer while loop; 'i' is already at the next line to process or end
            else:
                # This line is not "tags:" (or "tags:" has already been processed and replaced).
                # So, preserve this line.
                reconstructed_fm_content_lines.append(line_raw.rstrip('\r\n'))
                i += 1
        
        # If the original frontmatter didn't have a "tags:" key, but we have tags to write,
        # add them at the end of the collected frontmatter content.
        if not tags_block_written_for_new_fm and final_tags:
            reconstructed_fm_content_lines.append("tags:")
            for tag in final_tags:
                reconstructed_fm_content_lines.append(f"  - {tag}")
        
        new_frontmatter_lines = ["---"] + reconstructed_fm_content_lines + ["---"]
    else: # No existing frontmatter, or it was malformed. Create from scratch.
        new_frontmatter_lines = ["---"]
        # existing_tags would be empty, so final_tags are just new_defined_tags from config
        if final_tags: 
            new_frontmatter_lines.append("tags:")
            for tag in final_tags:
                new_frontmatter_lines.append(f"  - {tag}")
        new_frontmatter_lines.append("---")

    new_frontmatter_string = "\n".join(new_frontmatter_lines) + "\n"

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
            blend_vault_data_lines.append(f"[{os.path.basename(processed_lib_path)}](<{processed_lib_path}>)")
            
            # Prepare JSON data for the library
            parsed_library_identity = None

            if raw_library_identity_str != 'MISSING_HASH':
                try:
                    parsed_library_identity = json.loads(raw_library_identity_str)
                except json.JSONDecodeError:
                    print(f"[Blend Vault][Info] 'blend_vault_hash' for library {lib.filepath} is not valid JSON. Assuming it's a direct UUID or marker. Content: '{raw_library_identity_str}'")
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
    elif len(new_frontmatter_lines) > 2: # Frontmatter exists (more than just '---' boundaries)
        final_content_parts.append("\n") # Add a newline to separate frontmatter from data block

    blend_vault_data_block_string = "\n".join(blend_vault_data_lines) + "\n"
    final_content_parts.append(blend_vault_data_block_string)

    output_content = "".join(final_content_parts)

    try:
        with open(md_path, 'w', encoding='utf-8') as f_write:
            f_write.write(output_content)
        print(f"{GREEN}[Blend Vault] Sidecar file written to: {md_path}{RESET}")
    except Exception as e:
        print(f"[Blend Vault][Error] Failed to write sidecar file {md_path}: {e}")

# Ensure the handler remains persistent
write_library_info.persistent = True
