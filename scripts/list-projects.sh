#!/usr/bin/env bash
# List all projects with their status

usage() {
    echo "Usage: $0 [--verbose|-v]"
    echo ""
    echo "List all projects in the lab with their current status."
    echo "Use --verbose or -v for detailed information."
    exit 1
}

VERBOSE=false
if [ "$1" = "--verbose" ] || [ "$1" = "-v" ]; then
    VERBOSE=true
elif [ $# -gt 0 ]; then
    usage
fi

echo "Lab Projects:"
echo "============="

# Get worktree list and format it
git worktree list | while read -r line; do
    # Parse worktree list output: path commit [branch]
    path=$(echo "$line" | awk '{print $1}')
    commit=$(echo "$line" | awk '{print $2}')
    branch=$(echo "$line" | awk '{print $3}' | tr -d '[]')
    
    # Skip main branch
    if [ "$branch" = "main" ] || [ -z "$branch" ]; then
        continue
    fi
    
    # Get relative path from lab root and remove workbench/ prefix for cleaner display
    rel_path=$(realpath --relative-to="$(pwd)" "$path" 2>/dev/null || echo "$path")
    rel_path=${rel_path#workbench/}
    
    if [ "$VERBOSE" = true ]; then
        # Get last commit message
        last_commit=$(git --git-dir="$path/.git" log -1 --format="%s" 2>/dev/null || echo "No commits")
        
        # Check if there are uncommitted changes
        cd "$path" 2>/dev/null && {
            if [ -n "$(git status --porcelain 2>/dev/null)" ]; then
                status="(dirty)"
            else
                status="(clean)"
            fi
            cd - >/dev/null
        } || status="(unknown)"
        
        printf "%-40s %-20s %s %s\n" "$rel_path" "$branch" "$status" "$last_commit"
    else
        printf "%-40s %s\n" "$rel_path" "$branch"
    fi
done

if [ "$VERBOSE" = true ]; then
    echo ""
    echo "Legend: (clean) = no uncommitted changes, (dirty) = has uncommitted changes"
fi