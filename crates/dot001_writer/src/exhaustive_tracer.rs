use crate::dna_provider::SeedDnaProvider;
use crate::emitter::BlockInjection;
use dot001_events::error::Result;
use dot001_events::{
    event::{Event, WriterEvent},
    prelude::*,
};
use dot001_parser::{BlendFile, ReadSeekSend};
use std::collections::{HashSet, VecDeque};
use std::fs::File;
use std::io::{Cursor, Read};

/// Experimental exhaustive pointer tracer for research purposes.
///
/// This attempts to follow all pointer references recursively to build complete
/// dependency closures for block injection. It frequently crashes and is not
/// suitable for practical use.
///
/// The implementation represents research into understanding Blender's internal
/// data structures and pointer relationships, but does not constitute functional
/// software for block injection.
pub struct ExhaustivePointerTracer;

impl ExhaustivePointerTracer {
    /// Create a complete block injection by tracing ALL pointer dependencies
    pub fn trace_complete_dependencies(
        seed: &mut SeedDnaProvider,
        root_indices: &[usize],
    ) -> Result<BlockInjection> {
        // Create a fresh BlendFile instance for tracing
        let mut file = File::open(seed.source_path())?;
        let mut buf = Vec::new();
        file.read_to_end(&mut buf)?;
        let cursor = Cursor::new(buf);
        let mut blend_file = BlendFile::new(Box::new(cursor) as Box<dyn ReadSeekSend>)?;

        // Emit dependency tracing started event
        emit_global_sync!(Event::Writer(WriterEvent::Started {
            operation: "exhaustive_dependency_tracing".to_string(),
            target_file: seed.source_path().to_path_buf(),
        }));

        println!("=== Exhaustive Pointer Tracing ===");
        println!("Starting from {} root blocks:", root_indices.len());
        for &index in root_indices {
            if let Some(block) = blend_file.get_block(index) {
                let code_str = String::from_utf8_lossy(&block.header.code);
                let code = code_str.trim_end_matches('\0');
                println!("  [{index}] {code}");
            }
        }

        // Trace all dependencies exhaustively
        let all_dependencies =
            Self::trace_all_pointer_references(&mut blend_file, seed.dna(), root_indices)?;

        println!(
            "\nComplete dependency closure: {} blocks",
            all_dependencies.len()
        );
        for &index in &all_dependencies {
            if let Some(block) = blend_file.get_block(index) {
                let code_str = String::from_utf8_lossy(&block.header.code);
                let code = code_str.trim_end_matches('\0');
                println!("  [{index}] {code}");
            }
        }

        // Convert to Vec and extract all blocks
        let indices_vec: Vec<usize> = all_dependencies.into_iter().collect();
        let extracted_blocks = seed.extract_blocks_by_indices(&indices_vec)?;

        // Create injection with complete address remapping
        Ok(BlockInjection::from_extracted_blocks_with_dna(
            extracted_blocks,
            seed.dna(),
        ))
    }

    /// Recursively trace ALL pointer references from the given root blocks
    fn trace_all_pointer_references(
        blend_file: &mut BlendFile<impl ReadSeekSend>,
        dna: &dot001_parser::DnaCollection,
        root_indices: &[usize],
    ) -> Result<HashSet<usize>> {
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();

        // Start with root blocks
        for &index in root_indices {
            queue.push_back(index);
        }

        println!("\nTracing pointer dependencies:");

        while let Some(block_index) = queue.pop_front() {
            if visited.contains(&block_index) {
                continue;
            }

            visited.insert(block_index);

            // Get block info first
            let (code, sdna_index) = if let Some(block) = blend_file.get_block(block_index) {
                let code_str = String::from_utf8_lossy(&block.header.code);
                let code = code_str.trim_end_matches('\0').to_string();
                (code, block.header.sdna_index)
            } else {
                continue;
            };

            println!("  Analyzing [{block_index}] {code}");

            // Read the block data
            let block_data = match blend_file.read_block_data(block_index) {
                Ok(data) => data,
                Err(e) => {
                    println!("    ⚠️ Failed to read block data: {e}");
                    continue;
                }
            };

            // Get the struct definition for this block
            let struct_def = match dna.get_struct(sdna_index as usize) {
                Some(def) => def,
                None => {
                    println!("    ⚠️ Unknown struct type: {sdna_index}");
                    continue;
                }
            };

            // Scan all pointer fields in this struct
            let mut found_pointers = 0;
            for field in &struct_def.fields {
                if field.name.is_pointer {
                    let pointer_deps = Self::extract_pointers_from_field(
                        &block_data,
                        field,
                        blend_file,
                        &format!("{}:{}", code, field.name.name_only),
                    );

                    found_pointers += pointer_deps.len();

                    // Add new dependencies to the queue
                    for dep_index in pointer_deps {
                        if !visited.contains(&dep_index) {
                            queue.push_back(dep_index);
                        }
                    }
                }
            }

            // Special handling for complex structures with internal linked lists
            if struct_def.type_name == "bNodeTree" {
                println!("    🔍 Special NodeTree analysis - checking internal linked lists");
                let internal_deps =
                    Self::analyze_node_tree_internals(&block_data, struct_def, blend_file);
                found_pointers += internal_deps.len();

                for dep_index in internal_deps {
                    if !visited.contains(&dep_index) {
                        queue.push_back(dep_index);
                        println!("      → Adding internal NodeTree dependency [{dep_index}]");
                    }
                }
            }

            println!("    → Found {found_pointers} pointer references");
        }

        println!(
            "Exhaustive tracing complete: {} total blocks",
            visited.len()
        );
        Ok(visited)
    }

    /// Extract all pointer values from a field (handling arrays and single pointers)
    fn extract_pointers_from_field(
        data: &[u8],
        field: &dot001_parser::DnaField,
        blend_file: &mut BlendFile<impl ReadSeekSend>,
        field_context: &str,
    ) -> Vec<usize> {
        let mut dependencies = Vec::new();
        let pointer_size = 8; // 64-bit pointers
        let array_size = field.name.array_size;

        // Handle arrays of pointers
        for i in 0..array_size {
            let offset = field.offset + (i * pointer_size);

            if offset + pointer_size <= data.len() {
                let ptr_val = u64::from_le_bytes([
                    data[offset],
                    data[offset + 1],
                    data[offset + 2],
                    data[offset + 3],
                    data[offset + 4],
                    data[offset + 5],
                    data[offset + 6],
                    data[offset + 7],
                ]);

                if ptr_val != 0 {
                    // Try to find the block this pointer references
                    if let Some(target_index) = blend_file.find_block_by_address(ptr_val) {
                        dependencies.push(target_index);

                        let target_code = blend_file
                            .get_block(target_index)
                            .map(|b| {
                                String::from_utf8_lossy(&b.header.code)
                                    .trim_end_matches('\0')
                                    .to_string()
                            })
                            .unwrap_or_else(|| "?".to_string());

                        println!(
                            "      {field_context} → [{target_index}] {target_code} (0x{ptr_val:x})"
                        );
                    } else {
                        // Pointer doesn't reference a valid block - this is normal for some cases
                        println!("      {field_context} → 0x{ptr_val:x} (external/invalid)");
                    }
                }
            }
        }

        dependencies
    }

    /// Special handling for ListBase structures (first/last pointer pairs)
    fn extract_listbase_dependencies(
        data: &[u8],
        offset: usize,
        blend_file: &mut BlendFile<impl ReadSeekSend>,
        field_name: &str,
    ) -> Vec<usize> {
        let mut dependencies = Vec::new();

        if offset + 16 <= data.len() {
            // ListBase is 16 bytes (2 pointers)
            // First pointer
            let first_ptr = u64::from_le_bytes([
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
                data[offset + 4],
                data[offset + 5],
                data[offset + 6],
                data[offset + 7],
            ]);

            // Last pointer (for validation, not currently used in traversal)
            let _last_ptr = u64::from_le_bytes([
                data[offset + 8],
                data[offset + 9],
                data[offset + 10],
                data[offset + 11],
                data[offset + 12],
                data[offset + 13],
                data[offset + 14],
                data[offset + 15],
            ]);

            // Follow the linked list from first to last
            if first_ptr != 0 {
                dependencies.extend(Self::trace_linked_list(blend_file, first_ptr, field_name));
            }
        }

        dependencies
    }

    /// Trace a linked list starting from the given pointer
    fn trace_linked_list(
        blend_file: &mut BlendFile<impl ReadSeekSend>,
        start_ptr: u64,
        list_name: &str,
    ) -> Vec<usize> {
        let mut dependencies = Vec::new();
        let mut current_ptr = start_ptr;
        let mut node_count = 0;

        println!("        Tracing linked list '{list_name}' from 0x{start_ptr:x}");

        while current_ptr != 0 && node_count < 1000 {
            // Safety limit
            if let Some(node_index) = blend_file.find_block_by_address(current_ptr) {
                dependencies.push(node_index);
                node_count += 1;

                let node_code = blend_file
                    .get_block(node_index)
                    .map(|b| {
                        String::from_utf8_lossy(&b.header.code)
                            .trim_end_matches('\0')
                            .to_string()
                    })
                    .unwrap_or_else(|| "?".to_string());

                println!("          [{node_index}] {node_code} at 0x{current_ptr:x}");

                // Try to find the 'next' pointer to continue the list
                if let Ok(node_data) = blend_file.read_block_data(node_index) {
                    // For nodes, the 'next' pointer is typically at the beginning of the struct
                    // Most linked list nodes have 'next' as the first field
                    if node_data.len() >= 8 {
                        current_ptr = u64::from_le_bytes([
                            node_data[0],
                            node_data[1],
                            node_data[2],
                            node_data[3],
                            node_data[4],
                            node_data[5],
                            node_data[6],
                            node_data[7],
                        ]);

                        if current_ptr != 0 {
                            println!("          → Following 'next' pointer to 0x{current_ptr:x}");
                        }
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            } else {
                println!("          0x{current_ptr:x} not found in blocks");
                break;
            }
        }

        if node_count > 0 {
            println!("        → Found {node_count} nodes in linked list");
        }

        dependencies
    }

    /// Analyze NodeTree internal structures (nodes, links, etc.)
    fn analyze_node_tree_internals(
        data: &[u8],
        struct_def: &dot001_parser::DnaStruct,
        blend_file: &mut BlendFile<impl ReadSeekSend>,
    ) -> Vec<usize> {
        let mut dependencies = Vec::new();

        // Key NodeTree fields that contain linked lists
        let listbase_fields = ["nodes", "links", "inputs", "outputs"];

        for field_name in listbase_fields {
            if let Some(field) = struct_def
                .fields
                .iter()
                .find(|f| f.name.name_only == field_name)
            {
                println!("      Checking ListBase '{field_name}'");
                let list_deps =
                    Self::extract_listbase_dependencies(data, field.offset, blend_file, field_name);
                dependencies.extend(list_deps);
            }
        }

        dependencies
    }
}
