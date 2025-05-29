import bpy # type: ignore
import importlib

# Import preferences, operator, and logger components
from .fly_nav import preferences
from .fly_nav import operators # Import the operators module
from .fly_nav import logger # Import the logger module
from .fly_nav.preferences import FlyNavPreferences, FlyNavRefreshKeymapsOperator
from .fly_nav.operators import FLYNAV_OT_simple_fly, FLYNAV_OT_right_mouse_navigation
from .fly_nav.keymaps import (
	register_keymaps,
	unregister_keymaps,
	_disable_default_rmb_menus,
	_restore_default_rmb_menus,
	_modify_walk_modal_keymaps
)

# Global variable to store preferences across reloads
if 'fly_nav_stored_prefs' not in bpy.app.driver_namespace:
	bpy.app.driver_namespace['fly_nav_stored_prefs'] = {}

MODULES_TO_REGISTER = [
	'fly_nav.operators',
	# Add other modules that have their own register/unregister functions here
]

addon_keymaps = []  # Stores (keymap, keymap_item) tuples for cleanup
original_keymap_states = {}  # Store references to original keymap items

def get_prefs(context):
	return context.preferences.addons[__package__].preferences

def register():
	package_name = __package__
	if package_name:
		logger.set_package_name(package_name)
		# Optionally, set a default log level or load from preferences if implemented
		# logger.set_log_level("INFO") 
	else:
		# Fallback logger name if package_name is None (should not happen in normal addon operation)
		logger.set_package_name("fly_nav_ext_unknown_pkg")
		logger.log_warning("__package__ is None, logging might not be ideally configured.")

	# --- Preferences Registration ---
	if preferences:
		try:
			importlib.reload(preferences)
			if package_name:  # Ensures package_name is not None and not an empty string
				preferences.ADDON_PACKAGE_NAME = package_name
				if hasattr(preferences, 'FlyNavPreferences'):
					preferences.FlyNavPreferences.bl_idname = package_name
					bpy.utils.register_class(preferences.FlyNavPreferences)
				else:
					logger.log_error(f"FlyNavPreferences class not found in {package_name}.preferences module.")

				if hasattr(preferences, 'restore_preferences'):
					preferences.restore_preferences(bpy.context)
				else:
					logger.log_error(f"restore_preferences function not found in {package_name}.preferences module.")
			else:
				logger.log_error("CRITICAL ERROR: __package__ is not set. Preferences cannot be registered correctly.")
		except Exception as e:
			pkg_name_for_log = package_name if package_name else "fly_nav_ext (package name not resolved)"
			logger.log_error(f"Error during preferences registration/reload for {pkg_name_for_log}: {e}")
	else:
		pkg_name_for_log = package_name if package_name else "fly_nav_ext (package name not resolved)"
		logger.log_error(f"Preferences module not loaded for {pkg_name_for_log}. Skipping preferences registration.")

	# --- Register Modules with their own register functions ---
	for module_path in MODULES_TO_REGISTER:
		try:
			full_module_path = f"{package_name}.{module_path}" if package_name else module_path
			imported_module = importlib.import_module(full_module_path)
			if imported_module:
				reloaded_module = importlib.reload(imported_module)
				if hasattr(reloaded_module, 'register'):
					reloaded_module.register()
				else:
					logger.log_warning(f"Module {full_module_path} has no register function.")
			else:
				logger.log_warning(f"Module {full_module_path} for registration resolved to None. Skipping.")
		except ImportError:
			logger.log_info(f"Module {full_module_path} for registration not found or failed to import. Skipping.")
		except Exception as e:
			logger.log_error(f"Failed to register module {full_module_path}: {e}")

	# Register FlyNavRefreshKeymapsOperator
	bpy.utils.register_class(FlyNavRefreshKeymapsOperator)

	# Register keymaps
	register_keymaps()

	logger.log_info(f"Registered {package_name or 'fly_nav_ext'} successfully.")


def unregister():
	package_name = __package__
	pkg_name_for_log = package_name or "fly_nav_ext"


	# --- Unregister Modules (in reverse order of registration) ---
	for module_path in reversed(MODULES_TO_REGISTER):
		try:
			full_module_path = f"{package_name}.{module_path}" if package_name else module_path
			imported_module = importlib.import_module(full_module_path)
			if imported_module:
				if hasattr(imported_module, 'unregister'):
					imported_module.unregister()
				else:
					logger.log_warning(f"Module {full_module_path} has no unregister function.")
		except ImportError:
			logger.log_info(f"Module {full_module_path} for unregistration not found. Skipping.")
		except Exception as e:
			logger.log_error(f"Failed to unregister module {full_module_path}: {e}")


	# Unregister FlyNavRefreshKeymapsOperator
	bpy.utils.unregister_class(FlyNavRefreshKeymapsOperator)

	# Unregister keymaps
	unregister_keymaps()

	# --- Preferences Unregistration ---
	if preferences:
		try:
			if hasattr(preferences, 'store_preferences'):
				preferences.store_preferences(bpy.context) # Store before unregistering
			else:
				logger.log_error(f"store_preferences function not found in {pkg_name_for_log}.preferences module.")

			if hasattr(preferences, 'FlyNavPreferences'):
				bpy.utils.unregister_class(preferences.FlyNavPreferences)
		except Exception as e:
			logger.log_error(f"Error during preferences unregistration for {pkg_name_for_log}: {e}")

	logger.log_info(f"Unregistered {pkg_name_for_log} successfully.")

if __name__ == "__main__":
	# This block is for direct script execution, __package__ might be None here.
	# The logger will use a default name or the one set if register() is called.
	register()
