"""
Obsidian Integration Module

This module provides integration with Obsidian through URI protocol for
managing sidecar files and other Obsidian-related functionality.
"""

from . import uri_handler


def register():
	"""Register Obsidian integration components."""
	if hasattr(uri_handler, 'register'):
		uri_handler.register()


def unregister():
	"""Unregister Obsidian integration components."""
	if hasattr(uri_handler, 'unregister'):
		uri_handler.unregister()
