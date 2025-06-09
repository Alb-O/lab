# Building Blend Vault Extension

This document explains how to build the Blend Vault extension from source.

## Prerequisites

- Blender 4.2+ (required for extension building)
- Python 3.11+ 
- pip (for downloading wheel dependencies)
- PowerShell (Windows) or Bash (Linux/macOS)

## Quick Build

If you just want to build the extension without downloading wheels:

```bash
# Using Blender's extension build command
blender --command extension build
```

## Full Build with Wheels

To download all wheel dependencies and build the extension:

### 1. Download Required Wheels

#### Windows (PowerShell):
```powershell
.\download_wheels.ps1
```

#### Linux/macOS (Manual):
```bash
# Create wheels directory
mkdir wheels

# Download Pillow wheels for all platforms
pip download Pillow==10.4.0 --dest ./wheels --only-binary=:all: --python-version=3.11 --platform=win_amd64
pip download Pillow==10.4.0 --dest ./wheels --only-binary=:all: --python-version=3.11 --platform=macosx_11_0_arm64
pip download Pillow==10.4.0 --dest ./wheels --only-binary=:all: --python-version=3.11 --platform=manylinux_2_28_x86_64
```

### 2. Build the Extension

```bash
# Standard build (includes all platforms in one zip)
blender --command extension build

# Platform-specific builds (creates separate zips per platform)
blender --command extension build --split-platforms
```

## Installation

### For Testing

```bash
# Install the built extension for testing
blender --command extension install-file --repo user_default ./blend_vault-0.4.1.zip
```

### For Distribution

Upload the generated `.zip` file(s) to:
- [Blender Extensions Platform](https://extensions.blender.org/)
- GitHub Releases
- Direct distribution

## Build Outputs

- **blend_vault-0.4.1.zip** - Universal build with all platform wheels
- **blend_vault-0.4.1-windows_x64.zip** - Windows-specific build
- **blend_vault-0.4.1-macos_arm64.zip** - macOS ARM build  
- **blend_vault-0.4.1-linux_x64.zip** - Linux build

## Validation

To validate the extension manifest:

```bash
blender --command extension validate
```

## Troubleshooting

### Build Failures

1. **Missing wheels**: Run the wheel download script first
2. **Blender not found**: Ensure Blender 4.2+ is in your PATH
3. **Permission errors**: Run with administrator/sudo privileges

### Installation Issues

1. **Repository not found**: Use `--repo user_default` parameter
2. **Conflicting versions**: Uninstall previous versions first
3. **Missing dependencies**: Ensure all wheels are included in the build

## File Structure

```
blend_vault_ext/
├── __init__.py              # Main extension file with wheel loading
├── blender_manifest.toml    # Extension metadata and wheel dependencies
├── wheels/                  # Downloaded wheel files
│   └── pillow-*.whl
├── extracted_wheels/        # Runtime wheel extraction (created automatically)
├── blend_vault/             # Core extension modules
│   ├── core.py
│   ├── preferences.py
│   ├── obsidian_integration/
│   ├── paste_path/
│   ├── relink/
│   ├── sidecar_io/
│   └── utils/
└── BUILD.md                 # This file
```

## Development

For development builds, excluded files are defined in the `[build]` section of `blender_manifest.toml`:

```toml
[build]
paths_exclude_pattern = [
  "__pycache__/",
  "*.pyc",
  "*.ps1",
  "download_wheels.ps1",
  "extracted_wheels/"
]
```

## Dependencies

### Bundled Dependencies

- **Pillow (PIL) 10.4.0**: For saving preview images as PNG files
  - Provides significant performance improvement over Blender's internal image handling
  - Bundled as platform-specific wheels for Windows, macOS, and Linux

### Blender Dependencies

- **Blender 4.0+**: Core functionality
- **bpy.utils.previews**: For extracting blend file preview images
- **webbrowser**: For Obsidian URI integration
