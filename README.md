# Blender Build Manager Plugin for Obsidian

Download, manage, and launch Blender builds directly from Obsidian.

## Getting started

After installing, use the ribbon icon or command palette to open the Blender Build Manager. The plugin will automatically detect builds in your configured directories and provide options to download new builds from the official Blender build server.

## Features

- **Build Detection and Management:**
  - Automatically detect installed Blender builds
  - Download builds directly from the official Blender build server
  - Auto-extract downloaded archives

- **Version Filtering:**
  - Filter builds by version, branch, and build type
  - View stable releases, daily builds, and experimental branches

- **Quick Launch:**
  - Launch any installed Blender build directly from Obsidian
  - Organize builds by version and type

- **Settings:**
  - Configure download and extraction directories
  - Customize build detection and filtering options

## Debugging

To troubleshoot issues with the Blender Build Manager plugin, you can enable debug logging:

### Quick Setup
1. Open Developer Console (`Ctrl+Shift+I` or `Cmd+Option+I`)
2. Run:

```javascript
window.DEBUG.enable('blender-build-manager')
```

You should see `[blender-build-manager]` messages in the console when:
- Downloading or extracting builds
- Detecting installed builds
- Plugin encounters issues

### Disable Debug

```javascript
window.DEBUG.disable('blender-build-manager')
```
