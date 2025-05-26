"""
Refactored Blend Vault relink module initialization.
This demonstrates how to use the consolidated and refactored relinker architecture.
"""

# Import refactored modules
from . import asset_relinker
from . import library_relinker  
from . import resource_relinker
from . import polling
from . import redirect_handler
from . import shared_utils

import sys
import os

# Import utils from parent directory
sys.path.append(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import utils


def register():
    """Register all relinker modules using the refactored architecture."""
    
    # Register individual modules
    asset_relinker.register()
    library_relinker.register() 
    resource_relinker.register()
    polling.register()
    # redirect_handler.register() is called from polling.register()
    
    shared_utils.log_success("[Blend Vault] Refactored relink module registered.")


def unregister():
    """Unregister all relinker modules in reverse order."""
    import bpy  # type: ignore
    
    # Remove handlers if they exist
    if hasattr(asset_relinker, 'relink_renamed_assets'):
        if asset_relinker.relink_renamed_assets in bpy.app.handlers.load_post:
            bpy.app.handlers.load_post.remove(asset_relinker.relink_renamed_assets)
    
    if hasattr(library_relinker, 'relink_library_info'):
        if library_relinker.relink_library_info in bpy.app.handlers.load_post:
            bpy.app.handlers.load_post.remove(library_relinker.relink_library_info)
    
    if hasattr(resource_relinker, 'relink_resources'):
        if resource_relinker.relink_resources in bpy.app.handlers.load_post:
            bpy.app.handlers.load_post.remove(resource_relinker.relink_resources)
    
    # Unregister modules
    resource_relinker.unregister()
    asset_relinker.unregister()
    polling.unregister()  # This will also unregister redirect_handler
    library_relinker.unregister()
    
    shared_utils.log_warning("[Blend Vault] Refactored relink module unregistered.")


# Expose key classes and utilities for external use
__all__ = [
    'asset_relinker',
    'library_relinker', 
    'resource_relinker',
    'shared_utils',
    'polling',
    'redirect_handler'
]

# Convenience access to shared utilities
SidecarParser = shared_utils.SidecarParser
PathResolver = shared_utils.PathResolver
LibraryManager = shared_utils.LibraryManager
ResourceManager = shared_utils.ResourceManager
BaseRelinker = shared_utils.BaseRelinker

# Convenience access to logging functions
log_info = shared_utils.log_info
log_warning = shared_utils.log_warning
log_error = shared_utils.log_error
log_success = shared_utils.log_success
log_debug = shared_utils.log_debug
