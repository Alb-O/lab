import bpy # type: ignore

# This should be the name of the addon module (directory name).
# It's used as bl_idname for AddonPreferences and to retrieve them.
# This must match the __name__ of the __init__.py of the addon.
ADDON_ID = __name__.split('.')[0] if '.' in __name__ else "blend-vault"

# Storage key for persistent preferences across reloads
STORAGE_KEY = 'blend_vault_stored_prefs'

# List of preference properties that should be preserved across reloads
PERSISTENT_PROPERTIES = [
    'obsidian_vault_root',
    # Add more preference property names here as needed
]

class BlendVaultPreferences(bpy.types.AddonPreferences):
    """Blend Vault addon preferences"""
    bl_idname = ADDON_ID

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

def get_addon_preferences(context=None):
    """
    Get the Blend Vault addon preferences.
    Returns the preferences object which contains user settings like obsidian_vault_root.
    """
    if context is None:
        context = bpy.context
    try:
        return context.preferences.addons[ADDON_ID].preferences
    except KeyError:
        # This matches the original behavior in utils.py which didn't use LOG_COLORS for this print
        print(f"[Blend Vault] Warning: Could not find addon preferences for '{ADDON_ID}'. Make sure addon is enabled.")
        return None

def get_obsidian_vault_root(context=None):
    """
    Get the Obsidian vault root path from addon preferences.
    Returns the path as a string, or None if not set or preferences not found.
    """
    prefs = get_addon_preferences(context)
    if prefs and hasattr(prefs, 'obsidian_vault_root'):
        vault_root = prefs.obsidian_vault_root.strip()
        return vault_root if vault_root else None
    return None

def store_preferences():
    """
    Store current preference values in persistent storage for later restoration.
    This should be called before unregistering the preferences class.
    """
    if STORAGE_KEY not in bpy.app.driver_namespace:
        bpy.app.driver_namespace[STORAGE_KEY] = {}
    
    stored_prefs = bpy.app.driver_namespace[STORAGE_KEY]
    
    try:
        existing_prefs = bpy.context.preferences.addons.get(ADDON_ID)
        if existing_prefs and hasattr(existing_prefs, 'preferences'):
            for prop_name in PERSISTENT_PROPERTIES:
                if hasattr(existing_prefs.preferences, prop_name):
                    prop_value = getattr(existing_prefs.preferences, prop_name)
                    stored_prefs[prop_name] = prop_value
                    print(f"[Blend Vault] Stored preference '{prop_name}': {prop_value}")
    except Exception as e:
        print(f"[Blend Vault] Failed to store preferences: {e}")

def restore_preferences():
    """
    Restore previously stored preference values.
    This should be called after registering the preferences class.
    """
    if STORAGE_KEY not in bpy.app.driver_namespace:
        return
    
    stored_prefs = bpy.app.driver_namespace[STORAGE_KEY]
    
    try:
        current_prefs = bpy.context.preferences.addons[ADDON_ID].preferences
        for prop_name in PERSISTENT_PROPERTIES:
            if prop_name in stored_prefs:
                prop_value = stored_prefs[prop_name]
                if hasattr(current_prefs, prop_name):
                    setattr(current_prefs, prop_name, prop_value)
                    print(f"[Blend Vault] Restored preference '{prop_name}': {prop_value}")
    except Exception as e:
        print(f"[Blend Vault] Failed to restore preferences: {e}")
