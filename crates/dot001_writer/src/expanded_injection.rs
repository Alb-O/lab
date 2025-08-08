use crate::dna_provider::SeedDnaProvider;
use crate::emitter::BlockInjection;
use dot001_events::error::Result;
use dot001_parser::{BlendBuf, BlendFile};
use dot001_tracer::ParallelDependencyTracer;
use std::collections::HashSet;
use std::fs::File;
use std::io::Read;

/// Enhanced injection system that uses dependency tracing to expand
/// injected blocks with their complete dependency trees
pub struct ExpandedBlockInjection;

impl ExpandedBlockInjection {
    /// Create a block injection with expanded dependencies using the tracer system
    pub fn from_block_indices_with_expansion(
        seed: &mut SeedDnaProvider,
        block_indices: &[usize],
    ) -> Result<BlockInjection> {
        // Create a fresh BlendFile instance for the tracer
        // We need to read the seed file again since SeedDnaProvider doesn't keep the BlendFile around
        let mut file = File::open(seed.source_path())?;
        let mut buf = Vec::new();
        file.read_to_end(&mut buf)?;

        let blend_buf = BlendBuf::from_vec(buf);
        let blend_file = BlendFile::new(blend_buf)?;

        // Create a dependency tracer with standard expanders
        let mut tracer = ParallelDependencyTracer::new().with_default_expanders();

        // Collect all dependencies for the provided block indices
        let mut all_dependencies = HashSet::new();

        for &block_index in block_indices {
            println!("Tracing dependencies for block {block_index}");

            match tracer.trace_dependencies_parallel(block_index, &blend_file) {
                Ok(dependencies) => {
                    // Add all dependencies to our set
                    all_dependencies.extend(dependencies.iter());
                    println!(
                        "  Found {} total dependencies including root",
                        all_dependencies.len()
                    );
                }
                Err(e) => {
                    println!(
                        "  Warning: Failed to trace dependencies for block {block_index}: {e}"
                    );
                    // Still include the original block even if dependency tracing fails
                    all_dependencies.insert(block_index);
                }
            }
        }

        // Convert HashSet to Vec for extraction
        let expanded_indices: Vec<usize> = all_dependencies.into_iter().collect();

        println!(
            "Extracting {} blocks with expanded dependencies:",
            expanded_indices.len()
        );
        for &index in &expanded_indices {
            if let Some(block) = blend_file.get_block(index) {
                let code = String::from_utf8_lossy(&block.header.code)
                    .trim_end_matches('\0')
                    .to_string();
                println!("  [{index}] {code}");
            }
        }

        // Extract all the expanded blocks
        let extracted_blocks = seed.extract_blocks_by_indices(&expanded_indices)?;

        // Create injection with DNA-aware pointer remapping
        Ok(BlockInjection::from_extracted_blocks_with_dna(
            extracted_blocks,
            seed.dna(),
        ))
    }
}

/// Recursively collect all block indices from a dependency tree
#[allow(dead_code)]
fn collect_all_indices_from_tree(
    node: &dot001_tracer::DependencyNode,
    indices: &mut HashSet<usize>,
) {
    indices.insert(node.block_index);

    for child in &node.children {
        collect_all_indices_from_tree(child, indices);
    }
}
