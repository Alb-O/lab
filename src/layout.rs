use std::collections::HashMap;

use crate::error::Result;
use crate::member::{MemberKind, MemberNameSpec};
use crate::sdna::{Sdna, StructMember};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MemberLayout {
    pub index_in_struct: usize,
    pub name: String,
    pub offset: usize,
    pub size: usize,
    pub align: usize,
    pub kind: MemberKind,
    pub type_index: u32,
    pub array_len: usize,
}

#[derive(Clone, Debug)]
pub struct StructLayout {
    pub struct_index: u32,
    pub size: usize,
    pub members: Vec<MemberLayout>,
    pub index_by_name: HashMap<String, usize>,
}

fn align_up(off: usize, align: usize) -> usize {
    if align == 0 {
        return off;
    }
    (off + (align - 1)) & !(align - 1)
}

/// Compute a struct layout using SDNA sizes/alignments and pointer width.
pub fn compute_struct_layout(sdna: &Sdna, struct_index: u32) -> Result<StructLayout> {
    let sref = sdna.struct_by_index(struct_index)?;
    let struct_type_index = sref.type_index;
    let struct_size = sdna
        .types_size
        .get(struct_type_index as usize)
        .copied()
        .unwrap_or(0) as usize;

    let mut off = 0usize;
    let mut max_align = 1usize;
    let mut members_out = Vec::with_capacity(sref.members.len());
    let mut by_name = HashMap::new();

    for (
        i,
        StructMember {
            type_index,
            member_index,
        },
    ) in sref.members.iter().copied().enumerate()
    {
        let name = sdna.member_name(member_index)?.to_string();
        let spec = MemberNameSpec::parse(&name)?;

        let array_len = sdna
            .members_array_num
            .get(member_index as usize)
            .copied()
            .unwrap_or(1)
            .max(1) as usize;

        let (align, size_per_item) = match spec.kind {
            MemberKind::Pointer(_) => (sdna.pointer_size.bytes(), sdna.pointer_size.bytes()),
            MemberKind::Value => {
                let base_size = sdna
                    .types_size
                    .get(type_index as usize)
                    .copied()
                    .unwrap_or(0) as usize;
                let base_align = sdna
                    .types_alignment
                    .get(type_index as usize)
                    .copied()
                    .unwrap_or(1) as usize;
                (base_align.max(1), base_size.max(1))
            }
        };

        let size = size_per_item.saturating_mul(array_len);
        let align = align.max(1);
        off = align_up(off, align);
        max_align = max_align.max(align);

        members_out.push(MemberLayout {
            index_in_struct: i,
            name: spec.base.clone(),
            offset: off,
            size,
            align,
            kind: spec.kind,
            type_index,
            array_len,
        });

        by_name.entry(spec.base).or_insert(i);
        off = off.saturating_add(size);
    }

    let computed_size = align_up(off, max_align);
    let size = if struct_size != 0 {
        struct_size
    } else {
        computed_size
    };

    Ok(StructLayout {
        struct_index,
        size,
        members: members_out,
        index_by_name: by_name,
    })
}
