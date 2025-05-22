import uuid

# Log color codes (ANSI escape sequences)
LOG_COLORS = {
    'INFO': '\033[94m',    # Blue: Informational messages
    'SUCCESS': '\033[92m', # Green: Success/confirmation
    'WARN': '\033[93m',    # Yellow: Warnings
    'ERROR': '\033[91m',   # Red: Errors
    'RESET': '\033[0m',    # Reset to default
}

# Markdown link formats and regex patterns
MD_LINK_FORMATS = {
    'MD_ANGLE_BRACKETS': {
        'format': '#### [{name}](<{path}>)',
        'regex': r'^#### \[([^\]]+)\]\(<([^>]+)>\)$'
    }
}

# Sidecar file extension
SIDECAR_EXTENSION = ".side.md"

# Default frontmatter tags
FRONTMATTER_TAGS = {"sidecar", "blendvault"}

# Default poll interval (seconds) for checking sidecar file changes
POLL_INTERVAL = 5.0

# --- Blend Vault UUID/Hash Key Constants ---
BLEND_VAULT_HASH_PROP = "blend_vault_hash"
BLEND_VAULT_FILE_UUID_KEY = "blendfile_uuid"
BLEND_VAULT_UUID_KEY = "uuid"

def ensure_library_hash(lib):
    """Ensure a unique hash is stored in the datablock's custom properties, or generate a hash for a string path."""
    # If lib is a Blender datablock with id_properties_ensure
    if hasattr(lib, 'id_properties_ensure'):
        props = lib.id_properties_ensure()
        if BLEND_VAULT_HASH_PROP in props:
            print(f"[Blend Vault][LibraryHash] Existing hash for '{getattr(lib, 'name', repr(lib))}': {props[BLEND_VAULT_HASH_PROP]}")
            return props[BLEND_VAULT_HASH_PROP]
        # Generate a new UUID4 string
        new_hash = str(uuid.uuid4())
        props[BLEND_VAULT_HASH_PROP] = new_hash
        print(f"[Blend Vault][LibraryHash] Generated new hash for '{getattr(lib, 'name', repr(lib))}': {new_hash}")
        return new_hash
    # If lib is a string (e.g., file path), return a deterministic hash or UUID
    if isinstance(lib, str):
        # Optionally, use a deterministic hash for file paths
        import hashlib
        hash_str = hashlib.sha256(lib.encode('utf-8')).hexdigest()
        return hash_str
    # Fallback: just return a new UUID
    print(f"[Blend Vault][LibraryHash] Input is not a datablock or string, returning random UUID.")
    return str(uuid.uuid4())
