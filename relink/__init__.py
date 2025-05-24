from . import library_relinker
from . import polling
from . import asset_relinker
import sys
import os

# Import utils from parent directory
sys.path.append(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import utils

def register():
	library_relinker.register()
	polling.register()
	asset_relinker.register()
	
	# The asset relinking handler is registered in the main __init__.py
	print(f"{utils.LOG_COLORS['SUCCESS']}[Blend Vault] Relink module registered.{utils.LOG_COLORS['RESET']}")

def unregister():
	# Unregister in reverse order
	import bpy # type: ignore
	if asset_relinker.relink_renamed_assets in bpy.app.handlers.load_post:
		bpy.app.handlers.load_post.remove(asset_relinker.relink_renamed_assets)
	
	asset_relinker.unregister()
	polling.unregister()
	library_relinker.unregister()
	print(f"{utils.LOG_COLORS['WARN']}[Blend Vault] Relink module unregistered.{utils.LOG_COLORS['RESET']}")