"""
Obsidian Integration Module

This module provides integration with Obsidian through URI protocol for
managing sidecar files and other Obsidian-related functionality.
"""

from . import uri_handler
from . import uri_panel
from . import preview_image
from . import preview_panel
from . import main_panel


def register():
	"""Register Obsidian integration components."""
	if hasattr(uri_handler, 'register'):
		uri_handler.register()
	if hasattr(preview_image, 'register'):
		preview_image.register()
	if hasattr(preview_panel, 'register'):
		preview_panel.register()
	if hasattr(main_panel, 'register'):
		main_panel.register()


def unregister():
	"""Unregister Obsidian integration components."""
	if hasattr(main_panel, 'unregister'):
		main_panel.unregister()
	if hasattr(preview_panel, 'unregister'):
		preview_panel.unregister()
	if hasattr(preview_image, 'unregister'):
		preview_image.unregister()
	if hasattr(uri_handler, 'unregister'):
		uri_handler.unregister()
