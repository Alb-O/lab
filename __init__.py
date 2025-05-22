bl_info = {
    "name": "Blend Vault",
    "author": "Albert O'Shea",
    "version": (0, 2, 0),
    "blender": (4, 0, 0),
    "location": "File Save",
    "description": "Writes linked library info (path and session UID) to a markdown file on save",
    "category": "Development",
}

import sys
import os
# Ensure the addon root directory is in sys.path for package imports
addon_dir = os.path.dirname(os.path.abspath(__file__))
if addon_dir not in sys.path:
    sys.path.append(addon_dir)

import bpy  # type: ignore
from src.utils.config import LOG_SUCCESS, LOG_WARN, LOG_RESET
from src.io.sidecar_writer import write_library_info
from src.relinking.core import relink_library_info
from src.relinking.polling import start_sidecar_poll_timer, sidecar_poll_timer


def register():
    if write_library_info not in bpy.app.handlers.save_post:
        bpy.app.handlers.save_post.append(write_library_info)
    if relink_library_info not in bpy.app.handlers.load_post:
        bpy.app.handlers.load_post.append(relink_library_info)
    if start_sidecar_poll_timer not in bpy.app.handlers.load_post:
        bpy.app.handlers.load_post.append(start_sidecar_poll_timer)
    print(f"{LOG_SUCCESS}[Blend Vault] Main addon functionalities registered.{LOG_RESET}")


def unregister():
    if write_library_info in bpy.app.handlers.save_post:
        bpy.app.handlers.save_post.remove(write_library_info)
    if relink_library_info in bpy.app.handlers.load_post:
        bpy.app.handlers.load_post.remove(relink_library_info)
    if start_sidecar_poll_timer in bpy.app.handlers.load_post:
        bpy.app.handlers.load_post.remove(start_sidecar_poll_timer)
    if bpy.app.timers.is_registered(sidecar_poll_timer):
        bpy.app.timers.unregister(sidecar_poll_timer)
        print(f"{LOG_WARN}[Blend Vault] Sidecar polling timer unregistered.{LOG_RESET}")
    print(f"{LOG_WARN}[Blend Vault] Main addon functionalities unregistered.{LOG_RESET}")


if __name__ == "__main__":
    register()

print(f"{LOG_SUCCESS}[Blend Vault] Script loaded.{LOG_RESET}")