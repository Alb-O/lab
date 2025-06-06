#!/usr/bin/env python3
"""
Test script to verify vault-root-relative path handling logic.
"""

import os

# Simulate SIDECAR_EXTENSION
SIDECAR_EXTENSION = ".side.md"

def test_path_handling():
    """Test the path handling logic for library links."""
    
    test_cases = [
        # Test case: (input_path, expected_blend_path, expected_sidecar_path)
        ("Media/Library.blend.side.md", "Media/Library.blend", "Media/Library.blend.side.md"),
        ("Media/Library.blend", "Media/Library.blend", "Media/Library.blend.side.md"),
        ("Assets/Material.blend.side.md", "Assets/Material.blend", "Assets/Material.blend.side.md"),
        ("Textures/fabric.blend", "Textures/fabric.blend", "Textures/fabric.blend.side.md"),
    ]
    
    print("Testing vault-root-relative path handling logic:")
    print("=" * 60)
    
    for lib_vault_rel, expected_blend, expected_sidecar in test_cases:
        # Handle link path: if it ends with .side.md, extract the actual blend path
        if lib_vault_rel.endswith(SIDECAR_EXTENSION):
            # This is a link to the sidecar file, extract the blend path
            lib_blend_vault_rel = lib_vault_rel[:-len(SIDECAR_EXTENSION)]
            lib_sidecar_vault_rel = lib_vault_rel
        else:
            # This is a direct blend file path (legacy format)
            lib_blend_vault_rel = lib_vault_rel
            lib_sidecar_vault_rel = lib_vault_rel + SIDECAR_EXTENSION
        
        print(f"Input: {lib_vault_rel}")
        print(f"  Blend path: {lib_blend_vault_rel} (expected: {expected_blend})")
        print(f"  Sidecar path: {lib_sidecar_vault_rel} (expected: {expected_sidecar})")
        
        # Verify results
        blend_ok = lib_blend_vault_rel == expected_blend
        sidecar_ok = lib_sidecar_vault_rel == expected_sidecar
        status = "✓ PASS" if (blend_ok and sidecar_ok) else "✗ FAIL"
        print(f"  Result: {status}")
        
        if not blend_ok:
            print(f"    Blend path mismatch: got '{lib_blend_vault_rel}', expected '{expected_blend}'")
        if not sidecar_ok:
            print(f"    Sidecar path mismatch: got '{lib_sidecar_vault_rel}', expected '{expected_sidecar}'")
        
        print()

if __name__ == "__main__":
    test_path_handling()
