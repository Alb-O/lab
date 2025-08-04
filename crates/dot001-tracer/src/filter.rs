use dot001_error::{Dot001Error, Result as UnifiedResult, TracerErrorKind};
use dot001_parser::{BlendFile, BlendFileBlock, Result};
use regex::Regex;
use std::collections::HashSet;
use std::io::{Read, Seek};

/// Filter rule similar to Blender's `--filter-block`
/// include: true for '+', false for '-'
/// recursion: None for no recursion, Some(usize::MAX) for infinite, Some(n) for bounded
#[derive(Debug, Clone)]
pub struct FilterRule {
    pub include: bool,
    pub recursion: Option<usize>,
    pub key_regex: Regex,
    pub value_regex: Regex,
}

/// A set of rules evaluated in order (first match wins when exclude, include may propagate recursion)
#[derive(Debug, Default, Clone)]
pub struct FilterSpec {
    pub rules: Vec<FilterRule>,
}

#[derive(Debug)]
pub struct BlockMetaView {
    pub header_offset: u64,
    pub data_offset: u64,
    pub code_str: String,
    pub dna_index: u32,
    pub count: u32,
    pub size: u32,
    pub old_address: u64,
}

/// Data fields exposed for matching (stringified, best-effort)
/// For now we only provide a small bag of common keys for matching and leave full reflection to later phases.
#[derive(Debug)]
pub struct BlockDataView {
    /// key -> value (string) pairs
    pub pairs: Vec<(String, String)>,
}

/// Per-block filtering state (similar to Blender's user_data marks)
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum BlockMark {
    /// Unseen
    #[default]
    None,
    /// Included with the iteration depth at which it was added
    Included(usize),
    /// Excluded at a given rule index
    Excluded(usize),
}

/// Engine responsible for applying a FilterSpec on a BlendFile
pub struct FilterEngine {
    /// When true, treat any include rule with recursion: Some(usize::MAX) as infinite recursion
    pub allow_infinite_recursion: bool,
    /// Maximum global expansion to avoid explosions (safety valve)
    pub hard_cap_inclusions: usize,
}

impl Default for FilterEngine {
    fn default() -> Self {
        Self {
            allow_infinite_recursion: true,
            hard_cap_inclusions: 1_000_000,
        }
    }
}

impl FilterEngine {
    pub fn new() -> Self {
        Self::default()
    }

    /// Apply filter rules and return indices of blocks that pass (included and not excluded).
    /// This mirrors Blender's behavior:
    /// - Rules evaluated in order.
    /// - Exclude rules short-circuit on match.
    /// - Include rules can set marks and optionally recurse via pointers up to recursion depth.
    pub fn apply<R: Read + Seek>(
        &self,
        spec: &FilterSpec,
        blend: &mut BlendFile<R>,
    ) -> Result<HashSet<usize>> {
        let mut marks: Vec<BlockMark> = vec![BlockMark::None; blend.blocks.len()];
        let mut include_queue: Vec<(usize, usize)> = Vec::new(); // (block_index, current_depth)
        let mut included: HashSet<usize> = HashSet::new();
        let mut processed_count = 0usize;

        // First pass: evaluate rules against all blocks, enqueue recursive includes as needed.
        for (i, mark) in marks.iter_mut().enumerate() {
            let meta = Self::meta_view(&blend.blocks[i]);
            let data = Self::data_view_minimal(blend, i)?;

            let mut matched_include: Option<usize> = None;
            let mut matched_exclude: Option<usize> = None;
            let mut matched_recursion: Option<usize> = None;

            for (rule_idx, rule) in spec.rules.iter().enumerate() {
                if Self::rule_matches_rule(rule, &meta, &data) {
                    if rule.include {
                        matched_include = Some(rule_idx);
                        let depth = match rule.recursion {
                            None => 0,
                            Some(n) if n == usize::MAX && self.allow_infinite_recursion => {
                                usize::MAX
                            }
                            Some(n) => n,
                        };
                        matched_recursion = Some(depth);
                    } else {
                        matched_exclude = Some(rule_idx);
                        break;
                    }
                }
            }

            if let Some(exclude_idx) = matched_exclude {
                *mark = BlockMark::Excluded(exclude_idx);
                continue;
            }

            if let Some(_inc_idx) = matched_include {
                *mark = BlockMark::Included(0);
                included.insert(i);
                processed_count += 1;
                if processed_count >= self.hard_cap_inclusions {
                    break;
                }
                if let Some(depth) = matched_recursion {
                    if depth != 0 {
                        include_queue.push((i, depth));
                    }
                }
            }
        }

        // Recursively include through pointer fields if requested by include rules.
        while let Some((block_index, cur_depth)) = include_queue.pop() {
            if processed_count >= self.hard_cap_inclusions {
                break;
            }
            let next_depth = match Self::next_depth(cur_depth) {
                Some(d) => d,
                None => continue,
            };
            // Traverse pointer fields on this block and include targets
            {
                let targets = Self::pointer_targets(blend, block_index)?;
                for target in targets {
                    if included.insert(target) {
                        marks[target] = BlockMark::Included(next_depth);
                        processed_count += 1;
                        // Enqueue next if we still have recursion to spend
                        if next_depth != 0 {
                            include_queue.push((target, next_depth));
                        }
                    }
                }
            }
        }

        // If no include rules, include all by default (Blender behavior).
        let has_include = spec.rules.iter().any(|r| r.include);
        if !has_include {
            included = (0..blend.blocks.len()).collect();
        }

        // Remove explicitly excluded blocks
        for (i, mark) in marks.iter().enumerate() {
            if matches!(mark, BlockMark::Excluded(_)) {
                included.remove(&i);
            }
        }

        Ok(included)
    }

    fn next_depth(cur: usize) -> Option<usize> {
        if cur == usize::MAX {
            Some(usize::MAX)
        } else if cur == 0 {
            None
        } else {
            Some(cur.saturating_sub(1))
        }
    }

    fn meta_view(block: &BlendFileBlock) -> BlockMetaView {
        let code_str = String::from_utf8_lossy(&block.header.code)
            .trim_end_matches('\0')
            .to_string();
        BlockMetaView {
            header_offset: block.header_offset,
            data_offset: block.data_offset,
            code_str,
            dna_index: block.header.sdna_index,
            count: block.header.count,
            size: block.header.size,
            old_address: block.header.old_address,
        }
    }

    fn data_view_minimal<R: Read + Seek>(
        blend: &mut BlendFile<R>,
        block_index: usize,
    ) -> Result<BlockDataView> {
        // Minimal, fast data pairs. We avoid full reflection now.
        // Includes a few common keys that are cheap to obtain and useful for regex matching.
        let mut pairs: Vec<(String, String)> = Vec::with_capacity(8);
        // Copy header primitives without holding an immutable borrow across &mut calls
        let (code_bytes, size_v, count_v, sdna_v, old_addr_v) = {
            let h = &blend.blocks[block_index].header;
            (h.code, h.size, h.count, h.sdna_index, h.old_address)
        };

        // Meta mirrored as data for convenience
        pairs.push((
            "code".into(),
            String::from_utf8_lossy(&code_bytes)
                .trim_end_matches('\0')
                .to_string(),
        ));
        pairs.push(("size".into(), size_v.to_string()));
        pairs.push(("count".into(), count_v.to_string()));
        pairs.push(("sdna_index".into(), sdna_v.to_string()));
        pairs.push(("addr_old".into(), format!("{old_addr_v:#x}")));

        // Optionally, try to read an ID name if the block likely starts with ID
        // This is a fast path used by many filters (e.g., name).
        if let Ok(data) = blend.read_block_data(block_index) {
            if let Ok(reader) = blend.create_field_reader(&data) {
                if let Ok(name) = reader.read_field_string("ID", "name") {
                    let trimmed = name.trim_end_matches('\0').to_string();
                    if !trimmed.is_empty() {
                        pairs.push(("name".into(), trimmed));
                    }
                }
            }
        }

        Ok(BlockDataView { pairs })
    }

    fn rule_matches_rule(rule: &FilterRule, meta: &BlockMetaView, data: &BlockDataView) -> bool {
        // Try meta fields
        for (k, v) in [
            ("code", meta.code_str.as_str()),
            ("size", &meta.size.to_string()),
            ("file_offset", &meta.header_offset.to_string()),
            ("addr_old", &format!("{:#x}", meta.old_address)),
            ("dna_index", &meta.dna_index.to_string()),
            ("count", &meta.count.to_string()),
        ] {
            if rule.key_regex.is_match(k) && rule.value_regex.is_match(v) {
                return true;
            }
        }
        // Try data pairs
        for (k, v) in &data.pairs {
            if rule.key_regex.is_match(k) && rule.value_regex.is_match(v) {
                return true;
            }
        }
        false
    }

    /// Enumerate pointer targets from a block by inspecting common pointer fields.
    /// This is a pragmatic subset until full DNA reflection iteration lands.
    fn pointer_targets<R: Read + Seek>(
        blend: &mut BlendFile<R>,
        block_index: usize,
    ) -> Result<Vec<usize>> {
        let mut out = Vec::new();
        // Copy code bytes and pointer_size locally to avoid holding an immutable borrow on blend
        let (code, pointer_size) = {
            let h = &blend.blocks[block_index].header;
            (h.code, blend.header.pointer_size)
        };

        // Heuristics over known structures to discover pointer fields:
        // Object: data, mat (array)
        if code == *b"OB\0\0" {
            // data pointer
            {
                let data = blend.read_block_data(block_index)?;
                let reader = blend.create_field_reader(&data)?;
                if let Ok(ptr) = reader.read_field_pointer("Object", "data") {
                    Self::push_addr(blend, &mut out, ptr);
                }
            }
            // totcol and mat pointer
            let (totcol, mats_ptr) = {
                let data = blend.read_block_data(block_index)?;
                let reader = blend.create_field_reader(&data)?;
                (
                    reader.read_field_u32("Object", "totcol").ok(),
                    reader.read_field_pointer("Object", "mat").ok(),
                )
            };
            if let (Some(totcol), Some(mats_ptr)) = (totcol, mats_ptr) {
                if totcol > 0 {
                    let mats_index_opt = blend.find_block_by_address(mats_ptr);
                    if let Some(mats_index) = mats_index_opt {
                        let mats_data = blend.read_block_data(mats_index)?;
                        let mr = blend.create_field_reader(&mats_data)?;
                        let stride = pointer_size as usize;
                        let mut mat_ptrs = Vec::with_capacity(totcol as usize);
                        for i in 0..totcol {
                            if let Ok(p) = mr.read_pointer(i as usize * stride) {
                                mat_ptrs.push(p);
                            }
                        }
                        for p in mat_ptrs {
                            Self::push_addr(blend, &mut out, p);
                        }
                    }
                }
            }
        }

        // Mesh: mat array, common geometry arrays
        if code == *b"ME\0\0" {
            let (totcol, mats_ptr) = {
                let data = blend.read_block_data(block_index)?;
                let reader = blend.create_field_reader(&data)?;
                (
                    reader.read_field_u32("Mesh", "totcol").ok(),
                    reader.read_field_pointer("Mesh", "mat").ok(),
                )
            };
            if let (Some(totcol), Some(mats_ptr)) = (totcol, mats_ptr) {
                if totcol > 0 {
                    let mats_index_opt = blend.find_block_by_address(mats_ptr);
                    if let Some(mats_index) = mats_index_opt {
                        let mats_data = blend.read_block_data(mats_index)?;
                        let mr = blend.create_field_reader(&mats_data)?;
                        let stride = pointer_size as usize;
                        let mut mat_ptrs = Vec::with_capacity(totcol as usize);
                        for i in 0..totcol {
                            if let Ok(p) = mr.read_pointer(i as usize * stride) {
                                mat_ptrs.push(p);
                            }
                        }
                        for p in mat_ptrs {
                            Self::push_addr(blend, &mut out, p);
                        }
                    }
                }
            }
            for field in ["vert", "edge", "poly", "loop"] {
                let p = {
                    let data = blend.read_block_data(block_index)?;
                    let reader = blend.create_field_reader(&data)?;
                    reader.read_field_pointer("Mesh", field).ok()
                };
                if let Some(ptr) = p {
                    Self::push_addr(blend, &mut out, ptr);
                }
            }
        }

        // Collections: gobject and children listbases
        if code == *b"GR\0\0" || code == *b"DATA" {
            // gobject pointer (Collection or Group)
            let gobject_p = {
                let data = blend.read_block_data(block_index)?;
                let reader = blend.create_field_reader(&data)?;
                reader
                    .read_field_pointer("Collection", "gobject")
                    .or_else(|_| reader.read_field_pointer("Group", "gobject"))
                    .ok()
            };
            if let Some(p) = gobject_p {
                Self::push_addr(blend, &mut out, p);
            }
            // children pointer (Collection)
            let children_p = {
                let data = blend.read_block_data(block_index)?;
                let reader = blend.create_field_reader(&data)?;
                reader.read_field_pointer("Collection", "children").ok()
            };
            if let Some(p) = children_p {
                Self::push_addr(blend, &mut out, p);
            }
        }

        // NodeTree: id in nodes
        if code == *b"NT\0\0" || code == *b"DATA" {
            // Best-effort: try to resolve nodes.first and then traverse as in expander
            let nodes_ptr = {
                let data = blend.read_block_data(block_index)?;
                let reader = blend.create_field_reader(&data)?;
                reader
                    .read_field_pointer("bNodeTree", "nodes")
                    .or_else(|_| reader.read_field_pointer("NodeTree", "nodes"))
                    .ok()
            };
            if let Some(nodes_ptr) = nodes_ptr {
                let lb_index_opt = blend.find_block_by_address(nodes_ptr);
                if let Some(lb_index) = lb_index_opt {
                    let lb_data = blend.read_block_data(lb_index)?;
                    let lr = blend.create_field_reader(&lb_data)?;
                    if let Ok(first) = lr.read_field_pointer("ListBase", "first") {
                        // Traverse a few nodes and collect id pointers
                        let mut cur = first;
                        let mut guard = 0usize;
                        while cur != 0 && guard < 256 {
                            guard += 1;
                            let nidx_opt = blend.find_block_by_address(cur);
                            if let Some(nidx) = nidx_opt {
                                let nd = blend.read_block_data(nidx)?;
                                let nr = blend.create_field_reader(&nd)?;
                                if let Ok(idp) = nr.read_field_pointer("bNode", "id") {
                                    Self::push_addr(blend, &mut out, idp);
                                }
                                if let Ok(nextp) = nr.read_field_pointer("bNode", "next") {
                                    cur = nextp;
                                    continue;
                                }
                            }
                            break;
                        }
                    }
                }
            }
        }

        Ok(out)
    }

    fn push_addr<R: Read + Seek>(blend: &BlendFile<R>, out: &mut Vec<usize>, addr: u64) {
        if addr == 0 {
            return;
        }
        if let Some(idx) = blend.find_block_by_address(addr) {
            out.push(idx);
        }
    }
}

/// Utility to build FilterSpec from CLI-like triples
pub fn build_filter_spec(triples: &[(&str, &str, &str)]) -> UnifiedResult<FilterSpec> {
    let mut spec = FilterSpec { rules: Vec::new() };
    for (modif, key, val) in triples {
        let mut chars = modif.chars();
        let sign = chars.next().ok_or_else(|| {
            Dot001Error::tracer(
                "Empty filter modifier".to_string(),
                TracerErrorKind::DependencyResolutionFailed,
            )
        })?;
        let include = match sign {
            '+' => true,
            '-' => false,
            _ => {
                return Err(Dot001Error::tracer(
                    format!("Invalid filter modifier: {modif}"),
                    TracerErrorKind::DependencyResolutionFailed,
                ))
            }
        };
        // Recursion parse
        let recursion = if include {
            let rest: String = chars.collect();
            if rest.is_empty() {
                None
            } else if rest == "*" {
                Some(usize::MAX)
            } else {
                let n = rest.parse::<usize>().map_err(|_| {
                    Dot001Error::tracer(
                        format!("Invalid recursion level: {rest}"),
                        TracerErrorKind::DependencyResolutionFailed,
                    )
                })?;
                Some(n)
            }
        } else {
            None
        };

        let key_regex = Regex::new(key).map_err(|e| {
            Dot001Error::tracer(
                format!("Invalid key regex: {e}"),
                TracerErrorKind::DependencyResolutionFailed,
            )
        })?;
        let value_regex = Regex::new(val).map_err(|e| {
            Dot001Error::tracer(
                format!("Invalid value regex: {e}"),
                TracerErrorKind::DependencyResolutionFailed,
            )
        })?;

        spec.rules.push(FilterRule {
            include,
            recursion,
            key_regex,
            value_regex,
        });
    }
    Ok(spec)
}
