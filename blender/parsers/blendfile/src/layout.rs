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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::endian::PtrWidth;
    use crate::sdna::{Sdna, StructMember, StructRef};
    use std::collections::HashMap;
    use std::sync::Arc;

    fn mk_sdna(
        pointer_size: PtrWidth,
        types: Vec<&str>,
        types_size: Vec<u16>,
        types_alignment: Vec<u32>,
        members: Vec<&str>,
        members_array_num: Vec<u16>,
        structs: Vec<(u32, Vec<(u32, u32)>)>,
    ) -> Sdna {
        let types: Arc<[Arc<str>]> = types
            .into_iter()
            .map(Arc::<str>::from)
            .collect::<Vec<_>>()
            .into();
        let types_size: Arc<[u16]> = types_size.into();
        let types_alignment: Arc<[u32]> = types_alignment.into();
        let members: Arc<[Arc<str>]> = members
            .into_iter()
            .map(Arc::<str>::from)
            .collect::<Vec<_>>()
            .into();
        let members_array_num: Arc<[u16]> = members_array_num.into();
        let structs: Arc<[StructRef]> = structs
            .into_iter()
            .map(|(type_index, mlist)| StructRef {
                type_index,
                members: mlist
                    .into_iter()
                    .map(|(t, m)| StructMember {
                        type_index: t,
                        member_index: m,
                    })
                    .collect::<Vec<_>>()
                    .into(),
            })
            .collect::<Vec<_>>()
            .into();
        let mut type_to_struct_index: HashMap<Arc<str>, u32> = HashMap::new();
        for (i, s) in structs.iter().enumerate() {
            let name = types[s.type_index as usize].clone();
            type_to_struct_index.insert(name, i as u32);
        }
        Sdna {
            pointer_size,
            types,
            types_size,
            types_alignment,
            structs,
            members,
            members_array_num,
            type_to_struct_index,
        }
    }

    #[test]
    fn layout_with_pointers_and_scalars() {
        // types: 0=float(4), 1=int(4), 2=Node, 3=HasPtr
        let sdna = mk_sdna(
            PtrWidth::P64,
            vec!["float", "int", "Node", "HasPtr"],
            vec![4, 4, 0, 0],
            vec![4, 4, 1, 1],
            vec!["*next", "val"],
            vec![1, 1],
            vec![
                // struct Node { /* empty for this test */ }
                (2, vec![]),
                // struct HasPtr { Node *next; int val; }
                (3, vec![(2, 0), (1, 1)]),
            ],
        );

        let layout = compute_struct_layout(&sdna, 1).unwrap();
        assert_eq!(layout.size % 8, 0, "struct aligned to pointer size");
        assert_eq!(layout.members.len(), 2);
        let m_next = &layout.members[0];
        assert_eq!(m_next.name, "next");
        assert_eq!(m_next.offset, 0);
        assert_eq!(m_next.size, 8);
        assert!(matches!(m_next.kind, MemberKind::Pointer(1)));

        let m_val = &layout.members[1];
        assert_eq!(m_val.name, "val");
        assert_eq!(m_val.size, 4);
        assert_eq!(m_val.align, 4);
        // offset should be 8 due to pointer size alignment
        assert_eq!(m_val.offset, 8);
    }

    #[test]
    fn layout_with_arrays_and_alignment() {
        // types: 0=float(4), 1=MatHolder
        // struct MatHolder { float mat[4][4]; }
        let sdna = mk_sdna(
            PtrWidth::P64,
            vec!["float", "MatHolder"],
            vec![4, 0],
            vec![4, 1],
            vec!["mat[4][4]"],
            vec![16],
            vec![(1, vec![(0, 0)])],
        );
        let layout = compute_struct_layout(&sdna, 0).unwrap();
        assert_eq!(layout.members.len(), 1);
        let m = &layout.members[0];
        assert_eq!(m.name, "mat");
        assert_eq!(m.size, 4 * 16);
        assert_eq!(m.align, 4);
        assert_eq!(m.offset, 0);
        assert_eq!(layout.size, 64);
    }
}
