# Blend Vault Extension Architecture

## Overview

This document describes the simplified and consolidated architecture of the Blend Vault Blender extension after a comprehensive review and refactoring.

## Core Principles

1. **Simplicity**: Eliminated complex registration patterns and redundant code
2. **Consolidation**: Unified utility functions in a single core module
3. **Flexibility**: Maintained modular structure while reducing dependencies
4. **Reliability**: Robust error handling and graceful module loading

## Project Structure

```
blend_vault_ext/
├── __init__.py                     # Main extension entry point
├── blend_vault/
│   ├── __init__.py                 # Package exports and re-exports
│   ├── core.py                     # Consolidated utilities (NEW)
│   ├── preferences.py              # Extension preferences
│   ├── utils/
│   │   ├── __init__.py             # Backward compatibility exports
│   │   ├── constants.py            # Constants and configuration
│   │   └── templates.py            # Template system
│   ├── relink/
│   │   ├── __init__.py             # Simplified registration
│   │   ├── shared_utils.py         # Relink-specific utilities
│   │   ├── asset_relinker.py       # Asset relinking
│   │   ├── library_relinker.py     # Library relinking
│   │   ├── resource_relinker.py    # Resource relinking
│   │   ├── polling.py              # Polling mechanisms
│   │   └── redirect_handler.py     # Redirect handling
│   ├── paste_path/
│   │   ├── __init__.py             # Simplified registration
│   │   ├── core_operators.py       # Core paste operations
│   │   ├── asset_discovery.py      # Asset discovery
│   │   ├── dialogs.py              # User dialogs
│   │   ├── file_validation.py      # File validation
│   │   ├── smart_paste.py          # Smart paste logic
│   │   └── save_workflow.py        # Save workflow
│   ├── obsidian_integration/
│   │   ├── __init__.py             # Simplified registration
│   │   └── uri_handler.py          # URI protocol handling
│   └── sidecar_io/
│       ├── __init__.py             # Sidecar I/O module
│       ├── collectors.py           # Data collection
│       ├── content_builder.py      # Content generation
│       ├── file_operations.py      # File operations
│       ├── frontmatter.py          # Frontmatter handling
│       ├── uuid_manager.py         # UUID management
│       └── writer.py               # Sidecar writing
```

## Key Changes Made

### 1. Core Module Consolidation (`blend_vault/core.py`)

**Purpose**: Single source of truth for all common utilities.

**Consolidated Functions**:
- **Logging**: `log_info`, `log_warning`, `log_error`, `log_success`, `log_debug`
- **Asset/Datablock**: `get_asset_sources_map`, `get_or_create_datablock_uuid`
- **Path/Link**: `format_primary_link`, `parse_primary_link`, `generate_filepath_hash`, `get_resource_warning_prefix`
- **Regex Helpers**: `build_section_heading_regex`, `build_heading_section_break_regex`
- **Blender Utilities**: `ensure_saved_file`

**Benefits**:
- Eliminates code duplication
- Single import location for common functions
- Consistent error handling and logging

### 2. Simplified Registration Patterns

**Before**: Complex reload patterns with `importlib.reload()` and manual handler management.

**After**: Simple, robust registration using attribute checks:

```python
def register():
    """Register all components."""
    modules = [module1, module2, module3]
    for module in modules:
        if hasattr(module, 'register'):
            module.register()

def unregister():
    """Unregister all components."""
    modules = [module3, module2, module1]  # Reverse order
    for module in modules:
        if hasattr(module, 'unregister'):
            module.unregister()
```

**Benefits**:
- More reliable during development
- Graceful handling of missing functions
- Simpler debugging and maintenance

### 3. Import Consolidation

**Previous**: Scattered imports from `blend_vault.utils.helpers` throughout the codebase.

**Current**: Unified imports from `blend_vault.core` with backward-compatible re-exports.

**Migration Pattern**:
```python
# Old
from ..utils.helpers import log_info, log_warning, get_asset_sources_map

# New
from ..core import log_info, log_warning, get_asset_sources_map
```

### 4. Removed Files

- `blend_vault/utils/helpers.py` - Functions moved to `core.py`

## Module Responsibilities

### Core (`blend_vault/core.py`)
- Logging utilities with consistent formatting
- Asset and datablock management
- Path resolution and link formatting
- Regex pattern building
- Basic Blender file operations

### Utils (`blend_vault/utils/`)
- `constants.py`: Configuration constants and settings
- `templates.py`: Template system for sidecar generation
- `__init__.py`: Backward compatibility re-exports

### Relink (`blend_vault/relink/`)
- Automatic relinking of moved/renamed assets, libraries, and resources
- Sidecar parsing and path resolution utilities
- File polling and redirect handling

### Paste Path (`blend_vault/paste_path/`)
- Clipboard integration for .blend files
- Asset discovery and validation
- User interaction dialogs

### Obsidian Integration (`blend_vault/obsidian_integration/`)
- URI protocol handling
- Sidecar file management from Obsidian

### Sidecar I/O (`blend_vault/sidecar_io/`)
- Data collection from Blender scenes
- Markdown content generation
- File operations and UUID management

## Best Practices

### Imports
- Use relative imports within the package
- Import from `core` for common utilities
- Import from `constants` for configuration values
- Maintain backward compatibility through re-exports

### Registration
- Use simple attribute checking: `if hasattr(module, 'register'):`
- Unregister in reverse order of registration
- Keep registration functions minimal and focused

### Logging
- Use the centralized logging functions from `core`
- Include module names for context: `log_info("Message", module_name="ModuleName")`
- Use appropriate log levels consistently

### Error Handling
- Use the centralized logging for errors
- Implement graceful degradation where possible
- Provide meaningful error messages

## Migration Guide

If you need to update code that uses the old structure:

1. **Update imports**: Change from `..utils.helpers` to `..core`
2. **Remove importlib.reload()**: Not needed with simplified registration
3. **Use attribute checking**: `hasattr(module, 'register')` instead of direct calls
4. **Centralize utilities**: Move any duplicate functions to `core.py`

## Future Considerations

- The `shared_utils.py` in the relink module contains specialized classes that could potentially be further consolidated if other modules need similar functionality
- Consider moving template-related functions from `utils/templates.py` to `core.py` if they become more widely used
- Monitor for any new utility duplication and consolidate as needed
