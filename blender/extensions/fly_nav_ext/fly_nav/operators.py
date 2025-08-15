from .keymaps import event_matches_key, get_all_walk_modal_keys
import bpy # type: ignore
from bpy.types import Operator # type: ignore
from . import logger
from .focal_length_manager import FocalLengthManager
from .preferences import get_addon_preferences
from collections import deque

def start_walk_navigation(context, focal_manager=None, addon_prefs=None):
	"""Utility function to start Blender's walk navigation mode.
	
	Args:
		context: Blender context
		focal_manager: Optional FocalLengthManager instance
		addon_prefs: Optional addon preferences
		
	Returns:
		tuple: (success, restore_ortho)
			success: True if navigation was started successfully
			restore_ortho: True if orthographic view needs to be restored on exit
	"""
	logger.log_debug("Starting walk navigation mode")
	
	restore_ortho = False
	
	try:
		# Detect if Fast mode (Shift) is active from context or pass as argument (default False)
		fast_mode = False
		# Try to detect if Shift is held in the event queue (if available)
		if hasattr(context, 'window_manager') and hasattr(context.window_manager, 'events'):
			for evt in context.window_manager.events:
				if evt.type == 'LEFT_SHIFT' and evt.value in {'PRESS', 'CLICK_DRAG'}:
					fast_mode = True
					break
		# Start focal length transition if focal manager is provided
		if focal_manager and addon_prefs:
			focal_manager.start_entry_transition(context, addon_prefs, fast_mode=fast_mode)
			
		# Handle orthographic view
		region_3d = context.space_data.region_3d if context.space_data else None
		if region_3d and not region_3d.is_perspective:
			restore_ortho = addon_prefs.return_to_ortho_on_exit if addon_prefs else False
			logger.log_debug(f"In ortho view, restore_ortho={restore_ortho}")
			
		# Start Blender's walk mode
		bpy.ops.view3d.walk('INVOKE_DEFAULT')
		logger.log_debug("Walk navigation started successfully")
		
		return True, restore_ortho
		
	except RuntimeError as e:
		logger.log_error(f"Failed to start navigation: {e}")
		return False, False


class FLYNAV_OT_right_mouse_navigation(Operator):
	"""Handles right-click initiated navigation or context menu display."""
	bl_idname = "flynav.right_mouse_navigation"
	bl_label = "Right Mouse Navigation"
	bl_options = {"REGISTER", "UNDO"}
	_fast_key_active = False  # Track non-modifier fast key state (instance var, but safe as class default)

	# Global lock to prevent multiple instances
	_global_instance_running = False
	_global_instance_count = 0

	# --- Queue system for input buffering ---
	_activation_queue = deque()
	_MAX_QUEUE_SIZE = 4  # Limit to avoid runaway queue

	# Class variables for state tracking
	_timer = None
	_time_elapsed = 0.0
	_finished = False
	_call_menu = False
	_restore_ortho = False
	_focal_manager = None
	_waiting_for_input = True
	_navigation_active = False
	_initial_mouse_pos = None

	# Navigation keys that trigger walk mode
	NAV_KEYS = {
		'W', 'A', 'S', 'D',              # WASD movement
		'Q', 'E',                        # Up/Down
		'SPACE', 'LEFT_SHIFT',           # Alternative vertical movement
		'UP_ARROW', 'DOWN_ARROW',        # Arrow keys
		'LEFT_ARROW', 'RIGHT_ARROW'
	}

	# Track if initial fast mode was set to avoid double transition
	_initial_fast_mode_applied = False

	# Context menus for different modes
	CONTEXT_MENUS = {
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

	def invoke(self, context, event):
		"""Initialize the operator when invoked."""
		# Increment instance counter for tracking
		FLYNAV_OT_right_mouse_navigation._global_instance_count += 1
		current_instance_id = FLYNAV_OT_right_mouse_navigation._global_instance_count
		
		logger.log_debug(f"=== FLYNAV Operator Invoked (Instance #{current_instance_id}) ===")
		
		# --- Queue system: If another instance is running, buffer this activation ---
		if FLYNAV_OT_right_mouse_navigation._global_instance_running:
			# Only queue if not already at max size
			if len(FLYNAV_OT_right_mouse_navigation._activation_queue) < FLYNAV_OT_right_mouse_navigation._MAX_QUEUE_SIZE:
				FLYNAV_OT_right_mouse_navigation._activation_queue.append((context.copy(), event.type, event.value))
				logger.log_debug(f"Instance #{current_instance_id}: Operator busy, activation queued. Queue size: {len(FLYNAV_OT_right_mouse_navigation._activation_queue)}")
			else:
				logger.log_warning(f"Instance #{current_instance_id}: Operator busy, queue full. Activation ignored.")
			# Always cancel this invocation if another is running
			return {"CANCELLED"}

		# Set global lock
		FLYNAV_OT_right_mouse_navigation._global_instance_running = True
		logger.log_debug(f"Instance #{current_instance_id}: Global lock acquired")

		# Reset all state and initialize common members for this instance
		self._reset_state()
		self._initial_mouse_pos = (event.mouse_x, event.mouse_y)
		self._instance_id = current_instance_id # Set the instance's ID
		self._focal_manager = FocalLengthManager() # Initialize the instance's focal manager
		
		addon_prefs = get_addon_preferences(context) or self._get_default_prefs()
		time_val = getattr(addon_prefs, 'time', 0.1)
		
		if time_val == 0:
			logger.log_debug(f"Instance #{self._instance_id}: time=0, attempting to start navigation immediately")
			# Determine fast mode state at the moment navigation starts
			modal_keys = get_all_walk_modal_keys()
			fast_keys = modal_keys.get("FAST_ENABLE", [])
			fast_now = False
			for key in fast_keys:
				if event_matches_key(event, key):
					fast_now = True
					break
			self._initial_fast_mode_applied = False
			if self._start_navigation(context, addon_prefs, force_fast_mode=fast_now):
				logger.log_debug(f"Instance #{self._instance_id}: Navigation started (time=0), proceeding to execute for modal operation.")
				return self.execute(context)
			else:
				logger.log_warning(f"Instance #{self._instance_id}: _start_navigation failed for time=0 case.")
				self._finish_operator(context) 
				return {'CANCELLED'}
		else: # time_val > 0
			self._initial_fast_mode_applied = False
			logger.log_debug(f"Instance #{self._instance_id}: Operator initialized for time > 0 (timer will run). Mouse: {self._initial_mouse_pos}")
			return self.execute(context)

	def execute(self, context):
		"""Start the modal operator."""
		if not context.space_data or context.space_data.type != "VIEW_3D":
			logger.log_warning(f"Instance #{getattr(self, '_instance_id', 0)}: Not in 3D viewport, cancelling")
			# Release global lock if we acquired it
			FLYNAV_OT_right_mouse_navigation._global_instance_running = False
			return {"CANCELLED"}

		# Start timer for modal operation
		wm = context.window_manager
		self._timer = wm.event_timer_add(0.02, window=context.window)  # 50 FPS
		wm.modal_handler_add(self)
		
		instance_id = getattr(self, '_instance_id', 0)
		logger.log_debug(f"Instance #{instance_id}: Modal operator started with timer")
		return {"RUNNING_MODAL"}

	def modal(self, context, event):
		"""Handle modal events."""
		# Early exit if context is invalid
		if not context.space_data or context.space_data.type != "VIEW_3D":
			logger.log_warning("Lost 3D viewport context")
			return self._finish_operator(context)

		# Get addon preferences
		addon_prefs = get_addon_preferences(context)
		if not addon_prefs:
			logger.log_warning("Could not get addon preferences")
			addon_prefs = self._get_default_prefs()        # Log event for debugging
		"""instance_id = getattr(self, '_instance_id', 0)
		logger.log_debug(f"Instance #{instance_id}: Event: {event.type}({event.value}) | "
						f"waiting={self._waiting_for_input} | "
						f"nav_active={self._navigation_active} | "
						f"finished={self._finished}")"""

		# Generalized walk modal key detection for all walk modal actions
		if self._navigation_active:
			# Only update fast mode state if the event is a fast key or a modifier change
			modal_keys = get_all_walk_modal_keys()
			fast_keys = modal_keys.get("FAST_ENABLE", [])
			is_fast_key_event = False
			fast_now = self._fast_mode_applied if hasattr(self, '_fast_mode_applied') else False

			# On TIMER events, check modifier state
			if event.type == "TIMER":
				modifier_fast = False
				for key in fast_keys:
					if key["type"] in {"LEFT_SHIFT", "RIGHT_SHIFT"} and key["ctrl"] in {-1, 0} and key["alt"] in {-1, 0} and key["shift"] in {-1, 0}:
						if event.shift:
							modifier_fast = True
					elif key["type"] in {"LEFT_CTRL", "RIGHT_CTRL"} and key["ctrl"] in {-1, 0} and key["alt"] in {-1, 0} and key["shift"] in {-1, 0}:
						if event.ctrl:
							modifier_fast = True
					elif key["type"] in {"LEFT_ALT", "RIGHT_ALT"} and key["ctrl"] in {-1, 0} and key["alt"] in {-1, 0} and key["shift"] in {-1, 0}:
						if event.alt:
							modifier_fast = True
				fast_now = modifier_fast
				is_fast_key_event = True  # TIMER is used to poll modifier state
			else:
				# For non-TIMER events, only react to fast key events
				for key in fast_keys:
					if event_matches_key(event, key):
						is_fast_key_event = True
						if event.value == "PRESS":
							fast_now = True
							self._fast_key_active = True
							logger.log_debug(f"[DEBUG] Non-modifier fast key pressed: {key}")
						elif event.value == "RELEASE":
							fast_now = False
							self._fast_key_active = False
							logger.log_debug(f"[DEBUG] Non-modifier fast key released: {key}")
						break

			if not hasattr(self, '_fast_mode_applied'):
				self._fast_mode_applied = False
			# Only trigger transition if this is a fast key event or TIMER (modifier poll)
			if (is_fast_key_event or (self._navigation_active and not self._initial_fast_mode_applied)):
				# Prevent double-application of initial fast mode (for time==0)
				if self._navigation_active and not self._initial_fast_mode_applied:
					self._initial_fast_mode_applied = True
				if self._focal_manager and (fast_now != self._fast_mode_applied):
					is_fast_exit = self._fast_mode_applied and not fast_now
					logger.log_debug(f"[DEBUG] Triggering focal transition: fast_mode={fast_now}, fast_mode_exit={is_fast_exit}")
					self._focal_manager.start_entry_transition(context, addon_prefs, fast_mode=fast_now, fast_mode_exit=is_fast_exit)
					self._fast_mode_applied = fast_now

		# Handle timer events
		if event.type == "TIMER":
			return self._handle_timer_event(context, addon_prefs)

		# Handle events when operator is finishing
		if self._finished:
			return self._handle_finished_state(context, event, addon_prefs)

		# Handle events during navigation
		if self._navigation_active:
			return self._handle_navigation_events(context, event, addon_prefs)

		# Handle events while waiting for input
		if self._waiting_for_input:
			return self._handle_waiting_events(context, event, addon_prefs)

		# Default pass through
		return {"PASS_THROUGH"}

	def _handle_timer_event(self, context, addon_prefs):
		"""Handle timer events for timeouts and transitions."""
		if not self._finished:
			self._time_elapsed += 0.02

		# Handle focal length transitions
		if self._focal_manager and self._focal_manager.is_transitioning:
			transition_completed = self._focal_manager.update_transition(context)
			
			if transition_completed and self._finished:
				logger.log_debug("Exit transition completed, finishing operator")
				return self._finish_operator(context)

		# Handle auto-navigation timeout
		if (self._waiting_for_input and 
			not self._navigation_active and 
			addon_prefs.time > 0 and 
			self._time_elapsed >= addon_prefs.time):
			
			logger.log_debug("Auto-navigation timeout reached")
			if self._start_navigation(context, addon_prefs):
				return {"RUNNING_MODAL"}
			else:
				return self._finish_operator(context)

		return {"PASS_THROUGH"}

	def _handle_finished_state(self, context, event, addon_prefs):
		"""Handle events when operator is marked as finished."""
		logger.log_debug(f"Handling finished state for event: {event.type}")
		
		# If focal manager is still transitioning, wait for it
		if self._focal_manager and self._focal_manager.is_transitioning:
			logger.log_debug("Waiting for focal transition to complete")
			return {"PASS_THROUGH"}

		# Try to start exit transition if not already done
		if (self._focal_manager and 
			not self._focal_manager.exit_transition_attempted and
			self._focal_manager.start_exit_transition(context, addon_prefs)):
			logger.log_debug("Started exit transition")
			return {"PASS_THROUGH"}

		# No more transitions needed, finish now
		logger.log_debug("No more transitions, finishing operator")
		return self._finish_operator(context)

	def _handle_navigation_events(self, context, event, addon_prefs):
		"""Handle events while navigation is active."""
		instance_id = getattr(self, '_instance_id', 0)

		# Ignore mouse movement and timer events during navigation
		if event.type in {"MOUSEMOVE", "INBETWEEN_MOUSEMOVE", "TIMER"}:
			return {"PASS_THROUGH"}

		time_val = getattr(addon_prefs, 'time', 0.1)

		# Special handling for right mouse button release - this stops navigation for both time=0 and time>0
		if event.type == "RIGHTMOUSE" and event.value == "RELEASE":
			logger.log_debug(f"Instance #{instance_id}: Right mouse button released during navigation.")
			
			# Context menu only for time > 0 and quick click
			if time_val > 0 and self._time_elapsed < addon_prefs.time: # Using addon_prefs.time is fine here
				self._call_menu = True
				logger.log_debug(f"Instance #{instance_id}: Quick RMB release (time_val > 0), scheduling context menu.")
			
			self._navigation_active = False # Stop our operator's sense of navigation
			self._finished = True           # Mark operator as finishing

			if self._focal_manager:
				if self._focal_manager.interrupt_and_exit(context, addon_prefs):
					logger.log_debug(f"Instance #{instance_id}: Focal length exit transition started due to RMB release.")
					return {"PASS_THROUGH"} # Wait for transition to complete via timer
				else:
					logger.log_debug(f"Instance #{instance_id}: No focal length exit transition needed for RMB release.")
			
			# If no transition was started or needed, finish operator immediately
			return self._finish_operator(context)

		# For events other than RMB release:
		if time_val == 0:
			# When time is 0, navigation continues as long as RMB is held.
			# Keys like W,A,S,D are handled by Blender's walk modal.
			# We just pass them through and keep our operator active and focal length adjusted.
			logger.log_debug(f"Instance #{instance_id}: time=0, event {event.type}({event.value}) received. Passing to walk modal.")
			return {"PASS_THROUGH"}
		else: # time_val > 0
			# For time > 0, other events (like ESC from walk mode, or other configured walk-mode exit keys)
			# should terminate our operator's navigation mode.
			# Blender's walk modal will handle the event first (e.g., ESC closes walk modal).
			# Our operator then recognizes this as the end of navigation.
			logger.log_debug(f"Instance #{instance_id}: time > 0, navigation ending due to event: {event.type}({event.value}).")
			self._navigation_active = False
			self._finished = True
			
			if self._focal_manager:
				# interrupt_and_exit will start the exit transition if one hasn't started
				self._focal_manager.interrupt_and_exit(context, addon_prefs)
				logger.log_debug(f"Instance #{instance_id}: Focal length exit transition initiated due to navigation ending (time > 0).")
			
			# We return PASS_THROUGH here to:
			# 1. Allow Blender's walk modal to process the event that ended it (e.g., ESC).
			# 2. Allow our focal length exit transition (if started) to complete via the modal timer.
			# The _finished flag ensures the modal will eventually call _finish_operator.
			return {"PASS_THROUGH"}

	def _handle_waiting_events(self, context, event, addon_prefs):
		"""Handle events while waiting for user input."""
		# Check for navigation keys
		if event.type in self.NAV_KEYS and event.value == "PRESS":
			logger.log_debug(f"Navigation key pressed: {event.type}")
			if self._start_navigation(context, addon_prefs):
				return {"RUNNING_MODAL"}
			else:
				return self._finish_operator(context)

		# Check for right mouse button release
		if event.type == "RIGHTMOUSE" and event.value == "RELEASE":
			logger.log_debug("Right mouse button released while waiting")
			
			# Quick release means call menu
			if self._time_elapsed < addon_prefs.time:
				self._call_menu = True
				logger.log_debug("Quick release detected, will call menu")

			self._finished = True
			return self._finish_operator(context)
		return {"PASS_THROUGH"}

	def _start_navigation(self, context, addon_prefs, force_fast_mode=None):
		"""Start walk navigation mode."""
		logger.log_debug("Starting navigation mode")
		
		# Check camera navigation permissions
		if not self._check_camera_navigation_allowed(context, addon_prefs):
			logger.log_debug("Camera navigation not allowed")
			return False

		# If force_fast_mode is not None, pass it to focal_manager
		if force_fast_mode is not None and self._focal_manager and addon_prefs:
			self._focal_manager.start_entry_transition(context, addon_prefs, fast_mode=force_fast_mode)
			self._fast_mode_applied = force_fast_mode
		success, restore_ortho = start_walk_navigation(context, self._focal_manager, addon_prefs)
		if success:
			self._restore_ortho = restore_ortho
			self._navigation_active = True
			self._waiting_for_input = False
			return True
		else:
			self.report({"WARNING"}, "Navigation failed. View might be locked or constrained.")
			return False

	def _check_camera_navigation_allowed(self, context, addon_prefs):
		"""Check if camera navigation is allowed based on preferences."""
		if not context.space_data.region_3d:
			return False

		view_perspective = context.space_data.region_3d.view_perspective
		
		if view_perspective == "CAMERA":
			if not addon_prefs.enable_camera_navigation:
				logger.log_debug("Camera navigation disabled in preferences")
				return False
			
			if addon_prefs.camera_nav_only_if_locked and not context.space_data.lock_camera:
				logger.log_debug("Camera navigation requires locked camera")
				return False

		return True
	def _finish_operator(self, context):
		"""Perform final cleanup and finish the operator."""
		instance_id = getattr(self, '_instance_id', 0)
		logger.log_debug(f"=== Finishing Operator (Instance #{instance_id}) ===")
		
		# Remove timer
		if self._timer:
			try:
				context.window_manager.event_timer_remove(self._timer)
				logger.log_debug(f"Instance #{instance_id}: Timer removed")
			except Exception as e:
				logger.log_warning(f"Instance #{instance_id}: Failed to remove timer: {e}")
			self._timer = None

		# Call context menu if needed
		if self._call_menu:
			self._show_context_menu(context)

		# Restore orthographic view if needed
		if self._restore_ortho and context.space_data and context.space_data.region_3d:
			if context.space_data.region_3d.view_perspective != 'ORTHO':
				try:
					bpy.ops.view3d.view_persportho()
					logger.log_debug(f"Instance #{instance_id}: Restored orthographic view")
				except Exception as e:
					logger.log_warning(f"Instance #{instance_id}: Failed to restore ortho view: {e}")

		# Cleanup focal length manager
		if self._focal_manager:
			addon_prefs = get_addon_preferences(context) or self._get_default_prefs()
			self._focal_manager.cleanup(context, addon_prefs)
			logger.log_debug(f"Instance #{instance_id}: Focal manager cleanup completed")

		# Release global lock
		FLYNAV_OT_right_mouse_navigation._global_instance_running = False
		logger.log_debug(f"Instance #{instance_id}: Global lock released")

		# --- Queue system: If there are queued activations, start the next one ---
		if FLYNAV_OT_right_mouse_navigation._activation_queue:
			logger.log_debug(f"Operator finished. Activations in queue: {len(FLYNAV_OT_right_mouse_navigation._activation_queue)}. Launching next queued activation.")
			# Pop the next activation and start a new operator instance
			try:
				queued_context, queued_type, queued_value = FLYNAV_OT_right_mouse_navigation._activation_queue.popleft()
				# Use bpy.ops to invoke the operator again (simulate the input)
				bpy.ops.flynav.right_mouse_navigation('INVOKE_DEFAULT')
				logger.log_debug("Queued activation started.")
			except Exception as e:
				logger.log_error(f"Failed to start queued activation: {e}")

		logger.log_debug(f"=== Operator Finished (Instance #{instance_id}) ===")
		return {"CANCELLED"}

	def _show_context_menu(self, context):
		"""Show the appropriate context menu."""
		mode = context.mode
		menu_name = self.CONTEXT_MENUS.get(mode, "VIEW3D_MT_object_context_menu")
		
		try:
			bpy.ops.wm.call_menu(name=menu_name)
			logger.log_debug(f"Called context menu: {menu_name}")
		except Exception as e:
			logger.log_error(f"Failed to call menu {menu_name}: {e}")

	def _reset_state(self):
		"""Reset all operator state variables."""
		self._timer = None
		self._time_elapsed = 0.0
		self._finished = False
		self._call_menu = False
		self._restore_ortho = False
		self._focal_manager = None
		self._waiting_for_input = True
		self._navigation_active = False
		self._initial_mouse_pos = None
		self._instance_id = 0
		self._stored_view_matrix = None
		self._stored_view_location = None
		self._stored_view_rotation = None
		self._stored_view_distance = None
		self._initial_fast_mode_applied = False
		self._lock_fast_mode = False
		self._locked_fast_mode_value = False

	def _get_default_prefs(self):
		"""Get default preferences when addon prefs are unavailable."""
		class DefaultPrefs:
			time = 0.3
			enable_camera_navigation = True
			camera_nav_only_if_locked = False
			return_to_ortho_on_exit = True
		
		return DefaultPrefs()

	def cancel(self, context):
		"""Handle operator cancellation."""
		logger.log_debug("Operator cancel called")
		
		if self._focal_manager and context.space_data:
			addon_prefs = get_addon_preferences(context) or self._get_default_prefs()
			
			# If in entry transition, force restore
			if (self._focal_manager.is_transitioning and 
				not self._focal_manager.is_exit_transition):
				logger.log_debug("Forcing restore during entry transition")
				self._focal_manager.force_restore_original(context, addon_prefs)
			
			# If navigation was active, start exit transition
			elif self._navigation_active and not self._focal_manager.exit_transition_attempted:
				logger.log_debug("Starting exit transition from cancel")
				self._focal_manager.start_exit_transition(context, addon_prefs)

	@classmethod
	def poll(cls, context):
		"""Check if operator can run."""
		return context.area and context.area.type == 'VIEW_3D'


class FLYNAV_OT_simple_fly(Operator):
	"""Simple fly operator for direct navigation."""
	bl_idname = "flynav.simple_fly"
	bl_label = "Simple Fly Mode"
	bl_options = {'REGISTER', 'UNDO'}

	def execute(self, context):
		"""Execute simple fly mode."""
		logger.log_debug("Simple fly mode activated")
		
		# Get preferences if needed
		try:
			addon_prefs = get_addon_preferences(context)
		except:
			addon_prefs = None
			
		success, _ = start_walk_navigation(context)
		
		if success:
			self.report({'INFO'}, "Fly Mode Activated")
			return {'FINISHED'}
		else:
			self.report({'ERROR'}, "Failed to start navigation")
			return {'CANCELLED'}

	@classmethod
	def poll(cls, context):
		"""Check if operator can run."""
		return context.area and context.area.type == 'VIEW_3D'


# Registration
classes = (
	FLYNAV_OT_right_mouse_navigation,
	FLYNAV_OT_simple_fly,
)

def register():
	"""Register all operator classes."""
	for cls in classes:
		bpy.utils.register_class(cls)
	logger.log_debug("Operators registered")

def unregister():
	"""Unregister all operator classes."""
	for cls in reversed(classes):
		bpy.utils.unregister_class(cls)
	logger.log_debug("Operators unregistered")

if __name__ == "__main__":
	register()
