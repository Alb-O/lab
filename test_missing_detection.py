"""
Test script to understand how Blender reports missing linked assets.
This script explores various ways to detect missing linked data in Blender.
"""

import bpy

def test_missing_detection():
    """Test various ways to detect missing linked assets."""
    
    print("=== Testing Missing Asset Detection ===")
    
    # Check libraries
    print(f"\nLibraries in session: {len(bpy.data.libraries)}")
    for lib in bpy.data.libraries:
        print(f"  Library: {lib.name}")
        print(f"    Filepath: {lib.filepath}")
        if hasattr(lib, 'is_missing'):
            print(f"    Is missing: {lib.is_missing}")
        
        # Check if file exists
        abs_path = bpy.path.abspath(lib.filepath)
        import os
        exists = os.path.exists(abs_path)
        print(f"    File exists: {exists}")
        print(f"    Absolute path: {abs_path}")
    
    # Check collections with library references
    print(f"\nCollections in session: {len(bpy.data.collections)}")
    for col in bpy.data.collections:
        if col.library:
            print(f"  Collection: {col.name}")
            print(f"    Library: {col.library.name}")
            print(f"    Library filepath: {col.library.filepath}")
            if hasattr(col.library, 'is_missing'):
                print(f"    Library is missing: {col.library.is_missing}")
        
        # Check library_weak_reference
        if hasattr(col, 'library_weak_reference') and col.library_weak_reference:
            print(f"    Has weak reference: {col.library_weak_reference.filepath}")
    
    # Check objects with library references
    print(f"\nObjects in session: {len(bpy.data.objects)}")
    for obj in bpy.data.objects:
        if obj.library:
            print(f"  Object: {obj.name}")
            print(f"    Library: {obj.library.name}")
            if hasattr(obj.library, 'is_missing'):
                print(f"    Library is missing: {obj.library.is_missing}")
    
    # Check meshes with library references
    print(f"\nMeshes in session: {len(bpy.data.meshes)}")
    for mesh in bpy.data.meshes:
        if mesh.library:
            print(f"  Mesh: {mesh.name}")
            print(f"    Library: {mesh.library.name}")
            if hasattr(mesh.library, 'is_missing'):
                print(f"    Library is missing: {mesh.library.is_missing}")

if __name__ == "__main__":
    test_missing_detection()
