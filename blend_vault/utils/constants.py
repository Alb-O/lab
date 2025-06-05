import re

# Log color codes (ANSI escape sequences)
LOG_COLORS = {
    'INFO': '\033[94m',    # Blue: Informational messages
    'SUCCESS': '\033[92m', # Green: Success/confirmation
    'WARN': '\033[93m',    # Yellow: Warnings
    'ERROR': '\033[91m',   # Red: Errors
    'DEBUG': '\033[95m',   # Magenta: Debug messages (added)
    'RESET': '\033[0m',    # Reset to default
}

# Markdown link formats and regex patterns
MD_LINK_FORMATS = {
    'MD_ANGLE_BRACKETS': {
        'format': '[{name}](<{path}>)',
        'regex': r'\[([^\]]+)\]\(<([^>]+)>\)'
    },
    'MD_WIKILINK': {
        'format': '[[{path}|{name}]]',
        'regex': r'\[\[([^\]|]+)\|([^\]]+)\]\]'
    }
}

MD_PRIMARY_FORMAT = MD_LINK_FORMATS['MD_WIKILINK']  # Set Obsidian wikilink as primary format

# Compile primary link regex
PRIMARY_LINK_REGEX = re.compile(MD_PRIMARY_FORMAT['regex'])

# Obsidian-style embed wikilink format: ![[path|alias]] or ![[name]]
MD_EMBED_WIKILINK = {
    'format': '![[{name}]]',
    'regex': r'!\[\[([^\]|]+)(?:\|([^\]]+))?\]\]'
}

# Sidecar file extension
SIDECAR_EXTENSION = ".side.md"

# Redirect file extension
REDIRECT_EXTENSION = ".redirect.md"

# Default frontmatter tags
FRONTMATTER_TAGS = {"sidecar", "blendvault"}

# Default poll interval (seconds) for checking for file changes
POLL_INTERVAL = 1.0

# Warning prefix for resources outside the vault
RESOURCE_WARNING_PREFIX = "⚠️ "

# --- Blend Vault UUID/Hash Key Constants ---
BV_UUID_PROP = "BV_UUID"
BV_FILE_UUID_KEY = "blendfile_uuid"
BV_UUID_KEY = "uuid"

# --- Sidecar Content Constants ---
# Markdown heading levels
HEADING_LEVEL_2 = "## "
HEADING_LEVEL_3 = "### "
HEADING_LEVEL_4 = "#### "

# Section headings
SIDECAR_MAIN_HEADING = HEADING_LEVEL_2 + "%% Blend Vault Data"
SIDECAR_MESSAGE_EMBED = "![[BV_MSG#^bv-msg-sidecar]]"
SIDECAR_CURRENT_FILE_HEADING = "Current File"
SIDECAR_LINKED_LIBRARIES_HEADING = HEADING_LEVEL_3 + "Linked Libraries"
SIDECAR_RESOURCES_HEADING = HEADING_LEVEL_3 + "Resources"

# Default content
SIDECAR_NO_ITEMS = "- None"
SIDECAR_JSON_BLOCK_START = "```json"
SIDECAR_JSON_BLOCK_END = "```"

# Resource type mappings
RESOURCE_TYPE_ORDER = ["Image", "Video", "Audio", "Text", "Cache"]
RESOURCE_TYPE_DISPLAY_NAMES = {
    "Image": "Textures",
    "Video": "Videos", 
    "Audio": "Audio",
    "Text": "Scripts",
    "Cache": "Caches"
}