#!/usr/bin/env bash
# Sync all projects: pull latest changes, check status, and push if needed

usage() {
    echo "Usage: $0 [--dry-run] [--push]"
    echo ""
    echo "Options:"
    echo "  --dry-run    Show what would be done without making changes"
    echo "  --push       Automatically push changes to remote (if remote exists)"
    echo ""
    echo "Syncs all projects by checking for uncommitted changes and optionally"
    echo "pulling/pushing from remotes."
    exit 1
}

DRY_RUN=false
AUTO_PUSH=false

while [ $# -gt 0 ]; do
    case $1 in
        --dry-run)
            DRY_RUN=true
            shift
            ;;
        --push)
            AUTO_PUSH=true
            shift
            ;;
        *)
            usage
            ;;
    esac
done

echo "Syncing all projects..."
echo "======================"

# Track if any issues were found
ISSUES_FOUND=false

# Get worktree list and process each project
git worktree list | while read -r line; do
    # Parse worktree list output
    path=$(echo "$line" | awk '{print $1}')
    branch=$(echo "$line" | awk '{print $3}' | tr -d '[]')
    
    # Skip main branch
    if [ "$branch" = "main" ] || [ -z "$branch" ]; then
        continue
    fi
    
    # Get relative path and remove workbench/ prefix for cleaner display
    rel_path=$(realpath --relative-to="$(pwd)" "$path" 2>/dev/null || echo "$path")
    rel_path=${rel_path#workbench/}
    
    echo ""
    echo "Processing: $rel_path ($branch)"
    echo "----------------------------------------"
    
    # Change to project directory
    cd "$path" || {
        echo "Error: Cannot access $path"
        ISSUES_FOUND=true
        continue
    }
    
    # Check if it's a git repository (handles both normal repos and worktrees)
    if [ ! -d ".git" ] && [ ! -f ".git" ]; then
        echo "Warning: Not a git repository"
        cd - >/dev/null
        continue
    fi
    
    # Check for uncommitted changes
    if [ -n "$(git status --porcelain)" ]; then
        echo "Warning: Uncommitted changes found"
        git status --short
        ISSUES_FOUND=true
    else
        echo "Status: Clean working directory"
    fi
    
    # Check for remote
    remote=$(git remote 2>/dev/null | head -n1)
    if [ -n "$remote" ]; then
        echo "Remote: $remote ($(git remote get-url "$remote" 2>/dev/null || echo "unknown"))"
        
        # Fetch from remote
        if [ "$DRY_RUN" = false ]; then
            echo "Fetching from $remote..."
            git fetch "$remote" 2>/dev/null || echo "Warning: Failed to fetch from $remote"
        else
            echo "Would fetch from $remote"
        fi
        
        # Check if we're ahead/behind
        local_commit=$(git rev-parse HEAD 2>/dev/null)
        remote_commit=$(git rev-parse "$remote/$branch" 2>/dev/null || echo "")
        
        if [ -n "$remote_commit" ]; then
            if [ "$local_commit" != "$remote_commit" ]; then
                ahead=$(git rev-list --count "$remote/$branch..HEAD" 2>/dev/null || echo "0")
                behind=$(git rev-list --count "HEAD..$remote/$branch" 2>/dev/null || echo "0")
                
                if [ "$behind" -gt 0 ]; then
                    echo "Behind remote by $behind commits"
                    if [ "$DRY_RUN" = false ]; then
                        echo "Pulling changes..."
                        git pull "$remote" "$branch" || echo "Warning: Failed to pull"
                    else
                        echo "Would pull $behind commits"
                    fi
                fi
                
                if [ "$ahead" -gt 0 ]; then
                    echo "Ahead of remote by $ahead commits"
                    if [ "$AUTO_PUSH" = true ] && [ "$DRY_RUN" = false ]; then
                        echo "Pushing changes..."
                        git push "$remote" "$branch" || echo "Warning: Failed to push"
                    elif [ "$AUTO_PUSH" = true ]; then
                        echo "Would push $ahead commits"
                    else
                        echo "Use --push to automatically push changes"
                    fi
                fi
            else
                echo "Up to date with remote"
            fi
        else
            echo "Warning: Remote branch $branch does not exist"
        fi
    else
        echo "No remote configured"
    fi
    
    cd - >/dev/null
done

echo ""
echo "Sync complete!"
if [ "$ISSUES_FOUND" = true ]; then
    echo "Some issues were found. Review the output above."
    exit 1
fi