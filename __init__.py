bl_info = {
	"name": "Blend Vault",
	"author": "Albert O'Shea",
	"version": (0, 2, 2),
	"blender": (4, 0, 0),
	"location": "File Save",
	"description": "Writes linked library info (path and session UID) to a markdown file on save",
	"category": "Development",
}

import sys
import os
# Ensure the addon root directory is in sys.path for package imports
addon_dir = os.path.dirname(os.path.abspath(__file__))
if addon_dir not in sys.path:
	sys.path.append(addon_dir)

import bpy  # type: ignore
import importlib

# Dynamically import utils and LOG_COLORS for consistent access
utils = importlib.import_module('utils')
LOG_COLORS = utils.LOG_COLORS

# Registry of app handlers: event name -> list of (module path, function name)
HANDLERS = {
	'save_post': [
		('sidecar_io.writer', 'write_library_info'),
	],
	'load_post': [
		('relink.library_relinker', 'relink_library_info'),
		('relink.polling', 'start_sidecar_poll_timer'),
		('relink.asset_relinker', 'relink_renamed_assets'),
	],
}

def register():
	# Reload submodules first (important for dependencies)
	submodules_to_reload = [
		'sidecar_io.frontmatter',  # Reload frontmatter before writer
	]
	
	for module_path in submodules_to_reload:
		try:
			importlib.reload(importlib.import_module(module_path))
		except ImportError:
			pass  # Module might not be imported yet
	
	# Reload and register handlers from HANDLERS registry
	for event, entries in HANDLERS.items():
		handler_list = getattr(bpy.app.handlers, event)
		for module_path, fn_name in entries:
			module = importlib.reload(importlib.import_module(module_path))
			fn = getattr(module, fn_name)
			globals()[fn_name] = fn
			if fn not in handler_list:
				handler_list.append(fn)
				
	print(f"{LOG_COLORS['SUCCESS']}[Blend Vault] Main addon functionalities registered.{LOG_COLORS['RESET']}")

def unregister():
	# Unregister handlers based on HANDLERS registry
	for event, entries in HANDLERS.items():
		handler_list = getattr(bpy.app.handlers, event)
		for module_path, fn_name in entries:
			fn = globals().get(fn_name) or getattr(importlib.import_module(module_path), fn_name)
			if fn in handler_list:
				handler_list.remove(fn)
	# Also unregister any timers separately if used
	try:
		_, _, poll_entry = HANDLERS['load_post'][1]
		timer_fn = globals().get(poll_entry)
		if timer_fn and bpy.app.timers.is_registered(timer_fn):
			bpy.app.timers.unregister(timer_fn)
	except Exception:
		pass
	print(f"{LOG_COLORS['WARN']}[Blend Vault] Main addon functionalities unregistered.{LOG_COLORS['RESET']}")

if __name__ == "__main__":
	register()

print(f"{LOG_COLORS['SUCCESS']}[Blend Vault] Script loaded.{LOG_COLORS['RESET']}")