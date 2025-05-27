"""
Paste-Path Integration Module

Provides smart clipboard integration for .blend files, allowing users to paste
file paths and choose actions (Open, Link, Append) with automatic asset discovery.
"""

import bpy  # type: ignore
from . import core_operators
from . import asset_discovery
from . import dialogs
from . import smart_paste
from . import save_workflow


def register():
    """Register all paste-path components."""
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
