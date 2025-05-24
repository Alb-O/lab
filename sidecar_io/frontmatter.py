from typing import List, Tuple

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


def _build_frontmatter_content(existing_lines: List[str], start_idx: int, end_idx: int, all_tags: List[str], format_type: str) -> List[str]:
	"""Build new frontmatter content, preserving non-tag fields and updating tags"""
	content_lines = []
	tags_written = False
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
		else:
			# Preserve non-tag content
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
	
	return content_lines


def generate_frontmatter_string(original_lines: List[str], configured_tags: List[str]) -> Tuple[str, int]:
	"""
	Generate frontmatter string with tags, preserving existing frontmatter structure.
	Returns:
		- The frontmatter string (with --- fences and newline), or "" if no content
		- The end line index of original frontmatter in original_lines (-1 if none)
	"""
	fm_end_idx = -1
	existing_tags = []
	format_type = 'list'  # default format
	
	# Check for existing frontmatter
	if original_lines and original_lines[0].strip() == "---":
		for i in range(1, len(original_lines)):
			if original_lines[i].strip() == "---":
				fm_end_idx = i
				break
		
		if fm_end_idx != -1:
			existing_tags, format_type = _extract_existing_tags(original_lines, 1, fm_end_idx)
	# Combine and sort all tags
	all_tags = sorted(set(existing_tags) | set(configured_tags))
	
	if not all_tags:
		return "", fm_end_idx
		# Build frontmatter content
	if fm_end_idx != -1:
		# Update existing frontmatter
		content_lines = _build_frontmatter_content(original_lines, 1, fm_end_idx, all_tags, format_type)
	else:
		# Create new frontmatter
		content_lines = [
			"tags:"
		] + [f"  - {tag}" for tag in all_tags]
	
	if not content_lines:
		return "", fm_end_idx
	
	frontmatter_lines = ["---"] + content_lines + ["---"]
	return "\n".join(frontmatter_lines) + "\n", fm_end_idx