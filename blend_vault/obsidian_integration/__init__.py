"""
Obsidian Integration Module

This module provides integration with Obsidian through URI protocol for
managing sidecar files and other Obsidian-related functionality.
"""

import importlib
from . import uri_handler

def register():
    # Reload modules to ensure we get the latest versions during dynamic reloading
    importlib.reload(uri_handler)
    
    uri_handler.register()

def unregister():
    uri_handler.unregister()
