#!/usr/bin/env bash
# Create a new project with worktree and branch

set -e

usage() {
    echo "Usage: $0 <project-path> [base-branch]"
    echo ""
    echo "Examples:"
    echo "  $0 blender/extensions/new-addon"
    echo "  $0 obsidian/plugins/my-plugin"
    echo "  $0 misc/rust/cli-tool main"
    echo ""
    echo "The project-path will be used as both the directory path and branch name."
    echo "Base branch defaults to 'main' if not specified."
    exit 1
}

if [ $# -lt 1 ] || [ $# -gt 2 ]; then
    usage
fi

PROJECT_PATH="$1"
BASE_BRANCH="${2:-main}"

# Validate project path format
if [[ ! "$PROJECT_PATH" =~ ^[a-zA-Z0-9/_-]+$ ]]; then
    echo "Error: Project path contains invalid characters. Use only letters, numbers, hyphens, underscores, and forward slashes."
    exit 1
fi

# Check if branch already exists
if git show-ref --verify --quiet "refs/heads/$PROJECT_PATH"; then
    echo "Error: Branch '$PROJECT_PATH' already exists."
    exit 1
fi

# Check if directory already exists
if [ -d "workbench/$PROJECT_PATH" ]; then
    echo "Error: Directory 'workbench/$PROJECT_PATH' already exists."
    exit 1
fi

echo "Creating new project: $PROJECT_PATH"
echo "Base branch: $BASE_BRANCH"

# Create the directory structure under workbench
WORKBENCH_PATH="workbench/$PROJECT_PATH"
mkdir -p "$(dirname "$WORKBENCH_PATH")"

# Create orphan branch
echo "Creating branch '$PROJECT_PATH'..."
git checkout --orphan "$PROJECT_PATH"
git rm -rf . 2>/dev/null || true

# Create initial commit
echo "# $PROJECT_PATH" > README.md
git add README.md
git commit -m "Initial commit for $PROJECT_PATH"

# Switch back to main
git checkout main

# Create worktree
echo "Creating worktree at '$WORKBENCH_PATH'..."
git worktree add "$WORKBENCH_PATH" "$PROJECT_PATH"

echo "Project '$PROJECT_PATH' created successfully!"
echo "You can now work in: $WORKBENCH_PATH"