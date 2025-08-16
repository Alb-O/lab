#!/usr/bin/env bash
# Lab Project Manager - Unified worktree and template management
# Generic project lifecycle management with template support

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
LAB_ROOT="$(dirname "$SCRIPT_DIR")"
TEMPLATES_DIR="$SCRIPT_DIR/templates"
CONFIG_FILE="$LAB_ROOT/.lab-config"

# Load configuration if it exists
load_config() {
    if [ -f "$CONFIG_FILE" ]; then
        source "$CONFIG_FILE"
    fi
}

# Default configuration
DEFAULT_AUTHOR="${LAB_DEFAULT_AUTHOR:-$(git config user.name 2>/dev/null || echo "Your Name")}"
DEFAULT_EMAIL="${LAB_DEFAULT_EMAIL:-$(git config user.email 2>/dev/null || echo "your.email@example.com")}"
DEFAULT_BASE_BRANCH="${LAB_DEFAULT_BASE_BRANCH:-main}"

usage() {
    echo "Lab Project Manager"
    echo "=================="
    echo ""
    echo "Usage: $0 <command> [options]"
    echo ""
    echo "Commands:"
    echo "  create <path> [options]     Create a new project"
    echo "  delete <path> [--force]     Delete a project"
    echo "  list [--verbose]            List all projects"
    echo "  status <path>               Show project status"
    echo "  templates                   List available templates"
    echo ""
    echo "Create Options:"
    echo "  --template=TYPE             Use template (see 'templates' command)"
    echo "  --author=NAME               Set author name"
    echo "  --email=EMAIL               Set author email"
    echo "  --description=DESC          Set project description"
    echo "  --base-branch=BRANCH        Base branch (default: $DEFAULT_BASE_BRANCH)"
    echo ""
    echo "Examples:"
    echo "  $0 create misc/rust/new-tool --template=rust-nix"
    echo "  $0 create blender/extensions/addon --author=\"John Doe\""
    echo "  $0 delete old-project --force"
    echo "  $0 list --verbose"
    echo "  $0 status blender/extensions/my-addon"
    echo ""
    exit 1
}

# Utility functions
log_info() {
    echo "INFO: $*"
}

log_warn() {
    echo "WARN: $*" >&2
}

log_error() {
    echo "ERROR: $*" >&2
}

normalize_project_path() {
    local path="$1"
    
    # Remove trailing slashes
    path="${path%/}"
    
    # Remove leading ./ if present
    path="${path#./}"
    
    # Remove leading workbench/ if present (in case user includes it)
    path="${path#workbench/}"
    
    # Ensure path is not empty after normalization
    if [ -z "$path" ]; then
        log_error "Project path cannot be empty"
        exit 1
    fi
    
    echo "$path"
}

validate_project_path() {
    local path="$1"
    if [[ ! "$path" =~ ^[a-zA-Z0-9/_-]+$ ]]; then
        log_error "Invalid project path. Use only letters, numbers, hyphens, underscores, and forward slashes."
        exit 1
    fi
}

extract_project_name() {
    local path="$1"
    basename "$path"
}

branch_exists() {
    local branch="$1"
    git show-ref --verify --quiet "refs/heads/$branch"
}

dir_exists() {
    local path="$1"
    [ -d "$path" ]
}

# Template discovery and management
discover_templates() {
    local templates=()
    
    # Discover Nix templates
    if [ -d "$TEMPLATES_DIR/template-projects" ]; then
        for template_dir in "$TEMPLATES_DIR/template-projects"/*; do
            if [ -d "$template_dir" ]; then
                local template_name=$(basename "$template_dir")
                templates+=("$template_name")
            fi
        done
    fi
    
    printf '%s\n' "${templates[@]}"
}

get_template_type() {
    local template_name="$1"
    
    # Determine template type based on directory structure or metadata
    local template_path="$TEMPLATES_DIR/template-projects/$template_name"
    
    if [ -f "$template_path/Cargo.toml" ]; then
        echo "rust-nix"
    elif [ -f "$template_path/package.json" ]; then
        echo "node"
    elif [ -f "$template_path/pyproject.toml" ] || [ -f "$template_path/setup.py" ]; then
        echo "python"
    else
        echo "generic"
    fi
}

instantiate_template() {
    local template_name="$1"
    local project_name="$2"
    local author="$3"
    local email="$4"
    local description="$5"
    
    local template_type=$(get_template_type "$template_name")
    
    case "$template_type" in
        rust-nix)
            instantiate_rust_nix_template "$template_name" "$project_name" "$author" "$email" "$description"
            ;;
        *)
            log_error "Template type '$template_type' not supported yet"
            exit 1
            ;;
    esac
}

instantiate_rust_nix_template() {
    local template_name="$1"
    local project_name="$2"
    local author="$3"
    local email="$4"
    local description="$5"
    
    log_info "Instantiating Rust Nix template..."
    
    local template_result
    template_result=$(nix-build --no-out-link -E "
let
  templateEngine = import $TEMPLATES_DIR/nix/template-engine.nix {};
in
templateEngine.createRustProject {
  name = \"$project_name\";
  author = \"$author\";
  email = \"$email\";
  description = \"$description\";
}")
    
    if [ $? -eq 0 ] && [ -d "$template_result" ]; then
        cp -r "$template_result"/* .
        echo "$template_result"
    else
        log_error "Failed to instantiate template"
        exit 1
    fi
}

# Main commands
cmd_create() {
    load_config
    
    local project_path=""
    local template_type=""
    local author="$DEFAULT_AUTHOR"
    local email="$DEFAULT_EMAIL"
    local description=""
    local base_branch="$DEFAULT_BASE_BRANCH"
    
    # Parse arguments
    if [ $# -lt 1 ]; then
        log_error "create requires a project path"
        usage
    fi
    
    project_path="$1"
    shift
    
    while [ $# -gt 0 ]; do
        case $1 in
            --template=*)
                template_type="${1#*=}"
                ;;
            --author=*)
                author="${1#*=}"
                ;;
            --email=*)
                email="${1#*=}"
                ;;
            --description=*)
                description="${1#*=}"
                ;;
            --base-branch=*)
                base_branch="${1#*=}"
                ;;
            *)
                log_error "Unknown option: $1"
                usage
                ;;
        esac
        shift
    done
    
    # Normalize and validate inputs
    project_path=$(normalize_project_path "$project_path")
    validate_project_path "$project_path"
    
    local workbench_path="workbench/$project_path"
    local project_name=$(extract_project_name "$project_path")
    
    # Set default description if not provided
    if [ -z "$description" ]; then
        if [ -n "$template_type" ]; then
            description="A $project_name project from $template_type template"
        else
            description="Project: $project_path"
        fi
    fi
    
    # Validate template if specified
    if [ -n "$template_type" ]; then
        local available_templates=($(discover_templates))
        local template_found=false
        for tmpl in "${available_templates[@]}"; do
            if [ "$tmpl" = "$template_type" ]; then
                template_found=true
                break
            fi
        done
        
        if [ "$template_found" = false ]; then
            log_error "Template '$template_type' not found. Available templates:"
            cmd_templates
            exit 1
        fi
    fi
    
    # Check if branch already exists
    if branch_exists "$project_path"; then
        log_error "Branch '$project_path' already exists."
        exit 1
    fi
    
    # Check if directory already exists
    if dir_exists "$workbench_path"; then
        log_error "Directory '$workbench_path' already exists."
        exit 1
    fi
    
    log_info "Creating project: $project_path"
    log_info "Location: $workbench_path"
    log_info "Description: $description"
    if [ -n "$template_type" ]; then
        log_info "Template: $template_type"
        log_info "Author: $author <$email>"
    fi
    
    # Create directory structure
    mkdir -p "$(dirname "$workbench_path")"
    
    # Create orphan branch
    log_info "Creating branch '$project_path'..."
    git checkout --orphan "$project_path"
    git rm -rf . 2>/dev/null || true
    
    if [ -n "$template_type" ]; then
        # Use template system
        instantiate_template "$template_type" "$project_name" "$author" "$email" "$description"
        
        # Add all template files
        git add .
        git commit -m "Initialize from $template_type template

Generated with Lab template system

Author: $author <$email>
Template: $template_type
Description: $description"
    else
        # Create basic README
        echo "# $project_path" > README.md
        echo "" >> README.md
        echo "$description" >> README.md
        git add README.md
        git commit -m "Initial commit for $project_path"
    fi
    
    # Switch back to base branch
    git checkout "$base_branch"
    
    # Create worktree
    log_info "Creating worktree at '$workbench_path'..."
    git worktree add "$workbench_path" "$project_path"
    
    echo "Project '$project_path' created successfully!"
    echo "Location: $workbench_path"
    
    # Provide template-specific next steps
    if [ -n "$template_type" ]; then
        local template_type_detected=$(get_template_type "$template_type")
        case "$template_type_detected" in
            rust-nix)
                echo ""
                echo "Rust project ready! Next steps:"
                echo "  cd $workbench_path"
                echo "  nix develop    # Enter development shell"
                echo "  just run       # Build and run the project"
                ;;
        esac
    fi
}

cmd_delete() {
    local project_path=""
    local force=false
    
    # Parse arguments
    if [ $# -lt 1 ]; then
        log_error "delete requires a project path"
        usage
    fi
    
    project_path="$1"
    shift
    
    while [ $# -gt 0 ]; do
        case $1 in
            --force)
                force=true
                ;;
            *)
                log_error "Unknown option: $1"
                usage
                ;;
        esac
        shift
    done
    
    # Normalize and validate inputs
    project_path=$(normalize_project_path "$project_path")
    validate_project_path "$project_path"
    
    local workbench_path="workbench/$project_path"
    
    # Check if project exists
    if ! dir_exists "$workbench_path"; then
        log_error "Project directory '$workbench_path' does not exist."
        exit 1
    fi
    
    if ! branch_exists "$project_path"; then
        log_error "Branch '$project_path' does not exist."
        exit 1
    fi
    
    # Confirmation unless forced
    if [ "$force" != true ]; then
        log_warn "This will permanently delete:"
        echo "   Directory: $workbench_path"
        echo "   Branch: $project_path"
        echo "   All git history for this project"
        echo ""
        read -p "Are you sure? (y/N): " -n 1 -r
        echo
        if [[ ! $REPLY =~ ^[Yy]$ ]]; then
            echo "Operation cancelled."
            exit 0
        fi
    fi
    
    log_info "Deleting project: $project_path"
    
    # Remove worktree
    log_info "Removing worktree..."
    git worktree remove "$workbench_path" --force || true
    
    # Delete branch
    log_info "Deleting branch '$project_path'..."
    git branch -D "$project_path" || true
    
    # Clean up empty parent directories
    local parent_dir
    parent_dir=$(dirname "$workbench_path")
    while [ "$parent_dir" != "workbench" ] && [ -d "$parent_dir" ] && [ -z "$(ls -A "$parent_dir" 2>/dev/null)" ]; do
        log_info "Removing empty directory: $parent_dir"
        rmdir "$parent_dir"
        parent_dir=$(dirname "$parent_dir")
    done
    
    echo "Project '$project_path' deleted successfully!"
}

cmd_list() {
    local verbose=false
    
    # Parse arguments
    while [ $# -gt 0 ]; do
        case $1 in
            --verbose|-v)
                verbose=true
                ;;
            *)
                log_error "Unknown option: $1"
                usage
                ;;
        esac
        shift
    done
    
    echo "Lab Projects:"
    echo "============="
    
    git worktree list | while read -r line; do
        local path commit branch
        path=$(echo "$line" | awk '{print $1}')
        commit=$(echo "$line" | awk '{print $2}')
        branch=$(echo "$line" | awk '{print $3}' | tr -d '[]')
        
        # Skip main branch
        if [ "$branch" = "main" ] || [ -z "$branch" ]; then
            continue
        fi
        
        # Get relative path and remove workbench/ prefix
        local rel_path
        rel_path=$(realpath --relative-to="$(pwd)" "$path" 2>/dev/null || echo "$path")
        rel_path=${rel_path#workbench/}
        
        if [ "$verbose" = true ]; then
            # Get last commit message
            local last_commit
            last_commit=$(git --git-dir="$path/.git" log -1 --format="%s" 2>/dev/null || echo "No commits")
            
            # Check status
            local status
            if cd "$path" 2>/dev/null; then
                if [ -n "$(git status --porcelain 2>/dev/null)" ]; then
                    status="dirty"
                else
                    status="clean"
                fi
                cd - >/dev/null
            else
                status="unknown"
            fi
            
            printf "%-40s %-20s %-10s %s\n" "$rel_path" "$branch" "$status" "$last_commit"
        else
            printf "%-40s %s\n" "$rel_path" "$branch"
        fi
    done
    
    if [ "$verbose" = true ]; then
        echo ""
        echo "Legend: clean = no uncommitted changes, dirty = has uncommitted changes"
    fi
}

cmd_status() {
    local project_path=""
    
    # Parse arguments
    if [ $# -lt 1 ]; then
        log_error "status requires a project path"
        usage
    fi
    
    project_path="$1"
    
    # Normalize and validate inputs
    project_path=$(normalize_project_path "$project_path")
    validate_project_path "$project_path"
    
    local workbench_path="workbench/$project_path"
    
    if ! dir_exists "$workbench_path"; then
        log_error "Project directory '$workbench_path' does not exist."
        exit 1
    fi
    
    echo "Project Status: $project_path"
    echo "=============================="
    
    cd "$workbench_path"
    
    if [ ! -d ".git" ] && [ ! -f ".git" ]; then
        log_error "Not a git repository"
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
    
    # Recent commits
    echo "Recent Commits:"
    git log --oneline -5 | sed 's/^/  /'
    echo ""
    
    # File stats
    echo "Repository Stats:"
    echo "  Total files: $(find . -type f -not -path './.git/*' | wc -l)"
    echo "  Git tracked: $(git ls-files | wc -l)"
    echo "  Untracked: $(git ls-files --others --exclude-standard | wc -l)"
}

cmd_templates() {
    echo "Available Templates:"
    echo "==================="
    
    local templates=($(discover_templates))
    
    if [ ${#templates[@]} -eq 0 ]; then
        echo "No templates found in $TEMPLATES_DIR/template-projects/"
        return
    fi
    
    for template in "${templates[@]}"; do
        local template_type=$(get_template_type "$template")
        local template_path="$TEMPLATES_DIR/template-projects/$template"
        
        echo "Template: $template"
        echo "  Type: $template_type"
        echo "  Path: $template_path"
        
        # Try to get description from README if available
        if [ -f "$template_path/README.md" ]; then
            local description=$(head -n 5 "$template_path/README.md" | grep -v '^#' | head -n 1 | sed 's/^[[:space:]]*//')
            if [ -n "$description" ]; then
                echo "  Description: $description"
            fi
        fi
        echo ""
    done
}

# Main command dispatcher
main() {
    if [ $# -lt 1 ]; then
        usage
    fi
    
    local command="$1"
    shift
    
    case "$command" in
        create)
            cmd_create "$@"
            ;;
        delete)
            cmd_delete "$@"
            ;;
        list)
            cmd_list "$@"
            ;;
        status)
            cmd_status "$@"
            ;;
        templates)
            cmd_templates "$@"
            ;;
        help|--help|-h)
            usage
            ;;
        *)
            log_error "Unknown command: $command"
            usage
            ;;
    esac
}

# Change to lab root directory for git operations
cd "$LAB_ROOT"

# Run main function
main "$@"