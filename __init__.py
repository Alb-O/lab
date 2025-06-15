"""
HDiff Tool Extension for Blender
Provides differential patching capabilities for .blend files using hdiffz/hpatchz
"""

import os
import sys
import platform
import zipfile
from pathlib import Path
import bpy
import importlib

# TODO: Wheel dependency setup for bundling hdiffz binaries
# This will be investigated later for bundling hdiffz with the extension
def setup_wheel_dependencies():
	"""
	Future implementation for extracting and loading wheel dependencies
	Currently commented out as hdiffz bundling is under investigation
	"""
	# extension_dir = Path(__file__).parent
	# wheels_dir = extension_dir / "wheels" 
	# extracted_dir = extension_dir / "extracted_wheels"
	# 
	# if not wheels_dir.exists():
	# 	print("HDiff Tool: No wheels directory found")
	# 	return
	#
	# # Platform-specific wheel extraction logic would go here
	# # This includes hdiffz.exe, hpatchz.exe binaries
	pass

# Setup wheel dependencies (currently disabled)
# setup_wheel_dependencies()

# Import core modules
from .hdiff_tool import (
	operators,
	panels,
	preferences,
	core,
	utils,
	patch_creation,
	patch_application
)

# Core modules to register (preferences handled separately)
CORE_MODULES = [
	'hdiff_tool.core',
	'hdiff_tool.utils',
	'hdiff_tool.patch_creation',
	'hdiff_tool.patch_application',
	'hdiff_tool.operators',
	'hdiff_tool.panels',  # Must be last as it depends on operators and preferences
]

def register():
	"""Register the HDiff Tool extension."""
	package_name = __package__
	
	if not package_name:
		print("HDiff Tool: Package name not available")
		return

	print("HDiff Tool: Starting registration...")

	# Register preferences first
	try:
		preferences.ADDON_PACKAGE_NAME = package_name
		preferences.HDIFF_AddonPreferences.bl_idname = package_name
		bpy.utils.register_class(preferences.HDIFF_AddonPreferences)
		print("HDiff Tool: Preferences registered")
	except Exception as e:
		print(f"HDiff Tool: Failed to register preferences: {e}")

	# Register other core modules in order
	for module_path in CORE_MODULES:
		try:
			full_module_path = f"{package_name}.{module_path}"
			module = importlib.import_module(full_module_path)
			if hasattr(module, 'register'):
				module.register()
				print(f"HDiff Tool: Registered {module_path}")
			else:
				print(f"HDiff Tool: Module {module_path} has no register function")
		except Exception as e:
			print(f"HDiff Tool: Failed to register {module_path}: {e}")

	print("HDiff Tool: Extension registered successfully")


def unregister():
	"""Unregister the HDiff Tool extension."""
	print("HDiff Tool: Starting unregistration...")
	
	# Unregister core modules in reverse order
	package_name = __package__ or "hdiff_tool_ext"
	for module_path in reversed(CORE_MODULES):
		try:
			full_module_path = f"{package_name}.{module_path}"
			module = importlib.import_module(full_module_path)
			if hasattr(module, 'unregister'):
				module.unregister()
				print(f"HDiff Tool: Unregistered {module_path}")
		except Exception as e:
			print(f"HDiff Tool: Failed to unregister {module_path}: {e}")

	# Unregister preferences last
	try:
		bpy.utils.unregister_class(preferences.HDIFF_AddonPreferences)
		print("HDiff Tool: Preferences unregistered")
	except Exception as e:
		print(f"HDiff Tool: Failed to unregister preferences: {e}")

	print("HDiff Tool: Extension unregistered successfully")


if __name__ == "__main__":
	register()