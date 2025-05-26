"""
Resource relinking module for Blend Vault.
Handles relinking external resources (textures, videos, audio, scripts, caches) based on sidecar file information.
"""

import bpy  # type: ignore
import os
import re
import traceback
from typing import Dict, List, Optional, Any
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
from utils import (
    RESOURCE_WARNING_PREFIX,
    MD_PRIMARY_FORMAT
)


class ResourceRelinkProcessor(BaseRelinker):
    """Handles the resource relinking logic."""
    
    def process_relink(self) -> None:
        """Main entry point for resource relinking process."""
        if not self.ensure_sidecar_exists():
            return
        
        self.log_start("ResourceRelink")
        
        try:
            parser = self.get_parser()
            
            # Process each resource section
            resource_sections = [
                "Linked Texture Files",
                "Linked Video Files", 
                "Linked Audio Files",
                "Linked Text Files",
                "Linked Cache Files"
            ]
            
            found_any_resource = False
            
            for section in resource_sections:
                if self._process_resource_section(parser, section):
                    found_any_resource = True
            
            if not found_any_resource:
                log_info("[Blend Vault][ResourceRelink] No resource entries found in sidecar.")
            
        except Exception as e:
            log_error(f"[Blend Vault][ResourceRelink] Error during resource relinking: {e}")
            traceback.print_exc()
        
        self.log_finish("ResourceRelink")
    
    def _process_resource_section(self, parser: SidecarParser, section_name: str) -> bool:
        """Process a single resource section from the sidecar."""
        log_debug(f"[Blend Vault][ResourceRelink] Processing section: {section_name}")
        
        # Extract resource type from section name
        resource_type = self._extract_resource_type(section_name)
        if not resource_type:
            log_warning(f"[Blend Vault][ResourceRelink] Unknown resource section: {section_name}")
            return False
        
        # Get resources with markdown links
        resource_data = parser.extract_json_blocks_with_links(section_name)
        
        if not resource_data:
            log_debug(f"[Blend Vault][ResourceRelink] No resources found in {section_name}")
            return False
        
        found_any = False
        for resource_path, resource_info in resource_data.items():
            if self._process_single_resource(resource_info, resource_type):
                found_any = True
        
        return found_any
    
    def _extract_resource_type(self, section_name: str) -> Optional[str]:
        """Extract the resource type from section name."""
        type_mapping = {
            "Linked Texture Files": "Image",
            "Linked Video Files": "Video",
            "Linked Audio Files": "Audio", 
            "Linked Text Files": "Text",
            "Linked Cache Files": "Cache"
        }
        return type_mapping.get(section_name)
    
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
