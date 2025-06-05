"""
Shared utilities for all relinker modules.
Contains common functionality for sidecar parsing, path resolution, and error handling.
"""

import os
import json
import re
from typing import Dict, List, Optional, Tuple, Any
import bpy
from .. import (
    SIDECAR_EXTENSION,
    BV_UUID_PROP,
    BV_FILE_UUID_KEY,
    BV_UUID_KEY,
    log_info,
    log_warning,
    log_error,
    log_success,
    log_debug,
    parse_primary_link,
    ensure_saved_file
)
from ..utils.constants import MD_LINK_FORMATS
from ..utils.templates import (
    HEADING_LEVEL_2, 
    HEADING_LEVEL_3, 
    build_template_heading_regex,
    build_main_section_break_regex,
    get_main_section_heading_level
)
from ..core import build_section_heading_regex, build_heading_section_break_regex
class SidecarParser:
    """Utility class for parsing sidecar markdown files and extracting JSON blocks."""
    
    def __init__(self, sidecar_path: str):
        self.sidecar_path = sidecar_path
        self.lines: List[str] = []
        self._load_file()
    
    def _load_file(self) -> None:
        """Load the sidecar file content."""
        if not os.path.exists(self.sidecar_path):
            raise FileNotFoundError(f"Sidecar file not found: {self.sidecar_path}")
        
        try:
            with open(self.sidecar_path, 'r', encoding='utf-8') as f:
                self.lines = f.readlines()
        except Exception as e:
            raise IOError(f"Failed to read sidecar file {self.sidecar_path}: {e}")
    def find_section_start(self, section_name: str) -> int:
        """Find the line index where a section starts. Returns -1 if not found.
        Handles both plain headings and markdown link headings."""
        # Map common section names to template keys
        section_name_to_key = {
            "Linked Libraries": "linked_libraries",
            "Current File": "current_file", 
            "Resources": "resources"
        }
        
        # Try to use template system first
        template_key = section_name_to_key.get(section_name)
        if template_key:
            try:
                pattern = build_template_heading_regex(template_key)
                for i, line in enumerate(self.lines):
                    line_stripped = line.strip()
                    if re.match(pattern, line_stripped):
                        return i
            except (ValueError, KeyError) as e:
                log_error(f"[SidecarParser] Template pattern error for '{section_name}': {e}")
                # Fall back to old method if template key not found
                pass
        
        # Fallback to old regex method for unknown sections
        pattern = build_section_heading_regex(section_name)
        for i, line in enumerate(self.lines):
            line_stripped = line.strip()
            if re.match(pattern, line_stripped):
                return i
        
        log_warning(f"[SidecarParser] Section '{section_name}' not found in sidecar")
        return -1
    def extract_json_blocks_with_links(self, section_name: str) -> Dict[str, Dict[str, Any]]:
        """
        Extract JSON blocks associated with markdown links in a section.
        
        Returns:
            Dict mapping link paths to their JSON data:
            {
                "relative/path/to/file.blend": {
                    "link_name": "Display Name",
                    "link_path": "relative/path/to/file.blend", 
                    "json_data": {...parsed JSON...}
                }
            }
        """
        section_start = self.find_section_start(section_name)
        
        results = {}
        parsing_json_block = False
        json_accumulator = []
        active_link_name = None
        active_link_path = None
        
        current_line_idx = section_start + 1
        while current_line_idx < len(self.lines):
            line_raw = self.lines[current_line_idx]
            line_stripped = line_raw.strip()

            if parsing_json_block:
                if line_stripped == "```":  # End of JSON block
                    parsing_json_block = False
                    json_str = "".join(json_accumulator)
                    json_accumulator = []
                    
                    if active_link_path and json_str.strip():
                        try:
                            parsed_json = json.loads(json_str)
                            results[active_link_path] = {
                                "link_name": active_link_name,
                                "link_path": active_link_path,
                                "json_data": parsed_json
                            }
                        except json.JSONDecodeError as e:
                            log_error(f"Failed to parse JSON for '{active_link_name}': {e}")
                    active_link_name = None
                    active_link_path = None
                else:
                    json_accumulator.append(line_raw)
            
            elif line_stripped.startswith("```json"):
                if active_link_name:
                    parsing_json_block = True
                    json_accumulator = []
                else:
                    # Skip this JSON block as no link precedes it
                    self._skip_to_end_of_json_block(current_line_idx)
            elif re.match(build_heading_section_break_regex(), line_stripped):
                # Check if this is a main section break or just a subsection
                main_section_break_pattern = build_main_section_break_regex()
                if re.match(main_section_break_pattern, line_stripped):
                    # Hit another main section, stop processing
                    break
                # Otherwise it's a subsection, check for links in the heading
                line_no_heading = line_stripped.lstrip('#').strip()
                md_link_match = parse_primary_link(line_no_heading)
                if md_link_match:
                    active_link_path = md_link_match.group(1)
                    active_link_name = md_link_match.group(2) or active_link_path
            
            else:
                # Look for markdown links in regular lines
                line_no_heading = line_stripped.lstrip('#').strip() if line_stripped.startswith('#') else line_stripped
                md_link_match = parse_primary_link(line_no_heading)
                if md_link_match:
                    active_link_path = md_link_match.group(1)
                    active_link_name = md_link_match.group(2) or active_link_path
            
            current_line_idx += 1
        
        return results
    
    def _skip_to_end_of_json_block(self, start_idx: int) -> int:
        """Skip lines until the end of a JSON block. Returns the line index after the block."""
        current_idx = start_idx + 1
        while current_idx < len(self.lines):
            if self.lines[current_idx].strip() == "```":
                return current_idx
            if re.match(build_heading_section_break_regex(), self.lines[current_idx].strip()):
                return current_idx - 1
            current_idx += 1
        return current_idx
    
    def extract_current_file_section(self) -> Tuple[Optional[str], List[Dict[str, str]]]:
        """
        Extract data from the "Current File" section.
        
        Returns:
            Tuple of (file_uuid, list_of_assets)
            where list_of_assets contains dicts with "uuid", "name", "type" keys
        """
        section_start = self.find_section_start("Current File")
        if section_start == -1:
            return None, []
        
        parsing_json_block = False
        json_accumulator = []
        
        current_line_idx = section_start + 1
        while current_line_idx < len(self.lines):
            line_raw = self.lines[current_line_idx]
            line_stripped = line_raw.strip()
            
            if parsing_json_block:
                if line_stripped == "```":
                    json_str = "".join(json_accumulator)
                    if json_str.strip():
                        try:
                            data = json.loads(json_str)
                            file_uuid = data.get(BV_UUID_KEY) or data.get(BV_FILE_UUID_KEY)
                            assets = data.get("assets", [])
                            return file_uuid, [
                                {
                                    "uuid": asset.get("uuid"),
                                    "name": asset.get("name"),
                                    "type": asset.get("type")
                                }
                                for asset in assets
                                if asset.get("uuid") and asset.get("name") and asset.get("type")
                            ]
                        except json.JSONDecodeError as e:
                            log_error(f"Failed to parse Current File JSON: {e}")
                    break
                else:
                    json_accumulator.append(line_raw)
            
            elif line_stripped.startswith("```json"):
                parsing_json_block = True
                json_accumulator = []
            
            elif re.match(build_heading_section_break_regex(), line_stripped):
                break
            
            current_line_idx += 1
        
        return None, []


class PathResolver:
    """Utility class for handling path resolution and normalization."""
    
    @staticmethod
    def normalize_path(path: str) -> str:
        """Normalize a file path."""
        return os.path.normpath(path)
    
    @staticmethod
    def resolve_relative_to_absolute(relative_path: str, base_dir: str) -> str:
        """Convert a relative path to absolute based on a base directory."""
        return PathResolver.normalize_path(os.path.join(base_dir, relative_path))
    
    @staticmethod
    def blender_relative_path(path: str) -> str:
        """Convert a path to Blender's relative path format (// prefix)."""
        return '//' + path.replace('\\', '/')
    
    @staticmethod
    def resolve_blender_path(blender_path: str) -> str:
        """Resolve a Blender path (with //) to absolute path."""
        if blender_path.startswith("//"):
            return PathResolver.normalize_path(bpy.path.abspath(blender_path))
        return PathResolver.normalize_path(blender_path)


class LibraryManager:
    """Utility class for managing Blender library operations."""
    
    @staticmethod
    def reload_library(library_path: str) -> bool:
        """Reload a library by path. Returns True if successful."""
        try:
            abs_path = PathResolver.resolve_blender_path(library_path)
            bpy.ops.wm.lib_reload(filepath=abs_path)
            log_info(f"Reloaded library: {abs_path}")
            return True
        except Exception as e:
            log_error(f"Could not reload library '{library_path}': {e}")
            return False
    
    @staticmethod
    def find_library_by_uuid(target_uuid: str) -> Optional[bpy.types.Library]:
        """Find a library by its Blend Vault UUID."""
        for lib in bpy.data.libraries:
            lib_uuid = LibraryManager.get_library_uuid(lib)
            if lib_uuid == target_uuid:
                return lib
        return None
    
    @staticmethod
    def get_library_uuid(library: bpy.types.Library) -> Optional[str]:
        """Extract the Blend Vault UUID from a library."""
        lib_prop_val = library.get(BV_UUID_PROP)
        if not lib_prop_val:
            return None
        
        try:
            if isinstance(lib_prop_val, str):
                # Try parsing as JSON first
                try:
                    parsed = json.loads(lib_prop_val)
                    if isinstance(parsed, dict):
                        return parsed.get(BV_FILE_UUID_KEY)
                    return lib_prop_val
                except json.JSONDecodeError:
                    return lib_prop_val
            return str(lib_prop_val)
        except Exception:
            return None
    
    @staticmethod
    def find_library_by_filename(filename: str) -> Optional[bpy.types.Library]:
        """Find a library by its filename."""
        for lib in bpy.data.libraries:
            if os.path.basename(lib.filepath) == filename:
                return lib
        return None


def get_sidecar_path(blend_file_path: str) -> str:
    """
    Get the sidecar file path for a given blend file.
    Handles various input formats flexibly without hardcoded extensions.
    
    Examples:
        "file.blend" -> "file.blend.side.md"
        "file.blend.side" -> "file.blend.side.md" 
        "file.blend.side.md" -> "file.blend.side.md" (unchanged)
    """
    # If already ends with full sidecar extension, return as-is
    if blend_file_path.endswith(SIDECAR_EXTENSION):
        return blend_file_path
    
    # Parse the sidecar extension dynamically
    # Remove leading dot and split by dots to get extension components
    sidecar_clean = SIDECAR_EXTENSION.lstrip('.')  # "side.md"
    extension_parts = sidecar_clean.split('.')     # ["side", "md"]
    
    if len(extension_parts) >= 2:
        # Build partial extensions: [".side", ".side.md"]
        partial_extensions = []
        for i in range(1, len(extension_parts) + 1):
            partial_ext = '.' + '.'.join(extension_parts[:i])
            partial_extensions.append(partial_ext)
        
        # Check if the path ends with any partial extension (but not the full one)
        for partial_ext in partial_extensions[:-1]:  # Exclude the full extension
            if blend_file_path.endswith(partial_ext):
                # Calculate what needs to be added
                remaining_parts = extension_parts[len(partial_ext.lstrip('.').split('.')):]
                remaining_extension = '.' + '.'.join(remaining_parts)
                return blend_file_path + remaining_extension
    
    # Default case: add the full sidecar extension
    return blend_file_path + SIDECAR_EXTENSION

def get_blend_file_path_from_sidecar(sidecar_path: str) -> str:
    """
    Convert a sidecar file path back to its corresponding blend file path.
    Handles various sidecar formats dynamically.
    
    Examples:
        "file.blend.side.md" -> "file.blend"
        "file.blend.side" -> "file.blend"
        "file.blend" -> "file.blend" (unchanged)
    """
    # If it doesn't contain sidecar-related extensions, return as-is
    if not any(ext in sidecar_path for ext in ['.side', SIDECAR_EXTENSION]):
        return sidecar_path
    
    # If it ends with the full sidecar extension, remove it
    if sidecar_path.endswith(SIDECAR_EXTENSION):
        return sidecar_path[:-len(SIDECAR_EXTENSION)]
    
    # Parse the sidecar extension to handle partial matches
    sidecar_clean = SIDECAR_EXTENSION.lstrip('.')  # "side.md"
    extension_parts = sidecar_clean.split('.')     # ["side", "md"]
    
    # Try to remove partial sidecar extensions
    for i in range(len(extension_parts), 0, -1):
        partial_ext = '.' + '.'.join(extension_parts[:i])
        if sidecar_path.endswith(partial_ext):
            return sidecar_path[:-len(partial_ext)]
    
    return sidecar_path



class BaseRelinker:
    """Base class for all relinker modules providing common functionality."""
    
    def __init__(self, blend_file_path: Optional[str] = None):
        """Initialize the relinker with a blend file path."""
        self.blend_file_path = blend_file_path or bpy.data.filepath
        if not self.blend_file_path:
            raise ValueError("No blend file path provided and current file is not saved")
        
        self.blend_dir = os.path.dirname(self.blend_file_path)
        self.sidecar_path = get_sidecar_path(self.blend_file_path)
        self.parser: Optional[SidecarParser] = None
    
    def ensure_sidecar_exists(self) -> bool:
        """Check if the sidecar file exists."""
        if not os.path.exists(self.sidecar_path):
            log_warning(f"Sidecar file not found: {self.sidecar_path}")
            return False
        return True
    
    def get_parser(self) -> SidecarParser:
        """Get or create a sidecar parser instance."""
        if self.parser is None:
            self.parser = SidecarParser(self.sidecar_path)
        return self.parser
    
    def log_start(self, module_name: str) -> None:
        """Log the start of a relink process."""
        log_info(f"[{module_name}] Processing sidecar file: {self.sidecar_path}")
    
    def log_finish(self, module_name: str) -> None:
        """Log the completion of a relink process."""
        log_info(f"[{module_name}] Finished relink attempt.")


class ResourceManager:
    """Utility class for managing Blender resources (images, sounds, etc.)."""
    
    RESOURCE_COLLECTIONS = {
        "Image": "images",
        "Video": "movieclips", 
        "Audio": "sounds",
        "Text": "texts",
        "Cache": "cache_files"
    }
    
    @staticmethod
    def get_resource_collection(resource_type: str):
        """Get the bpy.data collection for a resource type."""
        collection_name = ResourceManager.RESOURCE_COLLECTIONS.get(resource_type)
        if collection_name:
            return getattr(bpy.data, collection_name, None)
        return None
    
    @staticmethod
    def find_resource_by_name(name: str, resource_type: str):
        """Find a resource by name and type."""
        collection = ResourceManager.get_resource_collection(resource_type)
        if not collection:
            return None
        
        for item in collection:
            if item and getattr(item, 'name', '') == name:
                return item
        return None
    
    @staticmethod
    def update_resource_filepath(resource, new_path: str, resource_type: str) -> bool:
        """Update a resource's filepath and reload if necessary."""
        try:
            rel_path = PathResolver.blender_relative_path(new_path)
            old_path = getattr(resource, 'filepath', '')
            
            if resource_type == "Image":
                resource.filepath = rel_path
                try:
                    resource.reload()
                    log_success(f"Reloaded image '{resource.name}' from '{old_path}' to '{rel_path}'")
                    return True
                except Exception as e:
                    log_error(f"Failed to reload image '{resource.name}': {e}")
                    return False
            
            elif resource_type in ["Video", "Audio"]:
                resource.filepath = rel_path
                log_success(f"Updated {resource_type.lower()} '{resource.name}' from '{old_path}' to '{rel_path}'")
                return True
            
            elif resource_type == "Text":
                # For text files, reload content
                working_dir = os.path.dirname(bpy.data.filepath)
                abs_path = PathResolver.resolve_relative_to_absolute(new_path, working_dir)
                
                if os.path.exists(abs_path):
                    try:
                        with open(abs_path, 'r', encoding='utf-8') as f:
                            content = f.read()
                        resource.from_string(content)
                        resource.filepath = rel_path
                        log_success(f"Reloaded text '{resource.name}' from '{old_path}' to '{rel_path}'")
                        return True
                    except Exception as e:
                        log_error(f"Failed to reload text '{resource.name}': {e}")
                        return False
                else:
                    log_warning(f"Text file not found: {abs_path}")
                    return False
            
            elif resource_type == "Cache":
                resource.filepath = rel_path
                log_success(f"Updated cache '{resource.name}' from '{old_path}' to '{rel_path}'")
                return True
            
            return False
            
        except Exception as e:
            log_error(f"Error updating {resource_type} '{getattr(resource, 'name', 'unknown')}': {e}")
            return False


def make_paths_relative() -> None:
    """Make all external file paths relative, with error handling."""
    if not bpy.data.is_saved:
        log_warning("Cannot make paths relative: file is not saved.")
        return
    
    try:
        bpy.ops.file.make_paths_relative()
        log_success("Made all external file paths relative.")
    except RuntimeError as e:
        log_warning(f"Could not make paths relative: {e}")
    except Exception as e:
        log_error(f"Error making paths relative: {e}")


def create_blender_operator_class(class_name: str, bl_idname: str, bl_label: str, execute_func):
    """
    Factory function to create Blender operator classes with common structure.
    
    Args:
        class_name: Name of the operator class
        bl_idname: Blender operator ID
        bl_label: Operator label
        execute_func: Function to call in execute method
    
    Returns:
        Blender operator class
    """
    class_dict = {
        'bl_idname': bl_idname,
        'bl_label': bl_label,
        'bl_options': {'REGISTER', 'UNDO'},
        'execute': execute_func
    }
    
    return type(class_name, (bpy.types.Operator,), class_dict)
