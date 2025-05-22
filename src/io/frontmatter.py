\
from typing import List, Set, Tuple

def parse_existing_frontmatter(original_lines: List[str]) -> Tuple[Set[str], int, List[str]]:
    """
    Parses existing frontmatter from a list of lines.
    Returns:
        - A set of existing tags.
        - The line index of the closing '---' in original_lines (or -1 if no valid frontmatter).
        - A list of the raw lines that constituted the frontmatter block (between '---' fences).
    """
    existing_tags: Set[str] = set()
    frontmatter_end_line_idx = -1
    frontmatter_content_lines: List[str] = []

    if not original_lines or original_lines[0].strip() != "---":
        return existing_tags, frontmatter_end_line_idx, frontmatter_content_lines

    try:
        for i in range(1, len(original_lines)):
            if original_lines[i].strip() == "---":
                frontmatter_end_line_idx = i
                break
        
        if frontmatter_end_line_idx == -1: # No closing '---'
            return existing_tags, -1, frontmatter_content_lines

        frontmatter_content_lines = original_lines[1:frontmatter_end_line_idx]
        is_tags_section = False
        for line_content_fm_raw in frontmatter_content_lines:
            line_content_fm_stripped = line_content_fm_raw.strip()
            if line_content_fm_stripped.startswith("tags:"):
                is_tags_section = True
                tag_value_str = line_content_fm_stripped.split("tags:", 1)[1].strip()
                if tag_value_str:
                    if tag_value_str.startswith('[') and tag_value_str.endswith(']'): # Handle JSON-like array
                        tag_value_str = tag_value_str[1:-1]
                    for t in tag_value_str.split(','):
                        cleaned_tag = t.strip()
                        if cleaned_tag:
                            existing_tags.add(cleaned_tag)
            elif is_tags_section and line_content_fm_stripped.startswith("- "):
                existing_tags.add(line_content_fm_stripped[2:].strip())
            elif is_tags_section and not (line_content_fm_stripped.startswith("  ") or line_content_fm_stripped.startswith("- ")):
                is_tags_section = False # End of current tags section
    except IndexError: # Should not happen with valid line list
        return set(), -1, [] # Malformed, treat as no valid FM
    
    return existing_tags, frontmatter_end_line_idx, frontmatter_content_lines


def reconstruct_frontmatter_internal_lines(
    original_fm_content_lines: List[str], 
    all_tags_to_write: List[str] # Already sorted and unique
    ) -> List[str]:
    """
    Reconstructs the internal lines of the frontmatter, replacing or adding the tags block.
    original_fm_content_lines: Lines *between* the '---' fences from original FM.
    all_tags_to_write: Final list of sorted, unique tags.
    Returns: List of strings, representing lines *between* '---' fences.
    """
    reconstructed_lines: List[str] = []
    tags_block_written = False
    
    i = 0
    while i < len(original_fm_content_lines):
        line_raw = original_fm_content_lines[i]
        line_stripped = line_raw.strip()

        if line_stripped.startswith("tags:") and not tags_block_written:
            if all_tags_to_write:
                reconstructed_lines.append("tags:")
                for tag in all_tags_to_write:
                    reconstructed_lines.append(f"  - {tag}")
            tags_block_written = True
            
            # Advance 'i' past the old "tags:" line itself
            i += 1 
            # Skip subsequent lines if they are part of the old tags list's values
            while i < len(original_fm_content_lines):
                current_line_in_old_tags_raw = original_fm_content_lines[i]
                if current_line_in_old_tags_raw.lstrip().startswith("- "):
                    i += 1 # Skip this old tag item line
                else:
                    break # This line is not an old tag item, so the old tags block has ended.
            continue # Continue the outer while loop
        else:
            reconstructed_lines.append(line_raw.rstrip('\\r\\n'))
            i += 1
    
    if not tags_block_written and all_tags_to_write:
        reconstructed_lines.append("tags:")
        for tag in all_tags_to_write:
            reconstructed_lines.append(f"  - {tag}")
            
    return reconstructed_lines

def generate_frontmatter_string(
    original_lines: List[str], 
    configured_frontmatter_tags: List[str]
    ) -> Tuple[str, int]:
    """
    Main function to generate the new frontmatter string.
    Returns:
        - The new frontmatter string (including --- fences and trailing newline), or "" if no FM.
        - The end line index of the original frontmatter block in original_lines.
    """
    existing_tags, fm_end_line_idx, original_fm_content_lines = parse_existing_frontmatter(original_lines)
    
    final_tags_set = existing_tags.union(set(configured_frontmatter_tags))
    sorted_final_tags = sorted(list(final_tags_set))

    new_fm_internal_lines: List[str] = []
    if fm_end_line_idx != -1: # Existing valid frontmatter
        new_fm_internal_lines = reconstruct_frontmatter_internal_lines(original_fm_content_lines, sorted_final_tags)
    elif sorted_final_tags: # No existing FM, but tags to write
        new_fm_internal_lines.append("tags:")
        for tag in sorted_final_tags:
            new_fm_internal_lines.append(f"  - {tag}")
            
    if not new_fm_internal_lines: # No content for frontmatter
        return "", fm_end_line_idx # Return original fm_end_line_idx for content splitting

    final_fm_lines = ["---"] + new_fm_internal_lines + ["---"]
    return "\\n".join(final_fm_lines) + "\\n", fm_end_line_idx
