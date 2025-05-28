import bpy  # type: ignore
import uuid
import hashlib
import os
import re
from typing import Optional  # required for format_primary_link annotation

def get_asset_sources_map():
	"""
	Initializes and returns the ASSET_SOURCES_MAP dictionary.
	This version does not cache the instance, ensuring fresh bpy.data references on each call.
	"""
	# Always create and return a new dictionary with current bpy.data references
	return {
		"Collection": bpy.data.collections,
		"Object": bpy.data.objects,
		"World": bpy.data.worlds,
		"Material": bpy.data.materials,
		"Brush": bpy.data.brushes,
		"Action": bpy.data.actions,
		"NodeTree": bpy.data.node_groups,
		"Scene": bpy.data.scenes,
	}

# Log color codes (ANSI escape sequences)
LOG_COLORS = {
	'INFO': '\033[94m',    # Blue: Informational messages
	'SUCCESS': '\033[92m', # Green: Success/confirmation
	'WARN': '\033[93m',    # Yellow: Warnings
	'ERROR': '\033[91m',   # Red: Errors
	'DEBUG': '\033[95m',   # Magenta: Debug messages (added)
	'RESET': '\033[0m',    # Reset to default
}

# Markdown link formats and regex patterns
MD_LINK_FORMATS = {
	'MD_ANGLE_BRACKETS': {
		'format': '[{name}](<{path}>)',
		'regex': r'\[([^\]]+)\]\(<([^>]+)>\)'
	},	'MD_WIKILINK': {
		'format': '[[{path}|{name}]]',
		'regex': r'\[\[([^\]|]+)\|([^\]]+)\]\]'
	}
}

MD_PRIMARY_FORMAT = MD_LINK_FORMATS['MD_WIKILINK']  # Set Obsidian wikilink as primary format

# Compile primary link regex and helper functions
PRIMARY_LINK_REGEX = re.compile(MD_PRIMARY_FORMAT['regex'])
def format_primary_link(path: str, name: Optional[str] = None) -> str:
    if name is None:
        name = os.path.basename(path)
    return MD_PRIMARY_FORMAT['format'].format(name=name, path=path)

def parse_primary_link(text: str):
    """Return regex match for primary wikilink format, or None."""
    return PRIMARY_LINK_REGEX.search(text)

# Obsidian-style embed wikilink format: ![[path|alias]] or ![[name]]
MD_EMBED_WIKILINK = {
	'format': '![[{name}]]',
	'regex': r'!\[\[([^\]|]+)(?:\|([^\]]+))?\]\]'
}

# Sidecar file extension
SIDECAR_EXTENSION = ".side.md"

# Redirect file extension
REDIRECT_EXTENSION = ".redirect.md"

# Default frontmatter tags
FRONTMATTER_TAGS = {"sidecar", "blendvault"}

# Default poll interval (seconds) for checking for file changes
POLL_INTERVAL = 1.0

# Warning prefix for resources outside the vault
RESOURCE_WARNING_PREFIX = "⚠️ "

# --- Blend Vault UUID/Hash Key Constants ---
BV_UUID_PROP = "BV_UUID"
BV_FILE_UUID_KEY = "blendfile_uuid"
BV_UUID_KEY = "uuid"

def get_or_create_datablock_uuid(datablock: 'bpy.types.ID') -> str:
    """
    Gets an existing Blend Vault UUID (BV_UUID_PROP) from a Blender datablock
    (any item with id_properties_ensure, e.g., asset, library object).
    If no UUID exists, generates a new UUID (v4), stores it, and returns it.
    """
    if not hasattr(datablock, 'id_properties_ensure'):
        error_msg = f"Cannot ensure UUID for item without 'id_properties_ensure': {datablock}"
        try:
            log_error(error_msg)
        except NameError:
            print(f"ERROR: {error_msg}")
        raise TypeError(error_msg)

    props = datablock.id_properties_ensure()
    if BV_UUID_PROP in props:
        existing_uuid = props[BV_UUID_PROP]
        if isinstance(existing_uuid, str):
            return existing_uuid
        else:
            try:
                log_warning(f"Found non-string BV_UUID_PROP on {datablock.name_full if hasattr(datablock, 'name_full') else datablock}: {existing_uuid}. Generating new UUID.")
            except NameError:
                print(f"WARNING: Found non-string BV_UUID_PROP on {datablock.name_full if hasattr(datablock, 'name_full') else datablock}: {existing_uuid}. Generating new UUID.")
            
    new_uuid = str(uuid.uuid4())
    props[BV_UUID_PROP] = new_uuid
    return new_uuid

def generate_filepath_hash(filepath: str) -> str:
    """
    Generates a deterministic SHA256 hash for a given file path string.
    Useful for creating a consistent ID for library files based on their path.
    """
    if not isinstance(filepath, str):
        error_msg = f"generate_filepath_hash expects a string path, received type {type(filepath)}: {filepath}"
        try:
            log_error(error_msg)
        except NameError:
            print(f"ERROR: {error_msg}")
        raise TypeError(error_msg)
        
    return hashlib.sha256(filepath.encode('utf-8')).hexdigest()

def get_resource_warning_prefix(resource_path: str, blend_file_path: str, vault_root: Optional[str] = None) -> str:
	"""
	Generate warning prefix for resources outside the Obsidian vault.
	
	Args:
		resource_path: The relative path to the resource from the blend file
		blend_file_path: The absolute path to the blend file
		vault_root: The absolute path to the Obsidian vault root (optional)
	
	Returns:
		RESOURCE_WARNING_PREFIX if the resource is outside the vault, empty string otherwise
	"""
	if not vault_root:
		return ""
	
	try:
		# Get absolute path of the resource relative to the blend file
		blend_dir = os.path.dirname(blend_file_path)
		resource_abs_path = os.path.normpath(os.path.join(blend_dir, resource_path))
		vault_root_abs = os.path.normpath(vault_root)
		
		# Check if resource is outside the vault
		rel_path = os.path.relpath(resource_abs_path, vault_root_abs)
		
		# If relpath starts with "..", it's outside the vault
		if rel_path.startswith('..'):
			return RESOURCE_WARNING_PREFIX
			
	except ValueError:
		# ValueError occurs when paths are on different drives (Windows)
		return RESOURCE_WARNING_PREFIX
	
	return ""

# Logging utilities
def log_info(message: str) -> None:
    """Log an info message."""
    print(f"{LOG_COLORS['INFO']}{message}{LOG_COLORS['RESET']}")

def log_warning(message: str) -> None:
    """Log a warning message."""
    print(f"{LOG_COLORS['WARN']}{message}{LOG_COLORS['RESET']}")

def log_error(message: str) -> None:
    """Log an error message."""
    print(f"{LOG_COLORS['ERROR']}{message}{LOG_COLORS['RESET']}")

def log_success(message: str) -> None:
    """Log a success message."""
    print(f"{LOG_COLORS['SUCCESS']}{message}{LOG_COLORS['RESET']}")

def log_debug(message: str) -> None:
    """Log a debug message."""
    print(f"{LOG_COLORS['DEBUG']}{message}{LOG_COLORS['RESET']}")
