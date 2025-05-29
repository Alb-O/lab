"""
Smart paste functionality and main entry points.
Provides the main SmartPasteOperator and clipboard monitoring.
"""

import bpy
from typing import List, Tuple, Literal # Added Literal, List, Tuple
from .file_validation import is_valid_blend_or_sidecar_path, normalize_path_from_clipboard
from .. import log_debug, log_error, log_warning

# Define the Literal for valid space_type strings for keymaps
BlenderSpaceType = Literal[
    'EMPTY', 'VIEW_3D', 'IMAGE_EDITOR', 'NODE_EDITOR', 'SEQUENCE_EDITOR',
    'CLIP_EDITOR', 'DOPESHEET_EDITOR', 'GRAPH_EDITOR', 'NLA_EDITOR',
    'TEXT_EDITOR', 'CONSOLE', 'INFO', 'TOPBAR', 'STATUSBAR',
    'OUTLINER', 'PROPERTIES', 'FILE_BROWSER', 'PREFERENCES'
]


class SmartPasteOperator(bpy.types.Operator):
    bl_idname = "wm.smart_paste"
    bl_label = "Smart Paste"
    bl_description = "Paste with .blend file interception, falls back to default paste"
    
    def execute(self, context):
        log_debug("SmartPasteOperator execute() called.")
        try:
            clipboard_text = context.window_manager.clipboard
            log_debug(f"Clipboard content: '{clipboard_text}'")

            if is_valid_blend_or_sidecar_path(clipboard_text):
                path = normalize_path_from_clipboard(clipboard_text)
                log_debug(f"Normalized path: '{path}'")

                # Always show the new choose action dialog
                bpy.ops.blend_vault.choose_action_before_open('INVOKE_DEFAULT', file_path=path) # type: ignore
                return {'FINISHED'}
            else:
                log_debug("Clipboard content not a valid blend/sidecar path.")
        except Exception as e:
            log_error(f"SmartPaste error: {e}")
            # Log or report error if needed, e.g., self.report({'ERROR'}, f"SmartPaste error: {e}")
            pass  # Fall through to default paste

        log_debug("Falling back to default paste behavior.")
        # Fallback to default paste behavior
        try:
            if context.space_data and context.space_data.type == 'VIEW_3D':
                return bpy.ops.view3d.pastebuffer('INVOKE_DEFAULT')
            # Attempt to find a generic paste operator if not in 3D View
            # This part might need more robust handling for different contexts
            elif hasattr(bpy.ops.ui, 'paste'):  # Check for generic UI paste
                 return bpy.ops.ui.paste('INVOKE_DEFAULT') # type: ignore
            elif hasattr(bpy.ops.text, 'paste'):  # Check for text editor paste
                 return bpy.ops.text.paste('INVOKE_DEFAULT') # type: ignore
            else:
                self.report({'WARNING'}, "No valid paste operation for this context")
                return {'CANCELLED'}
        except Exception as e:
            self.report({'WARNING'}, f"Default paste operation failed: {e}")
            return {'CANCELLED'}


class OpenBlendFromClipboardOperator(bpy.types.Operator):
    bl_idname = "wm.open_blend_from_clipboard"
    bl_label = "Open Blend File from Clipboard"
    bl_description = "Opens the .blend file at the path in the clipboard"

    def execute(self, context):
        try:
            clipboard_text = context.window_manager.clipboard
            if not is_valid_blend_or_sidecar_path(clipboard_text):
                self.report({'ERROR'}, f"Not a valid .blend file: {clipboard_text}")
                return {'CANCELLED'}
            
            path = normalize_path_from_clipboard(clipboard_text)
        except Exception as e:
            self.report({'ERROR'}, f"Clipboard error: {e}")
            return {'CANCELLED'}

        # Always show the new choose action dialog
        bpy.ops.blend_vault.choose_action_before_open('INVOKE_DEFAULT', file_path=path) # type: ignore
        return {'FINISHED'}


addon_keymaps = [] # Store keymaps for unregistration

# Store a list of keymap items that this addon deactivates
_deactivated_default_kmis = []

def _deactivate_default_paste_keymaps(active_kc):
    global _deactivated_default_kmis
    _deactivated_default_kmis.clear()
    print("Searching for default Ctrl+V keymaps to deactivate...")

    known_paste_idnames = {
        "view3d.pastebuffer", "node.pastebuffer", "text.pastebuffer",
        "console.pastebuffer", "object.pastebuffer", "graph.pastebuffer",
        "clip.pastebuffer", "sequencer.paste", "wm.paste"
    }

    for km in active_kc.keymaps: # Iterate through all keymaps in the active config
        for kmi in km.keymap_items:
            is_ctrl_v_binding = (
                kmi.type == 'V' and kmi.value == 'PRESS' and
                kmi.ctrl and not kmi.alt and not kmi.shift
            )
            # Exclude our own operator, and ensure it's an active, potentially conflicting paste
            if is_ctrl_v_binding and kmi.idname != SmartPasteOperator.bl_idname and kmi.active:
                if kmi.idname in known_paste_idnames:
                    print(f"Deactivating: {kmi.idname} in keymap '{km.name}' (Ctrl+V)")
                    kmi.active = False
                    _deactivated_default_kmis.append(kmi) # Store the KMI object

def _reactivate_default_paste_keymaps():
    global _deactivated_default_kmis
    log_debug(f"Reactivating {len(_deactivated_default_kmis)} default keymap items...")
    for kmi in _deactivated_default_kmis:
        try:
            keymap_name = kmi.keymap.name if hasattr(kmi, 'keymap') and kmi.keymap else 'Unknown'
            log_debug(f"Reactivating: {kmi.idname} in keymap '{keymap_name}'")
            kmi.active = True
        except Exception as e:
            log_debug(f"Error reactivating {getattr(kmi, 'idname', 'unknown KMI')}: {e}")
    _deactivated_default_kmis.clear()

def register():
    """Register smart paste operators and keybindings."""
    log_debug("smart_paste.register() called.")
    bpy.utils.register_class(OpenBlendFromClipboardOperator)
    bpy.utils.register_class(SmartPasteOperator)
    
    wm = bpy.context.window_manager

    # Deactivate potentially conflicting default Ctrl+V keymaps
    if wm and wm.keyconfigs:
         _deactivate_default_paste_keymaps(wm.keyconfigs.active)

    # Register addon's own keymaps
    kc = wm.keyconfigs.addon if wm.keyconfigs.addon else wm.keyconfigs.user
    keyconfig_name = "addon" if wm.keyconfigs.addon and kc == wm.keyconfigs.addon else "user"
    print(f"Registering own keymaps using {keyconfig_name} keyconfig...")

    global addon_keymaps 
    addon_keymaps.clear() # Clear previous registrations if any

    keymap_definitions: List[Tuple[str, BlenderSpaceType]] = [
        ('3D View', 'VIEW_3D'),
        ('Node Editor', 'NODE_EDITOR'),
        ('Window', 'EMPTY') # General fallback
    ]

    for name, space_type_value in keymap_definitions:
        km = kc.keymaps.new(name=name, space_type=space_type_value)
        # Ensure this is Ctrl+V, not Ctrl+Alt+Shift+V
        kmi = km.keymap_items.new(SmartPasteOperator.bl_idname, 'V', 'PRESS', ctrl=True, shift=False, alt=False)
        log_debug(f"Registered {SmartPasteOperator.bl_idname} for {name} with idname: {kmi.idname}")
        addon_keymaps.append((km, kmi))

def unregister():
    """Unregister smart paste operators and keybindings."""
    log_debug("smart_paste.unregister() called.")

    # Unregister addon's own keymaps first
    global addon_keymaps
    for km, kmi in addon_keymaps:
        try:
            km.keymap_items.remove(kmi)
        except Exception as e:
            log_debug(f"Error removing keymap item {getattr(kmi, 'idname', 'unknown KMI')} from {km.name}: {e}")
    addon_keymaps.clear()
    
    # Reactivate default keymaps that were deactivated by this addon
    _reactivate_default_paste_keymaps()

    # Unregister classes
    try:
        bpy.utils.unregister_class(SmartPasteOperator)
    except RuntimeError: 
        pass 
    try:
        bpy.utils.unregister_class(OpenBlendFromClipboardOperator)
    except RuntimeError:
        pass
