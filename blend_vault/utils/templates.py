"""
Markdown template system for Blend Vault sidecars.
Provides flexible and maintainable templates for generating sidecar content.

This module centralizes all template-related logic, including:
- Template structure definitions
- Heading level management
- Link format handling
- Regex pattern generation
- Backward compatibility utilities
"""

import re
from typing import Dict, Any, Optional, Union, Pattern

# Import only basic constants from constants.py that are not template-related
from .constants import (
    MD_LINK_FORMATS, 
    MD_PRIMARY_FORMAT, 
    SIDECAR_NO_ITEMS,
    SIDECAR_JSON_BLOCK_START,
    SIDECAR_JSON_BLOCK_END,
    RESOURCE_TYPE_ORDER,
    RESOURCE_TYPE_DISPLAY_NAMES
)


# Sidecar template configuration
SIDECAR_TEMPLATE_CONFIG = {
    "base_heading_level": 1,  # Base heading level (1 = H1, 2 = H2, etc.)
    "heading_increment": 1,   # How much to increment for sub-headings
}

# Template structure definition
SIDECAR_TEMPLATE_STRUCTURE = {
    "main_heading": {
        "level_offset": 0,
        "text": "%% BV Data",
        "has_link": False
    },
    "message_embed": {
        "content": "![[bv-autogen#^bv-autogen-sidecar]]"
    },
    "current_file": {
        "level_offset": 1,
        "text": "Current File",
        "has_link": True,
        "link_alias": None
    },
    "linked_libraries": {
        "level_offset": 1,
        "text": "Linked Libraries",
        "has_link": False
    },
    "resources": {
        "level_offset": 1,
        "text": "Resources",
        "has_link": False
    },
    "library_entry": {
        "level_offset": 2,
        "text": "Library",  # This will be replaced with actual library name
        "has_link": True,
        "link_alias": None
    },
    "resource_type": {
        "level_offset": 2,
        "text": "Resource Type",  # This will be replaced with actual resource type display name
        "has_link": False
    }
}

# Heading level constants
HEADING_LEVEL_1 = "# "
HEADING_LEVEL_2 = "## "
HEADING_LEVEL_3 = "### "
HEADING_LEVEL_4 = "#### "
HEADING_LEVEL_5 = "##### "
HEADING_LEVEL_6 = "###### "

# Mapping from heading level numbers to heading prefixes
HEADING_LEVEL_MAP = {
    1: HEADING_LEVEL_1,
    2: HEADING_LEVEL_2,
    3: HEADING_LEVEL_3,
    4: HEADING_LEVEL_4,
    5: HEADING_LEVEL_5,
    6: HEADING_LEVEL_6
}

# Additional template content constants
SIDECAR_MESSAGE_EMBED = "![[bv-autogen#^bv-autogen-sidecar]]"

def get_heading_prefix(level: int) -> str:
    """Get the heading prefix for a given level."""
    return HEADING_LEVEL_MAP.get(level, HEADING_LEVEL_6)  # Default to H6 for levels > 6


def build_template_heading(
    section_key: str,
    path: Optional[str] = None,
    alias: Optional[str] = None,
    link_format: str = 'MD_WIKILINK'
) -> str:
    """
    Build a heading for a template section with optional link formatting.
    
    Args:
        section_key: Key from SIDECAR_TEMPLATE_STRUCTURE
        path: Optional path for link
        alias: Optional alias for link (if None, uses section text)
        link_format: Link format key from MD_LINK_FORMATS
    
    Returns:
        Formatted heading string
    """
    if section_key not in SIDECAR_TEMPLATE_STRUCTURE:
        raise ValueError(f"Unknown section key: {section_key}")
    
    section = SIDECAR_TEMPLATE_STRUCTURE[section_key]
    
    # Calculate heading level
    base_level = SIDECAR_TEMPLATE_CONFIG["base_heading_level"]
    level_offset = section.get("level_offset", 0)
    heading_level = base_level + (level_offset * SIDECAR_TEMPLATE_CONFIG["heading_increment"])
    
    # Get heading prefix
    heading_prefix = get_heading_prefix(heading_level)
    
    # Build heading text
    section_text = section["text"]
    
    if section.get("has_link", False) and path:
        # Use provided alias or default to section text
        display_text = alias if alias is not None else section_text
        
        # Get link format
        if link_format in MD_LINK_FORMATS:
            format_info = MD_LINK_FORMATS[link_format]
            link_text = format_info['format'].format(name=display_text, path=path)
        else:
            # Fallback to wikilink format
            link_text = f"[[{path}|{display_text}]]"
        
        return f"{heading_prefix}{link_text}"
    else:
        return f"{heading_prefix}{section_text}"


def build_template_heading_regex(
    section_key: str,
    link_formats: Optional[list] = None
) -> str:
    """
    Build a regex pattern that matches template headings in various formats.
    
    Args:
        section_key: Key from SIDECAR_TEMPLATE_STRUCTURE
        link_formats: List of link format keys to support (default: all)
    
    Returns:
        Regex pattern string
    """
    if section_key not in SIDECAR_TEMPLATE_STRUCTURE:
        raise ValueError(f"Unknown section key: {section_key}")
    
    section = SIDECAR_TEMPLATE_STRUCTURE[section_key]
    section_text = section["text"]
    
    # Calculate heading level
    base_level = SIDECAR_TEMPLATE_CONFIG["base_heading_level"]
    level_offset = section.get("level_offset", 0)
    heading_level = base_level + (level_offset * SIDECAR_TEMPLATE_CONFIG["heading_increment"])
    
    # Get heading prefix and escape for regex
    heading_prefix = get_heading_prefix(heading_level)
    escaped_prefix = re.escape(heading_prefix)
    escaped_text = re.escape(section_text)
    
    # Build patterns
    patterns = [escaped_text]  # Plain section name
    
    if section.get("has_link", False):
        # Support all link formats if none specified
        if link_formats is None:
            link_formats = list(MD_LINK_FORMATS.keys())
        
        for format_key in link_formats:
            if format_key in MD_LINK_FORMATS:
                if format_key == 'MD_ANGLE_BRACKETS':
                    # For angle brackets: [Section Name](<path>)
                    patterns.append(rf"\[{escaped_text}\]\(<[^>]*>\)")
                elif format_key == 'MD_WIKILINK':
                    # For wikilinks: [[path|Section Name]]
                    patterns.append(rf"\[\[[^\]|]*\|{escaped_text}\]\]")
    
    # Combine all patterns
    combined_pattern = "|".join(patterns)
    return rf"{escaped_prefix}(?:{combined_pattern})"


def get_template_section_heading(section_key: str, **kwargs) -> str:
    """
    Convenience function to get a template section heading.
    
    Args:
        section_key: Key from SIDECAR_TEMPLATE_STRUCTURE
        **kwargs: Arguments passed to build_template_heading
    
    Returns:
        Formatted heading string
    """
    return build_template_heading(section_key, **kwargs)


def get_all_template_headings(path: Optional[str] = None) -> Dict[str, str]:
    """
    Get all template headings as a dictionary.
    
    Args:
        path: Optional path for sections that support links
    
    Returns:
        Dictionary mapping section keys to formatted headings
    """
    headings = {}
    
    for section_key in SIDECAR_TEMPLATE_STRUCTURE:
        section = SIDECAR_TEMPLATE_STRUCTURE[section_key]
        
        if section_key == "message_embed":
            # Special case - this is content, not a heading
            headings[section_key] = section["content"]
        else:
            # Build heading
            if section.get("has_link", False) and path:
                headings[section_key] = build_template_heading(section_key, path=path)
            else:
                headings[section_key] = build_template_heading(section_key)
    
    return headings


def update_template_config(base_level: Optional[int] = None, increment: Optional[int] = None):
    """
    Update the template configuration.
    
    Args:
        base_level: New base heading level
        increment: New heading increment
    """
    if base_level is not None:
        SIDECAR_TEMPLATE_CONFIG["base_heading_level"] = base_level
    if increment is not None:
        SIDECAR_TEMPLATE_CONFIG["heading_increment"] = increment


# Compiled regex patterns for performance (updated when config changes)
_COMPILED_REGEX_CACHE = {}

def get_compiled_template_regex(section_key: str, link_formats: Optional[list] = None) -> re.Pattern:
    """
    Get a compiled regex pattern for a template section.
    
    Args:
        section_key: Key from SIDECAR_TEMPLATE_STRUCTURE
        link_formats: List of link format keys to support
    
    Returns:
        Compiled regex pattern
    """
    cache_key = (section_key, tuple(link_formats) if link_formats else None)
    
    if cache_key not in _COMPILED_REGEX_CACHE:
        pattern = build_template_heading_regex(section_key, link_formats)
        _COMPILED_REGEX_CACHE[cache_key] = re.compile(pattern)
    
    return _COMPILED_REGEX_CACHE[cache_key]


def clear_regex_cache():
    """Clear the compiled regex cache (call when template config changes)."""
    global _COMPILED_REGEX_CACHE
    _COMPILED_REGEX_CACHE = {}


def format_link(text: str, path: str, format_type: str = "primary") -> str:
    """
    Format a link using the specified format type.
    
    Args:
        text: Display text for the link
        path: Path or target of the link
        format_type: Type of link format ("primary", "MD_WIKILINK", "MD_ANGLE_BRACKETS")
    
    Returns:
        Formatted link string
    """
    if format_type == "primary":
        return MD_PRIMARY_FORMAT['format'].format(name=text, path=path)
    elif format_type in MD_LINK_FORMATS:
        return MD_LINK_FORMATS[format_type]['format'].format(name=text, path=path)
    else:
        raise ValueError(f"Unknown format type: {format_type}")


def get_link_regex(format_type: str = "primary") -> str:
    """
    Get regex pattern for matching links of the specified format.
    
    Args:
        format_type: Type of link format to get regex for
    
    Returns:
        Regex pattern string
    """
    if format_type == "primary":
        return MD_PRIMARY_FORMAT['regex']
    elif format_type in MD_LINK_FORMATS:
        return MD_LINK_FORMATS[format_type]['regex']
    else:
        raise ValueError(f"Unknown format type: {format_type}")


def validate_template_change(new_config: dict) -> bool:
    """
    Validate a potential template configuration change.
    
    Args:
        new_config: Proposed new template configuration
    
    Returns:
        True if valid, False otherwise
    """
    try:
        base_level = new_config.get("base_heading_level", 1)
        if base_level < 1 or base_level > 6:
            return False
        
        # Check all sections would have valid heading levels
        for section in SIDECAR_TEMPLATE_STRUCTURE.values():
            if "level_offset" in section:
                final_level = base_level + section["level_offset"]
                if final_level < 1 or final_level > 6:
                    return False
        
        return True
    except (KeyError, TypeError, ValueError):
        return False


def get_template_info() -> dict:
    """
    Get comprehensive information about the current template configuration.
    
    Returns:
        Dictionary with template configuration details
    """
    return {
        "config": SIDECAR_TEMPLATE_CONFIG.copy(),
        "structure": SIDECAR_TEMPLATE_STRUCTURE.copy(),
        "available_sections": list(SIDECAR_TEMPLATE_STRUCTURE.keys()),
        "linkable_sections": [
            key for key, section in SIDECAR_TEMPLATE_STRUCTURE.items()
            if section.get("has_link", False)
        ],
        "generated_headings": get_all_template_headings()
    }


def parse_template_content(content: str) -> dict:
    """
    Parse sidecar content and identify template sections.
    
    Args:
        content: Raw sidecar markdown content
    
    Returns:
        Dictionary mapping section keys to their positions and content
    """
    sections = {}
    lines = content.split('\n')
    
    for i, line in enumerate(lines):
        for section_key in SIDECAR_TEMPLATE_STRUCTURE.keys():
            if section_key == "message_embed":
                continue  # Skip non-heading sections
            
            pattern = build_template_heading_regex(section_key)
            if re.match(pattern, line.strip()):
                sections[section_key] = {
                    "line_number": i + 1,
                    "line_content": line,
                    "matched_pattern": pattern
                }
                break
    
    return sections


def get_all_template_regexes() -> dict[str, str]:
    """
    Get regex patterns for all defined section headings.
    
    Returns:
        Dictionary mapping section keys to their regex patterns
    """
    return {
        section_key: build_template_heading_regex(section_key)
        for section_key in SIDECAR_TEMPLATE_STRUCTURE.keys()
        if section_key != "message_embed"  # Skip non-heading sections
    }


# Template examples and documentation
def get_template_examples() -> dict:
    """Get template examples for documentation and testing."""
    return {
        "basic_sidecar": f"""{build_template_heading("main_heading")}

{SIDECAR_TEMPLATE_STRUCTURE["message_embed"]["content"]}

{build_template_heading("current_file")}
- [[example.blend|example.blend]]

{build_template_heading("linked_libraries")}
{SIDECAR_NO_ITEMS}

{build_template_heading("resources")}
{SIDECAR_NO_ITEMS}
""",
        
        "linked_file_example": f"""{build_template_heading("main_heading")}

{SIDECAR_TEMPLATE_STRUCTURE["message_embed"]["content"]}

{build_template_heading("current_file", path="assets/models/character.blend", alias="Character Model")}
- [[assets/models/character.blend|Character Model]]

{build_template_heading("linked_libraries")}
- [[assets/materials/character_materials.blend|Character Materials]]
- [[assets/rigs/character_rig.blend|Character Rig]]

{build_template_heading("resources")}
**Textures**
- [[textures/character_diffuse.png|Diffuse Map]]
- [[textures/character_normal.png|Normal Map]]

**Scripts**
- [[scripts/character_controller.py|Character Controller]]
""",
    }


# Validation function to ensure template configuration is valid
def _validate_template_config():
    """Validate the template configuration."""
    base_level = SIDECAR_TEMPLATE_CONFIG.get("base_heading_level", 1)
    if base_level < 1 or base_level > 6:
        raise ValueError(f"Invalid base_heading_level: {base_level}. Must be between 1 and 6.")
    
    for section_key, section in SIDECAR_TEMPLATE_STRUCTURE.items():
        level_offset = section.get("level_offset", 0)
        final_level = base_level + level_offset
        if final_level < 1 or final_level > 6:
            raise ValueError(f"Section '{section_key}' would result in invalid heading level {final_level}")


# Validate configuration on import
_validate_template_config()


def get_main_section_heading_level() -> int:
    """
    Get the heading level used for main sections (current_file, linked_libraries, resources).
    
    Returns:
        Integer heading level (1-6)
    """
    # Main sections use level_offset = 1 in the template structure
    base_level = SIDECAR_TEMPLATE_CONFIG["base_heading_level"]
    level_offset = 1  # Main sections have level_offset of 1
    return base_level + (level_offset * SIDECAR_TEMPLATE_CONFIG["heading_increment"])


def get_main_section_heading_prefix() -> str:
    """
    Get the heading prefix (e.g., "## ") used for main sections.
    
    Returns:
        Heading prefix string
    """
    level = get_main_section_heading_level()
    return get_heading_prefix(level)


def build_main_section_break_regex() -> str:
    """
    Build a regex pattern that matches any heading at or above the main section level.
    This is used to detect section breaks when parsing sidecar content.
    
    Returns:
        Regex pattern string that matches section breaks
    """
    main_level = get_main_section_heading_level()
    
    # Build pattern that matches headings from level 1 up to main_level
    # For example, if main sections are level 2 (##), this matches # and ##
    patterns = []
    for level in range(1, main_level + 1):
        prefix = get_heading_prefix(level).strip()  # Remove trailing space
        escaped_prefix = re.escape(prefix)
        patterns.append(f"{escaped_prefix}\\s")  # Match prefix followed by space
    
    # Combine patterns with OR
    combined_pattern = "|".join(patterns)
    return f"^({combined_pattern})"
