"""
HDiff Tool - Preferences and settings
"""

import bpy
from bpy.props import StringProperty, BoolProperty, IntProperty
from .core import DEFAULT_HDIFFZ_PATH, DEFAULT_HPATCHZ_PATH

# Global variable to store the addon package name
# This will be set by the main addon module during registration
ADDON_PACKAGE_NAME = ""

class HDIFF_AddonPreferences(bpy.types.AddonPreferences):
	"""HDiff Tool extension preferences"""
	bl_idname = ""  # This will be set dynamically during registration
	
	# Tool paths
	hdiffz_path: StringProperty(
		name="HDiffZ Path",
		description="Path to hdiffz.exe executable",
		default=DEFAULT_HDIFFZ_PATH,
		subtype='FILE_PATH'
	)
	
	hpatchz_path: StringProperty(
		name="HPatchZ Path", 
		description="Path to hpatchz.exe executable",
		default=DEFAULT_HPATCHZ_PATH,
		subtype='FILE_PATH'
	)
	
	# Patch settings
	auto_save_before_patch: BoolProperty(
		name="Auto-save before creating patch",
		description="Automatically save the blend file before creating a patch",
		default=True
	)
	
	default_compression_level: IntProperty(
		name="Default Compression Level",
		description="Default compression level for hdiffz (higher = smaller patches, slower)",
		default=16,
		min=1,
		max=64
	)
	
	patch_timeout: IntProperty(
		name="Patch Creation Timeout (seconds)",
		description="Maximum time to wait for patch creation",
		default=300,
		min=10,
		max=3600
	)
	
	# Preview settings (for future thumbnail support)
	enable_preview_thumbnails: BoolProperty(
		name="Enable Preview Thumbnails",
		description="Generate preview thumbnails for patch versions (future feature)",
		default=False
	)
	
	thumbnail_size: IntProperty(
		name="Thumbnail Size",
		description="Size of preview thumbnails in pixels",
		default=256,
		min=64,
		max=1024
	)

	def draw(self, context):
		layout = self.layout
		
		# Tool Paths Section
		box = layout.box()
		box.label(text="Tool Paths:", icon='TOOL_SETTINGS')
		
		row = box.row()
		row.prop(self, "hdiffz_path")
		
		row = box.row()
		row.prop(self, "hpatchz_path")
		
		# Validation indicators
		import os
		if not os.path.exists(self.hdiffz_path):
			box.label(text="⚠ hdiffz.exe not found!", icon='ERROR')
		else:
			box.label(text="✓ hdiffz.exe found", icon='CHECKMARK')
			
		if not os.path.exists(self.hpatchz_path):
			box.label(text="⚠ hpatchz.exe not found!", icon='ERROR')
		else:
			box.label(text="✓ hpatchz.exe found", icon='CHECKMARK')
		
		# Patch Settings Section
		box = layout.box()
		box.label(text="Patch Settings:", icon='SETTINGS')
		
		box.prop(self, "auto_save_before_patch")
		box.prop(self, "default_compression_level")
		box.prop(self, "patch_timeout")
		
		# Preview Settings Section (Future)
		box = layout.box()
		box.label(text="Preview Settings (Future Feature):", icon='IMAGE_DATA')
		
		box.prop(self, "enable_preview_thumbnails")
		if self.enable_preview_thumbnails:
			box.prop(self, "thumbnail_size")
			box.label(text="Note: Thumbnail support is planned for future releases", icon='INFO')

def get_preferences():
	"""Get the addon preferences"""
	if not ADDON_PACKAGE_NAME:
		print("HDiff Tool: ADDON_PACKAGE_NAME not set. Cannot get preferences.")
		return None
	
	try:
		addon = bpy.context.preferences.addons.get(ADDON_PACKAGE_NAME)
		if addon and hasattr(addon, 'preferences'):
			return addon.preferences
		else:
			print(f"HDiff Tool: Could not retrieve preferences for {ADDON_PACKAGE_NAME}. Addon might not be enabled yet.")
			return None
	except Exception as e:
		print(f"HDiff Tool: Error accessing preferences: {e}")
		return None

# --- Registration ---
classes = (
	HDIFF_AddonPreferences,
)

def register():
	"""Register preferences"""
	for cls in classes:
		bpy.utils.register_class(cls)
	print("HDiff Tool: Preferences module registered")

def unregister():
	"""Unregister preferences"""
	for cls in reversed(classes):
		bpy.utils.unregister_class(cls)
	print("HDiff Tool: Preferences module unregistered")
