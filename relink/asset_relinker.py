import bpy  # type: ignore
import os
import json
import re
import traceback
from utils import SIDECAR_EXTENSION, LOG_COLORS, get_asset_sources_map, BV_UUID_PROP, BV_FILE_UUID_KEY, BV_UUID_KEY, MD_LINK_FORMATS, ensure_library_hash

# Helper function to parse a library's own sidecar for its "Current File" assets
def _get_current_file_assets_from_sidecar(sidecar_file_path: str):
    """
    Parses a library's .side.md file to find the "### Current File" section
    and extracts the library's own UUID and its list of assets.

    Returns:
        tuple: (library_blend_file_uuid, list_of_assets)
               list_of_assets are dicts like {"uuid": "...", "name": "...", "type": "..."}
               Returns (None, []) if parsing fails or section not found.
    """
    if not os.path.exists(sidecar_file_path):
        print(f"{LOG_COLORS['WARN']}[Blend Vault][AssetRelinkHelper] Library sidecar file not found: {sidecar_file_path}{LOG_COLORS['RESET']}")
        return None, []

    print(f"{LOG_COLORS['INFO']}[Blend Vault][AssetRelinkHelper] Reading library sidecar: {sidecar_file_path}{LOG_COLORS['RESET']}")
    
    assets_from_lib_sidecar = []
    library_blend_uuid_from_sidecar = None

    try:
        with open(sidecar_file_path, 'r', encoding='utf-8') as f:
            lines = f.readlines()

        current_file_header_idx = -1
        for i, line in enumerate(lines):
            if line.strip() == "### Current File":
                current_file_header_idx = i
                break
        
        if current_file_header_idx == -1:
            print(f"{LOG_COLORS['INFO']}[Blend Vault][AssetRelinkHelper] '### Current File' section not found in {sidecar_file_path}.{LOG_COLORS['RESET']}")
            return None, []

        parsing_json_block = False
        json_accumulator = []
        
        current_line_idx = current_file_header_idx + 1
        while current_line_idx < len(lines):
            line_raw = lines[current_line_idx]
            line_stripped = line_raw.strip()

            if parsing_json_block:
                if line_stripped == "```": # End of JSON block
                    parsing_json_block = False
                    json_str = "".join(json_accumulator)
                    
                    if json_str.strip():
                        try:
                            data = json.loads(json_str)
                            library_blend_uuid_from_sidecar = data.get(BV_UUID_KEY) or data.get(BV_FILE_UUID_KEY)
                            
                            current_file_assets = data.get("assets", [])
                            for asset_info in current_file_assets:
                                asset_uuid = asset_info.get("uuid")
                                asset_name = asset_info.get("name")
                                asset_type = asset_info.get("type")
                                if asset_uuid and asset_name and asset_type:
                                    assets_from_lib_sidecar.append({
                                        "uuid": asset_uuid,
                                        "name": asset_name,
                                        "type": asset_type
                                    })
                            break
                        except json.JSONDecodeError as e:
                            print(f"{LOG_COLORS['ERROR']}[Blend Vault][AssetRelinkHelper] Failed to parse JSON in 'Current File' of {sidecar_file_path}: {e}{LOG_COLORS['RESET']}")
                else:
                    json_accumulator.append(line_raw)
            
            elif line_stripped.startswith("```json"):
                parsing_json_block = True
                json_accumulator = []
            
            elif re.match(r"^(##[^#]|###[^#])", line_stripped):
                if parsing_json_block:
                    print(f"{LOG_COLORS['WARN']}[Blend Vault][AssetRelinkHelper] New header while parsing 'Current File' JSON in {sidecar_file_path}. Discarding.{LOG_COLORS['RESET']}")
                break 
            current_line_idx += 1
        
        return library_blend_uuid_from_sidecar, assets_from_lib_sidecar

    except Exception as e:
        print(f"{LOG_COLORS['ERROR']}[Blend Vault][AssetRelinkHelper] Error reading library sidecar {sidecar_file_path}: {e}{LOG_COLORS['RESET']}")
        traceback.print_exc()
        return None, []

# NEW HELPER FUNCTION
def _parse_main_sidecar_linked_libraries_section(main_sidecar_lines: list[str], main_blend_dir: str):
    """
    Parses the "### Linked Libraries" section of the main sidecar file.

    Returns:
        dict: {
            "relative_library_path_A": {
                "uuid": "library_A_blendfile_uuid", # UUID of the library .blend file itself
                "assets": [{"uuid": "asset1_uuid", "name": "NameInMain", "type": "Type1"}, ...]
            }, ...
        }
        Returns an empty dict if parsing fails or section not found.
    """
    print(f"{LOG_COLORS['INFO']}[Blend Vault][AssetRelinkHelper] Parsing 'Linked Libraries' from main sidecar.{LOG_COLORS['RESET']}")
    main_file_linked_data = {}
    
    linked_libraries_header_idx = -1
    for i, line in enumerate(main_sidecar_lines):
        if line.strip() == "### Linked Libraries":
            linked_libraries_header_idx = i
            break
    
    if linked_libraries_header_idx == -1:
        print(f"{LOG_COLORS['INFO']}[Blend Vault][AssetRelinkHelper] '### Linked Libraries' section not found in main sidecar.{LOG_COLORS['RESET']}")
        return {}

    parsing_json_block = False
    json_accumulator = []
    active_library_relative_path = None
    
    current_line_idx = linked_libraries_header_idx + 1
    while current_line_idx < len(main_sidecar_lines):
        line_raw = main_sidecar_lines[current_line_idx]
        line_stripped = line_raw.strip()

        if parsing_json_block:
            if line_stripped == "```":
                parsing_json_block = False
                json_str = "".join(json_accumulator)
                json_accumulator = []
                
                if active_library_relative_path and json_str.strip():
                    try:
                        lib_data_from_json = json.loads(json_str)
                        # Store the whole JSON content for this library entry
                        main_file_linked_data[active_library_relative_path] = {
                            "uuid": lib_data_from_json.get("uuid"), # UUID of the library .blend file
                            "assets": lib_data_from_json.get("assets", []) # List of assets linked from it
                        }
                        print(f"{LOG_COLORS['DEBUG']}[Blend Vault][AssetRelinkHelper]   Parsed library '{active_library_relative_path}' from main sidecar: {len(lib_data_from_json.get('assets',[]))} assets.{LOG_COLORS['RESET']}")
                    except json.JSONDecodeError as e:
                        print(f"{LOG_COLORS['ERROR']}[Blend Vault][AssetRelinkHelper] Failed to parse JSON for lib '{active_library_relative_path}' in main sidecar: {e}{LOG_COLORS['RESET']}")
                active_library_relative_path = None # Reset for the next library
            else:
                json_accumulator.append(line_raw)
        
        elif line_stripped.startswith("```json"):
            if active_library_relative_path: # We must have an active library path from a markdown link
                parsing_json_block = True
                json_accumulator = []
            else:
                print(f"{LOG_COLORS['WARN']}[Blend Vault][AssetRelinkHelper] Found ```json in 'Linked Libraries' but no active library path. Skipping block.{LOG_COLORS['RESET']}")
                # Skip this JSON block as we don't know which library it belongs to
                temp_skip_idx = current_line_idx + 1
                while temp_skip_idx < len(main_sidecar_lines):
                    if main_sidecar_lines[temp_skip_idx].strip() == "```": current_line_idx = temp_skip_idx; break
                    if re.match(r"^(##[^#]|###[^#])", main_sidecar_lines[temp_skip_idx].strip()): current_line_idx = temp_skip_idx -1; break # Stop if new major header
                    temp_skip_idx += 1
                else: current_line_idx = temp_skip_idx -1 # Reached EOF while skipping
        
        else: # Look for library markdown links
            link_regex_details = MD_LINK_FORMATS.get('MD_ANGLE_BRACKETS', {})
            link_regex = link_regex_details.get('regex')
            if not link_regex: link_regex = r"#### \\[(.+?)\\]\\(<(.+?)>\\)" # Default/fallback
            
            md_link_match = re.search(link_regex, line_stripped)
            if md_link_match:
                if parsing_json_block: # Should not happen if sidecar is well-formed
                    print(f"{LOG_COLORS['WARN']}[Blend Vault][AssetRelinkHelper] New library link '{md_link_match.group(1)}' found before previous JSON block for '{active_library_relative_path}' closed. Discarding previous.{LOG_COLORS['RESET']}")
                    parsing_json_block = False
                    json_accumulator = []
                
                # Group 2 is usually the name, group 3 (if angle brackets) or 2 (if not) is the path
                path_group_index = 3 if link_regex_details.get('format', '').count('%s') == 2 and '<' in link_regex_details.get('format', '') else 2
                try:
                    active_library_relative_path = md_link_match.group(path_group_index)
                    print(f"{LOG_COLORS['DEBUG']}[Blend Vault][AssetRelinkHelper]   Found library link in main sidecar: {md_link_match.group(1)} -> {active_library_relative_path}{LOG_COLORS['RESET']}")
                except IndexError:
                    print(f"{LOG_COLORS['ERROR']}[Blend Vault][AssetRelinkHelper]   Error parsing library link, regex group {path_group_index} not found in '{line_stripped}' with regex '{link_regex}'. Match groups: {md_link_match.groups()}{LOG_COLORS['RESET']}")
                    active_library_relative_path = None


            elif re.match(r"^(##[^#]|###[^#])", line_stripped): # Reached another major section
                if parsing_json_block:
                    print(f"{LOG_COLORS['WARN']}[Blend Vault][AssetRelinkHelper] New section header found while parsing JSON for '{active_library_relative_path}'. Discarding.{LOG_COLORS['RESET']}")
                break # Stop parsing "Linked Libraries"
        
        current_line_idx += 1
    
    if parsing_json_block: # EOF reached while still in a JSON block
         print(f"{LOG_COLORS['WARN']}[Blend Vault][AssetRelinkHelper] EOF reached while parsing JSON for '{active_library_relative_path}'. Discarding.{LOG_COLORS['RESET']}")
    
    print(f"{LOG_COLORS['INFO']}[Blend Vault][AssetRelinkHelper] Finished parsing 'Linked Libraries'. Found data for {len(main_file_linked_data)} libraries.{LOG_COLORS['RESET']}")
    return main_file_linked_data


@bpy.app.handlers.persistent
def relink_renamed_assets(*args, **kwargs):
    """Relinks renamed individual asset datablocks by comparing sidecar states."""
    if not bpy.data.is_saved:
        print(f"{LOG_COLORS['WARN']}[Blend Vault][AssetRelink] Current .blend file is not saved. Cannot process sidecar.{LOG_COLORS['RESET']}")
        return

    blend_path = bpy.data.filepath
    md_path = blend_path + SIDECAR_EXTENSION # Main sidecar path

    if not os.path.exists(md_path):
        print(f"{LOG_COLORS['WARN']}[Blend Vault][AssetRelink] Main sidecar file not found: {md_path}{LOG_COLORS['RESET']}")
        return

    print(f"{LOG_COLORS['INFO']}[Blend Vault][AssetRelink] Processing main sidecar for asset relinking: {md_path}{LOG_COLORS['RESET']}")
    
    # DEBUG: Check current session items before starting
    collection_map = get_asset_sources_map()
    total_session_items = 0
    for asset_type, bpy_collection in collection_map.items():
        count = len([item for item in bpy_collection if getattr(item, 'session_uid', None)])
        total_session_items += count
        print(f"{LOG_COLORS['DEBUG']}[Blend Vault][AssetRelink] Found {count} session items with session_uid in {asset_type}.{LOG_COLORS['RESET']}")
    print(f"{LOG_COLORS['DEBUG']}[Blend Vault][AssetRelink] Total session items with session_uid: {total_session_items}{LOG_COLORS['RESET']}")

    main_blend_dir = os.path.dirname(blend_path)

    try:
        with open(md_path, 'r', encoding='utf-8') as f:
            main_sidecar_lines = f.readlines()
    except Exception as e:
        print(f"{LOG_COLORS['ERROR']}[Blend Vault][AssetRelink] Failed to read main sidecar file {md_path}: {e}{LOG_COLORS['RESET']}")
        return

    try: # New try statement to wrap the main logic
        # Step 1: Parse the main sidecar's "Linked Libraries" section to know what was linked and how.
        # This gives us: { "rel_lib_path_A": {"uuid": "lib_A_file_uuid", "assets": [{"uuid": "asset1_uuid", "name": "OldNameInMain", "type": "Type1"}, ...]}, ... }
        main_file_linked_data = _parse_main_sidecar_linked_libraries_section(main_sidecar_lines, main_blend_dir)

        if not main_file_linked_data:
            print(f"{LOG_COLORS['INFO']}[Blend Vault][AssetRelink] No linked library data found in main sidecar. Nothing to relink.{LOG_COLORS['RESET']}")
            return

        # Step 2: Get authoritative asset info from each linked library's own sidecar.
        # This will be: { "rel_lib_path_A": { "asset1_uuid": {"name": "CurrentNameInLibA", "type": "Type1"}, ... }, ... }
        authoritative_asset_info_by_lib = {}
        for rel_lib_path, lib_link_details_from_main in main_file_linked_data.items():
            # Ensure the library is reloaded in this session so updates from other Blender instances are loaded
            try:
                abs_lib_to_reload = os.path.normpath(os.path.abspath(os.path.join(main_blend_dir, rel_lib_path)))
                bpy.ops.wm.lib_reload(filepath=abs_lib_to_reload)
                print(f"{LOG_COLORS['INFO']}[Blend Vault][AssetRelink] Reloaded library: {abs_lib_to_reload}{LOG_COLORS['RESET']}")
            except Exception as e:
                print(f"{LOG_COLORS['WARN']}[Blend Vault][AssetRelink] Could not reload library '{rel_lib_path}': {e}{LOG_COLORS['RESET']}")

            abs_library_blend_path = ""
            if rel_lib_path.startswith("//"):
                abs_library_blend_path = bpy.path.abspath(rel_lib_path)
            else:
                # Assuming rel_lib_path is relative to the main blend file's directory
                abs_library_blend_path = os.path.normpath(os.path.join(main_blend_dir, rel_lib_path))
            
            library_actual_sidecar_path = abs_library_blend_path + SIDECAR_EXTENSION
            
            # _get_current_file_assets_from_sidecar returns (library_blend_file_uuid, list_of_assets_in_lib)
            # list_of_assets_in_lib are dicts like {"uuid": "...", "name": "...", "type": "..."}
            _, assets_in_lib_source = _get_current_file_assets_from_sidecar(library_actual_sidecar_path)
            
            if assets_in_lib_source:
                authoritative_asset_info_by_lib[rel_lib_path] = {}
                for asset_data in assets_in_lib_source:
                    authoritative_asset_info_by_lib[rel_lib_path][asset_data["uuid"]] = {
                        "name": asset_data["name"],
                        "type": asset_data["type"]
                    }
                print(f"{LOG_COLORS['DEBUG']}[Blend Vault][AssetRelink]   Authoritative data for lib '{rel_lib_path}': Found {len(assets_in_lib_source)} assets in its sidecar.{LOG_COLORS['RESET']}")
            else:
                print(f"{LOG_COLORS['INFO']}[Blend Vault][AssetRelink]   No authoritative asset data found in sidecar for library '{rel_lib_path}' ({library_actual_sidecar_path}).{LOG_COLORS['RESET']}")

        if not authoritative_asset_info_by_lib:
            print(f"{LOG_COLORS['INFO']}[Blend Vault][AssetRelink] No authoritative data found from any library sidecars. Cannot determine renames.{LOG_COLORS['RESET']}")
            return
            
        # Step 3: Identify relink candidates by comparing main file's last known state with library's current state.
        relink_operations = []
        asset_types_to_bpy_collections = get_asset_sources_map() # e.g. {"Collection": bpy.data.collections, ...}

        for rel_lib_path, lib_link_details_from_main in main_file_linked_data.items():
            assets_linked_from_this_lib_in_main = lib_link_details_from_main.get("assets", [])
            authoritative_assets_in_this_lib = authoritative_asset_info_by_lib.get(rel_lib_path, {})

            if not authoritative_assets_in_this_lib:
                print(f"{LOG_COLORS['DEBUG']}[Blend Vault][AssetRelink] Skipping library '{rel_lib_path}' for relink check as no authoritative data was found for it.{LOG_COLORS['RESET']}")
                continue

            for old_asset_details in assets_linked_from_this_lib_in_main:
                old_name_in_main = old_asset_details.get("name")
                asset_uuid = old_asset_details.get("uuid")
                asset_type = old_asset_details.get("type")

                if not (old_name_in_main and asset_uuid and asset_type):
                    print(f"{LOG_COLORS['WARN']}[Blend Vault][AssetRelink] Incomplete asset data in main sidecar for an asset from '{rel_lib_path}': {old_asset_details}. Skipping.{LOG_COLORS['RESET']}")
                    continue

                current_asset_info_from_lib = authoritative_assets_in_this_lib.get(asset_uuid)
                if not current_asset_info_from_lib:
                    # This asset UUID, known to be linked in the main file, is no longer listed in its library's sidecar.
                    # This could mean it was deleted from the library. Relinking can't fix this.
                    print(f"{LOG_COLORS['INFO']}[Blend Vault][AssetRelink] Asset '{old_name_in_main}' (UUID: {asset_uuid}, Type: {asset_type}) from lib '{rel_lib_path}' is no longer listed in the library's sidecar. Cannot relink.{LOG_COLORS['RESET']}")
                    continue
                
                current_name_in_lib = current_asset_info_from_lib.get("name")
                # Type should ideally match, but relink primarily uses name. Add check if needed.

                if old_name_in_main != current_name_in_lib:
                    print(f"{LOG_COLORS['INFO']}[Blend Vault][AssetRelink] Name mismatch for UUID {asset_uuid} from lib '{rel_lib_path}'. Main file knew: '{old_name_in_main}', Library sidecar says: '{current_name_in_lib}'. Candidate for relink.{LOG_COLORS['RESET']}")
                    
                    bpy_collection = asset_types_to_bpy_collections.get(asset_type)
                    if not bpy_collection:
                        print(f"{LOG_COLORS['WARN']}[Blend Vault][AssetRelink] Unknown asset type '{asset_type}' for '{old_name_in_main}'. Cannot find in bpy.data.{LOG_COLORS['RESET']}")
                        continue

                    # Resolve expected absolute library path
                    path_from_main_dir = os.path.join(main_blend_dir, rel_lib_path)
                    expected_abs_lib_path_normalized = os.path.normpath(os.path.abspath(path_from_main_dir))

                    # Reset session lookup variable
                    found_item_in_session = None
                    # Ensure session item gets the correct UUID from the library sidecar
                    for item in bpy_collection:
                        # Check that item's library matches expected path
                        libpath = None
                        if getattr(item, 'library', None) and item.library.filepath:
                            libpath = bpy.path.abspath(item.library.filepath)
                        elif hasattr(item, 'library_weak_reference') and item.library_weak_reference and item.library_weak_reference.filepath:
                            libpath = bpy.path.abspath(item.library_weak_reference.filepath)
                        if libpath and os.path.normpath(libpath) == expected_abs_lib_path_normalized:
                            # Assign the sidecar's UUID to the session item
                            try:
                                item.id_properties_ensure()[BV_UUID_PROP] = asset_uuid
                                print(f"{LOG_COLORS['DEBUG']}[Blend Vault][AssetRelink]   Assigned correct UUID '{asset_uuid}' to session item '{item.name}'.{LOG_COLORS['RESET']}")
                            except Exception:
                                pass
                            # Use this item for subsequent relink
                            found_item_in_session = item
                            break
                    if not found_item_in_session:
                        print(f"{LOG_COLORS['WARN']}[Blend Vault][AssetRelink]   Could not find session item in library '{rel_lib_path}' to assign UUID. Cannot relink '{old_name_in_main}'.{LOG_COLORS['RESET']}")
                        continue
                    # prepare relink_op as before now that correct session item and UUID prop are set
                    session_uid = getattr(found_item_in_session, 'session_uid', None)
                    relink_op = {
                        "session_uid": session_uid,
                        "target_library_path": rel_lib_path,
                        "target_asset_type": asset_type,
                        "target_asset_name": current_name_in_lib,
                        "old_name_for_log": old_name_in_main,
                        "uuid_for_log": asset_uuid
                    }
                    relink_operations.append(relink_op)
                    print(f"{LOG_COLORS['SUCCESS']}[Blend Vault][AssetRelink]   Prepared relink for UUID {asset_uuid} (session_uid: {session_uid}).{LOG_COLORS['RESET']}")
                    continue  # Skip further search by UUID
                else:
                    print(f"{LOG_COLORS['DEBUG']}[Blend Vault][AssetRelink] Asset '{old_name_in_main}' (UUID {asset_uuid}) from lib '{rel_lib_path}' names match. No relink needed.{LOG_COLORS['RESET']}")
        
        # Step 4: Execute Relinking
        if not relink_operations:
            print(f"{LOG_COLORS['INFO']}[Blend Vault][AssetRelink] No relink operations to perform.{LOG_COLORS['RESET']}")
        else:
            print(f"{LOG_COLORS['INFO']}[Blend Vault][AssetRelink] Found {len(relink_operations)} operations. Attempting to relocate...{LOG_COLORS['RESET']}")
            for op_data in relink_operations:
                # Construct operator parameters properly
                # Find the session item by session_uid
                collection_map = get_asset_sources_map()
                bpy_collection = collection_map.get(op_data['target_asset_type'])
                session_item = None
                if bpy_collection:
                    session_item = next((item for item in bpy_collection if getattr(item, 'session_uid', None) == op_data['session_uid']), None)
                if not session_item:
                    print(f"{LOG_COLORS['WARN']}[Blend Vault][AssetRelink] Could not find session item for session_uid {op_data['session_uid']}. Skipping relink.{LOG_COLORS['RESET']}")
                    continue

                # Determine library filepath
                lib_fp = None
                if getattr(session_item, 'library', None) and session_item.library.filepath:
                    lib_fp = session_item.library.filepath
                elif getattr(session_item, 'library_weak_reference', None) and session_item.library_weak_reference.filepath:
                    lib_fp = session_item.library_weak_reference.filepath
                
                if not lib_fp:
                    print(f"{LOG_COLORS['WARN']}[Blend Vault][AssetRelink] Session item {session_item.name} has no library filepath. Skipping.{LOG_COLORS['RESET']}")
                    continue

                # These are the values derived from sidecars and session item
                # Properly resolve absolute path, ensuring any relative components like ".." are normalized
                if lib_fp.startswith("//"):
                    abs_libblend_path = os.path.normpath(bpy.path.abspath(lib_fp))
                    print(f"{LOG_COLORS['DEBUG']}[Blend Vault][AssetRelink]   Path resolution: '{lib_fp}' -> bpy.path.abspath -> normpath -> '{abs_libblend_path}'{LOG_COLORS['RESET']}")
                else:
                    # For absolute paths, still normalize to resolve any ".." components
                    abs_libblend_path = os.path.normpath(lib_fp)
                    print(f"{LOG_COLORS['DEBUG']}[Blend Vault][AssetRelink]   Path resolution: '{lib_fp}' -> normpath -> '{abs_libblend_path}'{LOG_COLORS['RESET']}")
                asset_type = op_data['target_asset_type']  # Asset type (e.g., "Collection")
                new_asset_name = op_data['target_asset_name']  # New name of the asset in the library
                
                # Construct parameters for bpy.ops.wm.id_linked_relocate
                filename_arg = new_asset_name
                op_relative_path_bool = lib_fp.startswith("//") # filepath_arg: Relative path to .blend file + internal datablock path
                if op_relative_path_bool:
                    # Use the original relative path with backslashes, append internal path
                    filepath_arg = f"{lib_fp}\\{asset_type}\\{new_asset_name}"
                else:
                    # For absolute paths, still need the internal structure
                    filepath_arg = f"{abs_libblend_path}\\{asset_type}\\{new_asset_name}"
                
                # directory_arg: Absolute path to the internal directory within the .blend file
                # Must always be absolute, even when relative_path=True
                directory_arg = f"{abs_libblend_path}\\{asset_type}\\"
                
                # Store the current name before relinking to verify the operation later
                original_session_uid = op_data['session_uid']
                
                # Also get a backup reference using the UUID instead of session_uid
                backup_uuid = getattr(session_item, BV_UUID_PROP, None)
                
                try:
                    result = bpy.ops.wm.id_linked_relocate(
                        # id_session_uid is the correct parameter name, not session_uid
                        id_session_uid=op_data['session_uid'],
                        filepath=filepath_arg,
                        directory=directory_arg,
                        filename=filename_arg,
                        relative_path=op_relative_path_bool
                    )
                    print(f"{LOG_COLORS['SUCCESS']}[Blend Vault][AssetRelink] bpy.ops.wm.id_linked_relocate for '{op_data['old_name_for_log']}' returned: {result}{LOG_COLORS['RESET']}")
                    
                    # Re-fetch the session item as it might have been replaced or invalidated by the relink operation
                    # Try multiple strategies to find the renamed item
                    bpy_collection_after_relink = collection_map.get(op_data['target_asset_type'])
                    refetched_session_item = None
                    
                    if bpy_collection_after_relink:
                        # Strategy 1: Try to find by original session_uid
                        refetched_session_item = next((item for item in bpy_collection_after_relink if getattr(item, 'session_uid', None) == original_session_uid), None)
                        
                        if not refetched_session_item and backup_uuid:
                            # Strategy 2: Try to find by UUID if session_uid lookup failed
                            print(f"{LOG_COLORS['DEBUG']}[Blend Vault][AssetRelink] Session UID lookup failed, trying UUID backup lookup for {backup_uuid}{LOG_COLORS['RESET']}")
                            refetched_session_item = next((item for item in bpy_collection_after_relink if getattr(item, BV_UUID_PROP, None) == backup_uuid), None)
                        
                        if not refetched_session_item:
                            # Strategy 3: Try to find by new name in same library
                            print(f"{LOG_COLORS['DEBUG']}[Blend Vault][AssetRelink] UUID lookup also failed, trying name-based lookup for '{new_asset_name}'{LOG_COLORS['RESET']}")
                            for item in bpy_collection_after_relink:
                                item_lib_path = None
                                if getattr(item, 'library', None) and item.library.filepath:
                                    item_lib_path = bpy.path.abspath(item.library.filepath)
                                if item_lib_path and os.path.normpath(item_lib_path) == abs_libblend_path and item.name == new_asset_name:
                                    refetched_session_item = item
                                    print(f"{LOG_COLORS['DEBUG']}[Blend Vault][AssetRelink] Found renamed item by name and library match: '{item.name}'{LOG_COLORS['RESET']}")
                                    break
                    
                    if refetched_session_item:
                        print(f"{LOG_COLORS['SUCCESS']}[Blend Vault][AssetRelink] Successfully refetched session item '{refetched_session_item.name}' after relink.{LOG_COLORS['RESET']}")
                        # The item should already have the new name after relinking, but ensure consistency
                        if refetched_session_item.name != new_asset_name:
                            print(f"{LOG_COLORS['WARN']}[Blend Vault][AssetRelink] Item name '{refetched_session_item.name}' doesn't match expected '{new_asset_name}', updating...{LOG_COLORS['RESET']}")
                            refetched_session_item.name = new_asset_name
                    else:
                        print(f"{LOG_COLORS['WARN']}[Blend Vault][AssetRelink] Could not re-find session item after relink. Original session_uid: {original_session_uid}, UUID: {backup_uuid}{LOG_COLORS['RESET']}")
                except RuntimeError as e:
                    print(f"{LOG_COLORS['ERROR']}[Blend Vault][AssetRelink] RuntimeError during bpy.ops.wm.id_linked_relocate: {e}{LOG_COLORS['RESET']}")
                except Exception as e:
                    print(f"{LOG_COLORS['ERROR']}[Blend Vault][AssetRelink] Exception during bpy.ops.wm.id_linked_relocate: {e}{LOG_COLORS['RESET']}")
                    traceback.print_exc()
    except Exception as e: # This existing except block is now correctly paired
        print(f"{LOG_COLORS['ERROR']}[Blend Vault][AssetRelink] An error occurred during the asset relinking process: {e}{LOG_COLORS['RESET']}")
        traceback.print_exc()
    
    print(f"{LOG_COLORS['INFO']}[Blend Vault][AssetRelink] Finished asset relinking attempt.{LOG_COLORS['RESET']}")

relink_renamed_assets.persistent = True

def register():
    print(f"{LOG_COLORS['SUCCESS']}[Blend Vault] Asset relinking module loaded.{LOG_COLORS['RESET']}")

def unregister():
    print(f"{LOG_COLORS['WARN']}[Blend Vault] Asset relinking module unloaded.{LOG_COLORS['RESET']}")

if __name__ == "__main__":
    register()
