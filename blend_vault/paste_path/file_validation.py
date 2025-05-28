"""
File validation utilities for clipboard content.
Provides functions to validate .blend file paths and sidecar files.
"""

import os
from .. import SIDECAR_EXTENSION


def is_valid_blend_file_path(clipboard_text):
    """Check if clipboard contains a valid .blend file path."""
    try:
        path = clipboard_text.strip()
        if path.startswith('"') and path.endswith('"'):
            path = path[1:-1]
        elif path.startswith("'") and path.endswith("'"):
            path = path[1:-1]
        
        return path and os.path.isfile(path) and path.lower().endswith('.blend')
    except:
        return False


def is_valid_blend_or_sidecar_path(clipboard_text):
    """Check if clipboard contains a valid .blend or sidecar file path."""
    try:
        path = clipboard_text.strip()
        if path.startswith('"') and path.endswith('"'):
            path = path[1:-1]
        elif path.startswith("'") and path.endswith("'"):
            path = path[1:-1]
        
        if not path:
            return False
        if os.path.isfile(path):
            if path.lower().endswith('.blend'):
                return True
            if path.lower().endswith(SIDECAR_EXTENSION):
                # Check if the corresponding .blend file exists next to it
                blend_path = path[: -len(SIDECAR_EXTENSION)]
                if os.path.isfile(blend_path) and blend_path.lower().endswith('.blend'):
                    return True
        return False
    except:
        return False


def normalize_path_from_clipboard(clipboard_text):
    """Normalize a file path from clipboard text, removing quotes and handling sidecar files."""
    path = clipboard_text.strip()
    if path.startswith('"') and path.endswith('"'):
        path = path[1:-1]
    elif path.startswith("'") and path.endswith("'"):
        path = path[1:-1]
    
    # If it's a sidecar file, convert to the corresponding .blend file
    if path.lower().endswith(SIDECAR_EXTENSION):
        path = path[: -len(SIDECAR_EXTENSION)]
    
    return path
