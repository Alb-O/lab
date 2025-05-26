"""
Obsidian Integration Module

This module provides integration with Obsidian through URI protocol for
managing sidecar files and other Obsidian-related functionality.
"""

import importlib
from . import uri_handler
from . import open_blend_clipboard

def register():
    # Reload modules to ensure we get the latest versions during dynamic reloading
    importlib.reload(uri_handler)
    importlib.reload(open_blend_clipboard)
    
    uri_handler.register()
    open_blend_clipboard.register()

def unregister():
    uri_handler.unregister()
    open_blend_clipboard.unregister()
