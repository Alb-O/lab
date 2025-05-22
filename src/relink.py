import bpy # type: ignore
import os
import json
import re
from .config import SIDECAR_EXTENSION, BLUE, RESET, RED, YELLOW, GREEN, POLL_INTERVAL # Added GREEN, POLL_INTERVAL
from .hashing import ensure_blendfile_hash # For reading hash from linked files if needed

@bpy.app.handlers.persistent
def relink_library_info(*args, **kwargs):
    """Relinks libraries based on information in the sidecar Markdown file."""
    if not bpy.data.is_saved:
        print(f"{YELLOW}[Blend Vault][LibraryRelink] Current .blend file is not saved. Cannot process sidecar.{RESET}")
        return

    blend_path = bpy.data.filepath
    md_path = blend_path + SIDECAR_EXTENSION

    if not os.path.exists(md_path):
        print(f"{YELLOW}[Blend Vault][LibraryRelink] Sidecar file not found: {md_path}{RESET}")
        return

    print(f"[Blend Vault][LibraryRelink] Processing sidecar file: {md_path}")
    
    found_any_link_to_process = False
    linked_libraries_header_idx = -1
    
    try:
        with open(md_path, 'r', encoding='utf-8') as f:
            lines = f.readlines()

        # Find the "### Linked Libraries" section
        for i, line in enumerate(lines):
            if line.strip() == "### Linked Libraries":
                linked_libraries_header_idx = i
                break
        
        if linked_libraries_header_idx == -1:
            print(f"[Blend Vault][LibraryRelink] '### Linked Libraries' section not found in {md_path}.")
            return

        parsing_json_block = False
        json_accumulator = []
        active_md_link_name_for_log = None # Stores the display name from the MD link [name](path)
        active_md_link_path = None       # Stores the path from the Markdown link
        
        current_line_idx = linked_libraries_header_idx + 1
        while current_line_idx < len(lines):
            line_raw = lines[current_line_idx]
            line_stripped = line_raw.strip()

            if parsing_json_block:
                if line_stripped == "```": # End of JSON block
                    parsing_json_block = False
                    json_str = "".join(json_accumulator)
                    json_accumulator = []
                    
                    current_link_name_for_processing = active_md_link_name_for_log 

                    if not current_link_name_for_processing:
                        print(f"{RED}[Blend Vault][LibraryRelink] ERROR: Ended JSON block but no active Markdown link context was found. JSON: {json_str[:100]}...{RESET}")
                    elif not json_str.strip():
                        print(f"[Blend Vault][LibraryRelink] Empty JSON block found for '{current_link_name_for_processing}'. Skipping.")
                    else:
                        try:
                            data = json.loads(json_str)
                            stored_path_from_json = data.get("path")
                            uuid_data_from_json = data.get("uuid")
                            
                            stored_blendfile_hash = None
                            if isinstance(uuid_data_from_json, dict):
                                stored_blendfile_hash = uuid_data_from_json.get("blendfile_uuid")
                            elif isinstance(uuid_data_from_json, str):
                                stored_blendfile_hash = uuid_data_from_json
                            
                            if stored_path_from_json and stored_blendfile_hash and stored_blendfile_hash != "MISSING_HASH":
                                print(f"[Blend Vault][LibraryRelink] Processing entry: Path='{stored_path_from_json}', Blendfile Hash='{stored_blendfile_hash}' (from MD link '{current_link_name_for_processing}')")
                                found_any_link_to_process = True
                                # Prefer the path from the Markdown link above the JSON block
                                target_rel_path = active_md_link_path or stored_path_from_json
                                rel_path = '//' + target_rel_path

                                found_matching_lib = False
                                for lib in bpy.data.libraries:
                                    lib_prop_val = lib.get('blend_vault_hash')
                                    actual_lib_identifier = None
                                    if lib_prop_val:
                                        try:
                                            parsed_lib_prop = json.loads(lib_prop_val)
                                            if isinstance(parsed_lib_prop, dict):
                                                actual_lib_identifier = parsed_lib_prop.get("blendfile_uuid")
                                            elif isinstance(parsed_lib_prop, str): 
                                                actual_lib_identifier = parsed_lib_prop
                                        except json.JSONDecodeError:
                                            actual_lib_identifier = lib_prop_val 
                                
                                    if actual_lib_identifier and actual_lib_identifier == stored_blendfile_hash:
                                        found_matching_lib = True
                                        print(f"[Blend Vault][LibraryRelink] Found library '{lib.name}' with matching Blend Vault ID: {actual_lib_identifier}")
                                        lib_path_norm = lib.filepath.replace('\\\\\\\\', '/').lstrip('//')
                                        if lib_path_norm != target_rel_path:
                                            print(f"{BLUE}[Blend Vault] Relinking '{lib.name}' from '{lib.filepath}' -> '{rel_path}'{RESET}")
                                            lib.filepath = rel_path
                                            try:
                                                lib.reload()
                                            except Exception as e:
                                                print(f"{RED}[Blend Vault][LibraryRelink] Failed to reload '{lib.name}' after path update: {e}{RESET}")
                                        else:
                                            print(f"[Blend Vault][LibraryRelink] Path for '{lib.name}' ('{lib.filepath}') already matches stored relative path ('{rel_path}').")
                                        break 
                                
                                if not found_matching_lib:
                                    print(f"[Blend Vault][LibraryRelink] Library with Blend Vault ID {stored_blendfile_hash} not found. Attempting to relink existing library by filename.")
                                    # Try to find an existing loaded library whose filename matches the Markdown link name
                                    md_basename = os.path.basename(active_md_link_path or stored_path_from_json)
                                    relinked_by_name = False
                                    for lib_match in bpy.data.libraries:
                                        if os.path.basename(lib_match.filepath) == md_basename:
                                            print(f"{BLUE}[Blend Vault][LibraryRelink] Found existing library entry '{lib_match.name}' matching filename '{md_basename}'. Relinking to '{rel_path}'{RESET}")
                                            lib_match.filepath = rel_path
                                            try:
                                                lib_match.reload()
                                                relinked_by_name = True
                                                found_any_link_to_process = True
                                            except Exception as e:
                                                print(f"{RED}[Blend Vault][LibraryRelink] Failed to reload '{lib_match.name}' after name-based relink: {e}{RESET}")
                                            break
                                    if relinked_by_name:
                                        # Skip loading a new library since we updated the existing one
                                        active_md_link_name_for_log = None
                                        current_line_idx += 1
                                        continue
                                    print(f"[Blend Vault][LibraryRelink] No existing library matched by filename '{md_basename}'. Loading new library." )
                                    working_dir = os.path.dirname(bpy.data.filepath)
                                    candidate_abs_path = os.path.normpath(os.path.join(working_dir, active_md_link_path or stored_path_from_json))
                                    
                                    relinked_or_loaded_by_path = False
                                    # Try to fix an existing broken library entry by matching name
                                    for lib_to_fix in bpy.data.libraries:
                                        is_missing = False
                                        if hasattr(lib_to_fix, 'is_missing'): 
                                            is_missing = lib_to_fix.is_missing
                                        else: 
                                            abs_lib_path = bpy.path.abspath(lib_to_fix.filepath)
                                            if not os.path.exists(abs_lib_path):
                                                is_missing = True
                                        
                                        if is_missing and current_link_name_for_processing: # Ensure current_link_name_for_processing is not None
                                            lib_to_fix_name_no_ext, _ = os.path.splitext(lib_to_fix.name)
                                            # Ensure current_link_name_for_processing is a string before os.path.splitext
                                            md_link_name_str = str(current_link_name_for_processing)
                                            md_link_name_no_ext, _ = os.path.splitext(md_link_name_str)

                                            if lib_to_fix_name_no_ext == md_link_name_no_ext:
                                                print(f"{BLUE}[Blend Vault][LibraryRelink] Found a missing library entry '{lib_to_fix.name}' (matching MD link name '{current_link_name_for_processing}'). Updating its path from '{lib_to_fix.filepath}' to '{rel_path}'.{RESET}")
                                                lib_to_fix.filepath = rel_path 
                                                try:
                                                    lib_to_fix.reload()
                                                    print(f"{BLUE}[Blend Vault][LibraryRelink] Successfully reloaded library '{lib_to_fix.name}' at new path: {rel_path}{RESET}")
                                                    relinked_or_loaded_by_path = True
                                                except Exception as e:
                                                    print(f"{RED}[Blend Vault][LibraryRelink] Failed to reload library '{lib_to_fix.name}' after path update to {rel_path}: {e}{RESET}")
                                                break 
                                            else:
                                                print(f"[Blend Vault][LibraryRelink] Skipping missing library '{lib_to_fix.name}' as its name does not match MD link '{current_link_name_for_processing}'.")

                                    if not relinked_or_loaded_by_path: 
                                        first_broken_lib_candidate = None
                                        for lib_candidate in bpy.data.libraries:
                                            is_missing_candidate = False
                                            if hasattr(lib_candidate, 'is_missing'): is_missing_candidate = lib_candidate.is_missing
                                            else: 
                                                if not os.path.exists(bpy.path.abspath(lib_candidate.filepath)): is_missing_candidate = True
                                            
                                            if is_missing_candidate:
                                                first_broken_lib_candidate = lib_candidate
                                                break
                                        
                                        if first_broken_lib_candidate:
                                            print(f"{BLUE}[Blend Vault][LibraryRelink] No specific missing library matched by name. Attempting to use first available missing library entry '{first_broken_lib_candidate.name}' for path '{rel_path}'.{RESET}")
                                            first_broken_lib_candidate.filepath = rel_path
                                            try:
                                                first_broken_lib_candidate.reload()
                                                print(f"{BLUE}[Blend Vault][LibraryRelink] Successfully reloaded library '{first_broken_lib_candidate.name}' at new path: {rel_path}{RESET}")
                                                relinked_or_loaded_by_path = True
                                            except Exception as e:
                                                print(f"{RED}[Blend Vault][LibraryRelink] Failed to reload library '{first_broken_lib_candidate.name}' using path {rel_path}: {e}{RESET}")

                                    if not relinked_or_loaded_by_path: 
                                        if os.path.exists(candidate_abs_path):
                                            print(f"[Blend Vault][LibraryRelink] Attempting to load missing library using Markdown link path: {rel_path}")
                                            try:
                                                # bpy.ops.wm.link_libraries(filepath=candidate_abs_path) # This was Blender 2.x
                                                with bpy.data.libraries.load(candidate_abs_path, link=True) as (data_from, data_to):
                                                    pass # Just loading the library via Markdown link path
                                                print(f"{BLUE}[Blend Vault][LibraryRelink] Successfully linked new library from {rel_path}{RESET}")
                                            except RuntimeError as rte:
                                                print(f"{RED}[Blend Vault][LibraryRelink] Runtime error linking new library from {rel_path}: {rte}{RESET}")
                                            except Exception as e:
                                                print(f"{RED}[Blend Vault][LibraryRelink] Failed to link new library from {rel_path}: {e}{RESET}")
                                        else:
                                            print(f"{YELLOW}[Blend Vault][LibraryRelink] Sidecar path '{stored_path_from_json}' (resolved to '{candidate_abs_path}') does not exist. Cannot link.{RESET}")
                            elif stored_blendfile_hash == "MISSING_HASH":
                                print(f"[Blend Vault][LibraryRelink] Entry for '{current_link_name_for_processing}' has 'MISSING_HASH'. Skipping relink by hash.")
                            else: 
                                print(f"{YELLOW}[Blend Vault][LibraryRelink] Invalid data in JSON block for '{current_link_name_for_processing}': Missing path or UUID info.{RESET}")
                        
                        except json.JSONDecodeError as jde:
                            error_msg = jde.msg
                            error_line_in_json = jde.lineno 
                            error_col_in_json = jde.colno 
                            print(f"{RED}[Blend Vault][LibraryRelink] Failed to parse JSON for '{current_link_name_for_processing}'.")
                            print(f"[Blend Vault][LibraryRelink] JSONDecodeError: {error_msg} (at line {error_line_in_json}, column {error_col_in_json} of the collected JSON string).")
                            print(f"[Blend Vault][LibraryRelink] Collected JSON string that failed was:\\n>>>>\\n{json_str}\\n<<<<{RESET}")
                    
                    active_md_link_name_for_log = None # Reset for the next link in the file
                else:
                    json_accumulator.append(line_raw) # Add raw line to preserve formatting within JSON
            
            elif line_stripped.startswith("```json"): # Start of JSON block
                if active_md_link_name_for_log is None:
                    print(f"{YELLOW}[Blend Vault][LibraryRelink] Found ```json block but no preceding Markdown link was active. Skipping this JSON block.{RESET}")
                    # Consume lines until end of this unexpected JSON block or end of section
                    while current_line_idx + 1 < len(lines) and lines[current_line_idx + 1].strip() != "```":
                        current_line_idx += 1
                        if lines[current_line_idx].strip().startswith("###") or lines[current_line_idx].strip().startswith("## "): # Stop if we hit another header
                            break
                    if current_line_idx + 1 < len(lines) and lines[current_line_idx + 1].strip() == "```": # consume the closing ```
                        current_line_idx +=1 
                else:
                    parsing_json_block = True
                    json_accumulator = []
            
            elif line_stripped.startswith("###") or line_stripped.startswith("## "): # Stop if we hit another header
                if parsing_json_block:
                    print(f"{YELLOW}[Blend Vault][LibraryRelink] Warning: Encountered new header while still parsing JSON for '{active_md_link_name_for_log}'. Discarding partial JSON.{RESET}")
                    parsing_json_block = False
                    json_accumulator = []
                    active_md_link_name_for_log = None
                break # End of "Linked Libraries" section or malformed entry

            else: # Potentially a Markdown link or other text
                md_link_match = re.match(r'^\[([^\]]+)\]\(<([^>]+)>\)$', line_stripped)
                if md_link_match:
                    if active_md_link_name_for_log and not parsing_json_block: 
                        print(f"{YELLOW}[Blend Vault][LibraryRelink] Warning: MD link for '{active_md_link_name_for_log}' wasn't followed by JSON before new link for '{md_link_match.group(1)}'.{RESET}")
                    
                    active_md_link_name_for_log = md_link_match.group(1)
                    # Store the relative path from the Markdown link for use in relinking
                    active_md_link_path = md_link_match.group(2)
                    # md_path_from_link = md_link_match.group(2) # For context if needed
                    print(f"[Blend Vault][LibraryRelink] Found Markdown link for library: {active_md_link_name_for_log} -> {active_md_link_path}")
                # else:
                    # It's some other text, or blank line, just ignore if not parsing JSON
            
            current_line_idx += 1
        
        if parsing_json_block: # Unclosed JSON block at EOF or end of section
            print(f"{YELLOW}[Blend Vault][LibraryRelink] Warning: Reached end of 'Linked Libraries' section while still parsing JSON for '{active_md_link_name_for_log}'. Discarding partial JSON.{RESET}")

    except Exception as e:
        print(f"{RED}[Blend Vault][LibraryRelink] An error occurred during the relinking process: {e}{RESET}")
        import traceback
        traceback.print_exc()

    if not found_any_link_to_process and linked_libraries_header_idx != -1:
        print(f"[Blend Vault][LibraryRelink] No valid library entries were processed from the sidecar file: {md_path}")
    
    try:
        bpy.ops.file.make_paths_relative()
        print(f"{BLUE}[Blend Vault][LibraryRelink] Made all external file paths relative.{RESET}")
    except RuntimeError as e:
        print(f"{YELLOW}[Blend Vault][LibraryRelink] Could not make paths relative: {e}. (This may happen if the file is not saved or has no external links).{RESET}")
    except Exception as e:
        print(f"{RED}[Blend Vault][LibraryRelink] Error making paths relative: {e}{RESET}")

    print("[Blend Vault][LibraryRelink] Finished relink attempt.")

relink_library_info.persistent = True

# Store last modification times for sidecar files
t_last_sidecar_mtimes = {}

def sidecar_poll_timer():
    """Timer callback to poll sidecar file changes and trigger relink if modified."""
    blend_path = bpy.data.filepath
    if blend_path:
        md_path = blend_path + SIDECAR_EXTENSION
        try:
            if os.path.exists(md_path):
                mtime = os.path.getmtime(md_path)
                last = t_last_sidecar_mtimes.get(md_path)
                if last is None:
                    # initialize
                    t_last_sidecar_mtimes[md_path] = mtime
                elif mtime > last:
                    # file changed: update timestamp and relink
                    t_last_sidecar_mtimes[md_path] = mtime
                    print(f"{GREEN}[Blend Vault] Sidecar file '{md_path}' modified. Triggering relink.{RESET}")
                    relink_library_info()
        except Exception as e:
            print(f"{RED}[Blend Vault][sidecar_poll_timer] Error checking sidecar file '{md_path}': {e}{RESET}")
    return POLL_INTERVAL

@bpy.app.handlers.persistent
def start_sidecar_poll_timer(*args, **kwargs):
    """Handler to register polling timer after file load, ensuring persistence across blend reloads."""
    # Check if timer is already registered to prevent duplicates if this handler runs multiple times
    # without an unregister call (e.g. script reload without full Blender restart)
    is_registered = False
    if bpy.app.timers.is_registered(sidecar_poll_timer):
        is_registered = True
        print("[Blend Vault] Sidecar polling timer already registered.")

    if not is_registered:
        try:
            bpy.app.timers.register(sidecar_poll_timer, first_interval=POLL_INTERVAL)
            print(f"{GREEN}[Blend Vault] Sidecar polling timer registered (interval: {POLL_INTERVAL}s).{RESET}")
        except Exception as e: # Catch any error during registration
            print(f"[Blend Vault][Error] Failed to register sidecar polling timer: {e}")
