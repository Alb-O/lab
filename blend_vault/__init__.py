# Conditional bpy import - only import if we're running in Blender
try:
	import bpy
	_BLENDER_AVAILABLE = True
except ImportError:
	bpy = None
	_BLENDER_AVAILABLE = False

import importlib

# Import from consolidated core module
from .core import (
	log_info, log_warning, log_error, log_success, log_debug,
	get_or_create_datablock_uuid, generate_filepath_hash,
	format_primary_link, parse_primary_link, get_asset_sources_map,
	get_resource_warning_prefix, ensure_saved_file,
	LOG_COLORS, BV_UUID_PROP, BV_FILE_UUID_KEY, BV_UUID_KEY,
	SIDECAR_EXTENSION, RESOURCE_WARNING_PREFIX,
	MD_PRIMARY_FORMAT, PRIMARY_LINK_REGEX
)

# Import remaining constants from utils
from .utils.constants import (
	REDIRECT_EXTENSION, FRONTMATTER_TAGS, POLL_INTERVAL,
	MD_EMBED_WIKILINK
)

# Only import preferences and its functions if we're in Blender
if _BLENDER_AVAILABLE:
	from . import preferences
	from .preferences import get_obsidian_vault_root, get_addon_preferences
else:
	preferences = None
	get_obsidian_vault_root = None
	get_addon_preferences = None

# Re-export commonly used items for easier importing
__all__ = [
	# Logging functions (most commonly used)
	'log_info', 'log_warning', 'log_error', 'log_success', 'log_debug',
	
	# Constants
	'LOG_COLORS', 'BV_UUID_PROP', 'BV_FILE_UUID_KEY', 'BV_UUID_KEY',
	'SIDECAR_EXTENSION', 'REDIRECT_EXTENSION', 'FRONTMATTER_TAGS', 'POLL_INTERVAL',
	'MD_PRIMARY_FORMAT', 'PRIMARY_LINK_REGEX', 'RESOURCE_WARNING_PREFIX',
	'MD_EMBED_WIKILINK',
	
	# Helper functions
	'get_or_create_datablock_uuid', 'generate_filepath_hash',
	'format_primary_link', 'parse_primary_link', 'get_asset_sources_map',
	'get_resource_warning_prefix', 'ensure_saved_file',
]

# Add Blender-specific exports if available
if _BLENDER_AVAILABLE:
	__all__.extend([
		# Core modules
		'preferences',
		# Preferences functions
		'get_obsidian_vault_root', 'get_addon_preferences',
	])

