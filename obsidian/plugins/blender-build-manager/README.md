# Blender Build Manager

Download, manage, and launch Blender builds directly from Obsidian. This is an unnoficial plugin, I am not affiliated with the Blender Foundation in any way.

> [!WARNING]
> This plugin requires an internet connection to fetch and download Blender builds from the official Blender servers.

## Getting started

After installing, use the ribbon icon or command palette to open the Blender Build Manager. The plugin will provide options to download new builds from the official Blender build server.

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
  - Symlink your build of choice to a consistently named directory (`bl_symlink`) and pin it in the UI.

## Debugging

In Devloper Console, run `window.DEBUG['blender-build-manager'].enable()`

To learn more, see [obsidian-logger](https://github.com/AMC-Albert/obsidian-logger).