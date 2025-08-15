# Fragments Plugin for Obsidian

Trim and timestamp media non-destructively in Obsidian.

## Getting started

After installing, the plugin automatically detects videos in your markdown notes and provides fragment management capabilities. Use the context menu on any video element to set time ranges, create embeds, or manage playback controls.

## Features

- **Non-destructive media trimming:**
  - Set start and end times for video fragments without modifying the original files.
  - Support for multiple time formats: seconds, HH:MM:SS, percentages, and natural language expressions.

- **Context menu integration:**
  - Right-click any video to access fragment controls, embedding options, and playback settings.
  - Quick actions for setting current time as start/end points.

- **Advanced time parsing:**
  - Parse time expressions like "2 minutes 30 seconds", "50%", "1:30", or simple seconds.
  - Intelligent validation and error handling for time inputs.

- **Timeline controls:**
  - Visual timeline representation of video fragments.
  - Hover controls for quick navigation and fragment preview.

- **Fragment persistence:**
  - Fragments are stored in your markdown files using standard media fragment syntax.
  - Compatible with web standards and other applications.

## Debugging

In Devloper Console, run `window.DEBUG.enable('fragments')`

To learn more, see [obsidian-logger](https://github.com/AMC-Albert/obsidian-logger).