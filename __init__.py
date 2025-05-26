bl_info = {
	"name": "Blend Vault",
	"author": "Albert O'Shea",
	"version": (0, 4, 0),
	"blender": (4, 0, 0),
	"description": "Automatically relink and manage libraries/assets with Obsidian integration",
	"category": "Development",
}

import sys
import os
# Ensure the addon root directory is in sys.path for package imports
addon_dir = os.path.dirname(os.path.abspath(__file__))
if addon_dir not in sys.path:
	sys.path.append(addon_dir)

import bpy  # type: ignore
import importlib

# Import preferences and utility functions
from . import preferences  # Import the new preferences module

# Dynamically import utils and LOG_COLORS for consistent access
utils = importlib.import_module('utils')
LOG_COLORS = utils.LOG_COLORS
log_info = utils.log_info
log_warning = utils.log_warning
log_error = utils.log_error
log_success = utils.log_success
log_debug = utils.log_debug

# Global variable to store preferences across reloads
# Use bpy.app.driver_namespace to persist data across module reloads
if 'blend_vault_stored_prefs' not in bpy.app.driver_namespace:
    bpy.app.driver_namespace['blend_vault_stored_prefs'] = {}

# Registry of app handlers: event name -> list of (module path, function name)
HANDLERS = {
	'save_post': [
		('sidecar_io.writer', 'write_library_info'),
	],
	# load_post handlers are now managed by the polling module to avoid conflicts
}

# List of modules that need their register/unregister functions called
MODULES_TO_REGISTER = [
	'relink.polling',  # Register polling module (includes redirect handler)
	'obsidian_integration',  # Register Obsidian integration features
]


def register():
	# Reload preferences module to get latest class definition
	importlib.reload(preferences)
	
	# Register preferences class
	bpy.utils.register_class(preferences.BlendVaultPreferences)
	
	# Restore stored preference values
	preferences.restore_preferences()
	
	# Reload submodules first (important for dependencies)
	submodules_to_reload = [
		'sidecar_io.frontmatter',  # Reload frontmatter before writer
		'relink',  # Import relink package first
	]
	
	for module_path in submodules_to_reload:
		try:
			importlib.reload(importlib.import_module(module_path))
		except ImportError:
			pass  # Module might not be imported yet
	
	# Register modules that have their own register/unregister functions
	for module_path in MODULES_TO_REGISTER:
		try:
			module = importlib.reload(importlib.import_module(module_path))
			if hasattr(module, 'register'):
				module.register()
		except Exception as e:
			log_error(f"[Blend Vault] Failed to register module {module_path}: {e}")
	
	# Reload and register handlers from HANDLERS registry
	for event, entries in HANDLERS.items():
		handler_list = getattr(bpy.app.handlers, event)
		for module_path, fn_name in entries:
			module = importlib.reload(importlib.import_module(module_path))
			fn = getattr(module, fn_name)
			globals()[fn_name] = fn
			if fn not in handler_list:
				handler_list.append(fn)
				
	log_success("[Blend Vault] Main addon functionalities registered.")

def unregister():
	# Store preference values before unregistering
	preferences.store_preferences()
	
	# Unregister preferences
	bpy.utils.unregister_class(preferences.BlendVaultPreferences)
	
	# Unregister modules that have their own register/unregister functions
	for module_path in MODULES_TO_REGISTER:
		try:
			module = importlib.import_module(module_path)
			if hasattr(module, 'unregister'):
				module.unregister()
		except Exception as e:
			log_error(f"[Blend Vault] Failed to unregister module {module_path}: {e}")
	
	# Unregister handlers based on HANDLERS registry
	for event, entries in HANDLERS.items():
		handler_list = getattr(bpy.app.handlers, event)
		for module_path, fn_name in entries:
			fn = globals().get(fn_name) or getattr(importlib.import_module(module_path), fn_name)
			if fn in handler_list:
				handler_list.remove(fn)
	log_warning("[Blend Vault] Main addon functionalities unregistered.")

if __name__ == "__main__":
	register()

log_success("[Blend Vault] Script loaded.")