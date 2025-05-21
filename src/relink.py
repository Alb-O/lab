import bpy  # type: ignore
import os
import re
from .config import GREEN, BLUE, RESET, SIDECAR_EXTENSION

# Regex to parse the specific "Linked Library" table row
# Captures: 1=blend_path, 2=sidecar_path_in_link, 3=hash
table_row_pattern = re.compile(r"^\|\s*Linked Library\s*\|\s*\[\[(.*?)]]\s*\|\s*\[\[(.*?)\\\|(.*?)]]\s*\|$")

@bpy.app.handlers.persistent
def relink_library_info(*args, **kwargs):  # Decorated as persistent handler
    """Read sidecar Markdown and relink libraries based on persistent hash."""
    print(f"{GREEN}[Blend Vault] Relinking libraries for: {bpy.data.filepath}{RESET}")
    blend_path = bpy.data.filepath
    if not blend_path:
        print("[Blend Vault][LibraryRelink] No blend file path found, skipping relink.")
        return
    md_path = blend_path + SIDECAR_EXTENSION  # Use SIDECAR_EXTENSION from config
    print(f"[Blend Vault][LibraryRelink] Looking for sidecar file at: {md_path}")
    if not os.path.exists(md_path):
        print(f"[Blend Vault][LibraryRelink] Sidecar file not found at: {md_path}. Skipping relink.")
        return
    
    print(f"[Blend Vault][LibraryRelink] Reading sidecar file: {md_path}")
    found_links = False
    with open(md_path, 'r', encoding='utf-8') as f:
        for i, line in enumerate(f):
            line_stripped = line.strip()

            match = table_row_pattern.match(line_stripped)
            if not match:
                # Optional: Add a more specific log if a line starts with | but doesn't match fully,
                # to catch malformed table rows that are not "Linked Library" rows.
                # For now, we only care about "Linked Library" rows.
                if line_stripped.startswith("| Linked Library |"):
                     print(f"[Blend Vault][LibraryRelink] Malformed 'Linked Library' table row (did not match regex): {line_stripped}")
                continue
            
            stored_path = match.group(1)  # Content of [[PATH_TO_BLEND]]
            # sidecar_path_in_link = match.group(2) # Content before \| in the third cell, e.g., ../../folder2/asset cube2.blend.side
            stored_hash = match.group(3)   # Content after \| in the third cell, e.g., HASH_VALUE

            if not stored_path or not stored_hash: # Should not happen if regex matches and groups are defined
                print(f"[Blend Vault][LibraryRelink] Empty path or hash extracted from row (regex issue?): {line_stripped}")
                continue
            
            print(f"[Blend Vault][LibraryRelink] Found link in table: Path='{stored_path}', Hash='{stored_hash}'")
            found_links = True
            
            # stored_path is relative to the blend file, as written by sidecar_writer.py
            # It should already use forward slashes.
            # Blender relative paths start with '//'
            rel_path = '//' + stored_path

            found_matching_lib = False
            for lib in bpy.data.libraries:
                lib_hash = lib.get('blend_vault_hash')
                if lib_hash == stored_hash:
                    found_matching_lib = True
                    print(f"[Blend Vault][LibraryRelink] Found library '{lib.name}' with matching hash: {lib_hash}")
                    # Normalize paths for comparison: use forward slashes and strip leading '//'
                    lib_path_norm = lib.filepath.replace('\\', '/').lstrip('/')
                    # rel_path is already like '//foo/bar.blend', lstrip makes it 'foo/bar.blend'
                    rel_path_norm = rel_path.lstrip('/') 
                    if lib_path_norm != rel_path_norm:
                        print(f"{BLUE}[Blend Vault] Relinked '{lib.name}' from '{lib.filepath}' -> '{rel_path}'{RESET}")
                        lib.filepath = rel_path
                        try:
                            lib.reload()
                        except Exception as e:
                            print(f"[Blend Vault][LibraryRelink] Failed to reload '{lib.name}': {e}")
                    else:
                        print(f"[Blend Vault][LibraryRelink] Path for '{lib.name}' ('{lib.filepath}') already matches stored relative path ('{rel_path}').")
                    break  # Found library by hash, no need to check other libraries for this sidecar entry
            if not found_matching_lib:
                print(f"[Blend Vault][LibraryRelink] Missing library with hash {stored_hash} not found; attempting to relink using sidecar path.")
                working_dir = os.path.dirname(bpy.data.filepath)
                candidate_path = os.path.normpath(os.path.join(working_dir, stored_path))
                # Try to find a library entry with the old (now missing) path and update it
                relinked = False
                for lib in bpy.data.libraries:
                    # If the library is missing (Blender can't find the file), its path will match the old path in the blend
                    if not os.path.exists(bpy.path.abspath(lib.filepath)):
                        print(f"{BLUE}[Blend Vault][LibraryRelink] Updating missing library path from '{lib.filepath}' to '{candidate_path}'{RESET}")
                        lib.filepath = candidate_path
                        try:
                            lib.reload()
                            print(f"{BLUE}[Blend Vault][LibraryRelink] Reloaded relinked library: {candidate_path}{RESET}")
                        except Exception as e:
                            print(f"[Blend Vault][LibraryRelink] Failed to reload relinked library at {candidate_path}: {e}")
                        relinked = True
                        break
                if not relinked:
                    if os.path.exists(candidate_path):
                        try:
                            bpy.data.libraries.load(candidate_path, link=True)
                            print(f"{BLUE}[Blend Vault][LibraryRelink] Linked missing library at: {candidate_path}{RESET}")
                            # Reload the newly linked library
                            for lib in bpy.data.libraries:
                                if os.path.normcase(os.path.abspath(bpy.path.abspath(lib.filepath))) == os.path.normcase(os.path.abspath(candidate_path)):
                                    try:
                                        lib.reload()
                                        print(f"{BLUE}[Blend Vault][LibraryRelink] Reloaded linked library: {candidate_path}{RESET}")
                                    except Exception as e:
                                        print(f"[Blend Vault][LibraryRelink] Failed to reload linked library at {candidate_path}: {e}")
                                    break
                        except Exception as e:
                            print(f"[Blend Vault][LibraryRelink] Failed to link missing library at {candidate_path}: {e}")
                    else:
                        print(f"[Blend Vault][LibraryRelink] Sidecar path not found: {candidate_path}")
    if not found_links:
        print(f"[Blend Vault][LibraryRelink] No valid links found in sidecar file: {md_path}")
    print("[Blend Vault][LibraryRelink] Finished relink attempt.")

relink_library_info.persistent = True
