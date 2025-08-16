#!/usr/bin/env bash
# Show detailed status for a specific project

usage() {
    echo "Usage: $0 <project-path>"
    echo ""
    echo "Examples:"
    echo "  $0 blender/extensions/blend-vault"
    echo "  $0 obsidian/plugins/fragments"
    echo ""
    echo "Shows detailed git status, branch info, and remote status for a project."
    exit 1
}

if [ $# -ne 1 ]; then
    usage
fi

PROJECT_PATH="$1"
WORKBENCH_PATH="workbench/$PROJECT_PATH"

# Check if project exists
if [ ! -d "$WORKBENCH_PATH" ]; then
    echo "Error: Project directory '$WORKBENCH_PATH' does not exist."
    exit 1
fi

echo "Project Status: $PROJECT_PATH"
echo "=============================="

# Change to project directory
cd "$WORKBENCH_PATH" || exit 1

# Check if it's a git repository (handles both normal repos and worktrees)
if [ ! -d ".git" ] && [ ! -f ".git" ]; then
    echo "Error: Not a git repository"
    exit 1
fi

# Basic info
echo "Branch: $(git branch --show-current)"
echo "Commit: $(git rev-parse --short HEAD) - $(git log -1 --format='%s')"
echo "Author: $(git log -1 --format='%an <%ae>')"
echo "Date: $(git log -1 --format='%ad' --date=relative)"
echo ""

# Working directory status
echo "Working Directory:"
if [ -n "$(git status --porcelain)" ]; then
    echo "  Status: Dirty (uncommitted changes)"
    echo "  Changes:"
    git status --short | sed 's/^/    /'
else
    echo "  Status: Clean"
fi
echo ""

# Stash info
stash_count=$(git stash list | wc -l)
if [ "$stash_count" -gt 0 ]; then
    echo "Stashes: $stash_count"
    git stash list | head -5 | sed 's/^/  /'
    if [ "$stash_count" -gt 5 ]; then
        echo "  ... and $((stash_count - 5)) more"
    fi
    echo ""
fi

# Remote info
remote=$(git remote 2>/dev/null | head -n1)
if [ -n "$remote" ]; then
    echo "Remote: $remote"
    echo "  URL: $(git remote get-url "$remote" 2>/dev/null || echo "unknown")"
    
    # Fetch to get latest remote info
    git fetch "$remote" 2>/dev/null || echo "  Warning: Failed to fetch from remote"
    
    branch=$(git branch --show-current)
    remote_branch="$remote/$branch"
    
    if git show-ref --verify --quiet "refs/remotes/$remote_branch"; then
        local_commit=$(git rev-parse HEAD)
        remote_commit=$(git rev-parse "$remote_branch")
        
        if [ "$local_commit" = "$remote_commit" ]; then
            echo "  Status: Up to date"
        else
            ahead=$(git rev-list --count "$remote_branch..HEAD" 2>/dev/null || echo "0")
            behind=$(git rev-list --count "HEAD..$remote_branch" 2>/dev/null || echo "0")
            
            if [ "$ahead" -gt 0 ] && [ "$behind" -gt 0 ]; then
                echo "  Status: Diverged ($ahead ahead, $behind behind)"
            elif [ "$ahead" -gt 0 ]; then
                echo "  Status: Ahead by $ahead commits"
            elif [ "$behind" -gt 0 ]; then
                echo "  Status: Behind by $behind commits"
            fi
        fi
    else
        echo "  Status: Remote branch does not exist"
    fi
else
    echo "Remote: None configured"
fi
echo ""

# Recent commits
echo "Recent Commits:"
git log --oneline -10 | sed 's/^/  /'
echo ""

# File statistics
echo "Repository Stats:"
echo "  Total files: $(find . -type f -not -path './.git/*' | wc -l)"
echo "  Git tracked: $(git ls-files | wc -l)"
echo "  Untracked: $(git ls-files --others --exclude-standard | wc -l)"

# Repository size
if command -v du >/dev/null 2>&1; then
    repo_size=$(du -sh . 2>/dev/null | cut -f1)
    git_size=$(du -sh .git 2>/dev/null | cut -f1)
    echo "  Disk usage: $repo_size (git: $git_size)"
fi