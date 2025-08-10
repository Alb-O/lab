use crate::block::Block;
use crate::member::{MemberKind, MemberNameSpec};
use crate::sdna::{Sdna, StructMember};

/// Extracts old-pointer references from a block by walking its SDNA struct definition.
///
/// This inspects member declarations and collects pointers. It does not follow them.
pub fn collect_pointer_fields(sdna: &Sdna, block: &Block) -> Vec<(usize, u8)> {
    // Returns a list of (byte_offset, pointer_depth)
    // Note: A full implementation needs per-member offset computation using type sizes & alignment.
    // Here we only expose location candidates at a coarse granularity (member order), leaving
    // exact offset math to the parser stage that knows packing rules for the saved file's platform.
    if block.header.sdna_index < 0 {
        return Vec::new();
    }
    let mut out = Vec::new();
    let sidx = block.header.sdna_index as u32;
    if let Ok(struc) = sdna.struct_by_index(sidx) {
        for StructMember {
            type_index: _,
            member_index,
        } in struc.members.iter().copied()
        {
            if let Ok(MemberNameSpec {
                kind: MemberKind::Pointer(depth),
                ..
            }) = sdna.member_spec(member_index)
            {
                // Offset calculation is left to the reading layer; use member order index for now.
                out.push((out.len(), depth));
            }
        }
    }
    out
}
