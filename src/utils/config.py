# Log color codes (ANSI escape sequences)
LOG_INFO = '\033[94m'    # Blue: Informational messages
LOG_SUCCESS = '\033[92m' # Green: Success/confirmation
LOG_WARN = '\033[93m'    # Yellow: Warnings
LOG_ERROR = '\033[91m'   # Red: Errors
LOG_RESET = '\033[0m'    # Reset to default

# Sidecar file extension
SIDECAR_EXTENSION = ".side.md"

# Default frontmatter tags
FRONTMATTER_TAGS = {"sidecar", "blendvault"}

# Default poll interval (seconds) for checking sidecar file changes
POLL_INTERVAL = 5.0
