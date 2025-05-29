# fly_nav/__init__.py

# Try to import bpy, but don't fail if it's not available (e.g. for external tools)
try:
	import bpy # type: ignore
	_BLENDER_AVAILABLE = True
except ImportError:
	bpy = None
	_BLENDER_AVAILABLE = False

# Import submodules
from . import operators
from . import preferences
from . import logger # Import the new logger module

# Re-export for easier access from the main __init__.py
log_info = logger.log_info
log_warning = logger.log_warning
log_error = logger.log_error
log_debug = logger.log_debug
set_log_level = logger.set_log_level

__all__ = [
	'operators',
	'preferences',
	'logger', # export the module itself if needed
	'log_info', 'log_warning', 'log_error', 'log_debug', 'set_log_level'
]

if _BLENDER_AVAILABLE:
	# You can add Blender-specific initializations here if needed
	pass

def register():
	"""
	Registers the submodules.
	This function might be called by the root __init__.py
	"""
	# Ensure logger has package name if called from here, though root __init__ should handle it
	if __package__ and not logger.ADDON_PACKAGE_NAME:
		logger.set_package_name(__package__)
		
	if hasattr(operators, 'register'):
		operators.register()
	# Preferences are registered by the root __init__.py directly with bpy.utils.register_class
	# No separate register() function is typically called on the preferences module itself.
	logger.log_info(f"fly_nav package modules registered (called from fly_nav.__init__)")

def unregister():
	"""
	Unregisters the submodules.
	This function might be called by the root __init__.py
	"""
	if hasattr(operators, 'unregister'):
		operators.unregister()
	# Preferences are unregistered by the root __init__.py directly with bpy.utils.unregister_class
	logger.log_info(f"fly_nav package modules unregistered (called from fly_nav.__init__)")

logger.log_info(f"fly_nav package loaded, attempting to set logger package name to: {__package__}")
if __package__:
	logger.set_package_name(__package__) # Set package name for logger
