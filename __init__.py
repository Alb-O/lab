bl_info = {
    "name": "Blend Vault",
    "author": "Albert O'Shea",
    "version": (0, 1, 0),
    "blender": (4, 0, 0),
    "location": "File Save",
    "description": "Writes linked library info (path and session UID) to a markdown file on save",
    "category": "Development",
}

import bpy  # type: ignore
import os
import re
import uuid
import time  # Add time import
from datetime import datetime, timezone
# Use Blender's persistent decorator directly from bpy.app.handlers


def ensure_library_hash(lib):
    """Ensure a unique hash is stored in the library's custom properties."""
    # Use the top-level ID property group for the library datablock
    if not hasattr(lib, 'id_properties_ensure'):  # Defensive: Blender 2.80+
        print(f"[Blend Vault][LibraryHash] Library '{lib.name}' does not support id_properties_ensure.")
        return None
    props = lib.id_properties_ensure()
    if 'blend_vault_hash' in props:
        print(f"[Blend Vault][LibraryHash] Existing hash for '{lib.name}': {props['blend_vault_hash']}")
        return props['blend_vault_hash']
    # Generate a new UUID4 string
    new_hash = str(uuid.uuid4())
    props['blend_vault_hash'] = new_hash
    print(f"[Blend Vault][LibraryHash] Generated new hash for '{lib.name}': {new_hash}")
    return new_hash


def ensure_blendfile_hash():
    """Ensure a unique hash stored in a hidden text data-block in the blend file."""
    txt_name = "blend_vault_hash"
    # Check for existing text block
    if txt_name in bpy.data.texts:
        txt = bpy.data.texts[txt_name]
        content = txt.as_string().strip()
        print(f"[Blend Vault][BlendfileHash] Existing hash in text block '{txt_name}': {content}")
        return content
    # Create a new text block and write the hash
    new_hash = str(uuid.uuid4())
    txt = bpy.data.texts.new(txt_name)
    txt.clear()
    txt.write(new_hash)
    # Optionally hide this text in UI
    txt.use_fake_user = True
    print(f"[Blend Vault][BlendfileHash] Generated new hash in text block '{txt_name}': {new_hash}")
    return new_hash


# Add color codes for log output
GREEN = "\033[92m"
BLUE = "\033[94m"
RESET = "\033[0m"


@bpy.app.handlers.persistent
def write_library_info(*args, **kwargs):  # Decorated as persistent handler
    print(f"{GREEN}[Blend Vault] Writing sidecar for: {bpy.data.filepath}{RESET}")
    blend_path = bpy.data.filepath
    if not blend_path:
        print("[Blend Vault][LibraryInfo] No blend file path found, skipping write.")
        return

    # Add a short delay
    time.sleep(1)  # Delay for 1 second

    md_path = blend_path + ".sidecar.md"  # Updated filename format
    # Always ensure the hash is written to the current blend file
    blendfile_hash = ensure_blendfile_hash()
    libraries = list(bpy.data.libraries)
    # Always create/update the sidecar file, even if there are no libraries
    print(f"{GREEN}[Blend Vault] Sidecar path: {md_path}{RESET}")
    with open(md_path, 'w', encoding='utf-8') as f:
        # Get current UTC time and format it
        current_time_utc = datetime.now(timezone.utc)
        formatted_time = current_time_utc.strftime('%Y-%m-%dT%H:%M:%S.%f')[:-3] + 'Z'

        # Write YAML frontmatter
        f.write('---\n')
        f.write(f'MC-last-updated: {formatted_time}\n')
        f.write('---\n\n')

        # Write header and blendfile hash
        f.write('# Linked Libraries\n\n')
        f.write(f'**Blendfile Hash:** {blendfile_hash}\n\n')
        if not libraries:
            print(f"[Blend Vault][LibraryInfo] No libraries to link in: {blend_path}")
            # No return here, an empty sidecar (with header) will be written
        else:
            # List each linked library with path and hash
            for lib in libraries:
                path = lib.filepath
                # Remove leading '//' from path if present
                if path.startswith('//'):
                    path = path[2:]
                # Normalize to forward slashes
                path = path.replace('\\', '/')
                # Resolve to absolute path using Blender's abspath
                abs_path = bpy.path.abspath(lib.filepath)
                # Log attempt to load the library blend file
                print(f"[Blend Vault][LibraryInfo] Attempting to read OBSIDIAN blendfile hash from library: {abs_path}")
                lib_hash = lib.get('blend_vault_hash')
                file_hash = None
                try:
                    # Track existing text datablocks before loading
                    existing_texts = set(bpy.data.texts.keys())
                    # Load the library .blend to fetch the blend_vault_hash text
                    with bpy.data.libraries.load(abs_path, link=False) as (data_src, data_dst):
                        if 'blend_vault_hash' in data_src.texts:
                            data_dst.texts = ['blend_vault_hash']
                    # Identify newly loaded text datablock
                    new_texts = set(bpy.data.texts.keys()) - existing_texts
                    for tname in new_texts:
                        txt = bpy.data.texts.get(tname)
                        if txt:
                            file_hash = txt.as_string().strip()
                            print(f"[Blend Vault][LibraryInfo] Read blend_vault_hash '{file_hash}' from library: {abs_path}")
                            bpy.data.texts.remove(txt)
                        break
                except Exception as e:
                    print(f"[Blend Vault][LibraryInfo] Failed to load blendfile hash from '{abs_path}': {e}")
                else:
                    if not file_hash:
                        print(f"[Blend Vault][LibraryInfo] No blend_vault_hash found in library: {abs_path}")
                # Use hash from blendfile if available, else property, else placeholder
                if file_hash:
                    if lib_hash != file_hash:
                        props = lib.id_properties_ensure()
                        props['blend_vault_hash'] = file_hash
                        lib_hash = file_hash
                # If neither hash is available, use a placeholder
                if not lib_hash:
                    lib_hash = 'MISSING_HASH'
                filename = os.path.basename(path)
                # Write a bullet link: [[path#hash|filename]]
                f.write(f'- [[{path}#{lib_hash}|{filename}]]\n')

write_library_info.persistent = True


@bpy.app.handlers.persistent
def relink_library_info(*args, **kwargs):  # Decorated as persistent handler
    """Read sidecar Markdown and relink libraries based on persistent hash."""
    print(f"{GREEN}[Blend Vault] Relinking libraries for: {bpy.data.filepath}{RESET}")
    blend_path = bpy.data.filepath
    if not blend_path:
        print("[Blend Vault][LibraryRelink] No blend file path found, skipping relink.")
        return
    md_path = blend_path + ".sidecar.md"  # Updated filename format
    print(f"[Blend Vault][LibraryRelink] Looking for sidecar file at: {md_path}")
    if not os.path.exists(md_path):
        print(f"[Blend Vault][LibraryRelink] Sidecar file not found at: {md_path}. Skipping relink.")
        return
    
    print(f"[Blend Vault][LibraryRelink] Reading sidecar file: {md_path}")
    # Fix regex: use raw string and single backslashes
    pattern = re.compile(r'\[\[(.+?)#(.+?)\|.*?\]\]')
    found_links = False
    with open(md_path, 'r', encoding='utf-8') as f:
        for i, line in enumerate(f):
            # Strip whitespace and optional bullet prefix '-' before parsing
            line_stripped = line.strip()
            # Handle bullet list prefix
            if line_stripped.startswith('- '):
                line_stripped = line_stripped[2:].strip()
            match = pattern.match(line_stripped)
            if not match:
                continue
            
            found_links = True
            stored_path, stored_hash = match.groups()
            stored_path = stored_path.replace('\\\\', '/')
            rel_path = '//' + stored_path if not os.path.isabs(stored_path) else stored_path
            
            found_matching_lib = False
            for lib in bpy.data.libraries:
                lib_hash = lib.get('blend_vault_hash')
                if lib_hash == stored_hash:
                    found_matching_lib = True
                    print(f"[Blend Vault][LibraryRelink] Found library '{lib.name}' with matching hash: {lib_hash}")
                    lib_path_norm = lib.filepath.replace('\\\\', '/').lstrip('/')
                    rel_path_norm = rel_path.replace('\\\\', '/').lstrip('/')
                    if lib_path_norm != rel_path_norm:
                        print(f"{BLUE}[Blend Vault] Relinked '{lib.name}' -> {rel_path}{RESET}")
                        lib.filepath = rel_path
                        try:
                            lib.reload()
                        except Exception as e:
                            print(f"[Blend Vault][LibraryRelink] Failed to reload '{lib.name}': {e}")
                    else:
                        print(f"[Blend Vault][LibraryRelink] Path for '{lib.name}' already matches: {lib_path_norm}")
                    break  # No need to check other libraries with the same hash
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


def register():
    # Attach handlers if not already present
    if write_library_info not in bpy.app.handlers.save_post:
        bpy.app.handlers.save_post.append(write_library_info)
    if relink_library_info not in bpy.app.handlers.load_post:
        bpy.app.handlers.load_post.append(relink_library_info)
    print(f"{GREEN}[Blend Vault] Addon registered and handlers attached.{RESET}")


def unregister():
    # Remove handlers
    if write_library_info in bpy.app.handlers.save_post:
        bpy.app.handlers.save_post.remove(write_library_info)
    if relink_library_info in bpy.app.handlers.load_post:
        bpy.app.handlers.load_post.remove(relink_library_info)
    print(f"{BLUE}[Blend Vault] Addon unregistered and handlers detached.{RESET}")


if __name__ == "__main__":
    register()

print(f"{GREEN}[Blend Vault] Script loaded.{RESET}")