import bpy # type: ignore
import time
from . import logger

def fml_log(message):
	# Legacy stub, replaced by logger.log_debug
	pass

class FocalLengthManager:
	"""Manages focal length transitions for walk/fly mode"""
	
	# Class variables to persist across instances
	_global_true_original_lens = None
	_global_walk_mode_session_active = False
	_last_activity_time = 0.0
	_cleanup_delay = 1.0
	
	def __init__(self):
		logger.log_debug("__init__ called", module_name="FocalLengthManager")
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
		logger.log_debug(f"__init__: Initial self.true_original_lens from global: {self.true_original_lens}", module_name="FocalLengthManager")
	
	def reset(self):
		logger.log_debug("reset called - Instance state cleared", module_name="FocalLengthManager")
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
	
	def start_entry_transition(self, context, addon_prefs, fast_mode=False, fast_mode_exit=False):
		logger.log_debug("start_entry_transition: Called", module_name="FocalLengthManager")
		if self.is_transitioning:
			logger.log_debug("start_entry_transition: Already transitioning, returning.", module_name="FocalLengthManager")
			return

		if not self.should_change_focal_length(addon_prefs):
			logger.log_debug("start_entry_transition: Focal length change not enabled or not needed, returning.", module_name="FocalLengthManager")
			return

		import time
		FocalLengthManager._last_activity_time = time.time()

		current_lens = context.space_data.lens
		walk_focal_length = getattr(addon_prefs, "walk_mode_focal_length", 30.0)
		fast_offset = getattr(addon_prefs, "walk_mode_fast_offset", 0.0)
		# Track last fast mode state for exit transition logic
		if not hasattr(self, '_last_fast_mode'):
			self._last_fast_mode = False
		# Choose transition duration based on fast mode and fast mode exit
		if fast_mode:
			walk_focal_length = max(0.0, walk_focal_length - fast_offset)
			transition_duration = getattr(addon_prefs, 'walk_mode_fast_transition_duration', 0.08) if addon_prefs else 0.08
		elif fast_mode_exit:
			transition_duration = getattr(addon_prefs, 'walk_mode_fast_transition_duration', 0.08) if addon_prefs else 0.08
		else:
			transition_duration = getattr(addon_prefs, 'walk_mode_transition_duration', 0.3) if addon_prefs else 0.3
		logger.log_debug(f"start_entry_transition: current_lens={current_lens}, walk_focal_length={walk_focal_length}, fast_mode={fast_mode}, fast_mode_exit={fast_mode_exit}, transition_duration={transition_duration}", module_name="FocalLengthManager")
		self._last_fast_mode = fast_mode

		if not FocalLengthManager._global_walk_mode_session_active:
			logger.log_debug("start_entry_transition: New global walk mode session.", module_name="FocalLengthManager")
			if FocalLengthManager._global_true_original_lens is None:
				FocalLengthManager._global_true_original_lens = current_lens
				logger.log_debug(f"start_entry_transition: Set _global_true_original_lens to current_lens: {current_lens}", module_name="FocalLengthManager")
			elif abs(current_lens - walk_focal_length) > 0.001: # Only update if not starting from the target FL
				FocalLengthManager._global_true_original_lens = current_lens
				logger.log_debug(f"start_entry_transition: Updated _global_true_original_lens to current_lens: {current_lens} (not starting from walk FL)", module_name="FocalLengthManager")
			else:
				logger.log_debug(f"start_entry_transition: Retaining existing _global_true_original_lens: {FocalLengthManager._global_true_original_lens} (current is walk FL)", module_name="FocalLengthManager")

		if FocalLengthManager._global_true_original_lens is not None:
			self.true_original_lens = FocalLengthManager._global_true_original_lens
		else: # Fallback, should ideally not be hit if logic above is sound
			self.true_original_lens = current_lens
			FocalLengthManager._global_true_original_lens = current_lens
			logger.log_debug(f"start_entry_transition: Fallback - Set _global_true_original_lens & self.true_original_lens to current_lens: {current_lens}", module_name="FocalLengthManager")

		self.original_lens = self.true_original_lens # Instance's original for this transition session
		logger.log_debug(f"start_entry_transition: self.true_original_lens = {self.true_original_lens}", module_name="FocalLengthManager")

		FocalLengthManager._global_walk_mode_session_active = True
		logger.log_debug("start_entry_transition: _global_walk_mode_session_active = True", module_name="FocalLengthManager")

		self.walk_mode_ever_activated = True
		self.exit_transition_attempted = False # Reset for new entry
		if abs(current_lens - walk_focal_length) > 0.001:
			logger.log_debug("start_entry_transition: Starting transition to walk_focal_length.", module_name="FocalLengthManager")
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
		logger.log_debug("start_exit_transition: Called", module_name="FocalLengthManager")
		if self.is_transitioning:
			logger.log_debug("start_exit_transition: Already transitioning, returning False.", module_name="FocalLengthManager")
			return False

		return self._start_exit_transition_internal(context, addon_prefs)

	def interrupt_and_exit(self, context, addon_prefs):
		"""Interrupt any ongoing transition and immediately start exit transition."""
		logger.log_debug("interrupt_and_exit: Called", module_name="FocalLengthManager")
		
		if self.is_transitioning and not self.is_exit_transition:
			logger.log_debug("interrupt_and_exit: Interrupting ongoing entry transition", module_name="FocalLengthManager")
			# Stop the current transition
			self.is_transitioning = False
			self.is_exit_transition = False
		
		# Now start the exit transition
		return self._start_exit_transition_internal(context, addon_prefs)

	def _start_exit_transition_internal(self, context, addon_prefs):
		"""Internal method to start exit transition without transition checking."""
		logger.log_debug("_start_exit_transition_internal: Called", module_name="FocalLengthManager")

		FocalLengthManager._last_activity_time = time.time()

		if self.exit_transition_attempted:
			fml_log("_start_exit_transition_internal: Exit transition already attempted this session, returning False.")
			return False

		self.exit_transition_attempted = True

		target_lens = self.true_original_lens if self.true_original_lens is not None else self.original_lens
		logger.log_debug(f"_start_exit_transition_internal: Determined target_lens={target_lens} (true_original_lens={self.true_original_lens}, original_lens={self.original_lens})", module_name="FocalLengthManager")

		if not self.should_change_focal_length(addon_prefs) or target_lens is None:
			logger.log_debug("_start_exit_transition_internal: Conditions not met (change disabled or target_lens is None), returning False.", module_name="FocalLengthManager")
			return False
		current_lens = context.space_data.lens
		logger.log_debug(f"_start_exit_transition_internal: current_lens={current_lens}", module_name="FocalLengthManager")
		# Use fast transition duration if last state was fast mode, else normal
		walk_focal_length = getattr(addon_prefs, "walk_mode_focal_length", 30.0)
		fast_offset = getattr(addon_prefs, "walk_mode_fast_offset", 0.0)
		if hasattr(self, '_last_fast_mode') and self._last_fast_mode:
			transition_duration = getattr(addon_prefs, 'walk_mode_fast_transition_duration', 0.08) if addon_prefs else 0.08
		else:
			transition_duration = getattr(addon_prefs, 'walk_mode_transition_duration', 0.3) if addon_prefs else 0.3
		self._last_fast_mode = False

		if abs(current_lens - target_lens) > 0.001:
			logger.log_debug(f"_start_exit_transition_internal: Starting transition to target_lens. transition_duration={transition_duration}", module_name="FocalLengthManager")
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
			logger.log_debug("_start_exit_transition_internal: Lens already at target, calling _cleanup_after_exit.", module_name="FocalLengthManager")
			self._cleanup_after_exit()

		return False
	
	def _cleanup_after_exit(self):
		logger.log_debug("_cleanup_after_exit: Called - Instance lens tracking reset.", module_name="FocalLengthManager")
		self.original_lens = None
		self.true_original_lens = None
		self.walk_mode_ever_activated = False
	
	def clear_global_state(self):
		logger.log_debug("clear_global_state: Called - Global lens and session activity cleared.", module_name="FocalLengthManager")
		FocalLengthManager._global_true_original_lens = None
		FocalLengthManager._global_walk_mode_session_active = False
		logger.log_debug(f"clear_global_state: _global_true_original_lens = {FocalLengthManager._global_true_original_lens}, _global_walk_mode_session_active = {FocalLengthManager._global_walk_mode_session_active}", module_name="FocalLengthManager")
	
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
			logger.log_debug("update_transition: Transition params missing, stopping.", module_name="FocalLengthManager")
			self.is_transitioning = False
			return True
		
		if elapsed >= self.transition_duration:
			logger.log_debug(f"update_transition: Transition complete. Target: {self.transition_target_lens}. Was exit: {self.is_exit_transition}", module_name="FocalLengthManager")
			context.space_data.lens = self.transition_target_lens
			context.area.tag_redraw()
			
			was_exit_transition = self.is_exit_transition
			
			self.is_transitioning = False
			self.is_exit_transition = False
			
			if was_exit_transition:
				logger.log_debug("update_transition: Successful exit transition, calling _cleanup_after_exit.", module_name="FocalLengthManager")
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
		logger.log_debug("force_restore_original: Called", module_name="FocalLengthManager")
		if self.is_transitioning:
			logger.log_debug("force_restore_original: Currently transitioning, returning to avoid interruption.", module_name="FocalLengthManager")
			return

		target_lens = self.true_original_lens if self.true_original_lens is not None else self.original_lens
		logger.log_debug(f"force_restore_original: Determined target_lens={target_lens}", module_name="FocalLengthManager")
		
		if (target_lens is not None and 
			self.should_change_focal_length(addon_prefs) and
			context.space_data and context.space_data.type == "VIEW_3D"):
			logger.log_debug(f"force_restore_original: Setting lens to {target_lens}", module_name="FocalLengthManager")
			context.space_data.lens = target_lens
		else:
			logger.log_debug("force_restore_original: Conditions not met to change lens.", module_name="FocalLengthManager")
		
		self.is_transitioning = False
		self.is_exit_transition = False
		logger.log_debug("force_restore_original: Transition flags reset.", module_name="FocalLengthManager")
	
	def cleanup(self, context, addon_prefs):
		logger.log_debug("cleanup: Called", module_name="FocalLengthManager")
		FocalLengthManager._global_walk_mode_session_active = False
		logger.log_debug(f"cleanup: _global_walk_mode_session_active set to False. Instance state: is_transitioning={self.is_transitioning}, exit_transition_attempted={self.exit_transition_attempted}", module_name="FocalLengthManager")

		should_clear = self._should_clear_global_state()
		logger.log_debug(f"cleanup: _should_clear_global_state() returned {should_clear}", module_name="FocalLengthManager")

		if not self.is_transitioning and should_clear:
			logger.log_debug("cleanup: Not transitioning and should_clear is True. Clearing global state.", module_name="FocalLengthManager")
			self.clear_global_state()
		elif self.exit_transition_attempted and not self.is_transitioning:
			logger.log_debug("cleanup: Exit attempted, not transitioning. Forcing restore.", module_name="FocalLengthManager")
			self.force_restore_original(context, addon_prefs)
			if self._should_clear_global_state():
				logger.log_debug("cleanup: Delay passed after force_restore, clearing global state.", module_name="FocalLengthManager")
				self.clear_global_state()
		else:
			logger.log_debug("cleanup: Conditions for global clear not met (either transitioning or delay not passed for non-exit-attempted cases).", module_name="FocalLengthManager")