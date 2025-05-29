import bpy # type: ignore
import time

# Simple print-based logger for debugging this module
def fml_log(message):
	print(f"[FML_DEBUG] {message}")

class FocalLengthManager:
	"""Manages focal length transitions for walk/fly mode"""
	
	# Class variables to persist across instances
	_global_true_original_lens = None
	_global_walk_mode_session_active = False
	_last_activity_time = 0.0
	_cleanup_delay = 1.0
	
	def __init__(self):
		fml_log("__init__ called")
		self.original_lens = None
		self.true_original_lens = None
		self.is_transitioning = False
		self.transition_start_time = 0.0
		self.transition_duration = 0.12
		self.transition_initial_lens = None
		self.transition_target_lens = None
		self.is_exit_transition = False
		self.exit_transition_attempted = False
		self.walk_mode_ever_activated = False
		
		self.true_original_lens = FocalLengthManager._global_true_original_lens
		fml_log(f"__init__: Initial self.true_original_lens from global: {self.true_original_lens}")
	
	def reset(self):
		fml_log("reset called - Instance state cleared")
		self.original_lens = None
		self.true_original_lens = None
		self.is_transitioning = False
		self.transition_start_time = 0.0
		self.transition_initial_lens = None
		self.transition_target_lens = None
		self.is_exit_transition = False
		self.exit_transition_attempted = False
		self.walk_mode_ever_activated = False
	
	def should_change_focal_length(self, addon_prefs):
		"""Check if focal length should be changed based on preferences"""
		if addon_prefs is None:
			return False
		walk_focal_length_enable = getattr(addon_prefs, "walk_mode_focal_length_enable", False)
		walk_focal_length = getattr(addon_prefs, "walk_mode_focal_length", 0)
		return walk_focal_length_enable and walk_focal_length > 0
	
	def start_entry_transition(self, context, addon_prefs):
		fml_log("start_entry_transition: Called")
		if self.is_transitioning:
			fml_log("start_entry_transition: Already transitioning, returning.")
			return

		if not self.should_change_focal_length(addon_prefs):
			fml_log("start_entry_transition: Focal length change not enabled or not needed, returning.")
			return
		
		import time
		FocalLengthManager._last_activity_time = time.time()
		
		current_lens = context.space_data.lens
		walk_focal_length = getattr(addon_prefs, "walk_mode_focal_length", 30.0)
		fml_log(f"start_entry_transition: current_lens={current_lens}, walk_focal_length={walk_focal_length}")
		
		if not FocalLengthManager._global_walk_mode_session_active:
			fml_log("start_entry_transition: New global walk mode session.")
			if FocalLengthManager._global_true_original_lens is None:
				FocalLengthManager._global_true_original_lens = current_lens
				fml_log(f"start_entry_transition: Set _global_true_original_lens to current_lens: {current_lens}")
			elif abs(current_lens - walk_focal_length) > 0.001: # Only update if not starting from the target FL
				FocalLengthManager._global_true_original_lens = current_lens
				fml_log(f"start_entry_transition: Updated _global_true_original_lens to current_lens: {current_lens} (not starting from walk FL)")
			else:
				fml_log(f"start_entry_transition: Retaining existing _global_true_original_lens: {FocalLengthManager._global_true_original_lens} (current is walk FL)")

		if FocalLengthManager._global_true_original_lens is not None:
			self.true_original_lens = FocalLengthManager._global_true_original_lens
		else: # Fallback, should ideally not be hit if logic above is sound
			self.true_original_lens = current_lens
			FocalLengthManager._global_true_original_lens = current_lens
			fml_log(f"start_entry_transition: Fallback - Set _global_true_original_lens & self.true_original_lens to current_lens: {current_lens}")

		self.original_lens = self.true_original_lens # Instance's original for this transition session
		fml_log(f"start_entry_transition: self.true_original_lens = {self.true_original_lens}")
		
		FocalLengthManager._global_walk_mode_session_active = True
		fml_log("start_entry_transition: _global_walk_mode_session_active = True")
		
		self.walk_mode_ever_activated = True
		self.exit_transition_attempted = False # Reset for new entry
		if abs(current_lens - walk_focal_length) > 0.001:
			fml_log("start_entry_transition: Starting transition to walk_focal_length.")
			transition_duration = getattr(addon_prefs, 'walk_mode_transition_duration', 0.3) if addon_prefs else 0.3
			if transition_duration == 0:
				context.space_data.lens = walk_focal_length
				return
			
			self.transition_initial_lens = current_lens
			self.transition_target_lens = walk_focal_length
			self.is_transitioning = True
			self.is_exit_transition = False
			self.transition_start_time = time.time()
			self.transition_duration = transition_duration
	def start_exit_transition(self, context, addon_prefs):
		fml_log("start_exit_transition: Called")
		if self.is_transitioning:
			fml_log("start_exit_transition: Already transitioning, returning False.")
			return False

		return self._start_exit_transition_internal(context, addon_prefs)

	def interrupt_and_exit(self, context, addon_prefs):
		"""Interrupt any ongoing transition and immediately start exit transition."""
		fml_log("interrupt_and_exit: Called")
		
		if self.is_transitioning and not self.is_exit_transition:
			fml_log("interrupt_and_exit: Interrupting ongoing entry transition")
			# Stop the current transition
			self.is_transitioning = False
			self.is_exit_transition = False
		
		# Now start the exit transition
		return self._start_exit_transition_internal(context, addon_prefs)

	def _start_exit_transition_internal(self, context, addon_prefs):
		"""Internal method to start exit transition without transition checking."""
		fml_log("_start_exit_transition_internal: Called")
		
		FocalLengthManager._last_activity_time = time.time()
		
		if self.exit_transition_attempted:
			fml_log("_start_exit_transition_internal: Exit transition already attempted this session, returning False.")
			return False
			
		self.exit_transition_attempted = True
		
		target_lens = self.true_original_lens if self.true_original_lens is not None else self.original_lens
		fml_log(f"_start_exit_transition_internal: Determined target_lens={target_lens} (true_original_lens={self.true_original_lens}, original_lens={self.original_lens})")
		
		if not self.should_change_focal_length(addon_prefs) or target_lens is None:
			fml_log("_start_exit_transition_internal: Conditions not met (change disabled or target_lens is None), returning False.")
			return False
		current_lens = context.space_data.lens
		fml_log(f"_start_exit_transition_internal: current_lens={current_lens}")        
		if abs(current_lens - target_lens) > 0.001:
			fml_log("_start_exit_transition_internal: Starting transition to target_lens.")
			transition_duration = getattr(addon_prefs, 'walk_mode_transition_duration', 0.3) if addon_prefs else 0.3
			if transition_duration == 0:
				context.space_data.lens = target_lens
				self._cleanup_after_exit()
				return False
			
			self.transition_initial_lens = current_lens
			self.transition_target_lens = target_lens
			self.is_transitioning = True
			self.is_exit_transition = True
			self.transition_start_time = time.time()
			self.transition_duration = transition_duration
			return True # Ensure True is returned when transition starts
		else:
			fml_log("_start_exit_transition_internal: Lens already at target, calling _cleanup_after_exit.")
			self._cleanup_after_exit()
		
		return False
	
	def _cleanup_after_exit(self):
		fml_log("_cleanup_after_exit: Called - Instance lens tracking reset.")
		self.original_lens = None
		self.true_original_lens = None
		self.walk_mode_ever_activated = False
	
	def clear_global_state(self):
		fml_log("clear_global_state: Called - Global lens and session activity cleared.")
		FocalLengthManager._global_true_original_lens = None
		FocalLengthManager._global_walk_mode_session_active = False
		fml_log(f"clear_global_state: _global_true_original_lens = {FocalLengthManager._global_true_original_lens}, _global_walk_mode_session_active = {FocalLengthManager._global_walk_mode_session_active}")
	
	def _should_clear_global_state(self):
		if FocalLengthManager._last_activity_time == 0.0:
			return True
		
		time_since_activity = time.time() - FocalLengthManager._last_activity_time
		return time_since_activity >= FocalLengthManager._cleanup_delay
	
	def update_transition(self, context):
		if not self.is_transitioning:
			return False
		
		elapsed = time.time() - self.transition_start_time
		
		if (self.transition_initial_lens is None or 
			self.transition_target_lens is None):
			fml_log("update_transition: Transition params missing, stopping.")
			self.is_transitioning = False
			return True
		
		if elapsed >= self.transition_duration:
			fml_log(f"update_transition: Transition complete. Target: {self.transition_target_lens}. Was exit: {self.is_exit_transition}")
			context.space_data.lens = self.transition_target_lens
			context.area.tag_redraw()
			
			was_exit_transition = self.is_exit_transition
			
			self.is_transitioning = False
			self.is_exit_transition = False
			
			if was_exit_transition:
				fml_log("update_transition: Successful exit transition, calling _cleanup_after_exit.")
				self._cleanup_after_exit()
			
			return True
		else:
			t = elapsed / self.transition_duration
			t = 1 - (1 - t) * (1 - t)
			new_lens = (self.transition_initial_lens + 
					   (self.transition_target_lens - self.transition_initial_lens) * t)
			context.space_data.lens = new_lens
			context.area.tag_redraw()
			
			return False
	
	def force_restore_original(self, context, addon_prefs):
		fml_log("force_restore_original: Called")
		if self.is_transitioning:
			fml_log("force_restore_original: Currently transitioning, returning to avoid interruption.")
			return

		target_lens = self.true_original_lens if self.true_original_lens is not None else self.original_lens
		fml_log(f"force_restore_original: Determined target_lens={target_lens}")
		
		if (target_lens is not None and 
			self.should_change_focal_length(addon_prefs) and
			context.space_data and context.space_data.type == "VIEW_3D"):
			fml_log(f"force_restore_original: Setting lens to {target_lens}")
			context.space_data.lens = target_lens
		else:
			fml_log("force_restore_original: Conditions not met to change lens.")
		
		self.is_transitioning = False
		self.is_exit_transition = False
		fml_log("force_restore_original: Transition flags reset.")
	
	def cleanup(self, context, addon_prefs):
		fml_log("cleanup: Called")
		FocalLengthManager._global_walk_mode_session_active = False
		fml_log(f"cleanup: _global_walk_mode_session_active set to False. Instance state: is_transitioning={self.is_transitioning}, exit_transition_attempted={self.exit_transition_attempted}")

		should_clear = self._should_clear_global_state()
		fml_log(f"cleanup: _should_clear_global_state() returned {should_clear}")

		if not self.is_transitioning and should_clear:
			fml_log("cleanup: Not transitioning and should_clear is True. Clearing global state.")
			self.clear_global_state()
		elif self.exit_transition_attempted and not self.is_transitioning:
			fml_log("cleanup: Exit attempted, not transitioning. Forcing restore.")
			self.force_restore_original(context, addon_prefs)
			if self._should_clear_global_state():
				fml_log("cleanup: Delay passed after force_restore, clearing global state.")
				self.clear_global_state()
		else:
			fml_log("cleanup: Conditions for global clear not met (either transitioning or delay not passed for non-exit-attempted cases).")