from typing import List, Tuple, Optional
from ..utils.templates import format_link
from ..utils.constants import PREVIEW_ALIAS

def _extract_existing_tags(lines: List[str], start_idx: int, end_idx: int) -> Tuple[List[str], str]:
	"""Extract tags from frontmatter content between start and end indices
	Returns: (tags_list, format_type) where format_type is 'inline', 'bracket', or 'list'
	"""
	tags = []
	format_type = 'list'  # default
	in_tags_section = False
	
	for i in range(start_idx, end_idx):
		line = lines[i].strip()
		
		if line.startswith("tags:"):
			in_tags_section = True
			# Handle inline tags: "tags: [tag1, tag2]" or "tags: tag1, tag2"
			tag_content = line.split("tags:", 1)[1].strip()
			if tag_content:
				if tag_content.startswith('[') and tag_content.endswith(']'):
					format_type = 'bracket'
					tag_content = tag_content[1:-1]
				else:
					format_type = 'inline'
				for tag in tag_content.split(','):
					tag = tag.strip().strip('"\'')
					if tag:
						tags.append(tag)
		elif in_tags_section and line.startswith("- "):
			# Handle list format: "- tag"
			format_type = 'list'
			tag = line[2:].strip().strip('"\'')
			if tag:
				tags.append(tag)
		elif in_tags_section and line and not line.startswith(("  ", "-")):
			# End of tags section
			in_tags_section = False
	
	return tags, format_type


def _extract_existing_preview(lines: List[str], start_idx: int, end_idx: int) -> Optional[str]:
	"""Extract preview field from frontmatter content between start and end indices"""
	for i in range(start_idx, end_idx):
		line = lines[i].strip()
		if line.startswith("preview:"):
			preview_content = line.split("preview:", 1)[1].strip()
			# Remove quotes if present
			preview_content = preview_content.strip('"\'')
			return preview_content if preview_content else None
	return None


def _build_frontmatter_content(existing_lines: List[str], start_idx: int, end_idx: int, all_tags: List[str], format_type: str, preview_link: Optional[str] = None) -> List[str]:
	"""Build new frontmatter content, preserving non-tag/non-preview fields and updating tags and preview"""
	content_lines = []
	tags_written = False
	preview_written = False
	i = start_idx
	
	while i < end_idx:
		line = existing_lines[i].strip()
		
		if line.startswith("tags:"):
			# Replace tags section with preserved format
			if all_tags and not tags_written:
				if format_type == 'bracket':
					content_lines.append(f"tags: [{', '.join(all_tags)}]")
				elif format_type == 'inline':
					content_lines.append(f"tags: {', '.join(all_tags)}")
				else:  # list format
					content_lines.append("tags:")
					for tag in all_tags:
						content_lines.append(f"  - {tag}")
				tags_written = True
			
			# Skip old tags section
			i += 1
			while i < end_idx:
				next_line = existing_lines[i].strip()
				if next_line.startswith("- ") or next_line.startswith("  "):
					i += 1
				else:
					break
			continue
		elif line.startswith("preview:"):
			# Replace preview field
			if preview_link and not preview_written:
				formatted_preview = format_link(PREVIEW_ALIAS, preview_link, "primary")
				content_lines.append(f"preview: \"{formatted_preview}\"")
				preview_written = True
			# Skip old preview line
			i += 1
			continue
		else:
			# Preserve non-tag/non-preview content
			content_lines.append(existing_lines[i].rstrip())
			i += 1
	
	# Add tags if not written and we have tags
	if not tags_written and all_tags:
		if format_type == 'bracket':
			content_lines.append(f"tags: [{', '.join(all_tags)}]")
		elif format_type == 'inline':
			content_lines.append(f"tags: {', '.join(all_tags)}")
		else:  # list format (default)
			content_lines.append("tags:")
			for tag in all_tags:
				content_lines.append(f"  - {tag}")
		# Add preview if not written and we have a preview link
	if not preview_written and preview_link:
		formatted_preview = format_link(PREVIEW_ALIAS, preview_link, "primary")
		content_lines.append(f"preview: \"{formatted_preview}\"")
	
	return content_lines


def generate_frontmatter_string(original_lines: List[str], configured_tags: List[str], preview_link: Optional[str] = None) -> Tuple[str, int]:
	"""
	Generate frontmatter string with tags and preview, preserving existing frontmatter structure.
	Returns:
		- The frontmatter string (with --- fences and newline), or "" if no content
		- The end line index of original frontmatter in original_lines (-1 if none)
	"""
	fm_end_idx = -1
	existing_tags = []
	existing_preview = None
	format_type = 'list'  # default format
	
	# Check for existing frontmatter
	if original_lines and original_lines[0].strip() == "---":
		for i in range(1, len(original_lines)):
			if original_lines[i].strip() == "---":
				fm_end_idx = i
				break
		
		if fm_end_idx != -1:
			existing_tags, format_type = _extract_existing_tags(original_lines, 1, fm_end_idx)
			existing_preview = _extract_existing_preview(original_lines, 1, fm_end_idx)
	
	# Combine and sort all tags
	all_tags = sorted(set(existing_tags) | set(configured_tags))
	
	# Use provided preview_link or fall back to existing preview
	final_preview_link = preview_link if preview_link is not None else existing_preview
	
	# If no tags and no preview, return empty
	if not all_tags and not final_preview_link:
		return "", fm_end_idx
	
	# Build frontmatter content
	if fm_end_idx != -1:
		# Update existing frontmatter
		content_lines = _build_frontmatter_content(original_lines, 1, fm_end_idx, all_tags, format_type, final_preview_link)
	else:
		# Create new frontmatter
		content_lines = []
		if all_tags:
			content_lines.extend([
				"tags:"
			] + [f"  - {tag}" for tag in all_tags])
		if final_preview_link:
			formatted_preview = format_link(PREVIEW_ALIAS, final_preview_link, "primary")
			content_lines.append(f"preview: \"{formatted_preview}\"")
	
	if not content_lines:
		return "", fm_end_idx
	
	frontmatter_lines = ["---"] + content_lines + ["---"]
	return "\n".join(frontmatter_lines) + "\n", fm_end_idx