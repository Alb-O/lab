"""
Obsidian Integration Module

This module provides integration with Obsidian through URI protocol for
managing sidecar files and other Obsidian-related functionality.
"""

from . import uri_handler

def register():
    uri_handler.register()

def unregister():
    uri_handler.unregister()
