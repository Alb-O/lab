"""
Library relinking module for Blend Vault.
Handles relinking libraries based on information in the sidecar Markdown file.
"""

import bpy
import os
import traceback
from typing import Dict, Optional, Any, Set  # Ensure Set is imported
from bpy.types import Context, Event, Operator  # Import specific bpy types for hinting

from .. import SIDECAR_EXTENSION
from ..core import log_info, log_warning, log_error, log_success, log_debug
from .shared_utils import (
    BaseRelinker,
    PathResolver,
    LibraryManager,
    ensure_saved_file,
    make_paths_relative,
    create_blender_operator_class,
    get_blend_file_path_from_sidecar
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
                log_info("[LibraryRelinker] No linked library data found in sidecar.", module_name='LibraryRelinker')
                return
            
            found_any_link = False
            
            for lib_path, lib_data in linked_libraries.items():
                if self._process_library_entry(lib_data):
                    found_any_link = True
            
            if not found_any_link:
                log_info("[LibraryRelinker] No valid library entries were processed.", module_name='LibraryRelinker')
            
            # Make paths relative at the end
            make_paths_relative()
            
        except Exception as e:
            log_error(f"[LibraryRelinker] Error during relinking process: {e}", module_name='LibraryRelinker')
            traceback.print_exc()
        
        self.log_finish("LibraryRelink")
    
    def _process_library_entry(self, lib_data: Dict[str, Any]) -> bool:
        """Process a single library entry from the sidecar."""
        link_name = lib_data["link_name"]
        link_path = lib_data["link_path"]  # This will be like 'mylib.blend.side.md'
        json_data = lib_data["json_data"]

        stored_path = json_data.get("path")
        stored_uuid = json_data.get("uuid")

        if not stored_path or not stored_uuid or stored_uuid == "MISSING_HASH":
            if stored_uuid == "MISSING_HASH":
                log_info(f"[LibraryRelinker] Entry for '{link_name}' has 'MISSING_HASH'. Skipping.", module_name='LibraryRelinker')
            else:
                log_warning(f"[LibraryRelinker] Invalid data for '{link_name}': Missing path or UUID", module_name='LibraryRelinker')
            return False

        # Convert sidecar path (e.g., 'mylib.blend.side.md') to blend path (e.g., 'mylib.blend')
        # Use the new utility function for robust conversion
        blend_file_link_path = get_blend_file_path_from_sidecar(link_path)

        log_info(f"[LibraryRelinker] Processing: '{link_name}' -> '{stored_path}' (UUID: {stored_uuid}). Link path from MD: '{link_path}' -> Blend path: '{blend_file_link_path}'", module_name='LibraryRelinker')

        # Use the markdown link path (converted to .blend) preferentially
        target_path = blend_file_link_path or stored_path
        relative_path = PathResolver.blender_relative_path(target_path)
        
        # Try to find existing library by UUID
        existing_lib = LibraryManager.find_library_by_uuid(stored_uuid)
        if existing_lib:
            return self._relink_existing_library(existing_lib, relative_path, target_path)
        
        # Try to find by filename
        filename = os.path.basename(target_path)
        existing_lib = LibraryManager.find_library_by_filename(filename)
        if existing_lib:
            log_info(f"[LibraryRelinker] Found library by filename: {filename}", module_name='LibraryRelinker')
            return self._relink_existing_library(existing_lib, relative_path, target_path)
        
        # Try to fix missing libraries
        return self._fix_missing_library(link_name, relative_path, target_path)
    
    def _relink_existing_library(self, library: bpy.types.Library, relative_path: str, target_path: str) -> bool:
        """Relink an existing library if its path differs."""
        # Normalize both the current library filepath and the new target relative_path for comparison.
        # library.filepath is usually already relative (e.g., //../libs/lib.blend or ../libs/lib.blend)
        # relative_path is the calculated desired relative path.
        current_lib_path_normalized = PathResolver.normalize_path(library.filepath)
        new_relative_path_normalized = PathResolver.normalize_path(relative_path)

        # Debugging paths
        log_debug(f"[LibraryRelinker] Relinking Check for '{library.name}':", module_name='LibraryRelinker')
        log_debug(f"    Current Library.filepath (raw): '{library.filepath}'", module_name='LibraryRelinker')
        log_debug(f"    Current Library.filepath (normalized): '{current_lib_path_normalized}'", module_name='LibraryRelinker')
        log_debug(f"    Target Relative Path (calculated): '{relative_path}'", module_name='LibraryRelinker')
        log_debug(f"    Target Relative Path (normalized): '{new_relative_path_normalized}'", module_name='LibraryRelinker')
        log_debug(f"    Target Absolute Path (from sidecar): '{target_path}'", module_name='LibraryRelinker')
        
        if current_lib_path_normalized != new_relative_path_normalized:
            log_info(f"[LibraryRelinker] Relinking '{library.name}' from '{library.filepath}' to '{relative_path}'", module_name='LibraryRelinker')
            library.filepath = relative_path  # Assign the non-normalized relative_path
            
            try:
                library.reload()
                log_success(f"[LibraryRelinker] Successfully reloaded library '{library.name}'", module_name='LibraryRelinker')
                return True
            except Exception as e:
                log_error(f"[LibraryRelinker] Failed to reload '{library.name}': {e}", module_name='LibraryRelinker')
                return False
        else:
            log_info(f"[LibraryRelinker] Path for '{library.name}' already matches stored path", module_name='LibraryRelinker')
            return True
    
    def _fix_missing_library(self, link_name: str, relative_path: str, target_path: str) -> bool:
        """Try to fix missing libraries by relinking or loading new ones."""
        log_info(f"[LibraryRelinker] Library with name '{link_name}' not found. Attempting to fix missing library.", module_name='LibraryRelinker')
        
        # Try to find a missing library that matches the link name
        missing_lib = self._find_missing_library_by_name(link_name)
        if missing_lib:
            log_info(f"[LibraryRelinker] Found missing library '{missing_lib.name}' matching link name", module_name='LibraryRelinker')
            missing_lib.filepath = relative_path
            
            try:
                missing_lib.reload()
                log_success(f"[LibraryRelinker] Successfully reloaded missing library '{missing_lib.name}'", module_name='LibraryRelinker')
                return True
            except Exception as e:
                log_error(f"[LibraryRelinker] Failed to reload missing library '{missing_lib.name}': {e}", module_name='LibraryRelinker')
        
        # Try to use any missing library
        any_missing_lib = self._find_any_missing_library()
        if any_missing_lib:
            log_info(f"[LibraryRelinker] Using any available missing library '{any_missing_lib.name}'", module_name='LibraryRelinker')
            any_missing_lib.filepath = relative_path
            
            try:
                any_missing_lib.reload()
                log_success(f"[LibraryRelinker] Successfully reloaded library '{any_missing_lib.name}' at new path", module_name='LibraryRelinker')
                return True
            except Exception as e:
                log_error(f"[LibraryRelinker] Failed to reload library '{any_missing_lib.name}': {e}", module_name='LibraryRelinker')
        
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
        # target_path is already the absolute path to the .blend file,
        # derived from the sidecar data (either markdown link or JSON 'path').
        abs_path = PathResolver.normalize_path(target_path)
        
        log_debug(f"[LibraryRelinker] Attempting to load new library:", module_name='LibraryRelinker')
        log_debug(f"    Absolute path for loading: '{abs_path}'", module_name='LibraryRelinker')
        log_debug(f"    Relative path for Blender: '{relative_path}'", module_name='LibraryRelinker')

        if os.path.exists(abs_path):
            log_info(f"[LibraryRelinker] Loading new library: {relative_path}", module_name='LibraryRelinker')
            try:
                with bpy.data.libraries.load(abs_path, link=True) as (data_from, data_to):
                    pass  # Just load the library, don't link specific items
                log_success(f"[LibraryRelinker] Successfully linked new library from {relative_path}", module_name='LibraryRelinker')
                return True
            except Exception as e:
                log_error(f"[LibraryRelinker] Failed to link new library from {relative_path}: {e}", module_name='LibraryRelinker')
        else:
            log_warning(f"[LibraryRelinker] Library file does not exist: {abs_path}", module_name='LibraryRelinker')
        
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
        log_error(f"[LibraryRelinker] Unexpected error: {e}", module_name='LibraryRelinker')
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
    
    log_info(f"Attempting to relink libraries from: {self.sidecar_file_path}", module_name='LibraryRelinker')
    
    try:
        # Extract blend file path from sidecar path
        # Use the robust utility function
        blend_path = get_blend_file_path_from_sidecar(self.sidecar_file_path)
        if not blend_path:
            self.report({'ERROR'}, f"Could not determine .blend file path from sidecar: {self.sidecar_file_path}")
            return {'CANCELLED'}
            
        processor = LibraryRelinkProcessor(blend_path)
        processor.process_relink()
        self.report({'INFO'}, "Library relinking process completed.")
        return {'FINISHED'}
    except Exception as e:
        log_error(f"Error during operator execution: {e}", module_name='LibraryRelinker')
        self.report({'ERROR'}, f"Relinking failed: {e}")
        return {'CANCELLED'}


# Create the operator class using the factory function
# We assume create_blender_operator_class returns a type that is a subclass of bpy.types.Operator
# and that it correctly sets up bl_idname, bl_label, and the execute method.
BV_OT_RelinkLibraries: Any = create_blender_operator_class(
    'BV_OT_RelinkLibraries',  # class_name
    'blend_vault.relink_libraries',  # bl_idname
    'Relink Libraries from Sidecar',  # bl_label
    execute_relink_operator  # execute_method (assuming this is how your factory takes it)
)

# Add the sidecar_file_path property to the operator
# We cast BV_OT_RelinkLibraries to Operator to help Pylance
if hasattr(BV_OT_RelinkLibraries, 'bl_rna'):  # A check to see if it's a Blender type
    setattr(BV_OT_RelinkLibraries, 'sidecar_file_path', bpy.props.StringProperty(
        name="Sidecar File Path",
        description="Path to the sidecar file containing library information",
        default="",
        subtype='FILE_PATH',
    ))


# Define and assign invoke method for file selection with correct type hints
def invoke_file_select(self: Operator, context: Context, event: Event) -> Set[str]:
    context.window_manager.fileselect_add(self)
    return {'RUNNING_MODAL'}

if hasattr(BV_OT_RelinkLibraries, 'bl_rna'):  # A check to see if it's a Blender type
    BV_OT_RelinkLibraries.invoke = invoke_file_select


def register():
    bpy.utils.register_class(BV_OT_RelinkLibraries)
    log_success("[LibraryRelinker] Library relinking operator registered.", module_name='LibraryRelinker')


def unregister():
    bpy.utils.unregister_class(BV_OT_RelinkLibraries)
    log_warning("[LibraryRelinker] Library relinking operator unregistered.", module_name='LibraryRelinker')


if __name__ == "__main__":
    register()
