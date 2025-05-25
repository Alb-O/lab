"""
UUID management utilities for Blend Vault.
Handles reading and extracting UUIDs from sidecar files.
"""

import os
import re
from typing import Optional
from utils import BV_FILE_UUID_KEY, BV_UUID_KEY


def extract_uuid_from_content(content: str) -> Optional[str]:
	"""Extract UUID from sidecar content, trying both key formats."""
	for key in [BV_FILE_UUID_KEY, BV_UUID_KEY]:
		match = re.search(rf'"{key}"\s*:\s*"([^"]+)"', content)
		if match:
			return match.group(1)
	return None


def read_sidecar_uuid(sidecar_path: str) -> Optional[str]:
	"""Read UUID from sidecar file."""
	if not os.path.exists(sidecar_path):
		return None
	
	try:
		with open(sidecar_path, 'r', encoding='utf-8') as f:
			return extract_uuid_from_content(f.read())
	except Exception:
		return None
