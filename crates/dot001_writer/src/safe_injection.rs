use crate::dna_provider::SeedDnaProvider;
use crate::emitter::BlockInjection;
use dot001_events::error::Result;
use dot001_events::{
    event::{Event, WriterEvent},
    prelude::*,
};
use log::info;

/// Experimental block injection that sanitizes dangerous pointers.
///
/// This attempts to prevent crashes by nullifying internal pointers that
/// may cause access violations, though this often results in limited functionality.
/// Even with sanitization, complex structures may still crash.
pub struct SafeBlockInjection;

impl SafeBlockInjection {
    /// Create a block injection with safe handling of complex internal pointers
    pub fn from_block_indices_with_safe_handling(
        seed: &mut SeedDnaProvider,
        block_indices: &[usize],
    ) -> Result<BlockInjection> {
        // Emit block extraction started event
        emit_global_sync!(Event::Writer(WriterEvent::BlockInjectionStarted {
            total_blocks: block_indices.len(),
        }));

        let start_time = std::time::Instant::now();

        // Extract the requested blocks
        let extracted_blocks = seed.extract_blocks_by_indices(block_indices)?;

        info!(
            "Creating safe injection with {} blocks:",
            extracted_blocks.len()
        );
        for (index, header, _) in &extracted_blocks {
            let code_str = String::from_utf8_lossy(&header.code);
            let code = code_str.trim_end_matches('\0');
            info!("  [{index}] {code}");
        }

        // Create the basic injection with address remapping
        let mut injection =
            BlockInjection::from_extracted_blocks_with_dna(extracted_blocks, seed.dna());

        // Apply safe handling to complex structures
        Self::apply_safe_handling_to_injection(&mut injection, seed.dna())?;

        // Calculate duration and bytes written (sum of block data lengths)
        let duration_ms = start_time.elapsed().as_millis() as u64;
        let bytes_written: u64 = injection.blocks.iter().map(|b| b.data.len() as u64).sum();

        // Emit completion event
        emit_global_sync!(Event::Writer(WriterEvent::Finished {
            operation: "safe_block_injection".to_string(),
            bytes_written,
            blocks_written: injection.blocks.len(),
            duration_ms,
            success: true,
        }));

        Ok(injection)
    }

    /// Apply safe handling by sanitizing dangerous internal pointers in complex structures
    fn apply_safe_handling_to_injection(
        injection: &mut BlockInjection,
        dna: &dot001_parser::DnaCollection,
    ) -> Result<()> {
        info!(
            "Applying safe handling to {} blocks...",
            injection.blocks.len()
        );

        for block in &mut injection.blocks {
            // Get the struct definition for this block
            if let Some(struct_def) = dna.get_struct(block.sdna_index as usize) {
                match struct_def.type_name.as_str() {
                    "bNodeTree" => {
                        info!("  Sanitizing NodeTree block (dangerous internal pointers)");
                        Self::sanitize_node_tree(&mut block.data, struct_def)?;
                    }
                    "bNode" => {
                        info!("  Sanitizing Node block (linked list pointers)");
                        Self::sanitize_node(&mut block.data, struct_def)?;
                    }
                    "bNodeLink" => {
                        info!("  Sanitizing NodeLink block (connection pointers)");
                        Self::sanitize_node_link(&mut block.data, struct_def)?;
                    }
                    _ => {
                        // For other types, just apply standard pointer remapping
                        // (this is already handled by the base injection system)
                    }
                }
            }
        }

        info!("Safe handling applied successfully");
        Ok(())
    }

    /// Sanitize NodeTree internal pointers that aren't in our injection
    fn sanitize_node_tree(data: &mut [u8], struct_def: &dot001_parser::DnaStruct) -> Result<()> {
        // Critical NodeTree fields that often cause crashes
        let dangerous_fields = [
            "nodes",   // ListBase - linked list of nodes
            "links",   // ListBase - linked list of connections
            "inputs",  // ListBase - input sockets
            "outputs", // ListBase - output sockets
        ];

        for field_name in dangerous_fields {
            if let Some(field) = struct_def
                .fields
                .iter()
                .find(|f| f.name.name_only == field_name)
            {
                Self::nullify_listbase_at_offset(data, field.offset, field_name);
            }
        }

        // Also nullify other risky pointers that might not be included
        let risky_pointers = ["nested_node_refs", "geometry_node_asset_traits", "preview"];
        for field_name in risky_pointers {
            if let Some(field) = struct_def
                .fields
                .iter()
                .find(|f| f.name.name_only == field_name)
            {
                if field.name.is_pointer {
                    Self::nullify_pointer_at_offset(data, field.offset, field_name);
                }
            }
        }

        Ok(())
    }

    /// Sanitize individual Node pointers
    fn sanitize_node(data: &mut [u8], struct_def: &dot001_parser::DnaStruct) -> Result<()> {
        // Node linked list pointers
        let list_fields = ["next", "prev"];
        for field_name in list_fields {
            if let Some(field) = struct_def
                .fields
                .iter()
                .find(|f| f.name.name_only == field_name)
            {
                Self::nullify_pointer_at_offset(data, field.offset, field_name);
            }
        }

        Ok(())
    }

    /// Sanitize NodeLink connection pointers  
    fn sanitize_node_link(data: &mut [u8], struct_def: &dot001_parser::DnaStruct) -> Result<()> {
        // NodeLink pointers to nodes and sockets
        let connection_fields = ["fromnode", "tonode", "fromsock", "tosock"];
        for field_name in connection_fields {
            if let Some(field) = struct_def
                .fields
                .iter()
                .find(|f| f.name.name_only == field_name)
            {
                Self::nullify_pointer_at_offset(data, field.offset, field_name);
            }
        }

        Ok(())
    }

    /// Nullify a ListBase structure (first/last pointers)
    fn nullify_listbase_at_offset(data: &mut [u8], offset: usize, field_name: &str) {
        if offset + 16 <= data.len() {
            // ListBase is typically 16 bytes (2 pointers)
            // Clear both pointers (16 bytes total)
            data[offset..offset + 16].fill(0);
            info!("    Nullified ListBase '{field_name}' at offset {offset}");
        }
    }

    /// Nullify a single pointer field
    fn nullify_pointer_at_offset(data: &mut [u8], offset: usize, field_name: &str) {
        if offset + 8 <= data.len() {
            data[offset..offset + 8].fill(0);
            info!("    Nullified pointer '{field_name}' at offset {offset}");
        }
    }
}
