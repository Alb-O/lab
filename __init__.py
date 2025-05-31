import bpy
import importlib

# Import preferences and utility functions
from .blend_vault import preferences, log_info, log_warning, log_error, log_success, log_debug

# Global variable to store preferences across reloads
# Use bpy.app.driver_namespace to persist data across module reloads
if 'blend_vault_stored_prefs' not in bpy.app.driver_namespace:
    bpy.app.driver_namespace['blend_vault_stored_prefs'] = {}

# Registry of app handlers: event name -> list of (module path, function name)
HANDLERS = {
    'save_post': [
        ('blend_vault.sidecar_io.writer', 'write_library_info'),
    ],
    # load_post handlers are now managed by the polling module to avoid conflicts
}

# List of modules that need their register/unregister functions called
MODULES_TO_REGISTER = [
    'blend_vault.relink.polling',  # Register polling module (includes redirect handler)
    'blend_vault.obsidian_integration',  # Register Obsidian integration features
    'blend_vault.paste_path',  # Register paste-path smart clipboard functionality
]

def register():
    package_name = __package__

    # --- Preferences Registration ---
    if preferences:
        try:
            importlib.reload(preferences)
            preferences.ADDON_PACKAGE_NAME = package_name
            if hasattr(preferences, 'BlendVaultPreferences'):
                preferences.BlendVaultPreferences.bl_idname = package_name
                bpy.utils.register_class(preferences.BlendVaultPreferences)
            else:
                log_error("BlendVaultPreferences class not found in preferences module.", module_name="Init")
            
            if hasattr(preferences, 'restore_preferences'):
                preferences.restore_preferences()
            else:
                log_error("restore_preferences function not found in preferences module.", module_name="Init")
        except Exception as e:
            log_error(f"Error during preferences registration/reload: {e}", module_name="Init")
    else:
        log_error("Preferences module not loaded. Skipping preferences registration.", module_name="Init")

    # --- Reload Submodules ---
    submodules_to_reload = [
        'blend_vault.sidecar_io.frontmatter',
        'blend_vault.sidecar_io.writer',
        'blend_vault.sidecar_io.collectors',
        'blend_vault.sidecar_io.content_builder',
        'blend_vault.sidecar_io.file_operations',
        'blend_vault.sidecar_io.uuid_manager',
        'blend_vault.relink',
        'blend_vault.paste_path.core_operators',
        'blend_vault.paste_path.asset_discovery',
        'blend_vault.paste_path.dialogs',
        'blend_vault.paste_path.file_validation',
        'blend_vault.paste_path.save_workflow',
        'blend_vault.paste_path.smart_paste',
    ]
    
    for module_path in submodules_to_reload:
        try:
            full_module_path = f"{package_name}.{module_path}"
            module_obj = importlib.import_module(full_module_path)
            if module_obj:
                importlib.reload(module_obj)
            else:
                log_info(f"Submodule {full_module_path} resolved to None during import. Skipping reload.", module_name="Init")
        except ImportError:
            log_info(f"Submodule {full_module_path} not found or failed to import. Skipping reload.", module_name="Init")
        except Exception as e:
            log_error(f"Error reloading submodule {full_module_path}: {e}", module_name="Init")

    # --- Register Modules with their own register functions ---
    for module_path in MODULES_TO_REGISTER:
        try:
            full_module_path = f"{package_name}.{module_path}"
            imported_module = importlib.import_module(full_module_path)
            if imported_module:
                reloaded_module = importlib.reload(imported_module)
                if hasattr(reloaded_module, 'register'):
                    reloaded_module.register()
                else:
                    log_warning(f"Module {full_module_path} has no register function.", module_name="Init")
            else:
                log_warning(f"Module {full_module_path} for registration resolved to None. Skipping.", module_name="Init")
        except ImportError:
            log_info(f"Module {full_module_path} for registration not found or failed to import. Skipping.", module_name="Init")
        except Exception as e:
            log_error(f"Failed to register module {full_module_path}: {e}", module_name="Init")    # --- Reload and Register Handlers ---
    for event, entries in HANDLERS.items():
        handler_list = getattr(bpy.app.handlers, event)
        log_debug(f"Registering handlers for event '{event}', current count: {len(handler_list)}", module_name="Init")
        for module_path, fn_name in entries:
            try:
                full_module_path = f"{package_name}.{module_path}"
                log_debug(f"Attempting to import handler module: {full_module_path}", module_name="Init")
                handler_module_obj = importlib.import_module(full_module_path)
                if handler_module_obj:
                    log_debug(f"Successfully imported {full_module_path}, reloading...", module_name="Init")
                    reloaded_handler_module = importlib.reload(handler_module_obj)
                    if hasattr(reloaded_handler_module, fn_name):
                        fn = getattr(reloaded_handler_module, fn_name)
                        globals()[fn_name] = fn  # Store reloaded function in globals for unregistration
                        if fn not in handler_list:
                            handler_list.append(fn)
                            log_success(f"Successfully registered handler {fn_name} for event '{event}'", module_name="Init")
                        else:
                            log_warning(f"Handler {fn_name} already registered for event '{event}'", module_name="Init")
                    else:
                        log_error(f"Handler function {fn_name} not found in module {full_module_path}.", module_name="Init")
                else:
                    log_warning(f"Handler module {full_module_path} resolved to None during import. Skipping handler registration.", module_name="Init")
            except ImportError as e:
                log_error(f"Handler module {full_module_path} import failed: {e}. Skipping handler.", module_name="Init")
            except Exception as e:
                log_error(f"Failed to load/register handler {fn_name} from {full_module_path}: {e}", module_name="Init")

    log_success("Main addon functionalities registered.", module_name="Init")
    debug_handler_status()  # Debug: Check handler registration


def unregister():
    # --- Unregister Preferences ---
    if preferences:
        try:
            if hasattr(preferences, 'store_preferences'):
                preferences.store_preferences()
            else:
                log_warning("store_preferences function not found in preferences module for unregistration.", module_name="Init")
            
            if hasattr(preferences, 'BlendVaultPreferences'):
                # Check if class is actually registered before trying to unregister
                # bpy.utils.unregister_class can error if class not registered.
                # A more robust check would be to see if it's in bpy.types
                # For now, assume if preferences and class exist, it was registered.
                bpy.utils.unregister_class(preferences.BlendVaultPreferences)
            else:
                log_warning("BlendVaultPreferences class not found in preferences module for unregistration.", module_name="Init")
        except Exception as e:
            log_error(f"Error during preferences unregistration: {e}", module_name="Init")
    else:
        log_warning("Preferences module not loaded. Skipping preferences unregistration.", module_name="Init")

    package_name = __package__ or "blend_vault_ext"

    # --- Unregister Modules with their own unregister functions ---
    for module_path in MODULES_TO_REGISTER:
        try:
            full_module_path = f"{package_name}.{module_path}"
            module_obj = importlib.import_module(full_module_path)  # No reload needed for unregister
            if module_obj and hasattr(module_obj, 'unregister'):
                module_obj.unregister()
            elif module_obj:
                log_warning(f"Module {full_module_path} has no unregister function.", module_name="Init")
            # If module_obj is None, import_module likely failed, error caught by except
        except ImportError:
            log_info(f"Module {full_module_path} for unregistration not found or failed to import. Skipping.", module_name="Init")
        except Exception as e:
            log_error(f"Failed to unregister module {full_module_path}: {e}", module_name="Init")

    # --- Unregister Handlers ---
    for event, entries in HANDLERS.items():
        if hasattr(bpy.app.handlers, event):
            handler_list = getattr(bpy.app.handlers, event)
            for module_path, fn_name in entries:
                try:
                    # Attempt to retrieve the function from globals() first, as it was stored there during registration
                    fn_to_remove = globals().get(fn_name)
                    
                    if not fn_to_remove:
                        # Fallback: if not in globals, try to get it from a fresh import of the module
                        # This might not be the exact same object if module was reloaded and original not cleaned from globals
                        full_module_path = f"{package_name}.{module_path}"
                        handler_module_obj = importlib.import_module(full_module_path)
                        if handler_module_obj and hasattr(handler_module_obj, fn_name):
                            fn_to_remove = getattr(handler_module_obj, fn_name)
                    
                    if fn_to_remove and fn_to_remove in handler_list:
                        handler_list.remove(fn_to_remove)
                    elif fn_to_remove:
                        log_warning(f"Handler {fn_name} was found but not in the active handler list for event '{event}'.", module_name="Init")
                    # If fn_to_remove is None here, it means it wasn't found in globals or module.
                except ImportError:
                    log_info(f"Handler module {module_path} for unregistration not found. Skipping handler {fn_name}.", module_name="Init")
                except Exception as e:
                    log_error(f"Failed to unregister handler {fn_name} from {module_path}: {e}", module_name="Init")
        else:
            log_warning(f"Event type '{event}' not found in bpy.app.handlers during unregistration.", module_name="Init")
            
    log_warning("Main addon functionalities unregistered.", module_name="Init")


def debug_handler_status():
    """Debug function to check if handlers are properly registered."""
    from . import log_info, log_warning
    
    save_post_handlers = bpy.app.handlers.save_post
    log_info(f"Total save_post handlers registered: {len(save_post_handlers)}", module_name="Init")
    
    # Look for our handler specifically
    write_library_info_found = False
    for handler in save_post_handlers:
        handler_name = getattr(handler, '__name__', 'unknown')
        handler_module = getattr(handler, '__module__', 'unknown')
        log_info(f"Handler: {handler_name} from module: {handler_module}", module_name="Init")
        if handler_name == 'write_library_info':
            write_library_info_found = True
    
    if write_library_info_found:
        log_info("write_library_info handler is registered!", module_name="Init")
    else:
        log_warning("write_library_info handler NOT found in save_post handlers!", module_name="Init")


if __name__ == "__main__":
    register()

log_success("Script loaded.", module_name="Init")