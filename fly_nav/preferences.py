import bpy
from bpy.props import (
	BoolProperty,
	FloatProperty,
	# StringProperty, # Not used in the final merged code
	EnumProperty,
)
from bpy.types import AddonPreferences, Operator
import rna_keymap_ui # Ensure it's imported for draw_kmi

from . import logger # Import the logger

ADDON_PACKAGE_NAME = "" # Will be set by __init__.py

# Constants for keymap identification
FLYNAV_OPERATOR_IDNAME = "flynav.right_mouse_navigation"  # Main operator ID for FlyNav
FLYNAV_CUSTOM_KMI_ID = "flynav_custom_activation_kmi" # Unique name for the custom keymap item


# Update function for keymap-affecting preferences
def _update_keymaps_logic(self_prefs, context):
	"""Requests keymap update when activation preferences change."""
	logger.log_debug(f"_update_keymaps_logic called by: {self_prefs}", module_name="Preferences")
	try:
		# This assumes a keymaps.py module will be created in the fly_nav package
		from . import keymaps 
		logger.log_info("Triggering keymap reregistration.", module_name="Preferences")
		keymaps.unregister_keymaps()
		keymaps.register_keymaps()
		logger.log_info("Keymaps reregistered.", module_name="Preferences")
	except ImportError:
		logger.log_info(
			"Keymap update skipped: 'fly_nav.keymaps' module or its functions not found. Please implement.",
			module_name="Preferences"
		)
	except Exception as e:
		logger.log_error(f"Error during keymap reregistration: {e}", module_name="Preferences")


class FlyNavRefreshKeymapsOperator(Operator):
	"""Operator to manually refresh keymaps based on current preferences."""

	bl_idname = "flynav.refresh_keymaps"
	bl_label = "Apply Custom Key and Refresh All Keymaps"
	bl_description = "Applies any changes to the custom activation key and re-registers all addon keymaps according to current settings."

	def execute(self, context):
		if not ADDON_PACKAGE_NAME:
			logger.log_error("ADDON_PACKAGE_NAME not set. Cannot refresh keymaps.", module_name="Preferences.Operator")
			self.report({'ERROR'}, "Addon package name not set.")
			return {'CANCELLED'}
		prefs = context.preferences.addons[ADDON_PACKAGE_NAME].preferences
		_update_keymaps_logic(prefs, context)
		self.report({'INFO'}, "Keymaps refreshed for FlyNav.")
		return {'FINISHED'}


# Move _enum_update_callback outside the FlyNavPreferences class
def _enum_update_callback(self, context):
	_update_keymaps_logic(self, context)

class FlyNavPreferences(AddonPreferences):
	# bl_idname will be set by the root __init__.py to ADDON_PACKAGE_NAME
	# For linters and direct access, we can assign it here if ADDON_PACKAGE_NAME is known at this point,
	# but the dynamic assignment from __init__ is typical for Blender addons.
	# bl_idname = ADDON_PACKAGE_NAME 

	# Original FlyNav property
	fly_speed: bpy.props.FloatProperty( # type: ignore
		name="Fly Speed",
		description="Speed of the fly navigation",
		default=1.0,
		min=0.1,
		max=10.0,
	)

	# --- Properties from RightMouseNavigation --- 
	time: FloatProperty( # type: ignore
		name="Time Threshold (RMB)",
		description="How long to hold right mouse before auto-activating walk mode (also determines menu timing on release)",
		default=0.1,
		min=0.01,
		max=1,
	)

	return_to_ortho_on_exit: BoolProperty( # type: ignore
		name="Return to Orthographic on Exit",
		description="After exiting navigation, determines if the Viewport returns to Orthographic or remains Perspective",
		default=True,
	)

	enable_camera_navigation: BoolProperty( # type: ignore
		name="Enable Navigation in Camera View",
		description="Allow navigation while in camera view. If disabled, navigation will not affect camera transform.",
		default=True,
	)

	camera_nav_only_if_locked: BoolProperty( # type: ignore
		name="Only when Camera is Locked to View",
		description="If enabled, navigation in camera view is only active when Blender's 'Lock Camera to View' is also active.",
		default=True,
	)

	walk_mode_focal_length_enable: BoolProperty( # type: ignore
		name="Switch Focal Length while Active",
		description="Enable to switch focal length during walk/fly mode",
		default=True,
	)

	walk_mode_focal_length: FloatProperty( # type: ignore
		name="Focal Length",
		description="Focal length for the viewport during walk/fly mode.",
		default=30.0,
		min=0.0,
		max=250.0,
		subtype='UNSIGNED',
		unit='CAMERA'
	)

	walk_mode_transition_duration: FloatProperty( # type: ignore
		name="Transition Duration",
		description="Duration of focal length transition in seconds (0 = instant)",
		default=0.1,
		min=0.0,
		max=1.0,
		step=1,
		precision=2,
		subtype='TIME'
	)

	activation_method: EnumProperty( # type: ignore
		name="Activation Method",
		description="Choose how to activate navigation",
		items=[
			('RMB', "Right Mouse", "Standard timed activation (allows context menu on short press)"),
			('KEY', "Keyboard Key", "Instant activation with a specified keyboard key (no context menu, editable below)")
		],
		default='RMB',
		update=_enum_update_callback
	)

	# --- End of Properties from RightMouseNavigation ---

	def draw(self, context):
		layout = self.layout
		wm = context.window_manager

		# Original Fly Speed Property
		box_general = layout.box()
		box_general.label(text="General Fly Settings", icon="SETTINGS")
		box_general.prop(self, "fly_speed")

		# Activation Settings Box (from RMN)
		box_activation = layout.box()
		box_activation.label(text="Activation Settings", icon="KEYINGSET")
		box_activation.prop(self, "activation_method")

		if self.activation_method == 'KEY':
			box_activation.label(text="Custom Activation Key (Editable):")
			addon_kc = wm.keyconfigs.addon
			km_3d_view_addon = None
			for km_map in addon_kc.keymaps:
				if km_map.name == "3D View" and km_map.space_type == "VIEW_3D":
					km_3d_view_addon = km_map
					break
			
			custom_kmi_to_draw = None
			logger.log_debug("Searching for custom keymap item in '3D View' keymap.", module_name="Preferences")
			if km_3d_view_addon:
				for kmi in km_3d_view_addon.keymap_items:
					if kmi.idname == FLYNAV_OPERATOR_IDNAME and kmi.type == "F" and kmi.ctrl:
						custom_kmi_to_draw = kmi
						break
				if not custom_kmi_to_draw:
					logger.log_warning("Custom keymap item not found. Ensure it is properly initialized.", module_name="Preferences")
			else:
				logger.log_error("'3D View' keymap not found in addon keyconfigs.", module_name="Preferences")
			
			if custom_kmi_to_draw:
				col = box_activation.column()
				col.context_pointer_set("keymap", km_3d_view_addon) 
				rna_keymap_ui.draw_kmi([], addon_kc, km_3d_view_addon, custom_kmi_to_draw, col, 0)
				box_activation.operator(FlyNavRefreshKeymapsOperator.bl_idname, icon='FILE_REFRESH')
			else:
				box_activation.label(text="Custom key item not found. Configure or re-select 'KEY'.", icon='ERROR')
				box_activation.operator(FlyNavRefreshKeymapsOperator.bl_idname, text="Initialize/Refresh Custom Key", icon='FILE_REFRESH')

		row_time_outer = layout.row()
		box_timing = row_time_outer.box()
		box_timing.label(text="Menu / Movement Delay (RMB Only)", icon="DRIVER_DISTANCE")
		row_time_prop = box_timing.row()
		row_time_prop.prop(self, "time")
		row_time_prop.enabled = self.activation_method == 'RMB'
		if self.activation_method != 'RMB':
			box_timing.label(text="Delay not applicable for current activation method.")

		row2 = layout.row()

		box_camera = row2.box()
		box_camera.label(text="Camera Navigation", icon="CAMERA_DATA")
		box_camera.prop(self, "enable_camera_navigation")
		if self.enable_camera_navigation:
			box_camera.prop(self, "camera_nav_only_if_locked")

		box_view = row2.box()
		box_view.label(text="View Settings", icon="VIEW3D")
		box_view.prop(self, "return_to_ortho_on_exit")
		box_view.prop(self, "walk_mode_focal_length_enable")
		if self.walk_mode_focal_length_enable:
			box_view.prop(self, "walk_mode_focal_length")
			box_view.prop(self, "walk_mode_transition_duration")

		if self.activation_method == 'RMB':
			layout.label(text="Hint: With Right Mouse, a quick click opens the context menu.")
		elif self.activation_method == 'KEY':
			layout.label(text="Hint: Custom key (configured above) activates navigation instantly.")

		# Keymap Customization for Walk Modal keys (from RMN)
		nav_prop_values = [ # These are kmi.propvalue from the walk modal keymap items
			"FORWARD", "FORWARD_STOP", "BACKWARD", "BACKWARD_STOP",
			"LEFT", "LEFT_STOP", "RIGHT", "RIGHT_STOP",
			"UP", "UP_STOP", "DOWN", "DOWN_STOP",
			"LOCAL_UP", "LOCAL_UP_STOP", "LOCAL_DOWN", "LOCAL_DOWN_STOP",
		]

		active_kc = wm.keyconfigs.active
		addon_keymaps_to_draw = []
		walk_km_name = "View3D Walk Modal" # Standard Blender keymap name

		if walk_km_name in active_kc.keymaps:
			walk_km = active_kc.keymaps[walk_km_name]
			for kmi in walk_km.keymap_items:
				if hasattr(kmi, 'propvalue') and kmi.propvalue in nav_prop_values: # Filter for relevant walk modal keys
					addon_keymaps_to_draw.append((walk_km, kmi))
		
		addon_keymaps_to_draw.sort(key=lambda item: (item[1].propvalue, item[1].type if hasattr(item[1], 'type') else ''))

		header, panel = layout.panel(idname="flynav_keymap_panel", default_closed=True)
		header.label(text="Navigation Modal Keymap (View3D Walk Modal)", icon="TOOL_SETTINGS") # Changed icon

		if panel:
			if not addon_keymaps_to_draw:
				panel.label(text="Could not find 'View3D Walk Modal' keymap items.", icon="ERROR")
			else:
				col = panel.column(align=True)
				for km_map, kmi_item in addon_keymaps_to_draw:
					col.context_pointer_set("keymap", km_map)
					rna_keymap_ui.draw_kmi([], active_kc, km_map, kmi_item, col, 0)
					col.separator()

# --- Preference Storage (Updated) ---
def get_addon_preferences(context): # Add context as a parameter
	"""Safely get addon preferences, even if the addon is not loaded."""
	if not ADDON_PACKAGE_NAME:
		logger.log_error("ADDON_PACKAGE_NAME not set. Cannot get preferences.", module_name="Preferences")
		return None
	try:
		return context.preferences.addons[ADDON_PACKAGE_NAME].preferences # Use the passed context
	except (KeyError, AttributeError):
		logger.log_warning(f"Could not retrieve preferences for {ADDON_PACKAGE_NAME}. Addon might not be enabled yet.", module_name="Preferences")
		return None

PREFERENCE_PROPERTIES = [
	'fly_speed',
	'time',
	'return_to_ortho_on_exit',
	'enable_camera_navigation',
	'camera_nav_only_if_locked',
	'walk_mode_focal_length_enable',
	'walk_mode_focal_length',
	'walk_mode_transition_duration',
	'activation_method',
]

def store_preferences(context): # Add context
	"""Store current preferences into bpy.app.driver_namespace."""
	prefs = get_addon_preferences(context) # Pass context
	if prefs and 'fly_nav_stored_prefs' in bpy.app.driver_namespace:
		stored_prefs_dict = bpy.app.driver_namespace['fly_nav_stored_prefs']
		for prop_name in PREFERENCE_PROPERTIES:
			if hasattr(prefs, prop_name):
				stored_prefs_dict[prop_name] = getattr(prefs, prop_name)
		logger.log_info(f"Stored preferences for {ADDON_PACKAGE_NAME}", module_name="Preferences")
	elif not prefs:
		logger.log_warning("Could not store preferences: Preferences object not found.", module_name="Preferences")
	elif 'fly_nav_stored_prefs' not in bpy.app.driver_namespace:
		logger.log_warning("Could not store preferences: 'fly_nav_stored_prefs' not in driver_namespace.", module_name="Preferences")

def restore_preferences(context): # Add context
	"""Restore preferences from bpy.app.driver_namespace."""
	prefs = get_addon_preferences(context) # Pass context
	if prefs and 'fly_nav_stored_prefs' in bpy.app.driver_namespace:
		stored_prefs_dict = bpy.app.driver_namespace['fly_nav_stored_prefs']
		for prop_name in PREFERENCE_PROPERTIES:
			if prop_name in stored_prefs_dict and hasattr(prefs, prop_name):
				try:
					setattr(prefs, prop_name, stored_prefs_dict[prop_name])
				except TypeError as e:
					logger.log_error(f"Error restoring preference '{prop_name}': {e}. Value: {stored_prefs_dict[prop_name]}", module_name="Preferences")
		logger.log_info(f"Restored preferences for {ADDON_PACKAGE_NAME}", module_name="Preferences")
	elif not prefs:
		logger.log_warning("Could not restore preferences: Preferences object not found.", module_name="Preferences")
	elif 'fly_nav_stored__prefs' not in bpy.app.driver_namespace: # Typo fixed here
		logger.log_warning("Could not restore preferences: 'fly_nav_stored_prefs' not in driver_namespace.", module_name="Preferences")

# Ensure bl_idname is set if ADDON_PACKAGE_NAME is available during module load.
# This helps linters but might be redundant if __init__.py always sets it before registration.
if ADDON_PACKAGE_NAME:
	FlyNavPreferences.bl_idname = ADDON_PACKAGE_NAME

logger.log_debug(f"Preferences module loaded for: {ADDON_PACKAGE_NAME or 'fly_nav (default)'}", module_name="Preferences")
