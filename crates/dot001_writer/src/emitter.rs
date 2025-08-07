use crate::dna_provider::SeedDnaProvider;
use crate::header_writer::HeaderWriter;
use dot001_error::{Dot001Error, Result};
use dot001_events::{
    event::{Event, WriterEvent},
    prelude::*,
};
use std::fs::File;
use std::io::{BufWriter, Write};

/// Represents a block to be injected into the output file.
#[derive(Debug, Clone)]
pub struct InjectableBlock {
    pub code: [u8; 4],
    pub sdna_index: u32,
    pub old_address: u64,
    pub data: Vec<u8>,
    pub count: u32,
}

/// Collection of blocks to inject into the output file.
#[derive(Debug, Default)]
pub struct BlockInjection {
    pub blocks: Vec<InjectableBlock>,
    /// Maps old addresses to new addresses for injected blocks
    pub address_map: std::collections::HashMap<u64, u64>,
}

impl BlockInjection {
    /// Create a new empty block injection.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a block from extracted block data.
    pub fn add_block(&mut self, header: &dot001_parser::BlockHeader, data: Vec<u8>) {
        self.blocks.push(InjectableBlock {
            code: header.code,
            sdna_index: header.sdna_index,
            old_address: header.old_address,
            data,
            count: header.count,
        });
    }

    /// Create an injection from a set of extracted blocks with address remapping.
    pub fn from_extracted_blocks(
        blocks: Vec<(usize, dot001_parser::BlockHeader, Vec<u8>)>,
    ) -> Self {
        let mut injection = Self::new();

        // First pass: assign new addresses and build the mapping
        let mut current_address = 0x1000u64; // Start from a safe base address
        for (_index, header, _data) in &blocks {
            injection
                .address_map
                .insert(header.old_address, current_address);
            current_address += 0x1000; // Space addresses apart
        }

        // Second pass: add blocks with remapped addresses and updated pointers
        for (_index, header, data) in blocks {
            let new_address = injection.address_map[&header.old_address];
            let remapped_data = data; // For now, skip pointer remapping until we have DNA access

            injection.blocks.push(InjectableBlock {
                code: header.code,
                sdna_index: header.sdna_index,
                old_address: new_address, // Use remapped address
                data: remapped_data,
                count: header.count,
            });
        }
        injection
    }

    /// Create an injection with DNA-aware pointer remapping.
    pub fn from_extracted_blocks_with_dna(
        blocks: Vec<(usize, dot001_parser::BlockHeader, Vec<u8>)>,
        dna: &dot001_parser::DnaCollection,
    ) -> Self {
        let mut injection = Self::new();

        // First pass: assign new addresses and build the mapping
        let mut current_address = 0x1000u64; // Start from a safe base address
        for (_index, header, _data) in &blocks {
            injection
                .address_map
                .insert(header.old_address, current_address);
            current_address += 0x1000; // Space addresses apart
        }

        // Second pass: add blocks with remapped addresses and updated pointers
        for (_index, header, data) in blocks {
            let new_address = injection.address_map[&header.old_address];
            let remapped_data = Self::remap_pointers_in_data(
                data,
                &injection.address_map,
                dna,
                header.sdna_index as usize,
            );

            injection.blocks.push(InjectableBlock {
                code: header.code,
                sdna_index: header.sdna_index,
                old_address: new_address, // Use remapped address
                data: remapped_data,
                count: header.count,
            });
        }

        // Third pass: special post-processing for linked list structures
        // This ensures that linked list nodes (bNode, bNodeSocket, etc.) have proper next/prev pointers
        Self::post_process_linked_list_structures(&mut injection, dna);

        injection
    }

    /// Remap pointers within block data using the address mapping.
    /// Includes special handling for ListBase structures following Blender's approach.
    fn remap_pointers_in_data(
        mut data: Vec<u8>,
        address_map: &std::collections::HashMap<u64, u64>,
        dna: &dot001_parser::DnaCollection,
        struct_index: usize,
    ) -> Vec<u8> {
        if struct_index >= dna.structs.len() {
            return data; // Invalid struct index, return unchanged
        }

        let struct_def = &dna.structs[struct_index];

        // Special handling for structures that contain ListBases
        if struct_def.type_name == "bNodeTree" {
            return Self::remap_node_tree_with_listbase_reconstruction(
                data,
                address_map,
                struct_def,
            );
        }

        // Standard pointer remapping for other structures
        for field in &struct_def.fields {
            // Check if this field is a pointer using the DNA info
            if field.name.is_pointer {
                // Use the properly calculated offset from DNA parsing
                let field_offset = field.offset;

                // Handle arrays of pointers
                let pointer_count = field.name.array_size;
                let pointer_size = 8; // 64-bit pointers

                for i in 0..pointer_count {
                    let offset = field_offset + (i * pointer_size);

                    // Ensure we don't read beyond the data
                    if offset + pointer_size <= data.len() {
                        // Read the current 64-bit pointer value (little-endian)
                        let old_ptr = u64::from_le_bytes([
                            data[offset],
                            data[offset + 1],
                            data[offset + 2],
                            data[offset + 3],
                            data[offset + 4],
                            data[offset + 5],
                            data[offset + 6],
                            data[offset + 7],
                        ]);

                        // Handle pointer remapping and sanitization
                        if old_ptr != 0 {
                            // Check if this pointer needs remapping
                            if let Some(&new_ptr) = address_map.get(&old_ptr) {
                                // Write the new pointer value back (little-endian)
                                let new_bytes = new_ptr.to_le_bytes();
                                data[offset..offset + pointer_size].copy_from_slice(&new_bytes);
                            } else {
                                // Unmapped pointer - set to NULL to prevent crashes
                                let null_bytes = 0u64.to_le_bytes();
                                data[offset..offset + pointer_size].copy_from_slice(&null_bytes);
                            }
                        }
                    }
                }
            }
        }

        data
    }

    /// Special handling for NodeTree structures with ListBase reconstruction
    /// Following Blender's BLO_read_struct_list_with_size() approach
    fn remap_node_tree_with_listbase_reconstruction(
        mut data: Vec<u8>,
        address_map: &std::collections::HashMap<u64, u64>,
        struct_def: &dot001_parser::DnaStruct,
    ) -> Vec<u8> {
        println!("🔧 Applying ListBase reconstruction to NodeTree");

        // NodeTree has several ListBase fields that need special handling
        let listbase_fields = ["nodes", "links", "inputs", "outputs"];

        for field_name in listbase_fields {
            if let Some(field) = struct_def
                .fields
                .iter()
                .find(|f| f.name.name_only == field_name)
            {
                println!("  Reconstructing ListBase '{field_name}'");
                Self::reconstruct_listbase_at_offset(
                    &mut data,
                    field.offset,
                    address_map,
                    field_name,
                );
            }
        }

        // Handle other regular pointer fields in the NodeTree
        for field in &struct_def.fields {
            if field.name.is_pointer && !listbase_fields.contains(&field.name.name_only.as_str()) {
                let offset = field.offset;
                let pointer_count = field.name.array_size;

                for i in 0..pointer_count {
                    let ptr_offset = offset + (i * 8);
                    if ptr_offset + 8 <= data.len() {
                        let old_ptr = u64::from_le_bytes([
                            data[ptr_offset],
                            data[ptr_offset + 1],
                            data[ptr_offset + 2],
                            data[ptr_offset + 3],
                            data[ptr_offset + 4],
                            data[ptr_offset + 5],
                            data[ptr_offset + 6],
                            data[ptr_offset + 7],
                        ]);

                        if old_ptr != 0 {
                            if let Some(&new_ptr) = address_map.get(&old_ptr) {
                                let new_bytes = new_ptr.to_le_bytes();
                                data[ptr_offset..ptr_offset + 8].copy_from_slice(&new_bytes);
                                println!(
                                    "    Remapped NodeTree:{} 0x{:x} → 0x{:x}",
                                    field.name.name_only, old_ptr, new_ptr
                                );
                            } else {
                                // Nullify unmapped pointers
                                let null_bytes = 0u64.to_le_bytes();
                                data[ptr_offset..ptr_offset + 8].copy_from_slice(&null_bytes);
                                println!(
                                    "    Nullified NodeTree:{} 0x{:x} (unmapped)",
                                    field.name.name_only, old_ptr
                                );
                            }
                        }
                    }
                }
            }
        }

        data
    }

    /// Reconstruct a ListBase structure following Blender's approach.
    /// This mimics BLO_read_struct_list_with_size() from readfile.cc but works with our injected blocks.
    ///
    /// The approach: build a chain of addresses that are in our address_map, then find the last one.
    fn reconstruct_listbase_at_offset(
        data: &mut [u8],
        offset: usize,
        address_map: &std::collections::HashMap<u64, u64>,
        list_name: &str,
    ) {
        if offset + 16 > data.len() {
            return; // Not enough space for ListBase (16 bytes)
        }

        // Read the original first pointer
        let old_first_ptr = u64::from_le_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
            data[offset + 4],
            data[offset + 5],
            data[offset + 6],
            data[offset + 7],
        ]);

        if old_first_ptr == 0 {
            // Empty list - ensure both first and last are null
            let null_bytes = 0u64.to_le_bytes();
            data[offset..offset + 8].copy_from_slice(&null_bytes); // first
            data[offset + 8..offset + 16].copy_from_slice(&null_bytes); // last
            println!("    ListBase '{list_name}' is empty - both pointers cleared");
            return;
        }

        // Map the first pointer
        let new_first_ptr = address_map.get(&old_first_ptr).copied().unwrap_or(0);

        if new_first_ptr == 0 {
            // First element not in our address map - clear the list
            let null_bytes = 0u64.to_le_bytes();
            data[offset..offset + 8].copy_from_slice(&null_bytes); // first
            data[offset + 8..offset + 16].copy_from_slice(&null_bytes); // last
            println!(
                "    ListBase '{list_name}' first pointer 0x{old_first_ptr:x} not mapped - list cleared"
            );
            return;
        }

        // Update the first pointer
        let first_bytes = new_first_ptr.to_le_bytes();
        data[offset..offset + 8].copy_from_slice(&first_bytes);

        // Better approach: analyze the node chain by finding elements that belong to this list
        // Look for addresses that have similar proximity and could form a chain
        let mut potential_chain_members: Vec<(u64, u64)> = address_map
            .iter()
            .filter(|(old_addr, _)| {
                // Include addresses in a reasonable range around the first pointer
                let distance = (**old_addr).abs_diff(old_first_ptr);
                distance < 0x100000 // Within 1MB - reasonable for a linked list
            })
            .map(|(&old_addr, &new_addr)| (old_addr, new_addr))
            .collect();

        // Sort by old address to maintain original order relationships
        potential_chain_members.sort_by_key(|(old_addr, _)| *old_addr);

        // Find the position of our first element
        let first_position = potential_chain_members
            .iter()
            .position(|(old_addr, _)| *old_addr == old_first_ptr);

        let new_last_ptr = if let Some(first_pos) = first_position {
            if potential_chain_members.len() > first_pos + 1 {
                // Multi-element list: the last is likely the element after first in our sorted list
                // or the highest address that's reasonably close
                let candidates: Vec<u64> = potential_chain_members[first_pos + 1..]
                    .iter()
                    .map(|(_, new_addr)| *new_addr)
                    .collect();
                candidates.last().copied().unwrap_or(new_first_ptr)
            } else {
                // Single element
                new_first_ptr
            }
        } else {
            // Fallback
            new_first_ptr
        };

        // Update the last pointer
        let last_bytes = new_last_ptr.to_le_bytes();
        data[offset + 8..offset + 16].copy_from_slice(&last_bytes);

        println!(
            "    ListBase '{list_name}' reconstructed: first=0x{new_first_ptr:x}, last=0x{new_last_ptr:x} (heuristic)"
        );
    }

    /// Post-process linked list structures to ensure proper next/prev pointer chains.
    /// This is critical for bNode, bNodeSocket, and bNodeLink structures.
    /// Also updates corresponding ListBase first/last pointers in NodeTree.
    fn post_process_linked_list_structures(
        injection: &mut BlockInjection,
        dna: &dot001_parser::DnaCollection,
    ) {
        println!("🔗 Post-processing linked list structures");

        // Find all blocks that are linked list nodes and organize them by type
        let mut node_blocks = Vec::new();
        let mut socket_blocks = Vec::new();
        let mut link_blocks = Vec::new();

        for (i, block) in injection.blocks.iter().enumerate() {
            if let Some(struct_def) = dna.get_struct(block.sdna_index as usize) {
                match struct_def.type_name.as_str() {
                    "bNode" => node_blocks.push(i),
                    "bNodeSocket" => socket_blocks.push(i),
                    "bNodeLink" => link_blocks.push(i),
                    _ => {}
                }
            }
        }

        println!(
            "  Found {} nodes, {} sockets, {} links",
            node_blocks.len(),
            socket_blocks.len(),
            link_blocks.len()
        );

        // For each type of linked list structure, rebuild the chain
        if !node_blocks.is_empty() {
            Self::rebuild_linked_list_chain(injection, dna, &node_blocks, "bNode");
        }
        if !socket_blocks.is_empty() {
            Self::rebuild_linked_list_chain(injection, dna, &socket_blocks, "bNodeSocket");
        }
        if !link_blocks.is_empty() {
            Self::rebuild_linked_list_chain(injection, dna, &link_blocks, "bNodeLink");
        }

        println!("  Note: Experimental linked list reconstruction completed");
    }

    /// Rebuild a linked list chain for a specific block type.
    /// This ensures that next/prev pointers form a proper chain.
    fn rebuild_linked_list_chain(
        injection: &mut BlockInjection,
        dna: &dot001_parser::DnaCollection,
        block_indices: &[usize],
        struct_type: &str,
    ) {
        println!(
            "    Rebuilding {} chain with {} elements",
            struct_type,
            block_indices.len()
        );

        if block_indices.is_empty() {
            return;
        }

        if block_indices.len() == 1 {
            // Single element - clear next/prev pointers
            let block_idx = block_indices[0];
            Self::clear_next_prev_pointers(injection, dna, block_idx, struct_type);
            return;
        }

        // Multi-element chain: wire them together in sequence
        for (i, &block_idx) in block_indices.iter().enumerate() {
            let prev_idx = if i > 0 {
                Some(block_indices[i - 1])
            } else {
                None
            };
            let next_idx = if i < block_indices.len() - 1 {
                Some(block_indices[i + 1])
            } else {
                None
            };

            Self::update_next_prev_pointers(
                injection,
                dna,
                block_idx,
                prev_idx,
                next_idx,
                struct_type,
            );
        }
    }

    /// Clear next/prev pointers for a single-element list
    fn clear_next_prev_pointers(
        injection: &mut BlockInjection,
        dna: &dot001_parser::DnaCollection,
        block_idx: usize,
        struct_type: &str,
    ) {
        if let Some(struct_def) = dna.get_struct(injection.blocks[block_idx].sdna_index as usize) {
            let data = &mut injection.blocks[block_idx].data;

            // Find and clear next pointer
            if let Some(next_field) = struct_def
                .fields
                .iter()
                .find(|f| f.name.name_only == "next")
            {
                if next_field.offset + 8 <= data.len() {
                    let null_bytes = 0u64.to_le_bytes();
                    data[next_field.offset..next_field.offset + 8].copy_from_slice(&null_bytes);
                }
            }

            // Find and clear prev pointer
            if let Some(prev_field) = struct_def
                .fields
                .iter()
                .find(|f| f.name.name_only == "prev")
            {
                if prev_field.offset + 8 <= data.len() {
                    let null_bytes = 0u64.to_le_bytes();
                    data[prev_field.offset..prev_field.offset + 8].copy_from_slice(&null_bytes);
                }
            }

            println!("      Cleared next/prev for single {struct_type} element");
        }
    }

    /// Update next/prev pointers for a chained element
    fn update_next_prev_pointers(
        injection: &mut BlockInjection,
        dna: &dot001_parser::DnaCollection,
        block_idx: usize,
        prev_idx: Option<usize>,
        next_idx: Option<usize>,
        struct_type: &str,
    ) {
        // First collect the addresses to avoid borrowing issues
        let next_address = next_idx
            .map(|idx| injection.blocks[idx].old_address)
            .unwrap_or(0);
        let prev_address = prev_idx
            .map(|idx| injection.blocks[idx].old_address)
            .unwrap_or(0);
        let current_address = injection.blocks[block_idx].old_address;

        if let Some(struct_def) = dna.get_struct(injection.blocks[block_idx].sdna_index as usize) {
            let data = &mut injection.blocks[block_idx].data;

            // Update next pointer
            if let Some(next_field) = struct_def
                .fields
                .iter()
                .find(|f| f.name.name_only == "next")
            {
                if next_field.offset + 8 <= data.len() {
                    let next_bytes = next_address.to_le_bytes();
                    data[next_field.offset..next_field.offset + 8].copy_from_slice(&next_bytes);
                }
            }

            // Update prev pointer
            if let Some(prev_field) = struct_def
                .fields
                .iter()
                .find(|f| f.name.name_only == "prev")
            {
                if prev_field.offset + 8 <= data.len() {
                    let prev_bytes = prev_address.to_le_bytes();
                    data[prev_field.offset..prev_field.offset + 8].copy_from_slice(&prev_bytes);
                }
            }

            println!(
                "      {struct_type} chain: prev=0x{prev_address:x}, current=0x{current_address:x}, next=0x{next_address:x}"
            );
        }
    }
}

/// Templates for initial milestones.
#[derive(Clone, Copy, Debug)]
pub enum WriteTemplate {
    /// M1: header + essential blocks + DNA1 + ENDB
    Minimal,
    /// M2: Minimal + injected blocks (collections, objects, etc.)
    WithInjection,
    // Future:
    // SceneOnly,
    // TriangleMesh,
}

/** Encapsulates writing a Blender 5.0-format .blend file.
Emits:
- 17-byte v1 header: "BLENDER17-01v0500"
- Minimum required ID blocks to satisfy Main:
    * "ID" (Main) placeholder with minimal header payload (zero length)
    * "GLOB" placeholder (zero length) for global state if required by reader paths
- DNA1 block with raw bytes extracted from a seed
- ENDB
  Note: Some Blender codepaths assume a Main/ID list exists before DNA when opening very minimal files.
*/
#[derive(Default)]
pub struct BlendWriter {
    pub header: HeaderWriter,
}

impl BlendWriter {
    /// Write a .blend file according to a chosen template, using DNA from the provided seed.
    pub fn write_with_seed<P: AsRef<std::path::Path>>(
        &self,
        out_path: P,
        template: WriteTemplate,
        seed: &SeedDnaProvider,
    ) -> Result<()> {
        // Emit write started event
        let file_path = out_path.as_ref().to_path_buf();
        emit_global_sync!(Event::Writer(WriterEvent::Started {
            operation: "write_blend_file".to_string(),
            target_file: file_path.clone(),
        }));

        let start_time = std::time::Instant::now();
        let result = self.write_with_seed_and_injection(out_path, template, seed, None);

        // Emit result event
        let duration_ms = start_time.elapsed().as_millis() as u64;
        match &result {
            Ok(()) => {
                emit_global_sync!(Event::Writer(WriterEvent::Finished {
                    operation: "write_blend_file".to_string(),
                    bytes_written: 0,  // TODO: Track bytes
                    blocks_written: 0, // TODO: Count blocks
                    duration_ms,
                    success: true,
                }));
            }
            Err(e) => {
                let writer_error = dot001_events::error::Error::writer(
                    e.user_message(),
                    dot001_events::error::WriterErrorKind::WriteFailed,
                );
                emit_global_sync!(Event::Writer(WriterEvent::Error {
                    error: writer_error,
                }));
            }
        }

        result
    }

    /// Write a .blend file with optional block injection.
    pub fn write_with_seed_and_injection<P: AsRef<std::path::Path>>(
        &self,
        out_path: P,
        template: WriteTemplate,
        seed: &SeedDnaProvider,
        injection: Option<&BlockInjection>,
    ) -> Result<()> {
        match template {
            WriteTemplate::Minimal => self.write_minimal(out_path, seed),
            WriteTemplate::WithInjection => self.write_with_injection(
                out_path,
                seed,
                injection.unwrap_or(&BlockInjection::default()),
            ),
        }
    }

    fn write_minimal<P: AsRef<std::path::Path>>(
        &self,
        out_path: P,
        seed: &SeedDnaProvider,
    ) -> Result<()> {
        // Emit template generation event
        emit_global_sync!(Event::Writer(WriterEvent::HeaderGenerated {
            version: "Blender 3.0".to_string(), // Default template version
            block_count: 4,                     // REND, TEST, GLOB, DNA1
        }));

        let file = File::create(out_path)?;
        let mut w = BufWriter::new(file);

        // 1) Header
        self.header.write(&mut w)?;

        // 2) REND block (render settings) - essential first block
        let rend_sdna_index = seed.sdna_index_for_struct("RenderData").unwrap_or(0u32);
        self.write_block_v1(&mut w, b"REND", rend_sdna_index, 0, seed.rend_bytes(), 1)?;

        // 3) TEST block - appears to be essential in working files
        let test_sdna_index = seed.sdna_index_for_struct("Test").unwrap_or(0u32);
        self.write_block_v1(&mut w, b"TEST", test_sdna_index, 0, seed.test_bytes(), 1)?;

        // 4) GLOB block (global settings) - use actual data from seed
        let glob_sdna_index = seed.sdna_index_for_struct("Global").unwrap_or(0u32);
        self.write_block_v1(&mut w, b"GLOB", glob_sdna_index, 0, seed.glob_bytes(), 1)?;

        // 5) DNA1 block
        self.write_block_v1(
            &mut w,
            b"DNA1",
            0u32,             // sdna_index is 0 for DNA itself
            0,                // old address not used for DNA
            seed.raw_bytes(), // payload copied from seed
            1,                // count
        )?;

        // 6) ENDB
        self.write_block_v1(&mut w, b"ENDB", 0u32, 0, &[], 0)?;

        w.flush()
            .map_err(|e| Dot001Error::io(format!("flush failed: {e}")))?;
        Ok(())
    }

    fn write_with_injection<P: AsRef<std::path::Path>>(
        &self,
        out_path: P,
        seed: &SeedDnaProvider,
        injection: &BlockInjection,
    ) -> Result<()> {
        let file = File::create(out_path)?;
        let mut w = BufWriter::new(file);

        // 1) Header
        self.header.write(&mut w)?;

        // 2) REND block (render settings) - essential first block
        let rend_sdna_index = seed.sdna_index_for_struct("RenderData").unwrap_or(0u32);
        self.write_block_v1(&mut w, b"REND", rend_sdna_index, 0, seed.rend_bytes(), 1)?;

        // 3) TEST block - appears to be essential in working files
        let test_sdna_index = seed.sdna_index_for_struct("Test").unwrap_or(0u32);
        self.write_block_v1(&mut w, b"TEST", test_sdna_index, 0, seed.test_bytes(), 1)?;

        // 4) GLOB block (global settings) - use actual data from seed
        let glob_sdna_index = seed.sdna_index_for_struct("Global").unwrap_or(0u32);
        self.write_block_v1(&mut w, b"GLOB", glob_sdna_index, 0, seed.glob_bytes(), 1)?;

        // 5) Injected blocks - write them in the order provided
        emit_global_sync!(Event::Writer(WriterEvent::BlockInjectionStarted {
            total_blocks: injection.blocks.len(),
        }));

        for (i, block) in injection.blocks.iter().enumerate() {
            let block_type = String::from_utf8_lossy(&block.code)
                .trim_end_matches('\0')
                .to_string();

            emit_global_sync!(
                Event::Writer(WriterEvent::BlockWritten {
                    block_type,
                    block_index: i,
                    size: block.data.len(),
                }),
                Severity::Debug
            );

            self.write_block_v1(
                &mut w,
                &block.code,
                block.sdna_index,
                block.old_address,
                &block.data,
                block.count,
            )?;
        }

        // Block injection completed - will be reported in main Finished event

        // 6) DNA1 block
        self.write_block_v1(
            &mut w,
            b"DNA1",
            0u32,             // sdna_index is 0 for DNA itself
            0,                // old address not used for DNA
            seed.raw_bytes(), // payload copied from seed
            1,                // count
        )?;

        // 7) ENDB
        self.write_block_v1(&mut w, b"ENDB", 0u32, 0, &[], 0)?;

        w.flush()
            .map_err(|e| Dot001Error::io(format!("flush failed: {e}")))?;
        Ok(())
    }

    /// Write a v1 (5.0) BHead+payload block.
    /// Binary layout for v1:
    ///   code[4]
    ///   sdna_index: u32 (ASCII-less integer, little-endian)
    ///   old_address: u64
    ///   len: u64 (payload length in bytes)
    ///   count: u64 (written as u64 but should fit in u32)
    fn write_block_v1<W: Write>(
        &self,
        mut w: W,
        code: &[u8; 4],
        sdna_index: u32,
        old_address: u64,
        payload: &[u8],
        count: u32,
    ) -> Result<()> {
        if code.len() != 4 {
            return Err(Dot001Error::blend_file(
                "block code must be 4 bytes",
                dot001_error::BlendFileErrorKind::InvalidHeader,
            ));
        }

        // code
        w.write_all(code)
            .map_err(|e| Dot001Error::io(format!("write block code failed: {e}")))?;

        // sdna_index (u32 LE)
        w.write_all(&sdna_index.to_le_bytes())
            .map_err(|e| Dot001Error::io(format!("write sdna_index failed: {e}")))?;

        // old_address (u64 LE)
        w.write_all(&old_address.to_le_bytes())
            .map_err(|e| Dot001Error::io(format!("write old_address failed: {e}")))?;

        // len (u64 LE)
        let len = payload.len() as u64;
        w.write_all(&len.to_le_bytes())
            .map_err(|e| Dot001Error::io(format!("write len failed: {e}")))?;

        // count (u64 LE, but input is u32)
        w.write_all(&(count as u64).to_le_bytes())
            .map_err(|e| Dot001Error::io(format!("write count failed: {e}")))?;

        // payload
        if len > 0 {
            w.write_all(payload)
                .map_err(|e| Dot001Error::io(format!("write payload failed: {e}")))?;
        }

        Ok(())
    }
}
