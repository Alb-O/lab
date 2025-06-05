"""
Blend Vault relink module initialization.
Provides automatic relinking functionality for assets, libraries, and resources.
"""

from . import asset_relinker
from . import library_relinker  
from . import resource_relinker
from . import polling
from . import redirect_handler
from . import shared_utils
from ..core import log_info, log_warning, log_success


def register():
    """Register all relinker modules."""
    modules = [asset_relinker, library_relinker, resource_relinker, polling]
    for module in modules:
        if hasattr(module, 'register'):
            module.register()
    
    log_success("Relink module registered", module_name='Relink')


def unregister():
    """Unregister all relinker modules."""
    modules = [polling, resource_relinker, library_relinker, asset_relinker]
    for module in modules:
        if hasattr(module, 'unregister'):
            module.unregister()
    
    log_warning("Relink module unregistered", module_name='Relink')


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
