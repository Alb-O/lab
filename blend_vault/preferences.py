import bpy
import os
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
        name="Obsidian Vault Root (Manual Override)",
        description="Manually set the path to your Obsidian vault root (only used if auto-detection fails)",
        default="",
        subtype='DIR_PATH'
    )

    def draw(self, context):
        layout = self.layout

        # Check for auto-detected vault
        detected_vault = detect_obsidian_vault_from_asset_libraries()
        
        if detected_vault:
            # Show auto-detected vault info
            detection_box = layout.box()
            detection_box.label(text="Auto-detected Obsidian Vault:", icon='CHECKMARK')
            detection_box.label(text=f"Path: {detected_vault}")
            
            # Manual override section
            layout.separator()
            override_box = layout.box()
            override_box.label(text="Manual Override (optional):", icon='SETTINGS')
            override_box.prop(self, "obsidian_vault_root", text="Custom Path")
            
            if self.obsidian_vault_root and self.obsidian_vault_root.strip():
                override_box.label(text="Note: Manual path will be used as fallback only.", icon='INFO')
        else:
            # No auto-detection, show manual input
            no_detection_box = layout.box()
            no_detection_box.label(text="No Obsidian vault auto-detected in asset libraries", icon='ERROR')
            no_detection_box.label(text="Add your vault folder as an asset library, or set path manually below:")
            
            layout.separator()
            layout.prop(self, "obsidian_vault_root")
            
            if not self.obsidian_vault_root:
                warning_box = layout.box()
                warning_box.label(text="Tip: Set up your Obsidian vault as a Blender asset library for automatic detection.", icon='LIGHT')

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

def detect_obsidian_vault_from_asset_libraries() -> Optional[str]:
    """
    Detect Obsidian vault root by checking asset library paths for .obsidian folders.
    Returns the first vault root found, or None if no vault is detected.
    """
    try:
        # Get user preferences
        user_prefs = bpy.context.preferences
        
        # Check if asset libraries exist
        if not hasattr(user_prefs, 'filepaths') or not hasattr(user_prefs.filepaths, 'asset_libraries'):
            log_debug("No asset libraries found in user preferences", module_name='Preferences')
            return None
        
        asset_libraries = user_prefs.filepaths.asset_libraries
        
        if not asset_libraries:
            log_debug("Asset libraries collection is empty", module_name='Preferences')
            return None
        
        log_debug(f"Checking {len(asset_libraries)} asset libraries for Obsidian vaults", module_name='Preferences')
        
        for library in asset_libraries:
            if hasattr(library, 'path') and library.path:
                library_path = library.path.strip()
                if library_path:
                    # Check if this path contains a .obsidian folder
                    obsidian_folder_path = os.path.join(library_path, '.obsidian')
                    
                    if os.path.exists(obsidian_folder_path) and os.path.isdir(obsidian_folder_path):
                        log_success(f"Found Obsidian vault at: {library_path}", module_name='Preferences')
                        return library_path
                    
                    # Also check the parent directory
                    # Remove trailing slashes to ensure dirname works correctly
                    normalized_library_path = library_path.rstrip(os.sep).rstrip('/')
                    parent_path = os.path.dirname(normalized_library_path)
                    
                    if parent_path and parent_path != normalized_library_path:  # Avoid infinite loop at root
                        parent_obsidian_folder = os.path.join(parent_path, '.obsidian')
                        
                        if os.path.exists(parent_obsidian_folder) and os.path.isdir(parent_obsidian_folder):
                            log_success(f"Found Obsidian vault at parent directory: {parent_path}", module_name='Preferences')
                            return parent_path
        
        log_info("No Obsidian vault detected in asset library paths", module_name='Preferences')
        return None
        
    except Exception as e:
        log_error(f"Error detecting Obsidian vault from asset libraries: {e}", module_name='Preferences')
        return None

def get_obsidian_vault_info() -> dict:
    """
    Get detailed information about Obsidian vault detection.
    Returns a dictionary with detection results and asset library info.
    """
    info = {
        'detected_vault': None,
        'asset_libraries_checked': 0,
        'asset_libraries_found': []
    }
    
    try:
        user_prefs = bpy.context.preferences
        
        if not hasattr(user_prefs, 'filepaths') or not hasattr(user_prefs.filepaths, 'asset_libraries'):
            return info
        
        asset_libraries = user_prefs.filepaths.asset_libraries
        info['asset_libraries_checked'] = len(asset_libraries) if asset_libraries else 0
        
        if asset_libraries:
            for library in asset_libraries:
                if hasattr(library, 'path') and library.path:
                    library_path = library.path.strip()
                    library_name = library.name if hasattr(library, 'name') else 'Unknown'
                    
                    library_info = {
                        'name': library_name,
                        'path': library_path,
                        'is_obsidian_vault': False
                    }
                    
                    if library_path:
                        obsidian_folder_path = os.path.join(library_path, '.obsidian')
                        if os.path.exists(obsidian_folder_path) and os.path.isdir(obsidian_folder_path):
                            library_info['is_obsidian_vault'] = True
                            if info['detected_vault'] is None:
                                info['detected_vault'] = library_path
                    
                    info['asset_libraries_found'].append(library_info)
    
    except Exception as e:
        log_error(f"Error getting vault info: {e}", module_name='Preferences')
    
    return info

def get_obsidian_vault_root(context=None) -> Optional[str]:
    """
    Get the Obsidian vault root path. First tries to auto-detect from asset libraries,
    then falls back to manually set preferences.
    Returns the path as a string, or None if not found.
    """
    # First, try auto-detection from asset libraries
    detected_vault = detect_obsidian_vault_from_asset_libraries()
    if detected_vault:
        return detected_vault
    
    # Fall back to manually set preference
    prefs: Optional[BlendVaultPreferences] = get_addon_preferences(context)
    if prefs:
        # Now that prefs is correctly typed as BlendVaultPreferences | None,
        # Pylance should recognize obsidian_vault_root directly.
        # The hasattr check is good for robustness if properties are dynamic.
        if hasattr(prefs, 'obsidian_vault_root'):
            vault_root_value = prefs.obsidian_vault_root
            if isinstance(vault_root_value, str):
                stripped_vault_root = vault_root_value.strip()
                if stripped_vault_root:
                    log_info(f"Using manually configured vault root: {stripped_vault_root}", module_name='Preferences')
                    return stripped_vault_root
            elif vault_root_value is not None:
                # Attempt to convert to string if it's not None and not a string
                try:
                    str_vault_root = str(vault_root_value)
                    stripped_vault_root = str_vault_root.strip()
                    if stripped_vault_root:
                        log_info(f"Using manually configured vault root: {stripped_vault_root}", module_name='Preferences')
                        return stripped_vault_root
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

def refresh_vault_detection() -> Optional[str]:
    """
    Manually refresh vault detection and return the result.
    Useful for operators or UI elements that want to re-check asset libraries.
    """
    detected_vault = detect_obsidian_vault_from_asset_libraries()
    if detected_vault:
        log_success(f"Vault detection refreshed - found vault at: {detected_vault}", module_name='Preferences')
    else:
        log_info("Vault detection refreshed - no vault found in asset libraries", module_name='Preferences')
    return detected_vault
