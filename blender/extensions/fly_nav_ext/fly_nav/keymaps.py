import bpy # type: ignore
from .logger import log_warning, log_error
from .preferences import FLYNAV_OPERATOR_IDNAME

# Placeholder for keymap items
keymap_items = []

menumodes = [
    "Object Mode",
	"Mesh", 
	"Curve",
	"Armature",
	"Metaball",
	"Lattice",
	"Font",
	"Pose",
]

panelmodes = [
    "Vertex Paint",
    "Weight Paint",
    "Image Paint",
    "Sculpt"
]

def register_keymaps():
	"""Register custom keymaps for the Fly Nav extension."""
	try:
		wm = bpy.context.window_manager
		
		# Register addon keymap for right mouse navigation
		addon_kc = wm.keyconfigs.addon
		if not addon_kc:
			log_warning("No addon keyconfig found. Keymaps will not be registered.")
			return

		# Add the main right mouse navigation keymap
		km = addon_kc.keymaps.new(name="3D View", space_type="VIEW_3D")
		kmi = km.keymap_items.new(FLYNAV_OPERATOR_IDNAME, type="RIGHTMOUSE", value="PRESS")
		kmi.active = True
		keymap_items.append((km, kmi))
		
		# Disable default right-click context menus in various modes
		_disable_default_rmb_menus()
		
		# Modify walk modal keymaps
		_modify_walk_modal_keymaps()
	except Exception as e:
		log_error(f"Failed to register keymaps: {e}")

def unregister_keymaps():
	"""Unregister custom keymaps for the Fly Nav extension."""
	try:
		# Remove addon keymap items
		for km, kmi in keymap_items:
			try:
				km.keymap_items.remove(kmi)
			except Exception as e:
				log_error(f"Could not remove keymap item {getattr(kmi, 'idname', 'unknown')} from {km.name}: {e}")
		keymap_items.clear()
		
		# Restore default right-click menus
		_restore_default_rmb_menus()
		
		# Restore default walk modal keymaps
		_restore_walk_modal_keymaps()
	except Exception as e:
		log_error(f"Failed to unregister keymaps: {e}")

def _disable_default_rmb_menus():
	"""Disable default RMB menus in Blender."""
	try:
		wm = bpy.context.window_manager
		active_kc = wm.keyconfigs.active
  
		# Disable menu modes
		for mode in menumodes:
			if mode in active_kc.keymaps:
				for key in active_kc.keymaps[mode].keymap_items:
					if key.type == "RIGHTMOUSE" and key.active:
						key.active = False
		
		# Disable panel modes  
		for mode in panelmodes:
			if mode in active_kc.keymaps:
				for key in active_kc.keymaps[mode].keymap_items:
					if (key.idname == "wm.call_panel" and 
						key.type == "RIGHTMOUSE" and key.active):
						key.active = False
	except Exception as e:
		log_error(f"Failed to disable default RMB menus: {e}")

def _restore_default_rmb_menus():
	"""Restore default RMB menus in Blender."""
	try:
		wm = bpy.context.window_manager
		active_kc = wm.keyconfigs.active
		
		# Restore menu modes
		for mode in menumodes:
			if mode in active_kc.keymaps:
				for key in active_kc.keymaps[mode].keymap_items:
					if key.idname == "wm.call_menu" and key.type == "RIGHTMOUSE":
						key.active = True
						
		# Restore panel modes
		for mode in panelmodes:
			if mode in active_kc.keymaps:
				for key in active_kc.keymaps[mode].keymap_items:
					if key.idname == "wm.call_panel" and key.type == "RIGHTMOUSE":
						key.active = True
	except Exception as e:
		log_error(f"Failed to restore default RMB menus: {e}")

def _modify_walk_modal_keymaps():
	"""Modify walk modal keymaps for custom navigation."""
	try:
		wm = bpy.context.window_manager
		active_kc = wm.keyconfigs.active
		
		if "View3D Walk Modal" not in active_kc.keymaps:
			log_warning("View3D Walk Modal keymap not found. Skipping keymap modification.")
			return
		
		# Get the walk modal keymap
		walk_modal_km = active_kc.keymaps["View3D Walk Modal"]
		
		for key_item in walk_modal_km.keymap_items:
			if key_item.type == "RIGHTMOUSE" and key_item.value == "ANY":
				key_item.active = False
				break
		
		# Define modifier combinations to ensure RIGHTMOUSE RELEASE works in all cases
		modifiers = [
			{"shift": False, "ctrl": False, "alt": False},  # No modifiers
			{"shift": True, "ctrl": False, "alt": False},   # Shift only
			{"shift": False, "ctrl": True, "alt": False},   # Ctrl only
			{"shift": False, "ctrl": False, "alt": True},   # Alt only
			{"shift": True, "ctrl": True, "alt": False},    # Shift+Ctrl
			{"shift": True, "ctrl": False, "alt": True},    # Shift+Alt
			{"shift": False, "ctrl": True, "alt": True},    # Ctrl+Alt
			{"shift": True, "ctrl": True, "alt": True}      # Shift+Ctrl+Alt
		]
		
		# Add a keymap entry for each modifier combination
		for mod in modifiers:
			try:
				new_kmi = walk_modal_km.keymap_items.new_modal(
					type="RIGHTMOUSE",
					value="RELEASE",
					propvalue="CONFIRM",
					shift=mod["shift"],
					ctrl=mod["ctrl"],
					alt=mod["alt"]
				)
			except Exception as e:
				log_error(f"Failed to add RIGHTMOUSE RELEASE CONFIRM action with modifiers {mod}: {e}")
	except Exception as e:
		log_error(f"Failed to modify walk modal keymaps: {e}")
		
def _restore_walk_modal_keymaps():
	"""Restore default walk modal keymaps."""
	try:
		wm = bpy.context.window_manager
		active_kc = wm.keyconfigs.active
		
		if "View3D Walk Modal" not in active_kc.keymaps:
			log_warning("View3D Walk Modal keymap not found. Skipping keymap restoration.")
			return
			
		walk_modal_km = active_kc.keymaps["View3D Walk Modal"]
		
		# Restore right mouse button for cancellation (re-enable it)
		for key_item in walk_modal_km.keymap_items:
			if key_item.type == "RIGHTMOUSE" and key_item.value == "ANY":
				key_item.active = True
				break
		
		# Remove any custom RIGHTMOUSE RELEASE confirm actions we added
		items_to_remove = []
		for idx, key_item in enumerate(walk_modal_km.keymap_items):
			if (key_item.type == "RIGHTMOUSE" and 
				key_item.value == "RELEASE" and 
				hasattr(key_item, 'propvalue') and 
				key_item.propvalue == "CONFIRM"):
				items_to_remove.append(key_item)
		
		# Remove the items (in reverse order to avoid index issues)
		for key_item in reversed(items_to_remove):
			try:
				walk_modal_km.keymap_items.remove(key_item)
			except Exception:
				pass
		
		# Restore any LEFTMOUSE confirm action we might have disabled
		for key_item in walk_modal_km.keymap_items:
			if key_item.type == "LEFTMOUSE" and key_item.value == "ANY":
				key_item.active = True
	except Exception as e:
		log_error(f"Failed to restore walk modal keymaps: {e}")

def get_walk_modal_keys(propvalue=None):
    """
    Return a list of keymap items (dicts) for the given walk modal propvalue, or all if None.
    Each key is a dict with type, ctrl, alt, shift, value, and propvalue.
    """
    keys = []
    wm = bpy.context.window_manager
    kc = wm.keyconfigs.active
    if "View3D Walk Modal" in kc.keymaps:
        walk_km = kc.keymaps["View3D Walk Modal"]
        for kmi in walk_km.keymap_items:
            if propvalue is None or getattr(kmi, "propvalue", None) == propvalue:
                keys.append({
                    "type": kmi.type,
                    "ctrl": kmi.ctrl,
                    "alt": kmi.alt,
                    "shift": kmi.shift,
                    "value": kmi.value,
                    "propvalue": getattr(kmi, "propvalue", None),
                })
    return keys

def get_all_walk_modal_keys():
    """
    Return a dict mapping each walk modal propvalue to a list of keymap item dicts.
    { propvalue: [keymap_item_dict, ...], ... }
    """
    wm = bpy.context.window_manager
    kc = wm.keyconfigs.active
    modal_map = {}
    if "View3D Walk Modal" in kc.keymaps:
        walk_km = kc.keymaps["View3D Walk Modal"]
        for kmi in walk_km.keymap_items:
            prop = getattr(kmi, "propvalue", None)
            key = {
                "type": kmi.type,
                "ctrl": kmi.ctrl,
                "alt": kmi.alt,
                "shift": kmi.shift,
                "value": kmi.value,
                "propvalue": prop,
            }
            if prop not in modal_map:
                modal_map[prop] = []
            modal_map[prop].append(key)
    return modal_map

def event_matches_key(event, key):
    """
    Return True if the Blender event matches the keymap item dict.
    """
    return (
        event.type == key["type"] and
        event.ctrl == key["ctrl"] and
        event.alt == key["alt"] and
        event.shift == key["shift"] and
        ("value" not in key or event.value == key["value"])
    )

def get_walk_modal_action_for_event(event):
    """
    Return the walk modal propvalue (action) for the given event, or None if not found.
    """
    modal_map = get_all_walk_modal_keys()
    for propvalue, keys in modal_map.items():
        for key in keys:
            if event_matches_key(event, key):
                return propvalue
    return None
