#!/usr/bin/env bash
# Rename a project (move worktree and rename branch)

set -e

usage() {
    echo "Usage: $0 <current-path> <new-path>"
    echo ""
    echo "Examples:"
    echo "  $0 blender/extensions/old-name blender/extensions/new-name"
    echo "  $0 misc/rust/tool obsidian/plugins/tool"
    echo ""
    echo "Both the directory and branch will be renamed to match the new path."
    exit 1
}

if [ $# -ne 2 ]; then
    usage
fi

CURRENT_PATH="$1"
NEW_PATH="$2"
CURRENT_WORKBENCH_PATH="workbench/$CURRENT_PATH"
NEW_WORKBENCH_PATH="workbench/$NEW_PATH"

# Validate new project path format
if [[ ! "$NEW_PATH" =~ ^[a-zA-Z0-9/_-]+$ ]]; then
    echo "Error: New project path contains invalid characters. Use only letters, numbers, hyphens, underscores, and forward slashes."
    exit 1
fi

# Check if current project exists
if [ ! -d "$CURRENT_WORKBENCH_PATH" ]; then
    echo "Error: Project directory '$CURRENT_WORKBENCH_PATH' does not exist."
    exit 1
fi

# Check if current branch exists
if ! git show-ref --verify --quiet "refs/heads/$CURRENT_PATH"; then
    echo "Error: Branch '$CURRENT_PATH' does not exist."
    exit 1
fi

# Check if new branch already exists
if git show-ref --verify --quiet "refs/heads/$NEW_PATH"; then
    echo "Error: Branch '$NEW_PATH' already exists."
    exit 1
fi

# Check if new directory already exists
if [ -d "$NEW_WORKBENCH_PATH" ]; then
    echo "Error: Directory '$NEW_WORKBENCH_PATH' already exists."
    exit 1
fi

echo "Renaming project: $CURRENT_PATH -> $NEW_PATH"

# Create the new directory structure if needed
mkdir -p "$(dirname "$NEW_WORKBENCH_PATH")"

# Move the worktree
echo "Moving worktree..."
git worktree move "$CURRENT_WORKBENCH_PATH" "$NEW_WORKBENCH_PATH"

# Rename the branch
echo "Renaming branch '$CURRENT_PATH' to '$NEW_PATH'..."
git branch -m "$CURRENT_PATH" "$NEW_PATH"

# Clean up empty parent directories from old path
PARENT_DIR=$(dirname "$CURRENT_WORKBENCH_PATH")
while [ "$PARENT_DIR" != "workbench" ] && [ -d "$PARENT_DIR" ] && [ -z "$(ls -A "$PARENT_DIR" 2>/dev/null)" ]; do
    echo "Removing empty directory: $PARENT_DIR"
    rmdir "$PARENT_DIR"
    PARENT_DIR=$(dirname "$PARENT_DIR")
done

echo "Project renamed successfully!"
echo "New location: $NEW_WORKBENCH_PATH"
echo "New branch: $NEW_PATH"