#!/usr/bin/env bash
# Downloads files listed in /blendfiles/blendfiles_map.json (grouped by folder) if they are missing.

set -euo pipefail

# Root directory to store blendfiles and subfolders
TARGET_ROOT="$(dirname "$0")/../blendfiles"
MAP_FILE="$TARGET_ROOT/blendfiles_map.json"

if ! command -v jq &> /dev/null; then
    echo "Error: jq is required but not installed. Please install jq." >&2
    exit 1
fi

if [[ ! -f "$MAP_FILE" ]]; then
    echo "Error: $MAP_FILE not found!" >&2
    exit 1
fi

# Flatten the grouped JSON: output lines of the form "subfolder/filename<TAB>url"
jq -r 'to_entries[] | .key as $folder | .value | to_entries[] | "\($folder)/\(.key)\t\(.value)"' "$MAP_FILE" | while IFS=$'\t' read -r relpath url; do
    dest="$TARGET_ROOT/$relpath"
    dest_dir="$(dirname "$dest")"
    mkdir -p "$dest_dir"
    if [[ -f "$dest" ]]; then
        echo "$relpath already exists, skipping."
    else
        echo "Downloading $relpath..."
        curl -L -o "$dest" "$url"
    fi
done
