"""
Library relinking module for Blend Vault.
Handles relinking libraries based on vault-root-relative paths from sidecar files.
"""

import bpy
import os
import traceback
from typing import Dict, Optional, Any, Set
from bpy.types import Context, Event, Operator

from .. import SIDECAR_EXTENSION
from ..preferences import get_obsidian_vault_root
from ..core import log_info, log_warning, log_error, log_success, log_debug
from .shared_utils import (
	SidecarParser,
	PathResolver,
	LibraryManager,
	ensure_saved_file,
	make_paths_relative,
	get_sidecar_path,
	get_blend_file_path_from_sidecar,
	create_blender_operator_class,
)


class LibraryRelinkProcessor:
	"""Handles library relinking using vault-root-relative paths."""
	
	def __init__(self, main_blend_path: str):
		# Get vault root and fail fast if not available
		self.vault_root = get_obsidian_vault_root()
		if not self.vault_root:
			log_error("Vault root not set. Library relinking cannot proceed.", module_name='LibraryRelink')
			self._valid_init = False
			return
		self._valid_init = True
		
		# Store original main blend path for Blender relative path computation
		self.original_main_blend_path = main_blend_path
		
		# Check if this is a relocated file using redirect handler
		from .redirect_handler import _pending_relocations
		for old_path, new_path in _pending_relocations.items():
			if new_path == main_blend_path:
				self.original_main_blend_path = old_path
				log_info(f"Detected relocation: using original path {old_path} for relative computations", module_name='LibraryRelink')
				break
		
		# All paths computed relative to vault root
		self.main_blend_path = main_blend_path
		self.main_vault_rel = os.path.relpath(self.main_blend_path, self.vault_root).replace(os.sep, '/')
		self.sidecar_path = os.path.normpath(os.path.join(self.vault_root, self.main_vault_rel + SIDECAR_EXTENSION))
		
		log_debug(f"LibraryRelinkProcessor initialized with vault_root={self.vault_root}, main_vault_rel={self.main_vault_rel}", module_name='LibraryRelink')
	
	def process_relink(self) -> None:
		"""Main entry point for library relinking process."""
		if not hasattr(self, '_valid_init') or not self._valid_init:
			log_warning("LibraryRelinkProcessor not initialized correctly or vault root not set. Skipping relink.", module_name='LibraryRelink')
			return
		# Skip if sidecar doesn't exist
		if not os.path.exists(self.sidecar_path):
			log_warning(f"Main sidecar file not found: {self.sidecar_path}", module_name='LibraryRelink')
			return
		
		log_info(f"Processing main sidecar for library relinking: {self.sidecar_path}", module_name='LibraryRelink')
		
		try:
			parser = SidecarParser(self.sidecar_path)
			
			# Parse linked libraries from main sidecar
			linked_libraries = parser.extract_json_blocks_with_links("Linked Libraries")
			if not linked_libraries:
				log_info("No linked library data found in main sidecar.", module_name='LibraryRelink')
				return
			
			# Process each library entry using vault-root-relative paths
			relink_count = 0
			for lib_vault_rel, lib_data in linked_libraries.items():
				if self._process_library_entry(lib_vault_rel, lib_data):
					relink_count += 1
			
			log_success(f"Successfully processed {relink_count}/{len(linked_libraries)} library entries", module_name='LibraryRelink')
			
			# Make paths relative at the end for Blender compatibility
			make_paths_relative()
			
		except Exception as e:
			log_error(f"Error during library relinking process: {e}", module_name='LibraryRelink')
			traceback.print_exc()
		
	def _process_library_entry(self, lib_vault_rel: str, lib_data: Dict[str, Any]) -> bool:
		"""
		Process a single library entry using vault-root-relative paths.
		
		Args:
			lib_vault_rel: Vault-relative path from sidecar link (may include .side.md extension)
			lib_data: Library data dictionary from sidecar parser
		
		Returns:
			True if the library was successfully processed, False otherwise
		"""
		if not self.vault_root:
			log_error(f"Vault root is None in _process_library_entry for sidecar entry '{lib_vault_rel}'. Cannot process.", module_name='LibraryRelink')
			return False
		json_data = lib_data["json_data"]
		stored_uuid = json_data.get("uuid")
		
		if not stored_uuid or stored_uuid == "MISSING_HASH":
			if stored_uuid == "MISSING_HASH":
				log_info(f"Library '{lib_vault_rel}' has 'MISSING_HASH'. Skipping.", module_name='LibraryRelink')
			else:
				log_warning(f"Library '{lib_vault_rel}': Missing or invalid UUID", module_name='LibraryRelink')
			return False
		
		# Use proper utility functions to derive blend and sidecar paths
		# The lib_vault_rel might be a direct blend path, sidecar path, or legacy format
		lib_blend_vault_rel = get_blend_file_path_from_sidecar(lib_vault_rel)
		lib_sidecar_vault_rel = get_sidecar_path(lib_blend_vault_rel)

		# Validate that we have a proper blend file path
		if not lib_blend_vault_rel.endswith(".blend"):
			log_error(
				f"Failed to derive a valid .blend path from sidecar link '{lib_vault_rel}'. "
				f"Derived blend path: '{lib_blend_vault_rel}'. Cannot process this library entry.",
				module_name='LibraryRelink'
			)
			return False

		# Convert vault-relative blend path to absolute
		lib_abs_path = os.path.normpath(os.path.join(self.vault_root, lib_blend_vault_rel))
		lib_sidecar_abs_path = os.path.normpath(os.path.join(self.vault_root, lib_sidecar_vault_rel))
		
		# Verify the library BLEND file exists
		if not os.path.exists(lib_abs_path):
			log_warning(f"Library blend file does not exist: {lib_abs_path} (derived from link '{lib_vault_rel}')", module_name='LibraryRelink')
			return False
		
		# Compute Blender-relative path from original main blend location
		original_main_dir = os.path.dirname(self.original_main_blend_path)
		blender_rel_path = os.path.relpath(lib_abs_path, original_main_dir).replace(os.sep, '/')
		blender_lib_path = PathResolver.blender_relative_path(blender_rel_path)
		
		log_debug(f"Processing library '{lib_vault_rel}' (UUID: {stored_uuid})", module_name='LibraryRelink')
		log_debug(f"  Absolute path: {lib_abs_path}", module_name='LibraryRelink')
		log_debug(f"  Blender path: {blender_lib_path}", module_name='LibraryRelink')
		
		# Try to find existing library by UUID
		existing_lib = LibraryManager.find_library_by_uuid(stored_uuid)
		if existing_lib:
			return self._relink_existing_library(existing_lib, blender_lib_path, lib_abs_path, lib_vault_rel)
		
		# Try to find by filename
		filename = os.path.basename(lib_vault_rel)
		existing_lib = LibraryManager.find_library_by_filename(filename)
		if existing_lib:
			log_info(f"Found library by filename: {filename}", module_name='LibraryRelink')
			return self._relink_existing_library(existing_lib, blender_lib_path, lib_abs_path, lib_vault_rel)
		
		# Library not found in Blender session - try to load it
		return self._load_new_library(blender_lib_path, lib_abs_path, lib_vault_rel)

	def _is_library_missing(self, library: bpy.types.Library) -> bool:
		"""Checks if a library is considered missing."""
		# Blender 4.0+ has a direct 'is_missing' attribute
		if hasattr(library, 'is_missing'):
			if library.is_missing:
				log_debug(f"Library '{library.name}' identified as missing via library.is_missing.", module_name='LibraryRelink')
				return True
			else:
				# If Blender 4.0+ says it's not missing, but a custom flag (if it exists) says it is,
				# log a discrepancy but prioritize library.is_missing.
				if hasattr(library, 'library_path_load_failure') and getattr(library, 'library_path_load_failure', False):
					log_warning(f"Library '{library.name}': library.is_missing is False, but custom 'library_path_load_failure' is True. Trusting library.is_missing.", module_name='LibraryRelink')
				return False

		# Fallback for Blender < 4.0 or if 'is_missing' attribute is somehow not present
		if hasattr(library, 'library_path_load_failure'):
			is_missing_custom = getattr(library, 'library_path_load_failure', False)
			if is_missing_custom:
				log_debug(f"Library '{library.name}' identified as missing via custom 'library_path_load_failure' attribute.", module_name='LibraryRelink')
			return is_missing_custom
		
		log_warning(f"Library '{library.name}' has neither 'is_missing' (Blender 4.0+) nor 'library_path_load_failure' attribute. Cannot reliably determine missing status by these primary methods. Assuming not missing to prevent relink loops.", module_name='LibraryRelink')
		
		# Basic fallback: if filepath is empty, it's definitely an issue.
		if not library.filepath:
			log_debug(f"Library '{library.name}' considered missing due to empty filepath (fallback check).", module_name='LibraryRelink')
			return True
            
		return False # Default to not missing if specific flags/attributes aren't available or conclusive.

	def _relink_existing_library(
		self, 
		library: bpy.types.Library, 
		blender_lib_path: str, 
		lib_abs_path: str,
		lib_vault_rel: str
	) -> bool:
		log_info(f"Attempting to relink library '{library.name}' from '{blender_lib_path}' to '{lib_abs_path}' (vault rel: '{lib_vault_rel}')", module_name='LibraryRelink')
		
		original_custom_flag_state = None
		custom_flag_exists = hasattr(library, 'library_path_load_failure')
		if custom_flag_exists:
			original_custom_flag_state = getattr(library, 'library_path_load_failure', None)

		try:
			library.filepath = lib_abs_path
			library.reload()
			log_debug(f"Library '{library.name}' filepath set to '{lib_abs_path}' and reload() called.", module_name='LibraryRelink')

			# If reload() succeeded without error, and a custom flag 'library_path_load_failure' is being used,
			# assume the condition that set the flag is now resolved by the successful reload.
			if custom_flag_exists:
				log_debug(f"Resetting 'library_path_load_failure' for '{library.name}' after successful reload call, pending final check.", module_name='LibraryRelink')
				setattr(library, 'library_path_load_failure', False)
			
			# Now, check the definitive missing status using the updated _is_library_missing logic
			if self._is_library_missing(library):
				log_error(f"Relink failed for library '{library.name}'. Path set to '{lib_abs_path}', but library still appears missing after reload and flag reset.", module_name='LibraryRelink')
				# If it failed, ensure the custom flag (if it exists) reflects this failure.
				# This is important if _is_library_missing decided based on library.is_missing (Blender 4.0+),
				# we want the custom flag to be consistent if it's also present.
				if custom_flag_exists:
					setattr(library, 'library_path_load_failure', True) 
				return False
			
			log_success(f"Successfully relinked library '{library.name}' to '{lib_abs_path}'", module_name='LibraryRelink')
			return True

		except RuntimeError as e: # Blender often raises RuntimeError for file/library issues
			log_warning(f"RuntimeError during relink (set filepath or reload) for library '{library.name}' to '{lib_abs_path}': {e}", module_name='LibraryRelink')
			if custom_flag_exists: # Ensure custom flag is set to True on error
				setattr(library, 'library_path_load_failure', True)
			return False
		except Exception as e: # Catch any other unexpected errors
			log_error(f"Unexpected error during relink for library '{library.name}' to '{lib_abs_path}': {e}", module_name='LibraryRelink')
			if custom_flag_exists: # Ensure custom flag is set to True on error
				setattr(library, 'library_path_load_failure', True)
			return False

	def _load_new_library(self, blender_lib_path: Optional[str], lib_abs_path: str, lib_vault_rel: str) -> bool:
		# blender_lib_path is likely None if this method is called, indicating it's "new"
		log_warning(f"Attempted to 'load new library' for path '{lib_abs_path}' (from sidecar entry '{lib_vault_rel}').", module_name='LibraryRelink')
		log_warning("This typically means the library was in the sidecar but not found in Blender's current libraries (e.g., by UUID).", module_name='LibraryRelink')
		log_warning("Automatically 'loading' it as a new library reference is not supported by this relinker. Please link assets from this library manually if needed.", module_name='LibraryRelink')
		# Original code might have attempted bpy.ops.wm.link, which is not suitable for this general case.
		# For safety, we are not attempting to load it.
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
		log_error(f"Library relinking failed: {e}", module_name='LibraryRelink')
		traceback.print_exc()


# Make the handler persistent
relink_library_info.persistent = True


def relink_libraries(main_blend_path: str) -> None:
	"""
	Main entry point for library relinking.
	
	Args:
		main_blend_path: Absolute path to the main blend file
	"""
	try:
		processor = LibraryRelinkProcessor(main_blend_path)
		processor.process_relink()
	except Exception as e:
		log_error(f"Library relinking failed: {e}", module_name='LibraryRelink')


def execute_relink_operator(self, context: bpy.types.Context):
	"""Execute function for the relink operator."""
	if not self.sidecar_file_path:
		self.report({'ERROR'}, "Sidecar file path not provided.")
		return {'CANCELLED'}
	
	if not os.path.exists(self.sidecar_file_path):
		self.report({'ERROR'}, f"Sidecar file not found: {self.sidecar_file_path}")
		return {'CANCELLED'}
	
	log_info(f"Attempting to relink libraries from: {self.sidecar_file_path}", module_name='LibraryRelink')
	
	try:
		# Extract blend file path from sidecar path
		blend_path = get_blend_file_path_from_sidecar(self.sidecar_file_path)
		if not blend_path:
			self.report({'ERROR'}, f"Could not determine .blend file path from sidecar: {self.sidecar_file_path}")
			return {'CANCELLED'}
			
		processor = LibraryRelinkProcessor(blend_path)
		processor.process_relink()
		self.report({'INFO'}, "Library relinking process completed.")
		return {'FINISHED'}
	except Exception as e:
		log_error(f"Error during operator execution: {e}", module_name='LibraryRelink')
		self.report({'ERROR'}, f"Relinking failed: {e}")
		return {'CANCELLED'}


# Create the operator class using the factory function
BV_OT_RelinkLibraries: Any = create_blender_operator_class(
	'BV_OT_RelinkLibraries',
	'blend_vault.relink_libraries',
	'Relink Libraries from Sidecar',
	execute_relink_operator
)

# Add the sidecar_file_path property to the operator
if hasattr(BV_OT_RelinkLibraries, 'bl_rna'):
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

if hasattr(BV_OT_RelinkLibraries, 'bl_rna'):
	BV_OT_RelinkLibraries.invoke = invoke_file_select


def register():
	bpy.utils.register_class(BV_OT_RelinkLibraries)
	log_success("Library relinking operator registered.", module_name='LibraryRelink')


def unregister():
	bpy.utils.unregister_class(BV_OT_RelinkLibraries)
	log_warning("Library relinking operator unregistered.", module_name='LibraryRelink')


if __name__ == "__main__":
	register()
