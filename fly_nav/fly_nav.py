import bpy
from bpy.types import Operator
from .focal_length_manager import FocalLengthManager

class FLYNAV_OT_right_mouse_navigation(Operator):
    """Handles right-click initiated navigation or context menu display."""

    bl_idname = "fly_nav.right_mouse_navigation"
    bl_label = "Right Mouse Navigation"
    bl_options = {"REGISTER", "UNDO"}

    _timer = None
    _count = 0.0  # Using float for time accumulation
    _finished = False
    _callMenu = False
    _ortho = False  # Tracks if the view was originally orthographic
    _back_to_ortho = False  # Flag to restore orthographic view on exit
    _focal_manager = None
    _waiting_for_input = False
    _navigation_started = False
    _initial_event = None  # Stores the initial mouse event from invoke

    NAV_KEYS = {
        'W', 'A', 'S', 'D',  # Standard movement
        'Q', 'E',            # Up/Down (often Z-axis)
        'SPACE', 'LEFT_SHIFT',  # Alternative Up/Down or modifiers
        'UP_ARROW', 'DOWN_ARROW', 'LEFT_ARROW', 'RIGHT_ARROW'  # Arrow key movement
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
        addon_prefs = context.preferences.addons["fly_nav_ext"].preferences

        if self._timer:
            context.window_manager.event_timer_remove(self._timer)
            self._timer = None

        if self._callMenu:
            self.callMenu(context)

        if self._back_to_ortho:
            bpy.ops.view3d.view_persportho()

        if self._focal_manager:
            self._focal_manager.cleanup(context, addon_prefs)

    def modal(self, context, event):
        """Handles events while the operator is in a modal state."""
        if context.space_data is None:
            if self._timer:
                try:
                    context.window_manager.event_timer_remove(self._timer)
                except Exception:
                    pass
                self._timer = None
            return {'CANCELLED'}

        addon_prefs = context.preferences.addons["fly_nav_ext"].preferences
        space_type = context.space_data.type

        if space_type == "VIEW_3D":
            if self._waiting_for_input:
                if event.type in self.NAV_KEYS and event.value == 'PRESS':
                    if self._start_navigation(context):
                        return {"PASS_THROUGH"}
                    self._perform_final_cleanup(context)
                    return {"CANCELLED"}

                if event.type == "RIGHTMOUSE" and event.value == "RELEASE":
                    if self._count < addon_prefs.time: # type: ignore
                        self._callMenu = True
                    self.cancel(context)
                    self._finished = True
                    return {"PASS_THROUGH"}

            if self._navigation_started and not self._finished:
                if event.type not in {"TIMER", "MOUSEMOVE", "INBETWEEN_MOUSEMOVE"}:
                    if event.type == "RIGHTMOUSE" and event.value == "RELEASE":
                        if self._count < addon_prefs.time: # type: ignore
                            self._callMenu = True
                    self.cancel(context)
                    self._finished = True
                    return {"PASS_THROUGH"}

        if event.type == "TIMER":
            if space_type == "VIEW_3D":
                auto_activation_threshold = addon_prefs.time # type: ignore
                if (self._waiting_for_input and
                        not self._navigation_started and
                        auto_activation_threshold > 0 and
                        self._count >= auto_activation_threshold):
                    if self._start_navigation(context):
                        return {"RUNNING_MODAL"}
                    self._perform_final_cleanup(context)
                    return {"CANCELLED"}

                if self._focal_manager:
                    was_exit_transition = self._focal_manager.is_exit_transition
                    transition_completed = self._focal_manager.update_transition(context)
                    if transition_completed and self._finished and was_exit_transition:
                        self._perform_final_cleanup(context)
                        return {"CANCELLED"}

            if not self._finished:
                self._count += 0.02
            return {"PASS_THROUGH"}

        if self._finished:
            if self._focal_manager and self._focal_manager.is_transitioning:
                return {"PASS_THROUGH"}

            if self._focal_manager and self._focal_manager.start_exit_transition(context, addon_prefs):
                return {"PASS_THROUGH"}

            self._perform_final_cleanup(context)
            return {"CANCELLED"}

        return {"PASS_THROUGH"}

    def callMenu(self, context):
        """Calls the appropriate context menu based on the current mode and selection."""
        prefs = context.preferences.addons["fly_nav_ext"].preferences
        if prefs.activation_method != 'RMB':
            return

        select_mouse = context.window_manager.keyconfigs.active.preferences.select_mouse
        space_type = context.space_data.type

        if space_type == "VIEW_3D":
            if select_mouse == "LEFT":
                try:
                    bpy.ops.wm.call_menu(name=self.menu_by_mode[context.mode])
                except (RuntimeError, KeyError):
                    pass
            else:
                bpy.ops.view3d.select("INVOKE_DEFAULT")

    def invoke(self, context, event):
        """Initializes the operator state when it's called."""
        prefs = context.preferences.addons["fly_nav_ext"].preferences
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

        if prefs.activation_method != 'RMB': # type: ignore
            if self._start_navigation(context):
                return {'PASS_THROUGH'}
            self._perform_final_cleanup(context)
            return {'CANCELLED'}

        return self.execute(context)

    def execute(self, context):
        """Sets up the modal timer if in the 3D View (for RMB activation)."""
        prefs = context.preferences.addons["fly_nav_ext"].preferences
        if prefs.activation_method != 'RMB': # type: ignore
            return {'CANCELLED'}

        space_type = context.space_data.type if context.space_data else None

        if space_type == "VIEW_3D":
            wm = context.window_manager
            self._timer = wm.event_timer_add(0.02, window=context.window)
            wm.modal_handler_add(self)
            return {"RUNNING_MODAL"}

        return {"FINISHED"}

    def _start_navigation(self, context):
        """Attempts to start Blender's walk/fly navigation."""
        addon_prefs = context.preferences.addons["fly_nav_ext"].preferences
        enable_camera_nav = addon_prefs.enable_camera_navigation
        only_if_locked = addon_prefs.camera_nav_only_if_locked

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
                if addon_prefs.return_to_ortho_on_exit:
                    self._back_to_ortho = True
            else:
                self._ortho = False
                self._back_to_ortho = False

            return True
        except RuntimeError:
            self.report({"WARNING"}, "Navigation failed. Object might have constraints or view is locked.")
            return False

    def cancel(self, context):
        """Handles operator cancellation, including focal length restoration for interrupted entry."""
        if context.space_data and context.space_data.type == "VIEW_3D" and self._focal_manager:
            addon_prefs = context.preferences.addons["fly_nav_ext"].preferences
            if self._focal_manager.is_transitioning and not self._focal_manager.is_exit_transition:
                self._focal_manager.force_restore_original(context, addon_prefs)
