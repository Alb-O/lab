import bpy  # type: ignore
import os
import importlib
from .. import SIDECAR_EXTENSION, POLL_INTERVAL, LOG_COLORS, log_success, log_error, log_warning, log_info
from . import asset_relinker, redirect_handler
from .library_relinker import relink_library_info
from .resource_relinker import relink_resources

# Store last modification times for sidecar files
t_last_sidecar_mtimes = {}
# Store last modification times for library files themselves
t_last_library_mtimes = {}

def sidecar_poll_timer():
	"""Timer callback to poll sidecar file changes and trigger relink if modified,
	and also polls library files for modifications."""
	blend_path = bpy.data.filepath
	if not blend_path: # Current .blend file is not saved or no file is open
		return POLL_INTERVAL

	# --- Part 1: Check sidecar file for modifications (triggers full relink if changed) ---
	md_path = blend_path + SIDECAR_EXTENSION
	try:
		if os.path.exists(md_path):
			sidecar_mtime = os.path.getmtime(md_path)
			last_known_sidecar_mtime = t_last_sidecar_mtimes.get(md_path)
			if last_known_sidecar_mtime is None:                # Initialize
				t_last_sidecar_mtimes[md_path] = sidecar_mtime
			elif sidecar_mtime > last_known_sidecar_mtime:
				# Sidecar file changed: update timestamp and trigger full relink logic
				t_last_sidecar_mtimes[md_path] = sidecar_mtime
				log_success(f"Sidecar file '{md_path}' modified. Triggering relinking sequence.", module_name='Polling')
				# Run asset relinking BEFORE library relinking to avoid session invalidation
				log_success("Running asset datablock relinking first (before library reloads).", module_name='Polling')
				asset_relinker.relink_renamed_assets()
				log_success("Running library path relinking second.", module_name='Polling')
				relink_library_info()
				log_success("Running resource relinking third.", module_name='Polling')
				relink_resources()
				# Sync library file mtimes to prevent polling-triggered reload wiping out relink
				try:
					for lib in bpy.data.libraries:
						if lib.filepath and not lib.filepath.startswith('<builtin>'):
							lib_abs_path = bpy.path.abspath(lib.filepath)
							if os.path.exists(lib_abs_path):
								t_last_library_mtimes[lib_abs_path] = os.path.getmtime(lib_abs_path)
				except Exception:
					pass
	except Exception as e:
		log_error(f"Error checking sidecar file '{md_path}': {e}", module_name='Polling')
	# --- Part 2: Check individual library files for modifications ---
	for lib in bpy.data.libraries:
		if not lib.filepath or lib.filepath.startswith("<builtin>"):
			continue # Skip libraries with no path or built-in ones

		try:
			lib_abs_path = bpy.path.abspath(lib.filepath)
			if not os.path.exists(lib_abs_path):
				if lib_abs_path in t_last_library_mtimes:
					del t_last_library_mtimes[lib_abs_path]
				continue

			current_lib_mtime = os.path.getmtime(lib_abs_path)
			last_known_lib_mtime = t_last_library_mtimes.get(lib_abs_path)

			if last_known_lib_mtime is None:
				t_last_library_mtimes[lib_abs_path] = current_lib_mtime
			elif current_lib_mtime > last_known_lib_mtime:
				t_last_library_mtimes[lib_abs_path] = current_lib_mtime
				log_warning(f"Library file '{lib.name}' ('{lib_abs_path}') modified. Triggering coordinated relinking sequence.", module_name='Polling')
				try:
					log_success("Running asset datablock relinking first (before library reload).", module_name='Polling')
					asset_relinker.relink_renamed_assets()
					log_success("Running library reload second.", module_name='Polling')
					lib.reload()
					log_success("Running resource relinking third.", module_name='Polling')
					relink_resources()
					log_success(f"Successfully completed coordinated relinking for library '{lib.name}'.", module_name='Polling')
				except Exception as reload_e:
					log_error(f"Error during coordinated relinking for library '{lib.name}': {reload_e}", module_name='Polling')
		except Exception as e:
			log_error(f"Error checking library '{lib.name}' ('{lib.filepath}'): {e}", module_name='Polling')

	# --- Part 3: Check for file relocation via redirect files ---
	try:
		redirect_handler.check_file_relocation()
	except Exception as e:
		log_error(f"Error checking file relocation: {e}", module_name='Polling')

	return POLL_INTERVAL

@bpy.app.handlers.persistent
def start_sidecar_poll_timer(*args, **kwargs):
	"""Handler to register polling timer after file load, ensuring persistence across blend reloads."""
	if bpy.app.timers.is_registered(sidecar_poll_timer):
		log_info("Sidecar polling timer already registered.", module_name='Polling')
		return

	try:
		bpy.app.timers.register(sidecar_poll_timer, first_interval=POLL_INTERVAL)
		log_success(f"Sidecar polling timer registered (interval: {POLL_INTERVAL}s).", module_name='Polling')
	except Exception as e: 
		log_error(f"Failed to register sidecar polling timer: {e}", module_name='Polling')

def register():
	bpy.app.handlers.load_post.append(start_sidecar_poll_timer)
	# Also run library and asset relinkers on file load
	bpy.app.handlers.load_post.append(relink_library_info)
	bpy.app.handlers.load_post.append(asset_relinker.relink_renamed_assets)
	bpy.app.handlers.load_post.append(relink_resources)
	# Reload and register redirect handler to ensure we get the latest version
	importlib.reload(redirect_handler)
	redirect_handler.register()
	
	# Always try to start the polling timer during registration
	# The timer function itself will handle cases where no file is open
	log_info("Starting polling timer during registration.", module_name='Polling')
	start_sidecar_poll_timer()
	
	log_success("Polling module registered.", module_name='Polling')

def unregister():
	if start_sidecar_poll_timer in bpy.app.handlers.load_post:
		bpy.app.handlers.load_post.remove(start_sidecar_poll_timer)
	# Remove library and asset relinker handlers
	if relink_library_info in bpy.app.handlers.load_post:
		bpy.app.handlers.load_post.remove(relink_library_info)
	if asset_relinker.relink_renamed_assets in bpy.app.handlers.load_post:
		bpy.app.handlers.load_post.remove(asset_relinker.relink_renamed_assets)
	if relink_resources in bpy.app.handlers.load_post:
		bpy.app.handlers.load_post.remove(relink_resources)
	# Unregister redirect handler
	redirect_handler.unregister()
	if bpy.app.timers.is_registered(sidecar_poll_timer):
		bpy.app.timers.unregister(sidecar_poll_timer)
		log_warning("Sidecar polling timer unregistered.", module_name='Polling')
	log_warning("Polling module unregistered.", module_name='Polling')
