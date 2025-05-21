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
from .src.config import GREEN, BLUE, RESET
from .src.hashing import ensure_library_hash, ensure_blendfile_hash
from .src.sidecar_writer import write_library_info
from .src.relink import relink_library_info


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