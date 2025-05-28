import bpy  # type: ignore
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
    # Set the package name for preferences
    package_name = __package__
    # Reload preferences module to get latest class definition
    importlib.reload(preferences)

    # NOW set the necessary attributes on the reloaded module's contents
    preferences.ADDON_PACKAGE_NAME = package_name
    preferences.BlendVaultPreferences.bl_idname = package_name # This is crucial

    # Register preferences class
    bpy.utils.register_class(preferences.BlendVaultPreferences)

    # Restore stored preference values
    preferences.restore_preferences()    # Reload submodules first (important for dependencies)
    submodules_to_reload = [
        'blend_vault.sidecar_io.frontmatter',  # Reload frontmatter before writer
        'blend_vault.relink',  # Import relink package first
        'blend_vault.paste_path.core_operators',  # Reload paste_path components
        'blend_vault.paste_path.asset_discovery',
        'blend_vault.paste_path.dialogs',
        'blend_vault.paste_path.file_validation',
        'blend_vault.paste_path.save_workflow',
        'blend_vault.paste_path.smart_paste',
    ]
    for module_path in submodules_to_reload:
        try:
            full_module_path = f"{package_name}.{module_path}"
            importlib.reload(importlib.import_module(full_module_path))
        except ImportError:
            pass  # Module might not be imported yet

    # Register modules that have their own register/unregister functions
    for module_path in MODULES_TO_REGISTER:
        try:
            full_module_path = f"{package_name}.{module_path}"
            module = importlib.reload(importlib.import_module(full_module_path))
            if hasattr(module, 'register'):
                module.register()
        except Exception as e:
            log_error(f"[Blend Vault] Failed to register module {full_module_path}: {e}")

    # Reload and register handlers from HANDLERS registry
    for event, entries in HANDLERS.items():
        handler_list = getattr(bpy.app.handlers, event)
        for module_path, fn_name in entries:
            full_module_path = f"{package_name}.{module_path}"
            module = importlib.reload(importlib.import_module(full_module_path))
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
    
    # Get package name for building absolute module paths
    package_name = __package__ or "blend_vault_ext"  # Fallback for development

    # Unregister modules that have their own register/unregister functions
    for module_path in MODULES_TO_REGISTER:
        try:
            full_module_path = f"{package_name}.{module_path}"
            module = importlib.import_module(full_module_path)
            if hasattr(module, 'unregister'):
                module.unregister()
        except Exception as e:
            log_error(f"[Blend Vault] Failed to unregister module {full_module_path}: {e}")

    # Unregister handlers based on HANDLERS registry
    for event, entries in HANDLERS.items():
        handler_list = getattr(bpy.app.handlers, event)
        for module_path, fn_name in entries:
            full_module_path = f"{package_name}.{module_path}"
            fn = globals().get(fn_name) or getattr(importlib.import_module(full_module_path), fn_name)
            if fn in handler_list:
                handler_list.remove(fn)
    log_warning("[Blend Vault] Main addon functionalities unregistered.")


if __name__ == "__main__":
    register()

log_success("[Blend Vault] Script loaded.")