import bpy
from .writer import write_library_info

def register():
    """Register sidecar writing components."""
    # Register the handler if not already present
    if write_library_info not in bpy.app.handlers.save_post:
        bpy.app.handlers.save_post.append(write_library_info)

def unregister():
    """Unregister sidecar writing components."""
    # Remove handler if present
    if write_library_info in bpy.app.handlers.save_post:
        bpy.app.handlers.save_post.remove(write_library_info)