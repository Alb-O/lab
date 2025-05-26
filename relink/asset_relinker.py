"""
Asset relinker for Blend Vault.
Handles relinking of renamed or moved assets within the Blender session.
"""

import bpy  # type: ignore
import traceback
from .shared_utils import (
    BaseRelinker, 
    SidecarParser, 
    PathResolver,
    log_info, 
    log_warning, 
    log_error, 
    log_success,
    ensure_saved_file
)


class AssetRelinkProcessor(BaseRelinker):
    """Processes asset relinking based on sidecar information."""
    
    def __init__(self, blend_path: str):
        super().__init__(blend_path)
    
    def process_relink(self) -> None:
        """Main entry point for asset relinking."""
        log_info("[Blend Vault][AssetRelink] Starting asset relinking process")

        if not self.ensure_sidecar_exists():
            # ensure_sidecar_exists already logs a warning if the file is not found
            return
        
        try:
            # get_parser() will instantiate SidecarParser.
            # SidecarParser's constructor raises FileNotFoundError if the file doesn't exist,
            # or IOError if it can't be read.
            parser = self.get_parser() 
            
            # Read content from the parser's lines attribute
            content = "".join(parser.lines) 
            if not content.strip(): # Check if content is empty or just whitespace
                log_warning("[Blend Vault][AssetRelink] Empty sidecar content")
                return
            
            self._process_asset_sections(content)
            
        except (FileNotFoundError, IOError) as e:
            # These errors might be raised by SidecarParser's constructor.
            log_error(f"[Blend Vault][AssetRelink] Failed to load or read sidecar file {self.sidecar_path}: {e}")
            return
        except Exception as e:
            log_error(f"[Blend Vault][AssetRelink] Unexpected error during asset relinking: {e}")
            traceback.print_exc()
            return
    
    def _process_asset_sections(self, content: str) -> None:
        """Process asset sections in the sidecar content."""
        lines = content.split('\n')
        
        # Look for asset sections (Local Assets, Linked Assets)
        current_section = None
        i = 0
        
        while i < len(lines):
            line = lines[i].strip()
            
            # Check for main sections
            if line.startswith('## '):
                section_name = line[3:].strip()
                if 'Assets' in section_name:
                    current_section = section_name
                    log_info(f"[Blend Vault][AssetRelink] Processing section: {current_section}")
                else:
                    current_section = None
            
            # Process assets in current section
            elif current_section and line.startswith('- '):
                self._process_asset_line(line)
            
            i += 1
    
    def _process_asset_line(self, line: str) -> None:
        """Process a single asset line from the sidecar."""
        # Asset lines typically look like:
        # - [AssetName](path/to/asset) - Type: Object, UUID: abc123
        
        # Extract asset name from markdown link
        import re
        md_link_pattern = r'\[([^\]]+)\]\(([^)]+)\)'
        match = re.search(md_link_pattern, line)
        
        if not match:
            return
        
        asset_name = match.group(1)
        asset_path = match.group(2)
        
        log_info(f"[Blend Vault][AssetRelink] Found asset: {asset_name} -> {asset_path}")
        
        # For now, this is a placeholder for asset relinking logic
        # In a real implementation, you would:
        # 1. Check if the asset exists in the current Blender session
        # 2. Check if the asset's current name/path differs from the sidecar
        # 3. Update the asset's properties if needed
        
        # This would require specific Blender API calls based on asset type
        # (Objects, Materials, Textures, etc.)


@bpy.app.handlers.persistent
def relink_renamed_assets(*args, **kwargs):
    """Main entry point for asset relinking. Called by Blender handlers."""
    blend_path = ensure_saved_file()
    if not blend_path:
        return
    
    try:
        processor = AssetRelinkProcessor(blend_path)
        processor.process_relink()
    except Exception as e:
        log_error(f"[Blend Vault][AssetRelink] Unexpected error: {e}")
        traceback.print_exc()


# Make the handler persistent
relink_renamed_assets.persistent = True


def register():
    log_success("[Blend Vault] Asset relinking module loaded.")


def unregister():
    log_warning("[Blend Vault] Asset relinking module unloaded.")


if __name__ == "__main__":
    register()
