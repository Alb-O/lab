
# Conditional bpy import - only import if we're running in Blender
try:
    import bpy  # type: ignore
    _BLENDER_AVAILABLE = True
except ImportError:
    bpy = None
    _BLENDER_AVAILABLE = False
import uuid
import hashlib
import os
from typing import Optional
from .constants import (
    LOG_COLORS, MD_PRIMARY_FORMAT, PRIMARY_LINK_REGEX, RESOURCE_WARNING_PREFIX, BV_UUID_PROP
)


def get_asset_sources_map():
    """
    Initializes and returns the ASSET_SOURCES_MAP dictionary.
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


def format_primary_link(path: str, name: Optional[str] = None) -> str:
    """Format a path and name into the primary markdown link format."""
    if name is None:
        name = os.path.basename(path)
    return MD_PRIMARY_FORMAT['format'].format(name=name, path=path)


def parse_primary_link(text: str):
    """Return regex match for primary wikilink format, or None."""
    return PRIMARY_LINK_REGEX.search(text)


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
