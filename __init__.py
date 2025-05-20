bl_info = {
    "name": "Obsidian Library",
    "author": "GitHub Copilot",
    "version": (1, 0, 0),
    "blender": (2, 80, 0),
    "location": "File Save",
    "description": "Writes linked library info (path and session UID) to a markdown file on save",
    "category": "Development",
}

import bpy  # type: ignore
import os
import re
import uuid
# Use Blender's persistent decorator directly from bpy.app.handlers


def ensure_library_hash(lib):
    """Ensure a unique hash is stored in the library's custom properties."""
    # Use the top-level ID property group for the library datablock
    if not hasattr(lib, 'id_properties_ensure'):  # Defensive: Blender 2.80+
        print(f"[Obsidian Library][LibraryHash] Library '{lib.name}' does not support id_properties_ensure.")
        return None
    props = lib.id_properties_ensure()
    if 'obsidian_library_hash' in props:
        print(f"[Obsidian Library][LibraryHash] Existing hash for '{lib.name}': {props['obsidian_library_hash']}")
        return props['obsidian_library_hash']
    # Generate a new UUID4 string
    new_hash = str(uuid.uuid4())
    props['obsidian_library_hash'] = new_hash
    print(f"[Obsidian Library][LibraryHash] Generated new hash for '{lib.name}': {new_hash}")
    return new_hash


def ensure_blendfile_hash():
    """Ensure a unique hash stored in a hidden text data-block in the blend file."""
    txt_name = "obsidian_blendfile_hash"
    # Check for existing text block
    if txt_name in bpy.data.texts:
        txt = bpy.data.texts[txt_name]
        content = txt.as_string().strip()
        print(f"[Obsidian Library][BlendfileHash] Existing hash in text block '{txt_name}': {content}")
        return content
    # Create a new text block and write the hash
    new_hash = str(uuid.uuid4())
    txt = bpy.data.texts.new(txt_name)
    txt.clear()
    txt.write(new_hash)
    # Optionally hide this text in UI
    txt.use_fake_user = True
    print(f"[Obsidian Library][BlendfileHash] Generated new hash in text block '{txt_name}': {new_hash}")
    return new_hash


# Add color codes for log output
GREEN = "\033[92m"
BLUE = "\033[94m"
RESET = "\033[0m"


@bpy.app.handlers.persistent
def write_library_info(*args, **kwargs):  # Decorated as persistent handler
    print(f"{GREEN}[Obsidian Library] Writing sidecar for: {bpy.data.filepath}{RESET}")
    blend_path = bpy.data.filepath
    if not blend_path:
        print("[Obsidian Library][LibraryInfo] No blend file path found, skipping write.")
        return
    md_path = os.path.splitext(blend_path)[0] + ".md"
    # Always ensure the hash is written to the current blend file
    blendfile_hash = ensure_blendfile_hash()
    libraries = list(bpy.data.libraries)
    if not libraries:
        # No linked libraries: remove sidecar if it exists
        if os.path.exists(md_path):
            try:
                os.remove(md_path)
                print(f"{BLUE}[Obsidian Library] Removed sidecar: {md_path}{RESET}")
            except Exception as e:
                print(f"{BLUE}[Obsidian Library] Error removing sidecar: {e}{RESET}")
        else:
            print(f"[Obsidian Library][LibraryInfo] No libraries and no sidecar file to remove at: {md_path}")
        return
    print(f"{GREEN}[Obsidian Library] Sidecar path: {md_path}{RESET}")
    # Clear file and write header and all library info
    with open(md_path, 'w', encoding='utf-8') as f:
        # Write header and blendfile hash
        f.write('# Linked Libraries\n\n')
        f.write(f'**Blendfile Hash:** {blendfile_hash}\n\n')
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
            print(f"[Obsidian Library][LibraryInfo] Attempting to read OBSIDIAN blendfile hash from library: {abs_path}")
            lib_hash = lib.get('obsidian_library_hash')
            file_hash = None
            try:
                # Track existing text datablocks before loading
                existing_texts = set(bpy.data.texts.keys())
                # Load the library .blend to fetch the obsidian_blendfile_hash text
                with bpy.data.libraries.load(abs_path, link=False) as (data_src, data_dst):
                    if 'obsidian_blendfile_hash' in data_src.texts:
                        data_dst.texts = ['obsidian_blendfile_hash']
                # Identify newly loaded text datablock
                new_texts = set(bpy.data.texts.keys()) - existing_texts
                for tname in new_texts:
                    txt = bpy.data.texts.get(tname)
                    if txt:
                        file_hash = txt.as_string().strip()
                        print(f"[Obsidian Library][LibraryInfo] Read obsidian_blendfile_hash '{file_hash}' from library: {abs_path}")
                        bpy.data.texts.remove(txt)
                    break
            except Exception as e:
                print(f"[Obsidian Library][LibraryInfo] Failed to load blendfile hash from '{abs_path}': {e}")
            else:
                if not file_hash:
                    print(f"[Obsidian Library][LibraryInfo] No obsidian_blendfile_hash found in library: {abs_path}")
            # Use hash from blendfile if available, else property, else placeholder
            if file_hash:
                if lib_hash != file_hash:
                    props = lib.id_properties_ensure()
                    props['obsidian_library_hash'] = file_hash
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
    print(f"{GREEN}[Obsidian Library] Relinking libraries for: {bpy.data.filepath}{RESET}")
    blend_path = bpy.data.filepath
    if not blend_path:
        print("[Obsidian Library][LibraryRelink] No blend file path found, skipping relink.")
        return
    md_path = os.path.splitext(blend_path)[0] + ".md"
    print(f"[Obsidian Library][LibraryRelink] Looking for sidecar file at: {md_path}")
    if not os.path.exists(md_path):
        print(f"[Obsidian Library][LibraryRelink] Sidecar file not found at: {md_path}. Skipping relink.")
        return
    
    print(f"[Obsidian Library][LibraryRelink] Reading sidecar file: {md_path}")
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
                lib_hash = lib.get('obsidian_library_hash')
                if lib_hash == stored_hash:
                    found_matching_lib = True
                    print(f"[Obsidian Library][LibraryRelink] Found library '{lib.name}' with matching hash: {lib_hash}")
                    lib_path_norm = lib.filepath.replace('\\\\', '/').lstrip('/')
                    rel_path_norm = rel_path.replace('\\\\', '/').lstrip('/')
                    if lib_path_norm != rel_path_norm:
                        print(f"{BLUE}[Obsidian Library] Relinked '{lib.name}' -> {rel_path}{RESET}")
                        lib.filepath = rel_path
                        try:
                            lib.reload()
                        except Exception as e:
                            print(f"[Obsidian Library][LibraryRelink] Failed to reload '{lib.name}': {e}")
                    else:
                        print(f"[Obsidian Library][LibraryRelink] Path for '{lib.name}' already matches: {lib_path_norm}")
                    break  # No need to check other libraries with the same hash
            if not found_matching_lib:
                print(f"[Obsidian Library][LibraryRelink] Missing library with hash {stored_hash} not found; attempting to relink using sidecar path.")
                working_dir = os.path.dirname(bpy.data.filepath)
                candidate_path = os.path.normpath(os.path.join(working_dir, stored_path))
                # Try to find a library entry with the old (now missing) path and update it
                relinked = False
                for lib in bpy.data.libraries:
                    # If the library is missing (Blender can't find the file), its path will match the old path in the blend
                    if not os.path.exists(bpy.path.abspath(lib.filepath)):
                        print(f"{BLUE}[Obsidian Library][LibraryRelink] Updating missing library path from '{lib.filepath}' to '{candidate_path}'{RESET}")
                        lib.filepath = candidate_path
                        try:
                            lib.reload()
                            print(f"{BLUE}[Obsidian Library][LibraryRelink] Reloaded relinked library: {candidate_path}{RESET}")
                        except Exception as e:
                            print(f"[Obsidian Library][LibraryRelink] Failed to reload relinked library at {candidate_path}: {e}")
                        relinked = True
                        break
                if not relinked:
                    if os.path.exists(candidate_path):
                        try:
                            bpy.data.libraries.load(candidate_path, link=True)
                            print(f"{BLUE}[Obsidian Library][LibraryRelink] Linked missing library at: {candidate_path}{RESET}")
                            # Reload the newly linked library
                            for lib in bpy.data.libraries:
                                if os.path.normcase(os.path.abspath(bpy.path.abspath(lib.filepath))) == os.path.normcase(os.path.abspath(candidate_path)):
                                    try:
                                        lib.reload()
                                        print(f"{BLUE}[Obsidian Library][LibraryRelink] Reloaded linked library: {candidate_path}{RESET}")
                                    except Exception as e:
                                        print(f"[Obsidian Library][LibraryRelink] Failed to reload linked library at {candidate_path}: {e}")
                                    break
                        except Exception as e:
                            print(f"[Obsidian Library][LibraryRelink] Failed to link missing library at {candidate_path}: {e}")
                    else:
                        print(f"[Obsidian Library][LibraryRelink] Sidecar path not found: {candidate_path}")
    if not found_links:
        print(f"[Obsidian Library][LibraryRelink] No valid links found in sidecar file: {md_path}")
    print("[Obsidian Library][LibraryRelink] Finished relink attempt.")


def register():
    # Attach handlers if not already present
    if write_library_info not in bpy.app.handlers.save_post:
        bpy.app.handlers.save_post.append(write_library_info)
    if relink_library_info not in bpy.app.handlers.load_post:
        bpy.app.handlers.load_post.append(relink_library_info)
    print(f"{GREEN}[Obsidian Library] Addon registered and handlers attached.{RESET}")


def unregister():
    # Remove handlers
    if write_library_info in bpy.app.handlers.save_post:
        bpy.app.handlers.save_post.remove(write_library_info)
    if relink_library_info in bpy.app.handlers.load_post:
        bpy.app.handlers.load_post.remove(relink_library_info)
    print(f"{BLUE}[Obsidian Library] Addon unregistered and handlers detached.{RESET}")


if __name__ == "__main__":
    register()

print(f"{GREEN}[Obsidian Library] Script loaded.{RESET}")