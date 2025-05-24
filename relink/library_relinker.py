import bpy  # type: ignore
import os
import json
import re
import traceback
from utils import (
	SIDECAR_EXTENSION,
	LOG_COLORS,
	BV_UUID_PROP,
	BV_FILE_UUID_KEY,
	BV_UUID_KEY,
	MD_PRIMARY_FORMAT
)

@bpy.app.handlers.persistent
def relink_library_info(*args, **kwargs):
	"""Relinks libraries based on information in the sidecar Markdown file."""
	if not bpy.data.is_saved:
		print(f"{LOG_COLORS['WARN']}[Blend Vault][LibraryRelink] Current .blend file is not saved. Cannot process sidecar.{LOG_COLORS['RESET']}")
		return

	blend_path = bpy.data.filepath
	md_path = blend_path + SIDECAR_EXTENSION

	if not os.path.exists(md_path):
		print(f"{LOG_COLORS['WARN']}[Blend Vault][LibraryRelink] Sidecar file not found: {md_path}{LOG_COLORS['RESET']}")
		return

	print(f"{LOG_COLORS['INFO']}[Blend Vault][LibraryRelink] Processing sidecar file: {md_path}{LOG_COLORS['RESET']}")
	
	found_any_link_to_process = False
	linked_libraries_header_idx = -1
	
	try:
		with open(md_path, 'r', encoding='utf-8') as f:
			lines = f.readlines()

		# Find the "### Linked Libraries" section
		for i, line in enumerate(lines):
			if line.strip() == "### Linked Libraries":
				linked_libraries_header_idx = i
				break
		
		if linked_libraries_header_idx == -1:
			print(f"{LOG_COLORS['INFO']}[Blend Vault][LibraryRelink] '### Linked Libraries' section not found in {md_path}.{LOG_COLORS['RESET']}")
			return

		parsing_json_block = False
		json_accumulator = []
		active_md_link_name_for_log = None # Stores the display name from the MD link [name](path)
		active_md_link_path = None       # Stores the path from the Markdown link
		
		current_line_idx = linked_libraries_header_idx + 1
		while current_line_idx < len(lines):
			line_raw = lines[current_line_idx]
			line_stripped = line_raw.strip()

			if parsing_json_block:
				if line_stripped == "```": # End of JSON block
					parsing_json_block = False
					json_str = "".join(json_accumulator)
					json_accumulator = []
					
					current_link_name_for_processing = active_md_link_name_for_log 

					if not current_link_name_for_processing:
						print(f"{LOG_COLORS['ERROR']}[Blend Vault][LibraryRelink] ERROR: Ended JSON block but no active Markdown link context was found. JSON: {json_str[:100]}...{LOG_COLORS['RESET']}")
					elif not json_str.strip():
						print(f"{LOG_COLORS['INFO']}[Blend Vault][LibraryRelink] Empty JSON block found for '{current_link_name_for_processing}'. Skipping.{LOG_COLORS['RESET']}")
					else:
						try:
							data = json.loads(json_str)
							stored_path_from_json = data.get("path")
							stored_blendfile_hash = data.get(BV_UUID_KEY)
							
							if stored_path_from_json and stored_blendfile_hash and stored_blendfile_hash != "MISSING_HASH":
								print(f"{LOG_COLORS['INFO']}[Blend Vault][LibraryRelink] Processing entry: Path='{stored_path_from_json}', Blendfile Hash='{stored_blendfile_hash}' (from MD link '{current_link_name_for_processing}'){LOG_COLORS['RESET']}")
								found_any_link_to_process = True
								# Prefer the path from the Markdown link above the JSON block
								target_rel_path = active_md_link_path or stored_path_from_json
								rel_path = '//' + target_rel_path

								found_matching_lib = False
								for lib in bpy.data.libraries:
									lib_prop_val = lib.get(BV_UUID_PROP)
									actual_lib_identifier = None
									if lib_prop_val:
										try:
											parsed_lib_prop = json.loads(lib_prop_val)
											if isinstance(parsed_lib_prop, dict):
												actual_lib_identifier = parsed_lib_prop.get(BV_FILE_UUID_KEY)
											elif isinstance(parsed_lib_prop, str): 
												actual_lib_identifier = parsed_lib_prop
										except json.JSONDecodeError:
											actual_lib_identifier = lib_prop_val 
								
									if actual_lib_identifier and actual_lib_identifier == stored_blendfile_hash:
										found_matching_lib = True
										print(f"{LOG_COLORS['INFO']}[Blend Vault][LibraryRelink] Found library '{lib.name}' with matching Blend Vault ID: {actual_lib_identifier}{LOG_COLORS['RESET']}")
										lib_path_norm = lib.filepath.replace('\\', '/').lstrip('//') # Corrected slash replacement
										if lib_path_norm != target_rel_path:
											print(f"{LOG_COLORS['INFO']}[Blend Vault] Relinking '{lib.name}' from '{lib.filepath}' -> '{rel_path}'{LOG_COLORS['RESET']}")
											lib.filepath = rel_path
											try:
												lib.reload() 
												print(f"{LOG_COLORS['SUCCESS']}[Blend Vault][LibraryRelink] Successfully reloaded library '{lib.name}'.{LOG_COLORS['RESET']}")
											except Exception as e:
												print(f"{LOG_COLORS['ERROR']}[Blend Vault][LibraryRelink] Failed to reload '{lib.name}' after path update: {e}{LOG_COLORS['RESET']}")
										else:
											print(f"{LOG_COLORS['INFO']}[Blend Vault][LibraryRelink] Path for '{lib.name}' ('{lib.filepath}') already matches stored relative path ('{rel_path}').{LOG_COLORS['RESET']}")
										break 
								
								if not found_matching_lib:
									print(f"{LOG_COLORS['INFO']}[Blend Vault][LibraryRelink] Library with Blend Vault ID {stored_blendfile_hash} not found. Attempting to relink existing library by filename.{LOG_COLORS['RESET']}")
									# Try to find an existing loaded library whose filename matches the Markdown link name
									md_basename = os.path.basename(active_md_link_path or stored_path_from_json)
									relinked_by_name = False
									for lib_match in bpy.data.libraries:
										if os.path.basename(lib_match.filepath) == md_basename:
											print(f"{LOG_COLORS['INFO']}[Blend Vault][LibraryRelink] Found existing library entry '{lib_match.name}' matching filename '{md_basename}'. Relinking to '{rel_path}'{LOG_COLORS['RESET']}")
											lib_match.filepath = rel_path
											try:
												lib_match.reload() 
												print(f"{LOG_COLORS['SUCCESS']}[Blend Vault][LibraryRelink] Successfully reloaded library '{lib_match.name}' (name match).{LOG_COLORS['RESET']}")
												relinked_by_name = True
												found_any_link_to_process = True
											except Exception as e:
												print(f"{LOG_COLORS['ERROR']}[Blend Vault][LibraryRelink] Failed to reload '{lib_match.name}' after name-based relink: {e}{LOG_COLORS['RESET']}")
											break
									if relinked_by_name:
										active_md_link_name_for_log = None
										current_line_idx += 1
										continue
									print(f"{LOG_COLORS['INFO']}[Blend Vault][LibraryRelink] No existing library matched by filename '{md_basename}'. Loading new library.{LOG_COLORS['RESET']}" )
									working_dir = os.path.dirname(bpy.data.filepath)
									candidate_abs_path = os.path.normpath(os.path.join(working_dir, active_md_link_path or stored_path_from_json))
									
									relinked_or_loaded_by_path = False
									for lib_to_fix in bpy.data.libraries:
										is_missing = False
										if hasattr(lib_to_fix, 'is_missing'): 
											is_missing = lib_to_fix.is_missing
										else: 
											abs_lib_path = bpy.path.abspath(lib_to_fix.filepath)
											if not os.path.exists(abs_lib_path):
												is_missing = True
										
										if is_missing and current_link_name_for_processing: 
											lib_to_fix_name_no_ext, _ = os.path.splitext(lib_to_fix.name)
											md_link_name_str = str(current_link_name_for_processing)
											md_link_name_no_ext, _ = os.path.splitext(md_link_name_str)

											if lib_to_fix_name_no_ext == md_link_name_no_ext:
												print(f"{LOG_COLORS['INFO']}[Blend Vault][LibraryRelink] Found a missing library entry '{lib_to_fix.name}' (matching MD link name '{current_link_name_for_processing}'). Updating its path from '{lib_to_fix.filepath}' to '{rel_path}'.{LOG_COLORS['RESET']}")
												lib_to_fix.filepath = rel_path 
												try:
													lib_to_fix.reload() 
													print(f"{LOG_COLORS['SUCCESS']}[Blend Vault][LibraryRelink] Successfully reloaded library '{lib_to_fix.name}' at new path: {rel_path}{LOG_COLORS['RESET']}")
													relinked_or_loaded_by_path = True
												except Exception as e:
													print(f"{LOG_COLORS['ERROR']}[Blend Vault][LibraryRelink] Failed to reload library '{lib_to_fix.name}' after path update to {rel_path}: {e}{LOG_COLORS['RESET']}")
												break 
											else:
												print(f"{LOG_COLORS['INFO']}[Blend Vault][LibraryRelink] Skipping missing library '{lib_to_fix.name}' as its name does not match MD link '{current_link_name_for_processing}'.{LOG_COLORS['RESET']}")

									if not relinked_or_loaded_by_path: 
										first_broken_lib_candidate = None
										for lib_candidate in bpy.data.libraries:
											is_missing_candidate = False
											if hasattr(lib_candidate, 'is_missing'): is_missing_candidate = lib_candidate.is_missing
											else: 
												if not os.path.exists(bpy.path.abspath(lib_candidate.filepath)): is_missing_candidate = True
											
											if is_missing_candidate:
												first_broken_lib_candidate = lib_candidate
												break
										
										if first_broken_lib_candidate:
											print(f"{LOG_COLORS['INFO']}[Blend Vault][LibraryRelink] No specific missing library matched by name. Attempting to use first available missing library entry '{first_broken_lib_candidate.name}' for path '{rel_path}'.{LOG_COLORS['RESET']}")
											first_broken_lib_candidate.filepath = rel_path
											try:
												first_broken_lib_candidate.reload() 
												print(f"{LOG_COLORS['SUCCESS']}[Blend Vault][LibraryRelink] Successfully reloaded library '{first_broken_lib_candidate.name}' at new path: {rel_path}{LOG_COLORS['RESET']}")
												relinked_or_loaded_by_path = True
											except Exception as e:
												print(f"{LOG_COLORS['ERROR']}[Blend Vault][LibraryRelink] Failed to reload library '{first_broken_lib_candidate.name}' using path {rel_path}: {e}{LOG_COLORS['RESET']}")

									if not relinked_or_loaded_by_path: 
										if os.path.exists(candidate_abs_path):
											print(f"{LOG_COLORS['INFO']}[Blend Vault][LibraryRelink] Attempting to load missing library using Markdown link path: {rel_path}{LOG_COLORS['RESET']}")
											try:
												with bpy.data.libraries.load(candidate_abs_path, link=True) as (data_from, data_to):
													pass 
												print(f"{LOG_COLORS['SUCCESS']}[Blend Vault][LibraryRelink] Successfully linked new library from {rel_path}{LOG_COLORS['RESET']}")
											except RuntimeError as rte:
												print(f"{LOG_COLORS['ERROR']}[Blend Vault][LibraryRelink] Runtime error linking new library from {rel_path}: {rte}{LOG_COLORS['RESET']}")
											except Exception as e:
												print(f"{LOG_COLORS['ERROR']}[Blend Vault][LibraryRelink] Failed to link new library from {rel_path}: {e}{LOG_COLORS['RESET']}")
										else:
											print(f"{LOG_COLORS['WARN']}[Blend Vault][LibraryRelink] Sidecar path '{stored_path_from_json}' (resolved to '{candidate_abs_path}') does not exist. Cannot link.{LOG_COLORS['RESET']}")
							elif stored_blendfile_hash == "MISSING_HASH":
								print(f"{LOG_COLORS['INFO']}[Blend Vault][LibraryRelink] Entry for '{current_link_name_for_processing}' has 'MISSING_HASH'. Skipping relink by hash.{LOG_COLORS['RESET']}")
							else: 
								print(f"{LOG_COLORS['WARN']}[Blend Vault][LibraryRelink] Invalid data in JSON block for '{current_link_name_for_processing}': Missing path or UUID info.{LOG_COLORS['RESET']}")
						
						except json.JSONDecodeError as jde:
							error_msg = jde.msg
							error_line_in_json = jde.lineno 
							error_col_in_json = jde.colno 
							print(f"{LOG_COLORS['ERROR']}[Blend Vault][LibraryRelink] Failed to parse JSON for '{current_link_name_for_processing}'.{LOG_COLORS['RESET']}")
							print(f"{LOG_COLORS['INFO']}[Blend Vault][LibraryRelink] JSONDecodeError: {error_msg} (at line {error_line_in_json}, column {error_col_in_json} of the collected JSON string).{LOG_COLORS['RESET']}")
							print(f"{LOG_COLORS['INFO']}[Blend Vault][LibraryRelink] Collected JSON string that failed was:\n>>>>\n{json_str}\n<<<<{LOG_COLORS['RESET']}")
					
					active_md_link_name_for_log = None 
				else:
					json_accumulator.append(line_raw) 
			
			elif line_stripped.startswith("```json"): 
				if active_md_link_name_for_log is None:
					print(f"{LOG_COLORS['WARN']}[Blend Vault][LibraryRelink] Found ```json block but no preceding Markdown link was active. Skipping this JSON block.{LOG_COLORS['RESET']}")
					while current_line_idx + 1 < len(lines) and lines[current_line_idx + 1].strip() != "```":
						current_line_idx += 1
						if lines[current_line_idx].strip().startswith("###") or lines[current_line_idx].strip().startswith("## "): 
							break
					if current_line_idx + 1 < len(lines) and lines[current_line_idx + 1].strip() == "```": 
						current_line_idx +=1 
				else:
					parsing_json_block = True
					json_accumulator = []
			
			# Correctly identify section breaks to avoid premature termination for '####' prefixed links.
			# A new section is ## followed by non-#, or ### followed by non-#.
			elif re.match(r"^(##[^#]|###[^#])", line_stripped):
				if parsing_json_block:
					print(f"{LOG_COLORS['WARN']}[Blend Vault][LibraryRelink] Warning: Encountered new header while still parsing JSON for '{active_md_link_name_for_log}'. Discarding partial JSON.{LOG_COLORS['RESET']}")
					parsing_json_block = False
					json_accumulator = []
					active_md_link_name_for_log = None
				break 

			else: 
				# Match new format using MD_PRIMARY_FORMAT
				# Use re.search to find link pattern anywhere in the line, accommodating prefixes like '####'
				# Remove heading if present before matching
				line_no_heading = line_stripped.lstrip('#').strip() if line_stripped.startswith('#') else line_stripped
				md_link_match = re.search(MD_PRIMARY_FORMAT['regex'], line_no_heading)
				if md_link_match:
					if active_md_link_name_for_log and not parsing_json_block: 
						print(f"{LOG_COLORS['WARN']}[Blend Vault][LibraryRelink] Warning: MD link for '{active_md_link_name_for_log}' wasn't followed by JSON before new link for '{md_link_match.group(1)}'.{LOG_COLORS['RESET']}")
					
					active_md_link_name_for_log = md_link_match.group(1)
					active_md_link_path = md_link_match.group(2)
					print(f"{LOG_COLORS['INFO']}[Blend Vault][LibraryRelink] Found Markdown link for library: {active_md_link_name_for_log} -> {active_md_link_path}{LOG_COLORS['RESET']}")
			
			current_line_idx += 1
		
		if parsing_json_block: 
			print(f"{LOG_COLORS['WARN']}[Blend Vault][LibraryRelink] Warning: Reached end of 'Linked Libraries' section while still parsing JSON for '{active_md_link_name_for_log}'. Discarding partial JSON.{LOG_COLORS['RESET']}")

	except Exception as e:
		print(f"{LOG_COLORS['ERROR']}[Blend Vault][LibraryRelink] An error occurred during the relinking process: {e}{LOG_COLORS['RESET']}")
		traceback.print_exc()

	if not found_any_link_to_process and linked_libraries_header_idx != -1:
		print(f"{LOG_COLORS['INFO']}[Blend Vault][LibraryRelink] No valid library entries were processed from the sidecar file: {md_path}{LOG_COLORS['RESET']}")
	
	try:
		bpy.ops.file.make_paths_relative()
		print(f"{LOG_COLORS['SUCCESS']}[Blend Vault][LibraryRelink] Made all external file paths relative.{LOG_COLORS['RESET']}")
	except RuntimeError as e:
		print(f"{LOG_COLORS['WARN']}[Blend Vault][LibraryRelink] Could not make paths relative: {e}. (This may happen if the file is not saved or has no external links).{LOG_COLORS['RESET']}")
	except Exception as e:
		print(f"{LOG_COLORS['ERROR']}[Blend Vault][LibraryRelink] Error making paths relative: {e}{LOG_COLORS['RESET']}")

	print(f"{LOG_COLORS['INFO']}[Blend Vault][LibraryRelink] Finished relink attempt.{LOG_COLORS['RESET']}")

relink_library_info.persistent = True


class BV_OT_RelinkLibraries(bpy.types.Operator):
	"""Operator to relink libraries based on sidecar file"""
	bl_idname = "blend_vault.relink_libraries"
	bl_label = "Relink Libraries from Sidecar"
	bl_options = {'REGISTER', 'UNDO'}

	sidecar_file_path: bpy.props.StringProperty(  # type: ignore
		name="Sidecar File Path",
		description="Path to the sidecar file containing library information",
		default="",
		subtype='FILE_PATH',
	)

	def execute(self, context: bpy.types.Context):
		if not self.sidecar_file_path:
			self.report({'ERROR'}, f"{LOG_COLORS['ERROR']}Sidecar file path not provided.{LOG_COLORS['RESET']}")
			return {'CANCELLED'}

		if not os.path.exists(self.sidecar_file_path):
			self.report({'ERROR'}, f"{LOG_COLORS['ERROR']}Sidecar file not found: {self.sidecar_file_path}{LOG_COLORS['RESET']}")
			return {'CANCELLED'}

		print(f"{LOG_COLORS['INFO']}Attempting to relink libraries from: {self.sidecar_file_path}{LOG_COLORS['RESET']}")
		return relink_library_info(self.sidecar_file_path)

	def invoke(self, context: bpy.types.Context, event):
		context.window_manager.fileselect_add(self)
		return {'RUNNING_MODAL'}


def register():
	bpy.utils.register_class(BV_OT_RelinkLibraries)
	print(f"{LOG_COLORS['SUCCESS']}[Blend Vault] Relinking operator registered.{LOG_COLORS['RESET']}")


def unregister():
	bpy.utils.unregister_class(BV_OT_RelinkLibraries)
	print(f"{LOG_COLORS['WARN']}[Blend Vault] Relinking operator unregistered.{LOG_COLORS['RESET']}")


if __name__ == "__main__":
	register()
