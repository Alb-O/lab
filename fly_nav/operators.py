import bpy
from bpy.types import Operator
from . import logger # Import the logger
from .focal_length_manager import FocalLengthManager
from .preferences import get_addon_preferences # Import the utility function

class FLYNAV_OT_right_mouse_navigation(Operator):
    """Handles right-click initiated navigation or context menu display."""
    bl_idname = "flynav.right_mouse_navigation"
    bl_label = "Right Mouse Navigation"
    bl_options = {"REGISTER", "UNDO"}

    _timer = None
    _count = 0.0 # Using float for time accumulation
    _finished = False
    _callMenu = False
    _ortho = False # Tracks if the view was originally orthographic
    _back_to_ortho = False # Flag to restore orthographic view on exit
    _focal_manager = None
    _waiting_for_input = False
    _navigation_started = False
    _initial_event = None # Stores the initial mouse event from invoke

    NAV_KEYS = {
        'W', 'A', 'S', 'D',  # Standard movement
        'Q', 'E',          # Up/Down (often Z-axis)
        'SPACE', 'LEFT_SHIFT', # Alternative Up/Down or modifiers
        'UP_ARROW', 'DOWN_ARROW', 'LEFT_ARROW', 'RIGHT_ARROW' # Arrow key movement
    }

    menu_by_mode = {
        "OBJECT": "VIEW3D_MT_object_context_menu",
        "EDIT_MESH": "VIEW3D_MT_edit_mesh_context_menu",
        "EDIT_SURFACE": "VIEW3D_MT_edit_surface",
        "EDIT_TEXT": "VIEW3D_MT_edit_font_context_menu",
        "EDIT_ARMATURE": "VIEW3D_MT_edit_armature",
        "EDIT_CURVE": "VIEW3D_MT_edit_curve_context_menu",
        "EDIT_METABALL": "VIEW3D_MT_edit_metaball_context_menu",
        "EDIT_LATTICE": "VIEW3D_MT_edit_lattice_context_menu",
        "POSE": "VIEW3D_MT_pose_context_menu",
        "PAINT_VERTEX": "VIEW3D_PT_paint_vertex_context_menu",
        "PAINT_WEIGHT": "VIEW3D_PT_paint_weight_context_menu",
        "PAINT_TEXTURE": "VIEW3D_PT_paint_texture_context_menu",
        "SCULPT": "VIEW3D_PT_sculpt_context_menu",
    }

    def _perform_final_cleanup(self, context):
        """Performs all necessary cleanup actions when the operator finishes or is cancelled."""
        addon_prefs = get_addon_preferences(context) # Use the utility function

        if self._timer:
            wm = context.window_manager
            wm.event_timer_remove(self._timer)
            self._timer = None

        if self._callMenu: # Should be called before potential focal length changes by cleanup
            self.callMenu(context)
            self._callMenu = False

        if self._back_to_ortho and context.space_data and context.space_data.region_3d:
            # Use operator for safer ortho/persp toggle
            if context.space_data.region_3d.view_perspective != 'ORTHO':
                 bpy.ops.view3d.view_persportho() # Toggles to Ortho if in Persp
            self._back_to_ortho = False


        if self._focal_manager:
            self._focal_manager.cleanup(context, addon_prefs) # CRITICAL: Call cleanup

        logger.log_info("Navigation operator cleanup completed")

    def modal(self, context, event):
        """Handles modal events for the navigation operator."""
        if context.space_data is None: # Important check from RMN
            if self._timer:
                try:
                    context.window_manager.event_timer_remove(self._timer)
                except Exception:
                    pass
                self._timer = None
            return {'CANCELLED'}

        addon_prefs = get_addon_preferences(context) # Use the utility function
        if addon_prefs is None:
            logger.log_warning(f"Could not get addon preferences in modal. Using hardcoded defaults.")
            class DummyPrefs:
                time = 0.3 
            addon_prefs = DummyPrefs()

        space_type = context.space_data.type
        
        if space_type == "VIEW_3D":
            if self._waiting_for_input:
                # Check for navigation key presses to start walk mode
                if event.type in self.NAV_KEYS and event.value == "PRESS":
                    if self._start_navigation(context):
                        return {"PASS_THROUGH"} # Changed from RUNNING_MODAL to PASS_THROUGH
                    self._perform_final_cleanup(context)
                    return {"CANCELLED"}
                
                # Corrected RIGHTMOUSE release logic
                if event.type == "RIGHTMOUSE" and event.value == "RELEASE":
                    time_threshold = getattr(addon_prefs, 'time', 0.3)
                    if self._count < time_threshold:
                        self._callMenu = True
                    self.cancel(context) # cancel() now primarily sets up for cleanup via _finished
                    self._finished = True
                    return {"PASS_THROUGH"}
                # Removed the broader "event.type not in TIMER/MOUSEMOVE" for cancel here

            # CRITICAL MISSING BLOCK from RMN
            if self._navigation_started and not self._finished:
                # If walk mode is active, any event other than Timer or MouseMove
                # indicates walk mode has ended.
                if event.type not in {"TIMER", "MOUSEMOVE", "INBETWEEN_MOUSEMOVE"}:
                    if event.type == "RIGHTMOUSE" and event.value == "RELEASE":
                        time_threshold = getattr(addon_prefs, 'time', 0.3)
                        if self._count < time_threshold: # Check if menu should be called
                            self._callMenu = True
                    self.cancel(context)
                    self._finished = True
                    return {"PASS_THROUGH"}

        if event.type == "TIMER":
            if space_type == "VIEW_3D":
                time_threshold = getattr(addon_prefs, 'time', 0.3)
                
                if (self._waiting_for_input and
                        not self._navigation_started and
                        time_threshold > 0 and # Use time_threshold here
                        self._count >= time_threshold): # Use time_threshold here
                    if self._start_navigation(context):
                        return {"RUNNING_MODAL"} # Correct: Stay modal
                    self._perform_final_cleanup(context)
                    return {"CANCELLED"}

                if self._focal_manager:
                    was_exit_transition = self._focal_manager.is_exit_transition
                    transition_completed = self._focal_manager.update_transition(context)
                    if transition_completed and self._finished and was_exit_transition:
                        self._perform_final_cleanup(context)
                        return {"CANCELLED"}

            if not self._finished:
                self._count += 0.02  # Timer interval
            return {"PASS_THROUGH"}

        if self._finished:
            if self._focal_manager and self._focal_manager.is_transitioning:
                return {"PASS_THROUGH"} # Let transition complete

            if self._focal_manager and self._focal_manager.start_exit_transition(context, addon_prefs):
                return {"PASS_THROUGH"} # Start exit transition if needed

            self._perform_final_cleanup(context)
            return {"CANCELLED"}

        return {"PASS_THROUGH"}

    def callMenu(self, context):
        """Calls the appropriate context menu based on the current mode and selection."""
        space_type = context.space_data.type
        
        if space_type == "VIEW_3D":
            mode = context.mode
            menu_idname = self.menu_by_mode.get(mode, "VIEW3D_MT_object_context_menu")
            
            try:
                bpy.ops.wm.call_menu(name=menu_idname)
                logger.log_info(f"Called menu: {menu_idname}")
            except Exception as e:
                logger.log_error(f"Failed to call menu {menu_idname}: {e}")

    def invoke(self, context, event):
        """Initializes the operator when invoked."""
        self._count = 0.0
        self._finished = False
        self._callMenu = False
        self._ortho = False
        self._back_to_ortho = False
        self._waiting_for_input = True
        self._navigation_started = False
        self._focal_manager = FocalLengthManager()
        self._initial_event = event

        self.view_x = event.mouse_x
        self.view_y = event.mouse_y
        return self.execute(context)

    def execute(self, context):
        """Sets up the modal timer if in the 3D View."""
        space_type = context.space_data.type if context.space_data else None
        
        if space_type == "VIEW_3D":
            wm = context.window_manager
            self._timer = wm.event_timer_add(0.02, window=context.window)  # 50 FPS timer
            wm.modal_handler_add(self)
            logger.log_info("3D View navigation timer created")
            return {"RUNNING_MODAL"}

        logger.log_info(f"No modal conditions met for {space_type}, returning FINISHED")
        return {"FINISHED"}
    def _start_navigation(self, context):
        """Attempts to start Blender's walk/fly navigation."""
        addon_prefs = get_addon_preferences(context) # Use the utility function
        if addon_prefs is None:
            logger.log_warning(f"Could not get addon preferences in _start_navigation. Using hardcoded defaults.")
            class DummyPrefs:
                enable_camera_navigation = True
                camera_nav_only_if_locked = False
                return_to_ortho_on_exit = True
            addon_prefs = DummyPrefs()

        enable_camera_nav = getattr(addon_prefs, 'enable_camera_navigation', True)
        only_if_locked = getattr(addon_prefs, 'camera_nav_only_if_locked', False)

        if not context.space_data or not context.space_data.region_3d:
            return False

        view_perspective_type = context.space_data.region_3d.view_perspective

        if view_perspective_type == "CAMERA":
            if not enable_camera_nav:
                return False
            
            if only_if_locked:
                if not context.space_data.lock_camera:
                    return False

        try:
            if self._focal_manager:
                self._focal_manager.start_entry_transition(context, addon_prefs)

            bpy.ops.view3d.walk('INVOKE_DEFAULT')
            self._navigation_started = True
            self._waiting_for_input = False

            if not context.region_data.is_perspective:
                self._ortho = True
                return_to_ortho = getattr(addon_prefs, 'return_to_ortho_on_exit', True)
                if return_to_ortho:
                    self._back_to_ortho = True
            else:
                self._ortho = False
                self._back_to_ortho = False

            logger.log_info("Walk navigation started successfully")
            return True
        except RuntimeError as e:
            logger.log_error(f"RuntimeError in _start_navigation: {e}")
            self.report({"WARNING"}, "Navigation failed. Object might have constraints or view is locked.")
            return False

    def cancel(self, context):
        """Handles operator cancellation."""
        if context.space_data and context.space_data.type == "VIEW_3D" and self._focal_manager:
            addon_prefs = get_addon_preferences(context) # Use the utility function

            if self._focal_manager.is_transitioning and not self._focal_manager.is_exit_transition:
                self._focal_manager.force_restore_original(context, addon_prefs)

    @classmethod
    def poll(cls, context):
        return context.area and context.area.type == 'VIEW_3D'


class FLYNAV_OT_simple_fly(bpy.types.Operator):
    """Simple Fly Operator (Legacy)"""
    bl_idname = "flynav.simple_fly"
    bl_label = "Simple Fly Mode"
    bl_options = {'REGISTER', 'UNDO'}

    def execute(self, context):
        logger.log_info(f"Simple Fly Operator executed from: {__package__}")
        self.report({'INFO'}, "Fly Mode Activated")
        
        # Start walk mode directly - this is the core functionality
        try:
            bpy.ops.view3d.walk('INVOKE_DEFAULT')
            logger.log_info("Walk mode started successfully via simple_fly operator")
        except RuntimeError as e:
            logger.log_error(f"Failed to start walk mode: {e}")
            self.report({'ERROR'}, "Failed to start navigation")
            return {'CANCELLED'}
        
        return {'FINISHED'}

    @classmethod
    def poll(cls, context):
        return context.area and context.area.type == 'VIEW_3D'

# --- Registration ---

classes = (
    FLYNAV_OT_right_mouse_navigation,
    FLYNAV_OT_simple_fly,
)

def register():
    for cls in classes:
        bpy.utils.register_class(cls)
    logger.log_info(f"Registered operators in: {__package__}")

def unregister():
    for cls in reversed(classes):
        bpy.utils.unregister_class(cls)
    logger.log_info(f"Unregistered operators in: {__package__}")

if __name__ == "__main__":
    # This is for direct testing. Ensure logger is minimally functional.
    if __package__ is None: # If run as script, __package__ is None
        # Attempt to set a package name for the logger for this test context
        # This won't reflect the actual addon structure but helps logger function
        try:
            from .. import logger as root_logger # try to get logger from parent if part of a package test
            root_logger.set_package_name("fly_nav.operators_test")
        except ImportError:
            logger.set_package_name("fly_nav_operators_direct_script") # fallback
            logger.set_log_level("DEBUG")

    register()
