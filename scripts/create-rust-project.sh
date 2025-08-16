#!/usr/bin/env bash

# Nix-based Rust project template instantiation script
# Usage: ./create-rust-project.sh <project-name> [author] [email] [output-dir]

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Default values
PROJECT_NAME="${1:-my-rust-project}"
AUTHOR="${2:-$(git config user.name 2>/dev/null || echo "Your Name")}"
EMAIL="${3:-$(git config user.email 2>/dev/null || echo "your.email@example.com")}"
OUTPUT_DIR="${4:-$PWD/$PROJECT_NAME}"
DESCRIPTION="${5:-A $PROJECT_NAME Rust project}"

echo "Creating Rust project: $PROJECT_NAME"
echo "Author: $AUTHOR <$EMAIL>"
echo "Description: $DESCRIPTION"
echo "Output directory: $OUTPUT_DIR"

# Create the output directory if it doesn't exist
mkdir -p "$OUTPUT_DIR"

# Use the template engine to instantiate the template
RESULT=$(nix-build --no-out-link -E "
let
  templateEngine = import $SCRIPT_DIR/template-engine.nix {};
in
templateEngine.createRustProject {
  name = \"$PROJECT_NAME\";
  author = \"$AUTHOR\";
  email = \"$EMAIL\";
  description = \"$DESCRIPTION\";
}")

echo "Template instantiated at: $RESULT"

# Copy the instantiated template to the output directory
echo "Copying files to $OUTPUT_DIR..."
cp -r "$RESULT"/* "$OUTPUT_DIR/"

# Make the output directory a git repository if it isn't already
if [ ! -d "$OUTPUT_DIR/.git" ]; then
  echo "Initializing git repository..."
  cd "$OUTPUT_DIR"
  git init
  git add .
  git commit -m "Initial commit from rust-nix-template

🤖 Generated with rust-nix-template
"
fi

echo "✅ Rust project '$PROJECT_NAME' created successfully at: $OUTPUT_DIR"
echo ""
echo "Next steps:"
echo "  cd $OUTPUT_DIR"
echo "  nix develop    # Enter development shell"
echo "  just run       # Build and run the project"