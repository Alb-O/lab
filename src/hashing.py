\
import bpy  # type: ignore
import uuid

def ensure_library_hash(lib):
    """Ensure a unique hash is stored in the library's custom properties."""
    # Use the top-level ID property group for the library datablock
    if not hasattr(lib, 'id_properties_ensure'):  # Defensive: Blender 2.80+
        print(f"[Blend Vault][LibraryHash] Library '{lib.name}' does not support id_properties_ensure.")
        return None
    props = lib.id_properties_ensure()
    if 'blend_vault_hash' in props:
        print(f"[Blend Vault][LibraryHash] Existing hash for '{lib.name}': {props['blend_vault_hash']}")
        return props['blend_vault_hash']
    # Generate a new UUID4 string
    new_hash = str(uuid.uuid4())
    props['blend_vault_hash'] = new_hash
    print(f"[Blend Vault][LibraryHash] Generated new hash for '{lib.name}': {new_hash}")
    return new_hash


def ensure_blendfile_hash():
    """Ensure a unique hash stored in a hidden text data-block in the blend file."""
    txt_name = "blend_vault_hash"
    # Check for existing text block
    if txt_name in bpy.data.texts:
        txt = bpy.data.texts[txt_name]
        content = txt.as_string().strip()
        print(f"[Blend Vault][BlendfileHash] Existing hash in text block '{txt_name}': {content}")
        return content
    # Create a new text block and write the hash
    new_hash = str(uuid.uuid4())
    txt = bpy.data.texts.new(txt_name)
    txt.clear()
    txt.write(new_hash)
    # Optionally hide this text in UI
    txt.use_fake_user = True
    print(f"[Blend Vault][BlendfileHash] Generated new hash in text block '{txt_name}': {new_hash}")
    return new_hash
