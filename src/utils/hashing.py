import bpy  # type: ignore
import uuid
import json

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
    """Ensure a unique hash stored in a hidden text data-block in the blend file, and store asset info as JSON. Update asset names if changed, keep UUIDs persistent."""
    txt_name = "blend_vault_hash"
    # Human-readable names for asset types
    asset_type_names = {
        'collections': 'Collection',
        'objects': 'Object',
        'worlds': 'World',
        'materials': 'Material',
        'brushes': 'Brush',
        'actions': 'Action',
        'node_groups': 'Node Group',
    }
    # Build a map of uuid -> (item, asset_type) for all datablocks with a blend_vault_uuid property
    uuid_to_item = {}
    for collection_name, asset_type in asset_type_names.items():
        collection = getattr(bpy.data, collection_name, [])
        for item in collection:
            if getattr(item, 'asset_data', None) and getattr(item, 'library', None) is None:
                try:
                    asset_uuid = item['blend_vault_uuid']
                    if asset_uuid:
                        uuid_to_item[asset_uuid] = (item, asset_type)
                except Exception:
                    pass
    # Try to load existing JSON if present
    existing_uuids = {}
    if txt_name in bpy.data.texts:
        txt = bpy.data.texts[txt_name]
        content = txt.as_string().strip()
        try:
            data = json.loads(content)
            # Build a map of uuid -> asset info
            if data and 'assets' in data:
                for asset in data['assets']:
                    existing_uuids[asset['uuid']] = asset
            print(f"[Blend Vault][BlendfileHash] Existing JSON in text block '{txt_name}': {data}")
        except Exception:
            print(f"[Blend Vault][BlendfileHash] Existing text block '{txt_name}' is not valid JSON.")
            data = None
        # Print assets in this blend file
        print("[Blend Vault][BlendfileHash] Assets in this blend:")
        if data and 'assets' in data:
            for asset in data['assets']:
                print(f"  - {asset['type']} '{asset['name']}', uuid: {asset['uuid']}")
        else:
            print("  No assets found.")
        if data and 'blendfile_uuid' in data:
            blendfile_uuid = data['blendfile_uuid']
        else:
            blendfile_uuid = str(uuid.uuid4())
    else:
        blendfile_uuid = str(uuid.uuid4())
    # Build new asset list, updating names if needed, keeping UUIDs persistent
    assets = []
    for asset_uuid, (item, asset_type) in uuid_to_item.items():
        assets.append({
            'name': item.name,
            'type': asset_type,
            'uuid': asset_uuid
        })
    # Add any new assets that don't have a uuid yet
    for collection_name, asset_type in asset_type_names.items():
        collection = getattr(bpy.data, collection_name, [])
        for item in collection:
            if getattr(item, 'asset_data', None) and getattr(item, 'library', None) is None:
                try:
                    asset_uuid = item['blend_vault_uuid']
                except Exception:
                    asset_uuid = None
                if not asset_uuid:
                    # Try to find by name/type in previous data (legacy)
                    for old_uuid, old_asset in existing_uuids.items():
                        if old_asset['name'] == item.name and old_asset['type'] == asset_type:
                            asset_uuid = old_uuid
                            break
                if not asset_uuid:
                    asset_uuid = str(uuid.uuid4())
                # Store on the asset if possible
                try:
                    item['blend_vault_uuid'] = asset_uuid
                except Exception:
                    pass
                # Only add if not already in assets
                if not any(a['uuid'] == asset_uuid for a in assets):
                    assets.append({
                        'name': item.name,
                        'type': asset_type,
                        'uuid': asset_uuid
                    })
    data = {
        'blendfile_uuid': blendfile_uuid,
        'assets': assets
    }
    # Overwrite or create the text block
    if txt_name in bpy.data.texts:
        txt = bpy.data.texts[txt_name]
        txt.clear()
    else:
        txt = bpy.data.texts.new(txt_name)
    txt.write(json.dumps(data, indent=2, ensure_ascii=False))
    txt.use_fake_user = True
    print(f"[Blend Vault][BlendfileHash] Generated new JSON in text block '{txt_name}': {data}")
    print("[Blend Vault][BlendfileHash] Assets in this blend:")
    for asset in assets:
        print(f"  - {asset['type']} '{asset['name']}', uuid: {asset['uuid']}" )
    return blendfile_uuid
