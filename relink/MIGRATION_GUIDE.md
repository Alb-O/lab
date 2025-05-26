# Migration Guide: Transitioning to Refactored Relinker Architecture

## Overview
This guide provides step-by-step instructions for safely migrating from the original relinker implementation to the refactored, consolidated architecture.

## Pre-Migration Checklist

### 1. Backup Current Setup
```powershell
# Create a backup of the current relink folder
Copy-Item "c:\Users\osheaa\Documents\_\blend-vault\relink" -Destination "c:\Users\osheaa\Documents\_\blend-vault\relink_backup" -Recurse
```

### 2. Verify Current Functionality
- Test asset relinking with existing .blend files
- Test library relinking functionality  
- Test resource relinking
- Document any custom behavior or edge cases

### 3. Check Dependencies
- Ensure all imports in the main addon still work
- Verify handler registration in main `__init__.py`
- Check any external code that imports relinker modules

## Migration Steps

### Step 1: Validate Refactored Modules (COMPLETED ✅)
The refactored modules have been created and validated:
- ✅ `shared_utils.py` - No syntax errors
- ✅ `asset_relinker_refactored.py` - No syntax errors  
- ✅ `library_relinker_refactored.py` - No syntax errors
- ✅ `resource_relinker_refactored.py` - No syntax errors

### Step 2: Test Refactored Modules in Isolation

#### Option A: Rename Files for Testing
```powershell
# Temporarily rename original files
Rename-Item "asset_relinker.py" "asset_relinker_original.py"
Rename-Item "library_relinker.py" "library_relinker_original.py"  
Rename-Item "resource_relinker.py" "resource_relinker_original.py"

# Rename refactored files to active names
Rename-Item "asset_relinker_refactored.py" "asset_relinker.py"
Rename-Item "library_relinker_refactored.py" "library_relinker.py"
Rename-Item "resource_relinker_refactored.py" "resource_relinker.py"
```

#### Option B: Update Import Statements
```python
# In __init__.py, change:
from . import asset_relinker
# To:
from . import asset_relinker_refactored as asset_relinker
```

### Step 3: Update Handler Registration

#### Check Main Addon `__init__.py`
Ensure the main addon properly registers relink handlers:

```python
# In main __init__.py, verify these patterns exist:
def register():
    # ... other registrations ...
    relink.register()
    
    # Add asset relink handler if not already present
    if relink.asset_relinker.relink_renamed_assets not in bpy.app.handlers.load_post:
        bpy.app.handlers.load_post.append(relink.asset_relinker.relink_renamed_assets)
```

### Step 4: Integration Testing

#### Test Asset Relinking
1. Open a .blend file with linked assets from libraries
2. Rename assets in the library files
3. Reopen the main file
4. Verify assets are automatically relinked

#### Test Library Relinking  
1. Move library .blend files to new locations
2. Update sidecar files with new paths
3. Reopen main files
4. Verify libraries are automatically relinked

#### Test Resource Relinking
1. Move texture/audio files to new locations
2. Update sidecar files with new resource paths
3. Reopen files
4. Verify resources are automatically relinked

### Step 5: Performance Validation

#### Compare Performance Metrics
- Time to process large sidecar files
- Memory usage during relinking operations
- Blender startup time with addon enabled

#### Expected Improvements
- Faster sidecar parsing (shared parser instances)
- Reduced memory footprint (focused classes)
- More consistent performance across different file types

## Rollback Plan

If issues are discovered during migration:

### Quick Rollback
```powershell
# Restore original files
Rename-Item "asset_relinker.py" "asset_relinker_refactored.py"
Rename-Item "library_relinker.py" "library_relinker_refactored.py"
Rename-Item "resource_relinker.py" "resource_relinker_refactored.py"

Rename-Item "asset_relinker_original.py" "asset_relinker.py"
Rename-Item "library_relinker_original.py" "library_relinker.py"
Rename-Item "resource_relinker_original.py" "resource_relinker.py"
```

### Full Restore
```powershell
# Restore from backup if needed
Remove-Item "c:\Users\osheaa\Documents\_\blend-vault\relink" -Recurse -Force
Copy-Item "c:\Users\osheaa\Documents\_\blend-vault\relink_backup" -Destination "c:\Users\osheaa\Documents\_\blend-vault\relink" -Recurse
```

## Post-Migration Cleanup

### After Successful Migration

#### Remove Legacy Files
```powershell
# Remove original files once refactored versions are stable
Remove-Item "asset_relinker_original.py"
Remove-Item "library_relinker_original.py"  
Remove-Item "resource_relinker_original.py"
```

#### Update Documentation
- Update any documentation references to old file names
- Update developer docs with new architecture
- Update API documentation if external code uses relinker modules

### Monitor for Issues
- Watch for any error messages in Blender console
- Monitor user reports of relinking failures
- Track performance improvements/regressions

## Benefits You Should See

### Immediate Benefits
- **Cleaner error messages** - Consistent logging format
- **More reliable relinking** - Better error handling
- **Faster startup** - Optimized imports and initialization

### Development Benefits  
- **Easier debugging** - Centralized logic is easier to trace
- **Faster feature development** - Reuse shared components
- **Better testing** - Shared utilities can be unit tested

### Long-term Benefits
- **Easier maintenance** - Single source of truth for common operations
- **Better extensibility** - Easy to add new relinker types
- **Consistent behavior** - All modules behave similarly

## Troubleshooting Common Issues

### Import Errors
**Problem**: `ModuleNotFoundError` for shared_utils
**Solution**: Ensure relative imports are correct and shared_utils.py is in the relink folder

### Handler Registration Issues  
**Problem**: Handlers not being called
**Solution**: Check that handler functions are marked `@bpy.app.handlers.persistent`

### Path Resolution Issues
**Problem**: Relative paths not resolving correctly
**Solution**: Verify `PathResolver` is being used consistently

### Performance Regression
**Problem**: Slower performance than original
**Solution**: Check for unnecessary repeated parsing; consider caching parser instances

## Support and Debugging

### Debug Mode
Enable verbose logging by temporarily adding:
```python
# Add to any relinker module for detailed logging
import logging
logging.basicConfig(level=logging.DEBUG)
```

### Log Analysis
Check Blender console for:
- `[Blend Vault][*]` messages for relinker activity
- Error traceback for debugging issues
- Success messages confirming operations

### Getting Help
- Check the `REFACTORING_SUMMARY.md` for architecture details
- Review `BEFORE_AFTER_COMPARISON.md` for understanding changes
- Consult shared utility docstrings for API usage

## Success Criteria

Migration is successful when:
- ✅ All existing relinking functionality works as before
- ✅ No new error messages in Blender console
- ✅ Performance is equal or better than original
- ✅ Code is easier to maintain and extend
- ✅ New shared utilities can be used for future features

This migration preserves all existing functionality while providing a significantly more maintainable and extensible codebase.
