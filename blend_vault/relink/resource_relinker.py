"""
Resource relinking module for Blend Vault.
Handles relinking external resources (textures, videos, audio, scripts, caches) based on sidecar file information.
"""

import bpy  # type: ignore
import os
import re
import traceback
from typing import Dict, Optional, Any
from .shared_utils import (
    BaseRelinker,
    SidecarParser,
    PathResolver,
    ResourceManager,
    log_info,
    log_warning,
    log_error,
    log_success,
    log_debug,
    ensure_saved_file
)
from ..utils import parse_primary_link, RESOURCE_WARNING_PREFIX


class ResourceRelinkProcessor(BaseRelinker):
    """Handles the resource relinking logic."""
    
    def process_relink(self) -> None:
        """Main entry point for resource relinking process."""
        if not self.ensure_sidecar_exists():
            return
        
        self.log_start("ResourceRelink")
        
        try:
            parser = self.get_parser()
            
            # Process the Resources section - check for subsections
            resource_subsections = [
                "Textures",
                "Videos", 
                "Audio",
                "Text Files",
                "Cache Files"
            ]
            
            found_any_resource = False
            
            for subsection in resource_subsections:
                if self._process_resource_subsection(parser, subsection):
                    found_any_resource = True
            
            if not found_any_resource:
                log_info("[Blend Vault][ResourceRelink] No resource entries found in sidecar.")
            
        except Exception as e:
            log_error(f"[Blend Vault][ResourceRelink] Error during resource relinking: {e}")
            traceback.print_exc()
        
        self.log_finish("ResourceRelink")
    
    def _process_resource_subsection(self, parser: SidecarParser, subsection_name: str) -> bool:
        """Process a single resource subsection from the Resources section."""
        log_debug(f"[Blend Vault][ResourceRelink] Processing subsection: {subsection_name}")
        
        # Extract resource type from subsection name
        resource_type = self._extract_resource_type_from_subsection(subsection_name)
        if not resource_type:
            log_warning(f"[Blend Vault][ResourceRelink] Unknown resource subsection: {subsection_name}")
            return False
        
        # First, check if we have a Resources section at all
        resources_section_start = parser.find_section_start("Resources")
        if resources_section_start == -1:
            log_debug("[Blend Vault][ResourceRelink] No Resources section found")
            return False
        
        # Look for the specific subsection within Resources
        resource_data = self._extract_resources_from_subsection(parser, resources_section_start, subsection_name)
        
        if not resource_data:
            log_debug(f"[Blend Vault][ResourceRelink] No resources found in {subsection_name}")
            return False
        
        found_any = False
        for resource_path, resource_info in resource_data.items():
            if self._process_single_resource(resource_info, resource_type):
                found_any = True
        
        return found_any
    
    def _extract_resource_type_from_subsection(self, subsection_name: str) -> Optional[str]:
        """Extract the resource type from subsection name."""
        type_mapping = {
            "Textures": "Image",
            "Videos": "Video",
            "Audio": "Audio", 
            "Text Files": "Text",
            "Cache Files": "Cache"
        }
        return type_mapping.get(subsection_name)
    
    def _extract_resources_from_subsection(self, parser: SidecarParser, resources_start: int, subsection_name: str) -> Dict[str, Dict[str, Any]]:
        """Extract resources from a specific subsection within the Resources section."""
        # Look for the subsection heading (#### {subsection_name})
        subsection_target = f"#### {subsection_name}"
        subsection_start = -1
        
        # Search from the Resources section start
        for i in range(resources_start + 1, len(parser.lines)):
            line = parser.lines[i].strip()
            if line == subsection_target:
                subsection_start = i
                break
            # Stop if we hit another ### section
            elif line.startswith("### "):
                break
        
        if subsection_start == -1:
            return {}
        
        # Extract markdown links from this subsection
        results = {}
        current_line_idx = subsection_start + 1
        
        while current_line_idx < len(parser.lines):
            line_stripped = parser.lines[current_line_idx].strip()
            
            # Stop if we hit another heading
            if re.match(r"^(###|####)", line_stripped):
                break
            
            # Look for markdown links using parse_primary_link
            md_link_match = parse_primary_link(line_stripped)
            if md_link_match:
                link_path = md_link_match.group(1)
                link_name = md_link_match.group(2) or link_path
                
                # For resources without JSON blocks, create a simple entry
                results[link_path] = {
                    "link_name": link_name,
                    "link_path": link_path,
                    "json_data": {"name": link_name, "path": link_path}
                }
            
            current_line_idx += 1
        
        return results
    
    def _process_single_resource(self, resource_info: Dict[str, Any], resource_type: str) -> bool:
        """Process a single resource entry."""
        link_name = resource_info["link_name"]
        link_path = resource_info["link_path"]
        json_data = resource_info["json_data"]
        
        # Extract resource details from JSON
        stored_path = json_data.get("path", "")
        display_name = json_data.get("name", link_name)
        
        # Handle warning prefixes in the name
        clean_name = self._remove_warning_prefix(display_name)
        
        # Unescape markdown characters (specifically underscores)
        clean_name = self._unescape_markdown_name(clean_name)
        
        log_info(f"[Blend Vault][ResourceRelink] Processing {resource_type}: '{clean_name}' -> '{link_path}'")
        
        # Find the resource in Blender
        resource = ResourceManager.find_resource_by_name(clean_name, resource_type)
        if not resource:
            log_warning(f"[Blend Vault][ResourceRelink] {resource_type} '{clean_name}' not found in session")
            return False
        
        # Use the markdown link path preferentially
        target_path = link_path or stored_path
        if not target_path:
            log_warning(f"[Blend Vault][ResourceRelink] No path found for {resource_type} '{clean_name}'")
            return False
        
        # Check if the file exists
        abs_path = PathResolver.resolve_relative_to_absolute(target_path, self.blend_dir)
        if not os.path.exists(abs_path):
            log_warning(f"[Blend Vault][ResourceRelink] File does not exist: {abs_path}")
            return False
        
        # Update the resource path
        success = ResourceManager.update_resource_filepath(resource, target_path, resource_type)
        if success:
            log_success(f"[Blend Vault][ResourceRelink] Successfully relinked {resource_type} '{clean_name}'")
        
        return success
    
    def _remove_warning_prefix(self, text: str) -> str:
        """Remove warning prefix from a text line if present."""
        if text.startswith(RESOURCE_WARNING_PREFIX):
            return text[len(RESOURCE_WARNING_PREFIX):].strip()
        return text
    
    def _unescape_markdown_name(self, name: str) -> str:
        """Unescape markdown characters in resource names."""
        # Unescape common markdown characters
        unescaped = name.replace('\\_', '_')  # Unescape underscores
        unescaped = unescaped.replace('\\*', '*')  # Unescape asterisks
        unescaped = unescaped.replace('\\-', '-')  # Unescape hyphens
        unescaped = unescaped.replace('\\#', '#')  # Unescape hash symbols
        return unescaped


@bpy.app.handlers.persistent
def relink_resources(*args, **kwargs):
    """Main entry point for resource relinking. Called by Blender handlers."""
    blend_path = ensure_saved_file()
    if not blend_path:
        return
    
    try:
        processor = ResourceRelinkProcessor(blend_path)
        processor.process_relink()
    except Exception as e:
        log_error(f"[Blend Vault][ResourceRelink] Unexpected error: {e}")
        traceback.print_exc()


# Make the handler persistent
relink_resources.persistent = True


def register():
    log_success("[Blend Vault] Resource relinking module loaded.")


def unregister():
    log_warning("[Blend Vault] Resource relinking module unloaded.")


if __name__ == "__main__":
    register()
