import bpy
from typing import Optional, cast # Added cast
from .utils.helpers import log_info, log_warning, log_error, log_success, log_debug

# Global variable to store the addon package name
# This will be set by the main addon module during registration
ADDON_PACKAGE_NAME = ""

# Storage key for persistent preferences across reloads
STORAGE_KEY = 'blend_vault_stored_prefs'

# List of preference properties that should be preserved across reloads
PERSISTENT_PROPERTIES = [
    'obsidian_vault_root',
    # Add more preference property names here as needed
]

class BlendVaultPreferences(bpy.types.AddonPreferences):
    """Blend Vault addon preferences"""
    bl_idname = ""  # This will be set dynamically during registration

    obsidian_vault_root: bpy.props.StringProperty(  # type: ignore
        name="Obsidian Vault Root",
        description="Path to the root directory of your Obsidian vault",
        default="",
        subtype='DIR_PATH'
    )

    def draw(self, context):
        layout = self.layout

        layout.prop(self, "obsidian_vault_root")
        
        # Info section
        if not self.obsidian_vault_root:
            warning_box = layout.box()
            warning_box.label(text="Setting the path of your Obsidian vault is only for displaying vault-relative paths in the UI.", icon='INFO')

def get_addon_preferences(context=None) -> Optional[BlendVaultPreferences]:
    """
    Get the Blend Vault addon preferences.
    Returns the preferences object (instance of BlendVaultPreferences) or None.
    """
    if context is None:
        context = bpy.context
    
    addon = context.preferences.addons.get(ADDON_PACKAGE_NAME)
    if addon and hasattr(addon, 'preferences') and isinstance(addon.preferences, BlendVaultPreferences):
        # Explicitly cast to BlendVaultPreferences after checking type
        return cast(BlendVaultPreferences, addon.preferences)
    else:
        if addon and hasattr(addon, 'preferences') and not isinstance(addon.preferences, BlendVaultPreferences):
            log_warning("Addon preferences found, but not of expected type BlendVaultPreferences. Type was {type(addon.preferences)}", module_name='Preferences')
        elif not addon:
            log_warning("Could not find addon preferences. Make sure addon is enabled.", module_name='Preferences')
        else: # Addon found, but no 'preferences' attribute
            log_warning("Addon found, but it has no 'preferences' attribute.", module_name='Preferences')
        return None

def get_obsidian_vault_root(context=None) -> Optional[str]:
    """
    Get the Obsidian vault root path from addon preferences.
    Returns the path as a string, or None if not set or preferences not found.
    """
    prefs: Optional[BlendVaultPreferences] = get_addon_preferences(context)
    if prefs:
        # Now that prefs is correctly typed as BlendVaultPreferences | None,
        # Pylance should recognize obsidian_vault_root directly.
        # The hasattr check is good for robustness if properties are dynamic.
        if hasattr(prefs, 'obsidian_vault_root'):
            vault_root_value = prefs.obsidian_vault_root
            if isinstance(vault_root_value, str):
                stripped_vault_root = vault_root_value.strip()
                return stripped_vault_root if stripped_vault_root else None
            elif vault_root_value is not None:
                # Attempt to convert to string if it's not None and not a string
                try:
                    str_vault_root = str(vault_root_value)
                    stripped_vault_root = str_vault_root.strip()
                    return stripped_vault_root if stripped_vault_root else None
                except Exception as e:
                    log_error(f"Error converting vault root to string: {e}", module_name='Preferences')
                    return None
        return None # obsidian_vault_root attribute doesn't exist
    return None # Preferences object not found

def store_preferences():
    """
    Store current preference values in persistent storage for later restoration.
    This should be called before unregistering the preferences class.
    """
    if STORAGE_KEY not in bpy.app.driver_namespace:
        bpy.app.driver_namespace[STORAGE_KEY] = {}
    
    stored_prefs = bpy.app.driver_namespace[STORAGE_KEY]
    
    try:
        existing_prefs: Optional[BlendVaultPreferences] = get_addon_preferences()
        if existing_prefs:
            for prop_name in PERSISTENT_PROPERTIES:
                if hasattr(existing_prefs, prop_name):
                    prop_value = getattr(existing_prefs, prop_name)
                    stored_prefs[prop_name] = prop_value
                    log_info(f"Stored preference '{prop_name}': {prop_value}", module_name='Preferences')
    except Exception as e:
        log_error(f"Failed to store preferences: {e}", module_name='Preferences')

def restore_preferences():
    """
    Restore previously stored preference values.
    This should be called after registering the preferences class.
    """
    if STORAGE_KEY not in bpy.app.driver_namespace:
        return
    
    stored_prefs = bpy.app.driver_namespace[STORAGE_KEY]
    
    try:
        current_prefs: Optional[BlendVaultPreferences] = get_addon_preferences()
        if current_prefs:
            for prop_name in PERSISTENT_PROPERTIES:
                if prop_name in stored_prefs:
                    prop_value = stored_prefs[prop_name]
                    if hasattr(current_prefs, prop_name):
                        setattr(current_prefs, prop_name, prop_value)
                        log_info(f"Restored preference '{prop_name}': {prop_value}", module_name='Preferences')
    except Exception as e:
        log_error(f"Failed to restore preferences: {e}", module_name='Preferences')
