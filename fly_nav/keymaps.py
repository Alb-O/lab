import bpy
from .logger import log_info, log_warning, log_error
from .preferences import FLYNAV_OPERATOR_IDNAME, FLYNAV_CUSTOM_KMI_ID

# Placeholder for keymap items
keymap_items = []

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
        
        log_info("Custom keymaps registered successfully.")
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
        
        log_info("Custom keymaps unregistered successfully.")
    except Exception as e:
        log_error(f"Failed to unregister keymaps: {e}")

def _disable_default_rmb_menus():
    """Disable default RMB menus in Blender."""
    try:
        wm = bpy.context.window_manager
        active_kc = wm.keyconfigs.active
        
        # Modes that call standard menus
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
        
        # Modes that call panels instead of menus
        panelmodes = ["Vertex Paint", "Weight Paint", "Image Paint", "Sculpt"]
        
        # Disable menu modes
        for mode in menumodes:
            if mode in active_kc.keymaps:
                for key in active_kc.keymaps[mode].keymap_items:
                    if key.type == "RIGHTMOUSE" and key.active:
                        key.active = False
                        log_info(f"Disabled RMB menu in {mode}")
        
        # Disable panel modes  
        for mode in panelmodes:
            if mode in active_kc.keymaps:
                for key in active_kc.keymaps[mode].keymap_items:
                    if (key.idname == "wm.call_panel" and 
                        key.type == "RIGHTMOUSE" and key.active):
                        key.active = False
                        log_info(f"Disabled RMB panel in {mode}")
                        
        log_info("Default RMB menus disabled.")
    except Exception as e:
        log_error(f"Failed to disable default RMB menus: {e}")

def _restore_default_rmb_menus():
    """Restore default RMB menus in Blender."""
    try:
        wm = bpy.context.window_manager
        active_kc = wm.keyconfigs.active
        
        # Modes that call standard menus
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
        
        # Modes that call panels instead of menus
        panelmodes = ["Vertex Paint", "Weight Paint", "Image Paint", "Sculpt"]
        
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
                        
        log_info("Default RMB menus restored.")
    except Exception as e:
        log_error(f"Failed to restore default RMB menus: {e}")

def _modify_walk_modal_keymaps():
    """Modify walk modal keymaps for custom navigation."""
    try:
        wm = bpy.context.window_manager
        active_kc = wm.keyconfigs.active
        
        if "View3D Walk Modal" in active_kc.keymaps:
            # Disable right mouse cancel in walk mode
            for key in active_kc.keymaps["View3D Walk Modal"].keymap_items:
                if key.propvalue == "CANCEL" and key.type == "RIGHTMOUSE" and key.active:
                    key.active = False
                    
            # Change left mouse confirm to right mouse release
            for key in active_kc.keymaps["View3D Walk Modal"].keymap_items:
                if key.propvalue == "CONFIRM" and key.type == "LEFTMOUSE" and key.active:
                    key.type = "RIGHTMOUSE"
                    key.value = "RELEASE"
                    
        log_info("Walk modal keymaps modified.")
    except Exception as e:
        log_error(f"Failed to modify walk modal keymaps: {e}")
        
def _restore_walk_modal_keymaps():
    """Restore default walk modal keymaps."""
    try:
        wm = bpy.context.window_manager
        active_kc = wm.keyconfigs.active
        
        if "View3D Walk Modal" in active_kc.keymaps:
            # Restore right mouse cancel in walk mode
            for key in active_kc.keymaps["View3D Walk Modal"].keymap_items:
                if key.propvalue == "CANCEL" and key.type == "RIGHTMOUSE":
                    key.active = True
                    
            # Restore left mouse confirm
            for key in active_kc.keymaps["View3D Walk Modal"].keymap_items:
                if key.propvalue == "CONFIRM" and key.type == "RIGHTMOUSE":
                    key.type = "LEFTMOUSE" 
                    key.value = "PRESS"
                    
        log_info("Walk modal keymaps restored.")
    except Exception as e:
        log_error(f"Failed to restore walk modal keymaps: {e}")
