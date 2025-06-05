"""
Paste-Path Integration Module

Provides smart clipboard integration for .blend files, allowing users to paste
file paths and choose actions (Open, Link, Append) with automatic asset discovery.
"""

from . import core_operators
from . import asset_discovery
from . import dialogs
from . import save_workflow
from . import smart_paste


def register():
    """Register all paste-path components."""
    modules = [core_operators, asset_discovery, dialogs, save_workflow, smart_paste]
    for module in modules:
        if hasattr(module, 'register'):
            module.register()


def unregister():
    """Unregister all paste-path components."""
    modules = [smart_paste, save_workflow, dialogs, asset_discovery, core_operators]
    for module in modules:
        if hasattr(module, 'unregister'):
            module.unregister()
