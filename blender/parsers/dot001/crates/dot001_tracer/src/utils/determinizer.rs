use dot001_parser::BlendFileBuf;
use std::collections::HashMap;

/// Utility for generating deterministic, stable outputs from BlendFile data.
/// This centralizes address remapping, sorting, and normalization logic
/// that was previously scattered across different modules.
#[derive(Clone)]
pub struct Determinizer {
    /// Mapping from original addresses to deterministic IDs
    address_map: HashMap<u64, u64>,
    /// Counter for generating sequential deterministic IDs
    next_id: u64,
}

impl Default for Determinizer {
    fn default() -> Self {
        Self::new()
    }
}

impl Determinizer {
    /// Create a new Determinizer
    pub fn new() -> Self {
        Self {
            address_map: HashMap::new(),
            next_id: 1, // Start from 1 to distinguish from null pointers
        }
    }

    /// Build address mapping from a BlendFile for deterministic output
    pub fn build_address_map(&mut self, blend_file: &BlendFileBuf) {
        let mut addresses: Vec<u64> = Vec::new();

        // Collect all unique addresses from blocks
        for i in 0..blend_file.blocks_len() {
            if let Some(block) = blend_file.get_block(i) {
                let addr = block.header.old_address;
                if addr != 0 && !addresses.contains(&addr) {
                    addresses.push(addr);
                }
            }
        }

        // Sort addresses for deterministic mapping
        addresses.sort_unstable();

        // Create mapping from sorted addresses to sequential IDs
        self.address_map.clear();
        self.next_id = 1;
        for addr in addresses {
            self.address_map.insert(addr, self.next_id);
            self.next_id += 1;
        }
    }

    /// Get the deterministic ID for an address
    pub fn remap_address(&self, addr: u64) -> u64 {
        if addr == 0 {
            return 0; // Preserve null pointers
        }
        self.address_map.get(&addr).copied().unwrap_or(addr)
    }

    /// Get a copy of the address mapping for use in other components
    pub fn get_address_map(&self) -> HashMap<u64, u64> {
        self.address_map.clone()
    }

    /// Sort a list of block indices deterministically by their addresses
    pub fn sort_blocks_by_address(
        &self,
        mut block_indices: Vec<usize>,
        blend_file: &BlendFileBuf,
    ) -> Vec<usize> {
        block_indices.sort_by_key(|&index| {
            blend_file
                .get_block(index)
                .map(|block| self.remap_address(block.header.old_address))
                .unwrap_or(0)
        });
        block_indices
    }

    /// Normalize a block code by removing null terminators and ensuring consistent format
    pub fn normalize_block_code(code: &[u8; 4]) -> String {
        String::from_utf8_lossy(code)
            .trim_end_matches('\0')
            .to_string()
    }

    /// Generate a fallback identifier for blocks without names
    /// Format: "code#deterministic_id" (e.g., "OB#5", "ME#12")
    pub fn generate_fallback_id(&self, code: &[u8; 4], address: u64) -> String {
        let code_str = Self::normalize_block_code(code);
        let det_id = self.remap_address(address);
        format!("{code_str}#{det_id}")
    }

    /// Create a stable, deterministic identifier for a block
    /// Uses the NameResolver if available, falls back to address-based ID
    pub fn create_stable_id(
        &self,
        block_index: usize,
        blend_file: &BlendFileBuf,
        name_resolver: Option<&dyn NameResolverTrait>,
    ) -> String {
        // Copy block info first to avoid borrowing conflicts
        let (code, address) = if let Some(block) = blend_file.get_block(block_index) {
            (block.header.code, block.header.old_address)
        } else {
            return format!("INVALID#{block_index}");
        };

        // Try to get user-defined name first
        if let Some(resolver) = name_resolver {
            if let Some(name) = resolver.resolve_name(block_index, blend_file) {
                let code_str = Self::normalize_block_code(&code);
                return format!("{code_str} ({name})");
            }
        }

        // Fall back to deterministic address-based ID
        self.generate_fallback_id(&code, address)
    }
}

/// Trait for name resolution to allow different implementations
pub trait NameResolverTrait {
    /// Resolve the user-defined name for a block
    fn resolve_name(&self, block_index: usize, blend_file: &BlendFileBuf) -> Option<String>;

    /// Get a display name combining type and user name
    fn get_display_name(
        &self,
        block_index: usize,
        blend_file: &BlendFileBuf,
        block_code: &str,
    ) -> String;
}
