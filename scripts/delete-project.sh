#!/usr/bin/env bash
# Delete a project including its worktree and branch

set -e

usage() {
    echo "Usage: $0 <project-path> [--force]"
    echo ""
    echo "Examples:"
    echo "  $0 blender/extensions/old-addon"
    echo "  $0 obsidian/plugins/unused-plugin --force"
    echo ""
    echo "Use --force to skip confirmation prompt."
    exit 1
}

if [ $# -lt 1 ] || [ $# -gt 2 ]; then
    usage
fi

PROJECT_PATH="$1"
FORCE_FLAG="$2"
WORKBENCH_PATH="workbench/$PROJECT_PATH"

# Check if project exists
if [ ! -d "$WORKBENCH_PATH" ]; then
    echo "Error: Project directory '$WORKBENCH_PATH' does not exist."
    exit 1
fi

# Check if branch exists
if ! git show-ref --verify --quiet "refs/heads/$PROJECT_PATH"; then
    echo "Error: Branch '$PROJECT_PATH' does not exist."
    exit 1
fi

# Check if worktree exists
if ! git worktree list | grep -q "$WORKBENCH_PATH"; then
    echo "Error: Worktree for '$WORKBENCH_PATH' does not exist."
    exit 1
fi

# Confirmation unless forced
if [ "$FORCE_FLAG" != "--force" ]; then
    echo "This will permanently delete:"
    echo "  - Directory: $WORKBENCH_PATH"
    echo "  - Branch: $PROJECT_PATH"
    echo "  - All git history for this project"
    echo ""
    read -p "Are you sure? (y/N): " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        echo "Operation cancelled."
        exit 0
    fi
fi

echo "Deleting project: $PROJECT_PATH"

# Remove worktree (this also removes the directory)
echo "Removing worktree..."
git worktree remove "$WORKBENCH_PATH" --force

# Delete branch
echo "Deleting branch '$PROJECT_PATH'..."
git branch -D "$PROJECT_PATH"

# Clean up empty parent directories
PARENT_DIR=$(dirname "$WORKBENCH_PATH")
while [ "$PARENT_DIR" != "workbench" ] && [ -d "$PARENT_DIR" ] && [ -z "$(ls -A "$PARENT_DIR" 2>/dev/null)" ]; do
    echo "Removing empty directory: $PARENT_DIR"
    rmdir "$PARENT_DIR"
    PARENT_DIR=$(dirname "$PARENT_DIR")
done

echo "Project '$PROJECT_PATH' deleted successfully!"