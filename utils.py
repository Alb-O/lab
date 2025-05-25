import bpy  # type: ignore
import uuid
import hashlib
import os

def get_asset_sources_map():
	"""
	Initializes and returns the ASSET_SOURCES_MAP dictionary.
	This version does not cache the instance, ensuring fresh bpy.data references on each call.
	"""
	# Always create and return a new dictionary with current bpy.data references
	return {
		"Collection": bpy.data.collections,
		"Object": bpy.data.objects,
		"World": bpy.data.worlds,
		"Material": bpy.data.materials,
		"Brush": bpy.data.brushes,
		"Action": bpy.data.actions,
		"NodeTree": bpy.data.node_groups,
		"Scene": bpy.data.scenes,
	}

# Log color codes (ANSI escape sequences)
LOG_COLORS = {
	'INFO': '\033[94m',    # Blue: Informational messages
	'SUCCESS': '\033[92m', # Green: Success/confirmation
	'WARN': '\033[93m',    # Yellow: Warnings
	'ERROR': '\033[91m',   # Red: Errors
	'DEBUG': '\033[95m',   # Magenta: Debug messages (added)
	'RESET': '\033[0m',    # Reset to default
}

# Markdown link formats and regex patterns
MD_LINK_FORMATS = {
	'MD_ANGLE_BRACKETS': {
		'format': '[{name}](<{path}>)',
		'regex': r'\[([^\]]+)\]\(<([^>]+)>\)'
	}
}

MD_PRIMARY_FORMAT = MD_LINK_FORMATS['MD_ANGLE_BRACKETS']

# Obsidian-style embed wikilink format: ![[path|alias]] or ![[name]]
MD_EMBED_WIKILINK = {
	'format': '![[{name}]]',
	'regex': r'!\[\[([^\]|]+)(?:\|([^\]]+))?\]\]'
}

# Sidecar file extension
SIDECAR_EXTENSION = ".side.md"

# Redirect file extension
REDIRECT_EXTENSION = ".redirect.md"

# Default frontmatter tags
FRONTMATTER_TAGS = {"sidecar", "blendvault"}

# Default poll interval (seconds) for checking for file changes
POLL_INTERVAL = 1.0

# Warning prefix for resources outside the vault
RESOURCE_WARNING_PREFIX = "⚠️ "

# --- Blend Vault UUID/Hash Key Constants ---
BV_UUID_PROP = "BV_UUID"
BV_FILE_UUID_KEY = "blendfile_uuid"
BV_UUID_KEY = "uuid"

def ensure_library_hash(lib):
	"""Ensure a unique hash is stored in the datablock's custom properties, or generate a hash for a string path."""
	# If lib is a Blender datablock with id_properties_ensure
	if hasattr(lib, 'id_properties_ensure'):
		props = lib.id_properties_ensure()
		if BV_UUID_PROP in props:
			print(f"[Blend Vault][LibraryHash] Existing hash for '{getattr(lib, 'name', repr(lib))}': {props[BV_UUID_PROP]}")
			return props[BV_UUID_PROP]
		# Generate a new UUID4 string
		new_hash = str(uuid.uuid4())
		props[BV_UUID_PROP] = new_hash
		print(f"[Blend Vault][LibraryHash] Generated new hash for '{getattr(lib, 'name', repr(lib))}': {new_hash}")
		return new_hash
	# If lib is a string (e.g., file path), return a deterministic hash or UUID
	if isinstance(lib, str):
		hash_str = hashlib.sha256(lib.encode('utf-8')).hexdigest()
		return hash_str
	# Fallback: just return a new UUID
	print(f"[Blend Vault][LibraryHash] Input is not a datablock or string, returning random UUID.")
	return str(uuid.uuid4())

def get_resource_warning_prefix(resource_path: str, blend_file_path: str, vault_root: str = None) -> str:
	"""
	Generate warning prefix for resources outside the Obsidian vault.
	
	Args:
		resource_path: The relative path to the resource from the blend file
		blend_file_path: The absolute path to the blend file
		vault_root: The absolute path to the Obsidian vault root (optional)
	
	Returns:
		RESOURCE_WARNING_PREFIX if the resource is outside the vault, empty string otherwise
	"""
	if not vault_root:
		return ""
	
	try:
		# Get absolute path of the resource relative to the blend file
		blend_dir = os.path.dirname(blend_file_path)
		resource_abs_path = os.path.normpath(os.path.join(blend_dir, resource_path))
		vault_root_abs = os.path.normpath(vault_root)
		
		# Check if resource is outside the vault
		rel_path = os.path.relpath(resource_abs_path, vault_root_abs)
		
		# If relpath starts with "..", it's outside the vault
		if rel_path.startswith('..'):
			return RESOURCE_WARNING_PREFIX
			
	except ValueError:
		# ValueError occurs when paths are on different drives (Windows)
		return RESOURCE_WARNING_PREFIX
	
	return ""
