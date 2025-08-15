#!/usr/bin/env bash

# Blendfile downloader script
# Downloads files listed in blendfiles/blendfiles_map.json

set -euo pipefail

# Default values
ROOT_DIR="blendfiles"
MAP_FILE=""
FORCE=false
FOLDER=""
DRY_RUN=false

# Function to show usage
show_help() {
    cat << EOF
Download blendfiles listed in blendfiles/blendfiles_map.json

Usage: $(basename "$0") [OPTIONS]

Options:
    --root DIR        Root dir that contains the blendfiles_map.json and where files are written (default: blendfiles)
    --map FILE        Path to map JSON (defaults to <root>/blendfiles_map.json)
    --force           Force re-download even if file exists
    --folder FOLDER   Only process a specific folder key from the JSON map
    --dry-run         Dry-run: print what would be downloaded without performing network I/O
    -h, --help        Show this help message

EOF
}

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --root)
            ROOT_DIR="$2"
            shift 2
            ;;
        --map)
            MAP_FILE="$2"
            shift 2
            ;;
        --force)
            FORCE=true
            shift
            ;;
        --folder)
            FOLDER="$2"
            shift 2
            ;;
        --dry-run)
            DRY_RUN=true
            shift
            ;;
        -h|--help)
            show_help
            exit 0
            ;;
        *)
            echo "Unknown option: $1" >&2
            show_help >&2
            exit 1
            ;;
    esac
done

# Find workspace root by looking for Cargo.toml with [workspace]
find_workspace_root() {
    local current_dir="$(pwd)"
    
    while true; do
        if [[ -f "$current_dir/Cargo.toml" ]] && grep -q "^\[workspace\]" "$current_dir/Cargo.toml"; then
            echo "$current_dir"
            return 0
        fi
        
        local parent_dir="$(dirname "$current_dir")"
        if [[ "$parent_dir" == "$current_dir" ]]; then
            # Reached filesystem root
            echo "$(pwd)"
            return 0
        fi
        current_dir="$parent_dir"
    done
}

# Resolve root directory
if [[ "$ROOT_DIR" = /* ]]; then
    # Absolute path
    RESOLVED_ROOT="$ROOT_DIR"
else
    # Relative path - resolve relative to workspace root
    WORKSPACE_ROOT="${CARGO_WORKSPACE_DIR:-$(find_workspace_root)}"
    RESOLVED_ROOT="$WORKSPACE_ROOT/$ROOT_DIR"
fi

# Resolve map file path
if [[ -z "$MAP_FILE" ]]; then
    MAP_FILE="$RESOLVED_ROOT/blendfiles_map.json"
fi

# Check if map file exists
if [[ ! -f "$MAP_FILE" ]]; then
    echo "Error: map file not found: $MAP_FILE" >&2
    exit 1
fi

# Check if jq is available for JSON parsing
if ! command -v jq &> /dev/null; then
    echo "Error: jq is required for JSON parsing but not installed" >&2
    exit 1
fi

# Check if curl is available for downloading
if ! command -v curl &> /dev/null; then
    echo "Error: curl is required for downloading but not installed" >&2
    exit 1
fi

# Process the JSON map
if [[ -n "$FOLDER" ]]; then
    # Filter for specific folder
    FOLDERS=$(jq -r --arg folder "$FOLDER" 'to_entries | map(select(.key == $folder)) | from_entries | keys[]' "$MAP_FILE")
else
    # Process all folders
    FOLDERS=$(jq -r 'keys[]' "$MAP_FILE")
fi

# Download files
for folder in $FOLDERS; do
    echo "Processing folder: $folder"
    
    # Get all files for this folder
    jq -r --arg folder "$folder" '.[$folder] | to_entries[] | "\(.key)\t\(.value)"' "$MAP_FILE" | \
    while IFS=$'\t' read -r rel_path url; do
        # Construct destination path
        dest_path="$RESOLVED_ROOT/$folder/$rel_path"
        dest_dir="$(dirname "$dest_path")"
        
        # Create directory if it doesn't exist
        mkdir -p "$dest_dir"
        
        # Check if file already exists
        if [[ -f "$dest_path" ]] && [[ "$FORCE" != true ]]; then
            echo "$folder/$rel_path already exists, skipping."
            continue
        fi
        
        if [[ "$DRY_RUN" == true ]]; then
            echo "Would download $folder/$rel_path from $url"
        else
            echo "Downloading $folder/$rel_path..."
            if curl -L --fail --progress-bar -o "$dest_path" "$url"; then
                echo "✓ Downloaded $folder/$rel_path"
            else
                echo "✗ Failed to download $folder/$rel_path from $url" >&2
            fi
        fi
    done
done

echo "Done!"