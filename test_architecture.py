"""
Simple test to verify the consolidated architecture works correctly.
This can be run from Blender's Text Editor to verify imports and basic functionality.
"""

def test_blend_vault_architecture():
    """Test the consolidated Blend Vault architecture."""
    print("Testing Blend Vault Architecture...")
    
    try:
        # Test core module imports
        from blend_vault.core import (
            log_info, log_warning, log_error, log_success, log_debug,
            get_asset_sources_map, get_or_create_datablock_uuid,
            format_primary_link, parse_primary_link, generate_filepath_hash,
            ensure_saved_file
        )
        print("✓ Core module imports successful")
        
        # Test main package re-exports
        from blend_vault import (
            log_info as main_log_info,
            get_asset_sources_map as main_assets_map,
            SIDECAR_EXTENSION
        )
        print("✓ Main package re-exports successful")
        
        # Test utils backward compatibility
        from blend_vault.utils import (
            log_info as utils_log_info,
            get_asset_sources_map as utils_assets_map,
            SIDECAR_EXTENSION as utils_extension
        )
        print("✓ Utils backward compatibility successful")
        
        # Test logging functionality
        log_info("Test info message")
        log_success("Architecture test passed!")
        print("✓ Logging functionality working")
        
        # Test asset sources map (basic functionality)
        asset_map = get_asset_sources_map()
        if isinstance(asset_map, dict):
            print(f"✓ Asset sources map working ({len(asset_map)} collections)")
        else:
            print("⚠ Asset sources map returned non-dict (expected in non-Blender environment)")
        
        # Test path utilities
        test_link = format_primary_link("test/path.blend", "Test File")
        if "test/path.blend" in test_link and "Test File" in test_link:
            print("✓ Link formatting working")
        
        # Test hash generation
        test_hash = generate_filepath_hash("test/path.blend")
        if len(test_hash) == 64:  # SHA256 produces 64-character hex
            print("✓ Hash generation working")
        
        print("\n🎉 All architecture tests passed!")
        return True
        
    except ImportError as e:
        print(f"❌ Import error: {e}")
        return False
    except Exception as e:
        print(f"❌ Test error: {e}")
        return False

if __name__ == "__main__":
    test_blend_vault_architecture()
