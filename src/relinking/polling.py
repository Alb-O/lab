import bpy # type: ignore
import os
from ..utils.config import SIDECAR_EXTENSION, POLL_INTERVAL, LOG_INFO, LOG_ERROR, LOG_SUCCESS, LOG_WARN, LOG_RESET
from .core import relink_library_info

# Store last modification times for sidecar files
t_last_sidecar_mtimes = {}
# Store last modification times for library files themselves
t_last_library_mtimes = {}

def sidecar_poll_timer():
    """Timer callback to poll sidecar file changes and trigger relink if modified,
    and also polls library files for modifications."""
    blend_path = bpy.data.filepath
    if not blend_path: # Current .blend file is not saved or no file is open
        return POLL_INTERVAL

    # --- Part 1: Check sidecar file for modifications (triggers full relink if changed) ---
    md_path = blend_path + SIDECAR_EXTENSION
    try:
        if os.path.exists(md_path):
            sidecar_mtime = os.path.getmtime(md_path)
            last_known_sidecar_mtime = t_last_sidecar_mtimes.get(md_path)
            if last_known_sidecar_mtime is None:
                # Initialize
                t_last_sidecar_mtimes[md_path] = sidecar_mtime
            elif sidecar_mtime > last_known_sidecar_mtime:
                # Sidecar file changed: update timestamp and trigger full relink logic
                t_last_sidecar_mtimes[md_path] = sidecar_mtime
                print(f"{LOG_SUCCESS}[Blend Vault] Sidecar file '{md_path}' modified. Triggering relink_library_info().{LOG_RESET}")
                relink_library_info()
    except Exception as e:
        print(f"{LOG_ERROR}[Blend Vault][sidecar_poll_timer] Error checking sidecar file '{md_path}': {e}{LOG_RESET}")

    # --- Part 2: Check individual library files for modifications ---
    for lib in bpy.data.libraries:
        if not lib.filepath or lib.filepath.startswith("<builtin>"):
            continue # Skip libraries with no path or built-in ones

        try:
            lib_abs_path = bpy.path.abspath(lib.filepath)
            if not os.path.exists(lib_abs_path):
                if lib_abs_path in t_last_library_mtimes:
                    del t_last_library_mtimes[lib_abs_path]
                continue

            current_lib_mtime = os.path.getmtime(lib_abs_path)
            last_known_lib_mtime = t_last_library_mtimes.get(lib_abs_path)

            if last_known_lib_mtime is None:
                t_last_library_mtimes[lib_abs_path] = current_lib_mtime
            elif current_lib_mtime > last_known_lib_mtime:
                t_last_library_mtimes[lib_abs_path] = current_lib_mtime
                print(f"{LOG_WARN}[Blend Vault] Library file '{lib.name}' ('{lib_abs_path}') modified. Triggering reload.{LOG_RESET}")
                try:
                    lib.reload()
                    print(f"{LOG_SUCCESS}[Blend Vault] Successfully reloaded library '{lib.name}'.{LOG_RESET}")
                except Exception as reload_e:
                    print(f"{LOG_ERROR}[Blend Vault][sidecar_poll_timer] Error reloading library '{lib.name}': {reload_e}{LOG_RESET}")
        except Exception as e:
            print(f"{LOG_ERROR}[Blend Vault][sidecar_poll_timer] Error checking library '{lib.name}' ('{lib.filepath}'): {e}{LOG_RESET}")

    return POLL_INTERVAL

@bpy.app.handlers.persistent
def start_sidecar_poll_timer(*args, **kwargs):
    """Handler to register polling timer after file load, ensuring persistence across blend reloads."""
    is_registered = False
    if bpy.app.timers.is_registered(sidecar_poll_timer):
        is_registered = True
        print(f"{LOG_INFO}[Blend Vault] Sidecar polling timer already registered.{LOG_RESET}")

    if not is_registered:
        try:
            bpy.app.timers.register(sidecar_poll_timer, first_interval=POLL_INTERVAL)
            print(f"{LOG_SUCCESS}[Blend Vault] Sidecar polling timer registered (interval: {POLL_INTERVAL}s).{LOG_RESET}")
        except Exception as e: 
            print(f"{LOG_ERROR}[Blend Vault][Error] Failed to register sidecar polling timer: {e}{LOG_RESET}")


def register():
    bpy.app.handlers.load_post.append(start_sidecar_poll_timer)
    print(f"{LOG_SUCCESS}[Blend Vault] Polling module registered.{LOG_RESET}")

def unregister():
    if start_sidecar_poll_timer in bpy.app.handlers.load_post:
        bpy.app.handlers.load_post.remove(start_sidecar_poll_timer)
    if bpy.app.timers.is_registered(sidecar_poll_timer):
        bpy.app.timers.unregister(sidecar_poll_timer)
        print(f"{LOG_WARN}[Blend Vault] Sidecar polling timer unregistered.{LOG_RESET}")
    print(f"{LOG_WARN}[Blend Vault] Polling module unregistered.{LOG_RESET}")

