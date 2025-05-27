"""
Paste-Path Integration Module

Provides smart clipboard integration for .blend files, allowing users to paste
file paths and choose actions (Open, Link, Append) with automatic asset discovery.
"""

import bpy  # type: ignore
import importlib
from . import core_operators
from . import asset_discovery
from . import dialogs
from . import file_validation
from . import smart_paste
from . import save_workflow


def register():
    """Register all paste-path components."""
    # Reload submodules to ensure latest code is used
    importlib.reload(core_operators)
    importlib.reload(asset_discovery)
    importlib.reload(dialogs)
    importlib.reload(file_validation)
    importlib.reload(smart_paste)
    importlib.reload(save_workflow)
    
    # Register components (only those that have register functions)
    core_operators.register()
    asset_discovery.register()
    dialogs.register()
    save_workflow.register()
    smart_paste.register()


def unregister():
    """Unregister all paste-path components."""
    smart_paste.unregister()
    save_workflow.unregister()
    dialogs.unregister()
    asset_discovery.unregister()
    core_operators.unregister()
