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
	create_blender_operator_class,
	get_blend_file_path_from_sidecar
)


class LibraryRelinkProcessor:
	"""Handles library relinking using vault-root-relative paths."""
	
	def __init__(self, main_blend_path: str):
		# Get vault root and fail fast if not available
		vault_root = get_obsidian_vault_root()
		if not vault_root:
			raise ValueError("Obsidian vault root is not configured. Library relinking requires a configured vault root.")
		self.vault_root = vault_root
		
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
		"""Main entry point for library relinking process."""		# Skip if sidecar doesn't exist
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
		json_data = lib_data["json_data"]
		stored_uuid = json_data.get("uuid")
		
		if not stored_uuid or stored_uuid == "MISSING_HASH":
			if stored_uuid == "MISSING_HASH":
				log_info(f"Library '{lib_vault_rel}' has 'MISSING_HASH'. Skipping.", module_name='LibraryRelink')
			else:
				log_warning(f"Library '{lib_vault_rel}': Missing or invalid UUID", module_name='LibraryRelink')
			return False
		
		# Handle link path: if it ends with .side.md, extract the actual blend path
		if lib_vault_rel.endswith(SIDECAR_EXTENSION):
			# This is a link to the sidecar file, extract the blend path
			lib_blend_vault_rel = lib_vault_rel[:-len(SIDECAR_EXTENSION)]
			lib_sidecar_vault_rel = lib_vault_rel
		else:
			# This is a direct blend file path (legacy format)
			lib_blend_vault_rel = lib_vault_rel
			lib_sidecar_vault_rel = lib_vault_rel + SIDECAR_EXTENSION
		
		# Convert vault-relative blend path to absolute
		lib_abs_path = os.path.normpath(os.path.join(self.vault_root, lib_blend_vault_rel))
		lib_sidecar_abs_path = os.path.normpath(os.path.join(self.vault_root, lib_sidecar_vault_rel))
		
		# Verify the library file exists
		if not os.path.exists(lib_abs_path):
			log_warning(f"Library file does not exist: {lib_abs_path}", module_name='LibraryRelink')
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

	def _relink_existing_library(
		self, 
		library: bpy.types.Library, 
		blender_lib_path: str, 
		lib_abs_path: str,
		lib_vault_rel: str
	) -> bool:
		"""Relink an existing library to the correct vault-root-relative path."""
		# Normalize paths for comparison
		current_lib_path_normalized = PathResolver.normalize_path(library.filepath)
		new_lib_path_normalized = PathResolver.normalize_path(blender_lib_path)
		
		log_debug(f"Relinking check for '{library.name}':", module_name='LibraryRelink')
		log_debug(f"  Current: '{library.filepath}' -> '{current_lib_path_normalized}'", module_name='LibraryRelink')
		log_debug(f"  Target: '{blender_lib_path}' -> '{new_lib_path_normalized}'", module_name='LibraryRelink')
		
		if current_lib_path_normalized != new_lib_path_normalized:
			log_info(f"Relinking '{library.name}' from '{library.filepath}' to '{blender_lib_path}'", module_name='LibraryRelink')
			library.filepath = blender_lib_path
			
			try:
				library.reload()
				log_success(f"Successfully reloaded library '{library.name}' from {lib_vault_rel}", module_name='LibraryRelink')
				return True
			except Exception as e:
				log_error(f"Failed to reload '{library.name}': {e}", module_name='LibraryRelink')
				return False
		else:
			# Paths match, but check if library is still missing/broken
			is_missing = self._is_library_missing(library)
			if is_missing:
				log_info(f"Library '{library.name}' path matches but is missing - forcing reload", module_name='LibraryRelink')
				try:
					library.reload()
					log_success(f"Successfully reloaded missing library '{library.name}'", module_name='LibraryRelink')
					return True
				except Exception as e:
					log_error(f"Failed to reload missing library '{library.name}': {e}", module_name='LibraryRelink')
					return False
			else:
				log_info(f"Library '{library.name}' is already correctly linked", module_name='LibraryRelink')
				return True

	def _load_new_library(self, blender_lib_path: str, lib_abs_path: str, lib_vault_rel: str) -> bool:
		"""
		Load a new library that isn't currently in the Blender session.
		
		Note: This method attempts to load the library but doesn't automatically link all assets.
		The library will be available for manual linking or will be picked up by asset relinking.
		"""
		log_info(f"Loading new library: {lib_vault_rel}", module_name='LibraryRelink')
		log_debug(f"  Blender path: {blender_lib_path}", module_name='LibraryRelink')
		log_debug(f"  Absolute path: {lib_abs_path}", module_name='LibraryRelink')
		
		try:
			# Load the library without linking specific assets
			# This makes the library available in Blender's library list
			with bpy.data.libraries.load(lib_abs_path, link=False, relative=True) as (data_from, data_to):
				# Don't link any specific assets - just load the library reference
				pass
			
			log_success(f"Successfully loaded new library: {lib_vault_rel}", module_name='LibraryRelink')
			return True
			
		except Exception as e:
			log_error(f"Failed to load new library '{lib_vault_rel}': {e}", module_name='LibraryRelink')
			return False

	def _is_library_missing(self, library: bpy.types.Library) -> bool:
		"""Check if a library is missing or broken."""
		# Check Blender's built-in missing flag
		if hasattr(library, 'is_missing') and library.is_missing:
			return True
		
		# Fallback: check if file exists
		abs_path = PathResolver.resolve_blender_path(library.filepath)
		return not os.path.exists(abs_path)
	
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
