import os
import sys
import platform
import zipfile
from pathlib import Path
import bpy
import importlib

# Extract and load wheel dependencies
def setup_wheel_dependencies():
	"""Extract and load wheel dependencies for Pillow (PIL)"""
	extension_dir = Path(__file__).parent
	wheels_dir = extension_dir / "wheels"
	extracted_dir = extension_dir / "extracted_wheels"
	
	if not wheels_dir.exists():
		print("Blend Vault: No wheels directory found")
		return
	
	# Create extraction directory if it doesn't exist
	extracted_dir.mkdir(exist_ok=True)
	
	# Determine platform-specific wheel patterns
	system = platform.system()
	if system == "Windows":
		wheel_patterns = ["win_amd64", "win32"]
	elif system == "Darwin":
		wheel_patterns = ["macosx"]
	else:  # Linux
		wheel_patterns = ["linux", "manylinux"]
	
	# Extract wheels
	extracted_paths = []
	wheel_files = list(wheels_dir.glob("*.whl"))
	
	# Process Pillow wheels
	pillow_wheels = [w for w in wheel_files if "pillow" in w.name.lower()]
	
	for wheel_file in pillow_wheels:
		# Check if this wheel is for our platform
		wheel_matches = any(pattern in wheel_file.name for pattern in wheel_patterns) or "py3-none-any" in wheel_file.name
		
		if wheel_matches:
			wheel_extract_dir = extracted_dir / wheel_file.stem
			
			if not wheel_extract_dir.exists():
				try:
					with zipfile.ZipFile(wheel_file, 'r') as zip_ref:
						zip_ref.extractall(wheel_extract_dir)
					print(f"Blend Vault: Extracted {wheel_file.name}")
				except Exception as e:
					print(f"Blend Vault: Error extracting {wheel_file.name}: {e}")
					continue
			
			extracted_paths.append(str(wheel_extract_dir))
	
	# Add extracted paths to sys.path
	for path in extracted_paths:
		if path not in sys.path:
			sys.path.insert(0, path)
			print(f"Blend Vault: Added extracted wheel to sys.path: {path}")

# Setup wheel dependencies before importing anything else
setup_wheel_dependencies()

# Import core functionality - use module-level imports
from .blend_vault import preferences as prefs_module
from .blend_vault.core import log_info, log_warning, log_error, log_success, log_debug

# Core modules to register (simplified - no complex handler management)
CORE_MODULES = [
	'blend_vault.relink',
	'blend_vault.obsidian_integration', 
	'blend_vault.paste_path',
	'blend_vault.sidecar_io',
]

def register():
	"""Register the addon with simplified logic."""
	package_name = __package__
	
	if not package_name:
		log_error("Package name not available", module_name="Init")
		return

	# Register preferences
	try:
		if prefs_module:
			prefs_module.ADDON_PACKAGE_NAME = package_name
			if hasattr(prefs_module, 'BlendVaultPreferences'):
				prefs_module.BlendVaultPreferences.bl_idname = package_name
				bpy.utils.register_class(prefs_module.BlendVaultPreferences)
				if hasattr(prefs_module, 'restore_preferences'):
					prefs_module.restore_preferences()
				log_success("Preferences registered", module_name="Init")
			else:
				log_error("BlendVaultPreferences class not found", module_name="Init")
		else:
			log_warning("Preferences module not available", module_name="Init")
	except Exception as e:
		log_error(f"Failed to register preferences: {e}", module_name="Init")

	# Register core modules
	for module_path in CORE_MODULES:
		try:
			full_module_path = f"{package_name}.{module_path}"
			module = importlib.import_module(full_module_path)
			if hasattr(module, 'register'):
				module.register()
				log_success(f"Registered {module_path}", module_name="Init")
			else:
				log_warning(f"Module {module_path} has no register function", module_name="Init")
		except Exception as e:
			log_error(f"Failed to register {module_path}: {e}", module_name="Init")

	log_success("Blend Vault extension registered", module_name="Init")


def unregister():
	"""Unregister the addon."""
	# Store preferences before unregistering
	try:
		if prefs_module:
			if hasattr(prefs_module, 'store_preferences'):
				prefs_module.store_preferences()
			if hasattr(prefs_module, 'BlendVaultPreferences'):
				bpy.utils.unregister_class(prefs_module.BlendVaultPreferences)
			log_success("Preferences unregistered", module_name="Init")
		else:
			log_warning("Preferences module not available for unregistration", module_name="Init")
	except Exception as e:
		log_error(f"Failed to unregister preferences: {e}", module_name="Init")

	# Unregister core modules (reverse order)
	package_name = __package__ or "blend_vault_ext"
	for module_path in reversed(CORE_MODULES):
		try:
			full_module_path = f"{package_name}.{module_path}"
			module = importlib.import_module(full_module_path)
			if hasattr(module, 'unregister'):
				module.unregister()
				log_success(f"Unregistered {module_path}", module_name="Init")
		except Exception as e:
			log_error(f"Failed to unregister {module_path}: {e}", module_name="Init")

	log_success("Blend Vault extension unregistered", module_name="Init")

if __name__ == "__main__":
	register()