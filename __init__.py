bl_info = {
    "name": "Blend Vault",
    "author": "Albert O'Shea",
    "version": (0, 1, 1),
    "blender": (4, 0, 0),
    "location": "File Save",
    "description": "Writes linked library info (path and session UID) to a markdown file on save",
    "category": "Development",
}

import bpy  # type: ignore
from .src.config import GREEN, BLUE, RESET
from .src.sidecar_writer import write_library_info
from .src.relink import relink_library_info, sidecar_poll_timer, start_sidecar_poll_timer


def register():
    # Attach handlers if not already present
    if write_library_info not in bpy.app.handlers.save_post:
        bpy.app.handlers.save_post.append(write_library_info)
    if relink_library_info not in bpy.app.handlers.load_post:
        bpy.app.handlers.load_post.append(relink_library_info)
    # Ensure polling timer restarts after each file load
    if start_sidecar_poll_timer not in bpy.app.handlers.load_post:
        bpy.app.handlers.load_post.append(start_sidecar_poll_timer)
    # Start polling timers for sidecar changes
    bpy.app.timers.register(sidecar_poll_timer)
    print(f"{GREEN}[Blend Vault] Addon registered and handlers attached.{RESET}")


def unregister():
    # Remove handlers
    if write_library_info in bpy.app.handlers.save_post:
        bpy.app.handlers.save_post.remove(write_library_info)
    if relink_library_info in bpy.app.handlers.load_post:
        bpy.app.handlers.load_post.remove(relink_library_info)
    # Remove persistent start timer handler
    if start_sidecar_poll_timer in bpy.app.handlers.load_post:
        bpy.app.handlers.load_post.remove(start_sidecar_poll_timer)
    # Stop polling timer
    try:
        bpy.app.timers.unregister(sidecar_poll_timer)
    except Exception:
        pass
    print(f"{BLUE}[Blend Vault] Addon unregistered and handlers detached.{RESET}")


if __name__ == "__main__":
    register()

print(f"{GREEN}[Blend Vault] Script loaded.{RESET}")