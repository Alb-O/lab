# URI panel components for Obsidian integration

import bpy
import os
from .. import SIDECAR_EXTENSION
from ..preferences import get_obsidian_vault_root, detect_obsidian_vault_from_asset_libraries


def draw_vault_status_section(layout, context):
	"""
	Draw the vault status section of the panel.
	
	Args:
		layout: The layout to draw into
		context: The Blender context
	"""
	# Vault Status Section
	vault_box = layout.box()
	header_row = vault_box.row()
	
	vault_root = get_obsidian_vault_root(context)
	detected_vault = detect_obsidian_vault_from_asset_libraries()
	
	if detected_vault:
		if vault_root == detected_vault:
			# Auto-detected vault is being used
			header_row.label(text="Vault Status: Auto-detected", icon='CHECKMARK')
			vault_box.label(text=f"Path: {detected_vault}")
		else:
			# Manual override is being used
			header_row.label(text="Vault Status: Manual override", icon='SETTINGS')
			vault_box.label(text=f"Path: {vault_root}")
	elif vault_root:
		# Only manual vault set
		header_row.label(text="Vault Status: Manual", icon='SETTINGS')
		vault_box.label(text=f"Path: {vault_root}")
	else:
		# No vault configured
		header_row.label(text="Vault Status: Not configured", icon='ERROR')
		vault_box.label(text="Set vault in preferences or add as asset library")
	
	header_row.operator("blend_vault.refresh_vault_detection", text="", icon='FILE_REFRESH')

	# Grouped buttons in a single column for a unified look
	col = vault_box.column(align=True)
	col.scale_y = 1.1

	if bpy.data.is_saved:
		blend_path = bpy.data.filepath
		sidecar_path = blend_path + SIDECAR_EXTENSION
		if os.path.exists(sidecar_path):
			col.operator("blend_vault.open_sidecar_in_obsidian", text="Open Sidecar File", icon='CURRENT_FILE')
		else:
			col.enabled = False
			col.label(text="No sidecar file for current .blend")
	else:
		col.enabled = False
		col.label(text="Save .blend to manage sidecar")

	# Open Vault in Explorer button (directly in the same column, always shown, but may be disabled)
	if vault_root and os.path.isdir(vault_root):
		col.operator("wm.path_open", text="Open Vault in Explorer", icon='FILEBROWSER').filepath = vault_root
	else:
		col_sub = col.column()
		col_sub.enabled = False
		if not vault_root:
			col_sub.label(text="Vault path not configured")
		elif not os.path.exists(vault_root):
			col_sub.label(text="Vault folder does not exist")
		else:
			col_sub.label(text="Vault path is not a folder")
