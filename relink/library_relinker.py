"""
Library relinking module for Blend Vault.
Handles relinking libraries based on information in the sidecar Markdown file.
"""

import bpy  # type: ignore
import os
import traceback
from typing import Dict, Optional, Any
from .shared_utils import (
    BaseRelinker,
    PathResolver,
    LibraryManager,
    log_info,
    log_warning,
    log_error,
    log_success,
    log_debug,
    ensure_saved_file,
    make_paths_relative,
    create_blender_operator_class
)


class LibraryRelinkProcessor(BaseRelinker):
    """Handles the library relinking logic."""
    
    def process_relink(self) -> None:
        """Main entry point for library relinking process."""
        if not self.ensure_sidecar_exists():
            return
        
        self.log_start("LibraryRelink")
        
        try:
            parser = self.get_parser()
            linked_libraries = parser.extract_json_blocks_with_links("Linked Libraries")
            
            if not linked_libraries:
                log_info("[Blend Vault][LibraryRelink] No linked library data found in sidecar.")
                return
            
            found_any_link = False
            
            for lib_path, lib_data in linked_libraries.items():
                if self._process_library_entry(lib_data):
                    found_any_link = True
            
            if not found_any_link:
                log_info("[Blend Vault][LibraryRelink] No valid library entries were processed.")
            
            # Make paths relative at the end
            make_paths_relative()
            
        except Exception as e:
            log_error(f"[Blend Vault][LibraryRelink] Error during relinking process: {e}")
            traceback.print_exc()
        
        self.log_finish("LibraryRelink")
    
    def _process_library_entry(self, lib_data: Dict[str, Any]) -> bool:
        """Process a single library entry from the sidecar."""
        link_name = lib_data["link_name"]
        link_path = lib_data["link_path"]
        json_data = lib_data["json_data"]
        
        stored_path = json_data.get("path")
        stored_uuid = json_data.get("uuid")
        
        if not stored_path or not stored_uuid or stored_uuid == "MISSING_HASH":
            if stored_uuid == "MISSING_HASH":
                log_info(f"[Blend Vault][LibraryRelink] Entry for '{link_name}' has 'MISSING_HASH'. Skipping.")
            else:
                log_warning(f"[Blend Vault][LibraryRelink] Invalid data for '{link_name}': Missing path or UUID")
            return False
        
        log_info(f"[Blend Vault][LibraryRelink] Processing: '{link_name}' -> '{stored_path}' (UUID: {stored_uuid})")
        
        # Use the markdown link path preferentially
        target_path = link_path or stored_path
        relative_path = PathResolver.blender_relative_path(target_path)
        
        # Try to find existing library by UUID
        existing_lib = LibraryManager.find_library_by_uuid(stored_uuid)
        if existing_lib:
            return self._relink_existing_library(existing_lib, relative_path, target_path)
        
        # Try to find by filename
        filename = os.path.basename(target_path)
        existing_lib = LibraryManager.find_library_by_filename(filename)
        if existing_lib:
            log_info(f"[Blend Vault][LibraryRelink] Found library by filename: {filename}")
            return self._relink_existing_library(existing_lib, relative_path, target_path)
        
        # Try to fix missing libraries
        return self._fix_missing_library(link_name, relative_path, target_path)
    
    def _relink_existing_library(self, library: bpy.types.Library, relative_path: str, target_path: str) -> bool:
        """Relink an existing library if its path differs."""
        current_path_normalized = library.filepath.replace('\\', '/').lstrip('//')
        
        if current_path_normalized != target_path:
            log_info(f"[Blend Vault][LibraryRelink] Relinking '{library.name}' from '{library.filepath}' to '{relative_path}'")
            library.filepath = relative_path
            
            try:
                library.reload()
                log_success(f"[Blend Vault][LibraryRelink] Successfully reloaded library '{library.name}'")
                return True
            except Exception as e:
                log_error(f"[Blend Vault][LibraryRelink] Failed to reload '{library.name}': {e}")
                return False
        else:
            log_info(f"[Blend Vault][LibraryRelink] Path for '{library.name}' already matches stored path")
            return True
    
    def _fix_missing_library(self, link_name: str, relative_path: str, target_path: str) -> bool:
        """Try to fix missing libraries by relinking or loading new ones."""
        log_info(f"[Blend Vault][LibraryRelink] Library with name '{link_name}' not found. Attempting to fix missing library.")
        
        # Try to find a missing library that matches the link name
        missing_lib = self._find_missing_library_by_name(link_name)
        if missing_lib:
            log_info(f"[Blend Vault][LibraryRelink] Found missing library '{missing_lib.name}' matching link name")
            missing_lib.filepath = relative_path
            
            try:
                missing_lib.reload()
                log_success(f"[Blend Vault][LibraryRelink] Successfully reloaded missing library '{missing_lib.name}'")
                return True
            except Exception as e:
                log_error(f"[Blend Vault][LibraryRelink] Failed to reload missing library '{missing_lib.name}': {e}")
        
        # Try to use any missing library
        any_missing_lib = self._find_any_missing_library()
        if any_missing_lib:
            log_info(f"[Blend Vault][LibraryRelink] Using any available missing library '{any_missing_lib.name}'")
            any_missing_lib.filepath = relative_path
            
            try:
                any_missing_lib.reload()
                log_success(f"[Blend Vault][LibraryRelink] Successfully reloaded library '{any_missing_lib.name}' at new path")
                return True
            except Exception as e:
                log_error(f"[Blend Vault][LibraryRelink] Failed to reload library '{any_missing_lib.name}': {e}")
        
        # Try to load a new library
        return self._load_new_library(relative_path, target_path)
    
    def _find_missing_library_by_name(self, link_name: str) -> Optional[bpy.types.Library]:
        """Find a missing library that matches the given name."""
        link_name_no_ext = os.path.splitext(link_name)[0]
        
        for lib in bpy.data.libraries:
            if self._is_library_missing(lib):
                lib_name_no_ext = os.path.splitext(lib.name)[0]
                if lib_name_no_ext == link_name_no_ext:
                    return lib
        return None
    
    def _find_any_missing_library(self) -> Optional[bpy.types.Library]:
        """Find any missing library."""
        for lib in bpy.data.libraries:
            if self._is_library_missing(lib):
                return lib
        return None
    
    def _is_library_missing(self, library: bpy.types.Library) -> bool:
        """Check if a library is missing."""
        if hasattr(library, 'is_missing'):
            return library.is_missing
        
        # Fallback: check if file exists
        abs_path = PathResolver.resolve_blender_path(library.filepath)
        return not os.path.exists(abs_path)
    
    def _load_new_library(self, relative_path: str, target_path: str) -> bool:
        """Load a new library if the file exists."""
        abs_path = PathResolver.resolve_relative_to_absolute(target_path, self.blend_dir)
        
        if os.path.exists(abs_path):
            log_info(f"[Blend Vault][LibraryRelink] Loading new library: {relative_path}")
            try:
                with bpy.data.libraries.load(abs_path, link=True) as (data_from, data_to):
                    pass  # Just load the library, don't link specific items
                log_success(f"[Blend Vault][LibraryRelink] Successfully linked new library from {relative_path}")
                return True
            except Exception as e:
                log_error(f"[Blend Vault][LibraryRelink] Failed to link new library from {relative_path}: {e}")
        else:
            log_warning(f"[Blend Vault][LibraryRelink] Library file does not exist: {abs_path}")
        
        return False


@bpy.app.handlers.persistent
def relink_library_info(*args, **kwargs):
    """Main entry point for library relinking. Called by Blender handlers."""
    blend_path = ensure_saved_file()
    if not blend_path:
        return
    
    try:
        processor = LibraryRelinkProcessor(blend_path)
        processor.process_relink()
    except Exception as e:
        log_error(f"[Blend Vault][LibraryRelink] Unexpected error: {e}")
        traceback.print_exc()


# Make the handler persistent
relink_library_info.persistent = True


def execute_relink_operator(self, context: bpy.types.Context):
    """Execute function for the relink operator."""
    if not self.sidecar_file_path:
        self.report({'ERROR'}, "Sidecar file path not provided.")
        return {'CANCELLED'}
    
    if not os.path.exists(self.sidecar_file_path):
        self.report({'ERROR'}, f"Sidecar file not found: {self.sidecar_file_path}")
        return {'CANCELLED'}
    
    log_info(f"Attempting to relink libraries from: {self.sidecar_file_path}")
    
    try:
        # Extract blend file path from sidecar path
        blend_path = self.sidecar_file_path.replace('.side.md', '')
        processor = LibraryRelinkProcessor(blend_path)
        processor.process_relink()
        return {'FINISHED'}
    except Exception as e:
        log_error(f"Error during operator execution: {e}")
        return {'CANCELLED'}


# Create the operator class using the factory function
BV_OT_RelinkLibraries = create_blender_operator_class(
    'BV_OT_RelinkLibraries',
    'blend_vault.relink_libraries',
    'Relink Libraries from Sidecar',
    execute_relink_operator
)

# Add the sidecar_file_path property to the operator
BV_OT_RelinkLibraries.sidecar_file_path = bpy.props.StringProperty(
    name="Sidecar File Path",
    description="Path to the sidecar file containing library information",
    default="",
    subtype='FILE_PATH',
)

# Add invoke method for file selection
def invoke_file_select(self, context: bpy.types.Context, event):
    context.window_manager.fileselect_add(self)
    return {'RUNNING_MODAL'}

BV_OT_RelinkLibraries.invoke = invoke_file_select


def register():
    bpy.utils.register_class(BV_OT_RelinkLibraries)
    log_success("[Blend Vault] Library relinking operator registered.")


def unregister():
    bpy.utils.unregister_class(BV_OT_RelinkLibraries)
    log_warning("[Blend Vault] Library relinking operator unregistered.")


if __name__ == "__main__":
    register()
