use crate::Result;
use dot001_error::Dot001Error;
use dot001_parser::{BlendFile, DnaCollection};
#[cfg(feature = "tracer_integration")]
use dot001_tracer::{BlockExpander, MeshExpander};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::io::{Read, Seek};

/// Provenance graph for tracking ME block dependencies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvenanceGraph {
    /// Root ME block index
    pub me_block_index: usize,
    /// Set of DATA block indices referenced by this ME
    pub referenced_data_blocks: HashSet<usize>,
    /// Metadata about each referenced DATA block
    pub data_block_info: HashMap<usize, DataBlockInfo>,
}

/// Information about a DATA block referenced by an ME block
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataBlockInfo {
    pub block_index: usize,
    pub size: u32,
    pub raw_hash: String,
    pub pointer_masked_hash: Option<String>,
    pub numeric_hash: Option<String>,
    pub element_type: Option<String>,
    pub element_count: Option<usize>,
    pub element_stride: Option<usize>,
}

/// Enhanced change classification for DATA blocks
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DataChangeClass {
    /// Content changed (numeric/semantic data differs)
    ContentChange,
    /// Only layout changed (pointers/padding differs, content same)
    LayoutChange,
    /// Size changed (element count or structure changed)
    SizeChange,
    /// No change detected
    Unchanged,
}

/// Correlation between before and after DATA blocks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataBlockCorrelation {
    pub before_index: Option<usize>,
    pub after_index: Option<usize>,
    pub change_class: DataChangeClass,
    pub confidence: f32, // 0.0 to 1.0
    pub rationale: String,
}

/// ME-level analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeshAnalysisResult {
    pub me_block_index: usize,
    pub before_provenance: Option<ProvenanceGraph>,
    pub after_provenance: Option<ProvenanceGraph>,
    pub data_correlations: Vec<DataBlockCorrelation>,
    pub overall_classification: DataChangeClass,
    pub is_true_edit: bool,
    pub summary: String,
}

/// Provenance analyzer for ME-DATA correlation
pub struct ProvenanceAnalyzer {
    /// Epsilon tolerance for float comparisons
    pub float_epsilon: f64,
    /// Enable verbose logging of analysis steps
    pub verbose: bool,
}

impl ProvenanceAnalyzer {
    pub fn new() -> Self {
        Self {
            float_epsilon: 1e-6,
            verbose: false,
        }
    }

    pub fn with_epsilon(mut self, epsilon: f64) -> Self {
        self.float_epsilon = epsilon;
        self
    }

    pub fn with_verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }

    /// Extract provenance graph for an ME block
    pub fn extract_me_provenance<R: Read + Seek>(
        &self,
        me_block_index: usize,
        file: &mut BlendFile<R>,
    ) -> Result<ProvenanceGraph> {
        #[cfg(feature = "tracer_integration")]
        {
            // Use MeshExpander to get pointer dependencies
            let mesh_expander = MeshExpander;
            self.extract_me_provenance_with_tracer(me_block_index, file, mesh_expander)
        }
        #[cfg(not(feature = "tracer_integration"))]
        {
            // Fallback implementation without tracer
            self.extract_me_provenance_fallback(me_block_index, file)
        }
    }

    #[cfg(feature = "tracer_integration")]
    fn extract_me_provenance_with_tracer<R: Read + Seek>(
        &self,
        me_block_index: usize,
        file: &mut BlendFile<R>,
        mesh_expander: MeshExpander,
    ) -> Result<ProvenanceGraph> {
        let referenced_blocks = mesh_expander
            .expand_block(me_block_index, file)
            .map_err(|e| Dot001Error::diff_analysis_failed(format!("Blend file error: {e}")))?
            .dependencies;

        let mut referenced_data_blocks = HashSet::new();
        let mut data_block_info = HashMap::new();

        // Filter for DATA blocks and collect info
        for &block_idx in &referenced_blocks {
            if let Some(block) = file.get_block(block_idx) {
                let block_code = String::from_utf8_lossy(&block.header.code);
                if block_code.trim_end_matches('\0') == "DATA" {
                    referenced_data_blocks.insert(block_idx);

                    let info = self.analyze_data_block(block_idx, file)?;
                    data_block_info.insert(block_idx, info);
                }
            }
        }

        // If MeshExpander found very few DATA blocks, also check nearby blocks
        // as a fallback since geometric data might not be properly referenced
        // But be more conservative to avoid false positives in complex files
        if referenced_data_blocks.len() < 3 {
            let search_range = 10; // Reduced from 20 to be more conservative
            let start = me_block_index.saturating_sub(search_range);
            let end = (me_block_index + search_range).min(file.blocks_len());

            // Collect candidate blocks first to avoid borrowing conflicts
            let mut candidate_blocks = Vec::new();
            for block_idx in start..end {
                if block_idx == me_block_index || referenced_data_blocks.contains(&block_idx) {
                    continue;
                }

                if let Some(block) = file.get_block(block_idx) {
                    let block_code = String::from_utf8_lossy(&block.header.code);
                    if block_code.trim_end_matches('\0') == "DATA" {
                        // Be more restrictive: only include smaller DATA blocks that are likely mesh data
                        // Large blocks are more likely to be unrelated (textures, etc.)
                        if block.header.size > 8 && block.header.size < 10_000 {
                            candidate_blocks.push((block_idx, block.header.size));
                        }
                    }
                }
            }

            // Limit the number of nearby blocks to prevent over-inclusion
            candidate_blocks.sort_by_key(|(_, size)| *size);
            candidate_blocks.truncate(15); // Max 15 nearby blocks

            // Now process the candidate blocks
            for (block_idx, block_size) in candidate_blocks {
                referenced_data_blocks.insert(block_idx);
                let info = self.analyze_data_block(block_idx, file)?;
                data_block_info.insert(block_idx, info);

                if self.verbose {
                    log::debug!(
                        "Added nearby DATA block {block_idx} (size: {block_size}) to ME block {me_block_index} provenance"
                    );
                }
            }
        }

        if self.verbose {
            log::debug!(
                "ME block {} references {} DATA blocks: {:?}",
                me_block_index,
                referenced_data_blocks.len(),
                referenced_data_blocks
            );
        }

        Ok(ProvenanceGraph {
            me_block_index,
            referenced_data_blocks,
            data_block_info,
        })
    }

    #[cfg(not(feature = "tracer_integration"))]
    fn extract_me_provenance_fallback<R: Read + Seek>(
        &self,
        me_block_index: usize,
        file: &mut BlendFile<R>,
    ) -> Result<ProvenanceGraph> {
        let mut referenced_data_blocks = HashSet::new();
        let mut data_block_info = HashMap::new();

        // Without tracer, use a heuristic approach: look for DATA blocks near the ME block
        let search_range = 20;
        let start = me_block_index.saturating_sub(search_range);
        let end = (me_block_index + search_range).min(file.blocks.len());

        for block_idx in start..end {
            if block_idx == me_block_index {
                continue;
            }

            if let Some(block) = file.get_block(block_idx) {
                let block_code = String::from_utf8_lossy(&block.header.code);
                if block_code.trim_end_matches('\0') == "DATA" && block.header.size > 8 {
                    referenced_data_blocks.insert(block_idx);
                    let info = self.analyze_data_block(block_idx, file)?;
                    data_block_info.insert(block_idx, info);
                }
            }
        }

        if self.verbose {
            log::debug!(
                "ME block {} (fallback mode) found {} nearby DATA blocks: {:?}",
                me_block_index,
                referenced_data_blocks.len(),
                referenced_data_blocks
            );
        }

        Ok(ProvenanceGraph {
            me_block_index,
            referenced_data_blocks,
            data_block_info,
        })
    }

    /// Analyze a single DATA block and compute hashes
    pub fn analyze_data_block<R: Read + Seek>(
        &self,
        block_index: usize,
        file: &mut BlendFile<R>,
    ) -> Result<DataBlockInfo> {
        let block = file.get_block(block_index).ok_or_else(|| {
            Dot001Error::diff_insufficient_data(format!("Block not found: {block_index}"))
        })?;

        let size = block.header.size;
        let data = file.read_block_data(block_index)?;

        // Compute raw hash
        let raw_hash = self.compute_raw_hash(&data);

        // Try to compute pointer-masked hash if we have DNA info
        let pointer_masked_hash = if let Ok(dna) = file.dna() {
            self.compute_pointer_masked_hash(&data, dna, size).ok()
        } else {
            None
        };

        // Try to compute numeric-only hash
        let numeric_hash = self.compute_numeric_hash(&data).ok();

        // Try to infer element type and count
        let (element_type, element_count, element_stride) = self
            .infer_data_block_structure(&data, file)
            .unwrap_or((None, None, None));

        Ok(DataBlockInfo {
            block_index,
            size,
            raw_hash,
            pointer_masked_hash,
            numeric_hash,
            element_type,
            element_count,
            element_stride,
        })
    }

    /// Compute blake3 hash of raw block data
    fn compute_raw_hash(&self, data: &[u8]) -> String {
        use blake3::Hasher;
        let mut hasher = Hasher::new();
        hasher.update(data);
        hex::encode(hasher.finalize().as_bytes())
    }

    /// Compute hash with pointer fields zeroed out based on DNA
    fn compute_pointer_masked_hash(
        &self,
        data: &[u8],
        _dna: &DnaCollection,
        _size: u32,
    ) -> Result<String> {
        // For now, implement a simple approach that zeros out pointer-sized regions
        // This is a heuristic approach - in a full implementation we'd use DNA to
        // identify exact pointer field locations

        let mut masked_data = data.to_vec();
        let pointer_size = if data.len() > 1000 { 8 } else { 4 }; // Heuristic

        // Zero out regions that look like pointers (8-byte aligned addresses)
        for i in (0..masked_data.len()).step_by(pointer_size) {
            if i + pointer_size <= masked_data.len() {
                // Simple heuristic: if it looks like a pointer (high address), zero it
                if pointer_size == 8 {
                    let val =
                        u64::from_le_bytes(masked_data[i..i + 8].try_into().unwrap_or_default());
                    if val > 0x1000_0000 {
                        // Looks like a pointer
                        masked_data[i..i + 8].fill(0);
                    }
                } else {
                    let val =
                        u32::from_le_bytes(masked_data[i..i + 4].try_into().unwrap_or_default());
                    if val > 0x1000_0000 {
                        // Looks like a pointer
                        masked_data[i..i + 4].fill(0);
                    }
                }
            }
        }

        Ok(self.compute_raw_hash(&masked_data))
    }

    /// Compute hash focusing only on numeric content (floats)
    fn compute_numeric_hash(&self, data: &[u8]) -> Result<String> {
        use blake3::Hasher;
        let mut hasher = Hasher::new();

        // Extract potential float values and hash them
        // This is a heuristic approach looking for IEEE 754 float patterns
        for chunk in data.chunks_exact(4) {
            if let Ok(bytes) = chunk.try_into() {
                let val = f32::from_le_bytes(bytes);
                // Only include if it looks like a reasonable float
                if val.is_finite() && val.abs() < 1e6 {
                    hasher.update(chunk);
                }
            }
        }

        // Also try 8-byte doubles
        for chunk in data.chunks_exact(8) {
            if let Ok(bytes) = chunk.try_into() {
                let val = f64::from_le_bytes(bytes);
                if val.is_finite() && val.abs() < 1e6 {
                    hasher.update(chunk);
                }
            }
        }

        Ok(hex::encode(hasher.finalize().as_bytes()))
    }

    /// Try to infer the structure of a DATA block
    fn infer_data_block_structure<R: Read + Seek>(
        &self,
        data: &[u8],
        _file: &mut BlendFile<R>,
    ) -> Option<(Option<String>, Option<usize>, Option<usize>)> {
        // Heuristic inference of data block structure

        // Common patterns for mesh data
        let size = data.len();

        // Check for vertex data (typically 12 bytes per vertex: 3 floats)
        if size % 12 == 0 && size >= 12 {
            let vertex_count = size / 12;
            return Some((Some("MVert".to_string()), Some(vertex_count), Some(12)));
        }

        // Check for loop data (typically variable size)
        if size % 4 == 0 && size >= 4 {
            let element_count = size / 4;
            return Some((Some("MLoop".to_string()), Some(element_count), Some(4)));
        }

        // Check for polygon data
        if size % 8 == 0 && size >= 8 {
            let element_count = size / 8;
            return Some((Some("MPoly".to_string()), Some(element_count), Some(8)));
        }

        None
    }

    /// Correlate DATA blocks between before and after files
    pub fn correlate_data_blocks(
        &self,
        before_provenance: &ProvenanceGraph,
        after_provenance: &ProvenanceGraph,
    ) -> Vec<DataBlockCorrelation> {
        let mut correlations = Vec::new();

        // Create sets for matching
        let before_blocks: HashSet<_> = before_provenance.referenced_data_blocks.clone();
        let after_blocks: HashSet<_> = after_provenance.referenced_data_blocks.clone();

        // Find block pairs for correlation analysis
        let mut matched_blocks = HashSet::new();

        // Strategy 1: Try to match by size and content similarity
        for &before_idx in &before_blocks {
            if let Some(before_info) = before_provenance.data_block_info.get(&before_idx) {
                let mut best_match = None;
                let mut best_confidence = 0.0;

                for &after_idx in &after_blocks {
                    if matched_blocks.contains(&after_idx) {
                        continue;
                    }

                    if let Some(after_info) = after_provenance.data_block_info.get(&after_idx) {
                        let confidence =
                            self.compute_correlation_confidence(before_info, after_info);
                        if confidence > best_confidence && confidence > 0.6 {
                            best_match = Some(after_idx);
                            best_confidence = confidence;
                        }
                    }
                }

                if let Some(after_idx) = best_match {
                    matched_blocks.insert(after_idx);
                    let after_info = after_provenance.data_block_info.get(&after_idx).unwrap();

                    let change_class =
                        self.classify_data_change(before_info, after_info, best_confidence);

                    correlations.push(DataBlockCorrelation {
                        before_index: Some(before_idx),
                        after_index: Some(after_idx),
                        change_class,
                        confidence: best_confidence,
                        rationale: "Matched by size and content similarity".to_string(),
                    });
                }
            }
        }

        // Handle unmatched blocks
        for &before_idx in &before_blocks {
            if !correlations
                .iter()
                .any(|c| c.before_index == Some(before_idx))
            {
                correlations.push(DataBlockCorrelation {
                    before_index: Some(before_idx),
                    after_index: None,
                    change_class: DataChangeClass::SizeChange,
                    confidence: 1.0,
                    rationale: "Block removed or significantly changed".to_string(),
                });
            }
        }

        for &after_idx in &after_blocks {
            if !correlations
                .iter()
                .any(|c| c.after_index == Some(after_idx))
            {
                correlations.push(DataBlockCorrelation {
                    before_index: None,
                    after_index: Some(after_idx),
                    change_class: DataChangeClass::SizeChange,
                    confidence: 1.0,
                    rationale: "Block added or significantly changed".to_string(),
                });
            }
        }

        correlations
    }

    /// Compute confidence score for correlating two DATA blocks
    fn compute_correlation_confidence(
        &self,
        before_info: &DataBlockInfo,
        after_info: &DataBlockInfo,
    ) -> f32 {
        let mut confidence = 0.0;

        // Size similarity
        if before_info.size == after_info.size {
            confidence += 0.4;
        } else {
            let size_ratio = before_info.size.min(after_info.size) as f32
                / before_info.size.max(after_info.size) as f32;
            confidence += 0.2 * size_ratio;
        }

        // Element type and structure similarity
        if before_info.element_type == after_info.element_type && before_info.element_type.is_some()
        {
            confidence += 0.3;
        }

        // Hash similarity
        if let (Some(before_hash), Some(after_hash)) =
            (&before_info.numeric_hash, &after_info.numeric_hash)
        {
            if before_hash == after_hash {
                confidence += 0.3;
            }
        }

        confidence.min(1.0)
    }

    /// Classify the type of change between two DATA blocks
    fn classify_data_change(
        &self,
        before_info: &DataBlockInfo,
        after_info: &DataBlockInfo,
        _confidence: f32,
    ) -> DataChangeClass {
        // Size change is the most definitive signal
        if before_info.size != after_info.size {
            return DataChangeClass::SizeChange;
        }

        // Check numeric content if available
        if let (Some(before_numeric), Some(after_numeric)) =
            (&before_info.numeric_hash, &after_info.numeric_hash)
        {
            if before_numeric != after_numeric {
                return DataChangeClass::ContentChange;
            }
        }

        // Check pointer-masked content
        if let (Some(before_masked), Some(after_masked)) = (
            &before_info.pointer_masked_hash,
            &after_info.pointer_masked_hash,
        ) {
            if before_masked == after_masked {
                // Masked content is same but raw differs - layout change
                if before_info.raw_hash != after_info.raw_hash {
                    return DataChangeClass::LayoutChange;
                } else {
                    return DataChangeClass::Unchanged;
                }
            } else {
                return DataChangeClass::ContentChange;
            }
        }

        // Fallback to raw comparison
        if before_info.raw_hash == after_info.raw_hash {
            DataChangeClass::Unchanged
        } else {
            DataChangeClass::LayoutChange // Conservative assumption
        }
    }

    /// Analyze ME block changes with full provenance correlation
    pub fn analyze_mesh_changes<R1: Read + Seek, R2: Read + Seek>(
        &self,
        me_block_index: usize,
        before_file: &mut BlendFile<R1>,
        after_file: &mut BlendFile<R2>,
    ) -> Result<MeshAnalysisResult> {
        let before_provenance = self.extract_me_provenance(me_block_index, before_file).ok();
        let after_provenance = self.extract_me_provenance(me_block_index, after_file).ok();

        let data_correlations =
            if let (Some(ref before), Some(ref after)) = (&before_provenance, &after_provenance) {
                self.correlate_data_blocks(before, after)
            } else {
                Vec::new()
            };

        // Determine overall classification based on correlations
        let overall_classification = self.classify_overall_change(&data_correlations);
        let is_true_edit = self.is_likely_true_edit(&data_correlations);

        let summary = self.generate_summary(&data_correlations, &overall_classification);

        Ok(MeshAnalysisResult {
            me_block_index,
            before_provenance,
            after_provenance,
            data_correlations,
            overall_classification,
            is_true_edit,
            summary,
        })
    }

    fn classify_overall_change(&self, correlations: &[DataBlockCorrelation]) -> DataChangeClass {
        let mut has_content_change = false;
        let mut has_size_change = false;
        let mut has_layout_change = false;

        for correlation in correlations {
            match correlation.change_class {
                DataChangeClass::ContentChange => has_content_change = true,
                DataChangeClass::SizeChange => has_size_change = true,
                DataChangeClass::LayoutChange => has_layout_change = true,
                DataChangeClass::Unchanged => {}
            }
        }

        if has_size_change || has_content_change {
            DataChangeClass::SizeChange
        } else if has_layout_change {
            DataChangeClass::LayoutChange
        } else {
            DataChangeClass::Unchanged
        }
    }

    fn is_likely_true_edit(&self, correlations: &[DataBlockCorrelation]) -> bool {
        let high_confidence_content_changes = correlations
            .iter()
            .filter(|c| c.confidence > 0.7 && c.change_class == DataChangeClass::ContentChange)
            .count();

        let high_confidence_size_changes = correlations
            .iter()
            .filter(|c| c.confidence > 0.6 && c.change_class == DataChangeClass::SizeChange)
            .count();

        let total_correlations = correlations.len();

        // Calculate noise metrics first
        let unmatched_blocks = correlations
            .iter()
            .filter(|c| {
                c.confidence == 1.0 && (c.before_index.is_none() || c.after_index.is_none())
            })
            .count();

        let low_confidence_changes = correlations
            .iter()
            .filter(|c| c.confidence < 0.5 && c.confidence != 1.0)
            .count();

        let noise_blocks = unmatched_blocks + low_confidence_changes;
        let noise_ratio = if total_correlations > 0 {
            noise_blocks as f32 / total_correlations as f32
        } else {
            0.0
        };

        // Early rejection: if overwhelming noise, don't consider it a true edit regardless of content changes
        if noise_ratio > 0.9 {
            return false;
        }

        // Special case: if ALL blocks are unmatched, it's definitely noise
        if unmatched_blocks == total_correlations {
            return false;
        }

        // Different criteria for different scenarios:

        // Scenario 1: Strong evidence of mesh editing (multiple content changes OR content + size changes)
        if high_confidence_content_changes >= 3
            || (high_confidence_content_changes >= 2 && high_confidence_size_changes >= 2)
        {
            return true;
        }

        // Scenario 2: Moderate evidence with size changes (likely real geometry changes)
        if high_confidence_content_changes >= 2 && high_confidence_size_changes >= 1 {
            return true;
        }

        // Scenario 3: For very small datasets, be more lenient
        if total_correlations <= 10 && high_confidence_content_changes >= 2 {
            return true;
        }

        // Scenario 4: Single content change is almost always a false positive in complex files
        // unless accompanied by significant size changes
        if high_confidence_content_changes == 1 {
            // Only accept if there are substantial size changes indicating real geometry modification
            return high_confidence_size_changes >= 5;
        }

        // Scenario 5: Pure size changes without content changes are suspicious
        if high_confidence_content_changes == 0 {
            return false;
        }

        // Default: require at least some high-confidence changes
        high_confidence_content_changes > 0 || high_confidence_size_changes > 0
    }

    fn generate_summary(
        &self,
        correlations: &[DataBlockCorrelation],
        overall: &DataChangeClass,
    ) -> String {
        let total = correlations.len();
        let content_changes = correlations
            .iter()
            .filter(|c| c.change_class == DataChangeClass::ContentChange)
            .count();
        let size_changes = correlations
            .iter()
            .filter(|c| c.change_class == DataChangeClass::SizeChange)
            .count();
        let layout_changes = correlations
            .iter()
            .filter(|c| c.change_class == DataChangeClass::LayoutChange)
            .count();

        format!(
            "Analyzed {total} DATA blocks: {content_changes} content, {size_changes} size, {layout_changes} layout changes. Overall: {overall:?}"
        )
    }
}

impl Default for ProvenanceAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
