import bpy  # type: ignore
import os
from datetime import datetime, timezone  # Added datetime and timezone
from .hashing import ensure_blendfile_hash
from .config import GREEN, RESET, SIDECAR_EXTENSION, FRONTMATTER_TAGS  # Added FRONTMATTER_TAGS

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
    old_data_block_found = False

    for i in range(content_start_idx, len(original_lines)):
        # Check if the current line is the start of the old data block
        if original_lines[i].strip().startswith("> [!example] Blend Vault Data"):
            old_data_block_found = True
            # Stop collecting user content here; the rest of the old block will be discarded
            break 
        user_content_lines.append(original_lines[i])

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

    # Prepare Blend Vault Data block
    blend_vault_data_lines = []
    blend_vault_data_lines.append("> [!example] Blend Vault Data")
    blend_vault_data_lines.append("| Type | Path | UUID4 Hash (links to sidecar) |")
    blend_vault_data_lines.append("| --- | --- | --- |")

    this_file_link_path = os.path.basename(blend_path).replace('\\', '/') # Normalize path
    sidecar_file_link_path = os.path.basename(md_path).replace('\\', '/') # Normalize path
    
    blend_vault_data_lines.append(f"| This file | [[{this_file_link_path}]] | [[{sidecar_file_link_path}\\|{blendfile_hash}]] |")

    if libraries:
        for lib in libraries:
            lib_path_original = lib.filepath
            # Process library path to be relative for the link
            processed_lib_path_for_link = lib_path_original
            if processed_lib_path_for_link.startswith('//'):
                processed_lib_path_for_link = processed_lib_path_for_link[2:]
            processed_lib_path_for_link = processed_lib_path_for_link.replace('\\', '/') # Ensure forward slashes

            # Logic to determine the hash for the library (property or from loaded .blend)
            abs_lib_path = bpy.path.abspath(lib_path_original)
            display_hash = lib.get('blend_vault_hash') # Start with property hash
            
            hash_from_lib_file = None
            try:
                # Temporarily load library to read its blend_vault_hash text block
                existing_texts_before_load = set(bpy.data.texts.keys())
                with bpy.data.libraries.load(abs_lib_path, link=False) as (data_from, data_to):
                    if 'blend_vault_hash' in data_from.texts:
                        data_to.texts = ['blend_vault_hash']
                
                newly_loaded_texts = set(bpy.data.texts.keys()) - existing_texts_before_load
                for text_name in newly_loaded_texts:
                    text_block = bpy.data.texts.get(text_name)
                    if text_block: # Should be the 'blend_vault_hash' text block
                        hash_from_lib_file = text_block.as_string().strip()
                        bpy.data.texts.remove(text_block) # Clean up loaded text block
                        break
            except Exception as e:
                print(f"[Blend Vault][LibraryInfo] Could not load hash from library file '{abs_lib_path}': {e}")
                pass # Fallback to property hash if loading fails

            if hash_from_lib_file:
                if display_hash != hash_from_lib_file:
                    # If hash from file is different, update property and use file hash
                    lib_props = lib.id_properties_ensure()
                    lib_props['blend_vault_hash'] = hash_from_lib_file
                    display_hash = hash_from_lib_file
            
            if not display_hash:
                display_hash = 'MISSING_HASH'
            
            linked_library_sidecar_link_path = processed_lib_path_for_link + SIDECAR_EXTENSION
            blend_vault_data_lines.append(f"| Linked Library | [[{processed_lib_path_for_link}]] | [[{linked_library_sidecar_link_path}\\|{display_hash}]] |")

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
