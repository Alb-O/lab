"""
Core utilities and common functionality for the Blend Vault extension.
This module consolidates shared functionality to reduce redundancy.
"""

# Conditional bpy import - only import if we're running in Blender
try:
    import bpy
    _BLENDER_AVAILABLE = True
except ImportError:
    bpy = None
    _BLENDER_AVAILABLE = False

import uuid
import hashlib
import os
import re
from typing import Optional, Dict, Any

# === CORE CONSTANTS ===
# Log color codes (ANSI escape sequences)
LOG_COLORS = {
    'INFO': '\033[94m',    # Blue: Informational messages
    'SUCCESS': '\033[92m', # Green: Success/confirmation
    'WARN': '\033[93m',    # Yellow: Warnings
    'ERROR': '\033[91m',   # Red: Errors
    'DEBUG': '\033[95m',   # Magenta: Debug messages
    'RESET': '\033[0m',    # Reset to default
}

# Markdown link formats and regex patterns
MD_LINK_FORMATS = {
    'MD_ANGLE_BRACKETS': {
        'format': '[{name}](<{path}>)',
        'regex': r'\[([^\]]+)\]\(<([^>]+)>\)'
    },
    'MD_WIKILINK': {
        'format': '[[{path}|{name}]]',
        'regex': r'\[\[([^\]|]+)\|([^\]]+)\]\]'
    }
}

MD_PRIMARY_FORMAT = MD_LINK_FORMATS['MD_WIKILINK']  # Set Obsidian wikilink as primary format

# Compile primary link regex
PRIMARY_LINK_REGEX = re.compile(MD_PRIMARY_FORMAT['regex'])

# Sidecar file extension
SIDECAR_EXTENSION = ".side.md"

# Warning prefix for resources outside the vault
RESOURCE_WARNING_PREFIX = "⚠️ "

# --- Blend Vault UUID/Hash Key Constants ---
BV_UUID_PROP = "BV_UUID"
BV_FILE_UUID_KEY = "blendfile_uuid"
BV_UUID_KEY = "uuid"

# --- Heading Level Constants ---
HEADING_LEVEL_2 = "## "
HEADING_LEVEL_3 = "### "


# === LOGGING FUNCTIONS ===
def log_info(message: str, extension_name: str = "Blend Vault", module_name: Optional[str] = None) -> None:
    """Log an info message."""
    prefix = f"[{extension_name}]"
    if module_name:
        prefix += f" [{module_name}]"
    print(f"{LOG_COLORS['INFO']}{prefix} {message}{LOG_COLORS['RESET']}")


def log_warning(message: str, extension_name: str = "Blend Vault", module_name: Optional[str] = None) -> None:
    """Log a warning message."""
    prefix = f"[{extension_name}]"
    if module_name:
        prefix += f" [{module_name}]"
    print(f"{LOG_COLORS['WARN']}{prefix} {message}{LOG_COLORS['RESET']}")


def log_error(message: str, extension_name: str = "Blend Vault", module_name: Optional[str] = None) -> None:
    """Log an error message."""
    prefix = f"[{extension_name}]"
    if module_name:
        prefix += f" [{module_name}]"
    print(f"{LOG_COLORS['ERROR']}{prefix} {message}{LOG_COLORS['RESET']}")


def log_success(message: str, extension_name: str = "Blend Vault", module_name: Optional[str] = None) -> None:
    """Log a success message."""
    prefix = f"[{extension_name}]"
    if module_name:
        prefix += f" [{module_name}]"
    print(f"{LOG_COLORS['SUCCESS']}{prefix} {message}{LOG_COLORS['RESET']}")


def log_debug(message: str, extension_name: str = "Blend Vault", module_name: Optional[str] = None) -> None:
    """Log a debug message."""
    prefix = f"[{extension_name}]"
    if module_name:
        prefix += f" [{module_name}]"
    print(f"{LOG_COLORS['DEBUG']}{prefix} {message}{LOG_COLORS['RESET']}")


# === ASSET AND DATABLOCK FUNCTIONS ===
def get_asset_sources_map() -> Dict[str, Any]:
    """
    Returns the ASSET_SOURCES_MAP dictionary.
    This version does not cache the instance, ensuring fresh bpy.data references on each call.
    """
    if not _BLENDER_AVAILABLE or bpy is None:
        return {}
        
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


def get_or_create_datablock_uuid(datablock) -> str:
    """
    Gets an existing Blend Vault UUID (BV_UUID_PROP) from a Blender datablock
    (any item with id_properties_ensure, e.g., asset, library object).
    If no UUID exists, generates a new UUID (v4), stores it, and returns it.
    """
    if not _BLENDER_AVAILABLE or bpy is None:
        raise RuntimeError("get_or_create_datablock_uuid requires Blender environment")
        
    if not hasattr(datablock, 'id_properties_ensure'):
        error_msg = f"Cannot ensure UUID for item without 'id_properties_ensure': {datablock}"
        log_error(error_msg)
        raise TypeError(error_msg)

    props = datablock.id_properties_ensure()
    if BV_UUID_PROP in props:
        existing_uuid = props[BV_UUID_PROP]
        if isinstance(existing_uuid, str):
            return existing_uuid
        else:
            log_warning(f"Found non-string BV_UUID_PROP on {datablock.name_full if hasattr(datablock, 'name_full') else datablock}: {existing_uuid}. Generating new UUID.")
            
    new_uuid = str(uuid.uuid4())
    props[BV_UUID_PROP] = new_uuid
    return new_uuid


# === PATH AND LINK FUNCTIONS ===
def format_primary_link(path: str, name: Optional[str] = None) -> str:
    """Format a path and name into the primary markdown link format."""
    if name is None:
        name = os.path.basename(path)
    return MD_PRIMARY_FORMAT['format'].format(name=name, path=path)


def parse_primary_link(text: str):
    """Return regex match for primary wikilink format, or None."""
    return PRIMARY_LINK_REGEX.search(text)


def generate_filepath_hash(filepath: str) -> str:
    """
    Generates a deterministic SHA256 hash for a given file path string.
    Useful for creating a consistent ID for library files based on their path.
    """
    if not isinstance(filepath, str):
        error_msg = f"generate_filepath_hash expects a string path, received type {type(filepath)}: {filepath}"
        log_error(error_msg)
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


# === REGEX HELPER FUNCTIONS ===
def build_section_heading_regex(section_name: str, heading_level: str = HEADING_LEVEL_3) -> str:
    """
    Build a regex pattern that can match section headings in both plain and markdown link formats.
    
    Args:
        section_name: The name of the section to match (e.g., "Current File")
        heading_level: The markdown heading level (default: "### ")
    
    Returns:
        A regex pattern that matches:
        - "### Section Name" (plain)
        - "### [Section Name](<path>)" (markdown reference link)
        - "### [[path|Section Name]]" (wikilink)
    """
    escaped_name = re.escape(section_name)
    heading_prefix = re.escape(heading_level)
    
    # Build patterns for each supported link format using constants
    patterns = [escaped_name]  # Plain section name
    
    # Add patterns for each link format
    for format_name, format_info in MD_LINK_FORMATS.items():
        if format_name == 'MD_ANGLE_BRACKETS':
            # For angle brackets: [Section Name](<path>)
            patterns.append(rf"\[{escaped_name}\]\(<[^>]*>\)")
        elif format_name == 'MD_WIKILINK':
            # For wikilinks: [[path|Section Name]]
            patterns.append(rf"\[\[[^\]|]*\|{escaped_name}\]\]")
    
    # Combine all patterns
    combined_pattern = "|".join(patterns)
    return rf"{heading_prefix}(?:{combined_pattern})"


def build_heading_section_break_regex() -> str:
    """Build a regex pattern to match heading section breaks (## or ###)."""
    h2_pattern = f"^{re.escape(HEADING_LEVEL_2.strip())}[^#]"
    h3_pattern = f"^{re.escape(HEADING_LEVEL_3.strip())}[^#]"
    return f"({h2_pattern}|{h3_pattern})"


# === BLENDER FILE UTILITIES ===
def ensure_saved_file() -> Optional[str]:
    """Ensure the current Blender file is saved and return its path."""
    if not _BLENDER_AVAILABLE or bpy is None:
        log_error("Blender environment not available")
        return None
        
    if not bpy.data.is_saved:
        log_warning("File must be saved before running this operation")
        return None
    
    return bpy.data.filepath
