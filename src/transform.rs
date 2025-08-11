use crate::view::StructView;

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, Default)]
pub struct Transform {
    pub loc: Option<[f32; 3]>,
    pub rot_euler: Option<[f32; 3]>,
    pub quat: Option<[f32; 4]>,
    pub scale: Option<[f32; 3]>,
}

pub fn extract_transform(view: &StructView<'_>) -> Transform {
    let loc = view
        .get_f32_array("loc")
        .and_then(|v| (v.len() >= 3).then(|| [v[0], v[1], v[2]]));
    let scale = view
        .get_f32_array("size")
        .or_else(|| view.get_f32_array("scale"))
        .and_then(|v| (v.len() >= 3).then(|| [v[0], v[1], v[2]]));
    let rot_euler = view
        .get_f32_array("rot")
        .or_else(|| view.get_f32_array("rot_euler"))
        .and_then(|v| (v.len() >= 3).then(|| [v[0], v[1], v[2]]));
    let quat = view
        .get_f32_array("quat")
        .and_then(|v| (v.len() >= 4).then(|| [v[0], v[1], v[2], v[3]]));

    Transform {
        loc,
        rot_euler,
        quat,
        scale,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bhead::{BHead, BHeadKind, BlockCode};
    use crate::block::Block;
    use crate::endian::{Endian, PtrWidth};
    use crate::layout::compute_struct_layout;
    use crate::sdna::{Sdna, StructMember, StructRef};
    use std::collections::HashMap;
    use std::sync::Arc;

    fn mk_sdna() -> Sdna {
        // types: 0=float, 1=Object
        let types: Arc<[Arc<str>]> = vec!["float", "Object"]
            .into_iter()
            .map(Arc::<str>::from)
            .collect::<Vec<_>>()
            .into();
        let types_size: Arc<[u16]> = vec![4u16, 0].into();
        let types_alignment: Arc<[u32]> = vec![4u32, 1].into();
        let members: Arc<[Arc<str>]> = vec!["loc[3]", "size[3]", "rot_euler[3]", "quat[4]"]
            .into_iter()
            .map(Arc::<str>::from)
            .collect::<Vec<_>>()
            .into();
        let members_array_num: Arc<[u16]> = vec![3u16, 3, 3, 4].into();
        let structs: Arc<[StructRef]> = vec![StructRef {
            type_index: 1,
            members: vec![
                StructMember {
                    type_index: 0,
                    member_index: 0,
                },
                StructMember {
                    type_index: 0,
                    member_index: 1,
                },
                StructMember {
                    type_index: 0,
                    member_index: 2,
                },
                StructMember {
                    type_index: 0,
                    member_index: 3,
                },
            ]
            .into(),
        }]
        .into();
        let mut type_to_struct_index: HashMap<Arc<str>, u32> = HashMap::new();
        type_to_struct_index.insert(types[1].clone(), 0);
        Sdna {
            pointer_size: PtrWidth::P64,
            types,
            types_size,
            types_alignment,
            structs,
            members,
            members_array_num,
            type_to_struct_index,
        }
    }

    fn f32_le_bytes(vals: &[f32]) -> Vec<u8> {
        let mut out = Vec::with_capacity(vals.len() * 4);
        for &v in vals {
            out.extend_from_slice(&v.to_bits().to_le_bytes());
        }
        out
    }

    #[test]
    fn extract_various_transform_fields() {
        let sdna = mk_sdna();
        let layout = compute_struct_layout(&sdna, 0).unwrap();
        let off = |name: &str| layout.members[*layout.index_by_name.get(name).unwrap()].offset;

        // Case 1: only loc and quat
        let mut bytes = vec![0u8; off("quat") + 16];
        bytes[off("loc")..off("loc") + 12].copy_from_slice(&f32_le_bytes(&[1.0, 2.0, 3.0]));
        bytes[off("quat")..off("quat") + 16].copy_from_slice(&f32_le_bytes(&[0.0, 0.0, 0.0, 1.0]));
        let block = Block {
            header: BHead {
                code: BlockCode::TEST,
                sdna_index: 0,
                old: crate::pointer::OldPtr::Null,
                len: bytes.len() as i64,
                count: 1,
                kind: BHeadKind::LargeBHead8,
            },
            data: bytes.clone().into(),
        };
        let view = StructView::new(&sdna, &block, Endian::Little, PtrWidth::P64).unwrap();
        let t = extract_transform(&view);
        assert_eq!(t.loc, Some([1.0, 2.0, 3.0]));
        assert_eq!(t.quat, Some([0.0, 0.0, 0.0, 1.0]));
        assert_eq!(t.rot_euler, Some([0.0, 0.0, 0.0]));
        assert_eq!(t.scale, Some([0.0, 0.0, 0.0]));

        // Case 2: rot_euler and size
        let mut bytes2 = vec![0u8; layout.size.max(off("rot_euler") + 12).max(off("size") + 12)];
        bytes2[off("rot_euler")..off("rot_euler") + 12]
            .copy_from_slice(&f32_le_bytes(&[0.1, 0.2, 0.3]));
        bytes2[off("size")..off("size") + 12].copy_from_slice(&f32_le_bytes(&[2.0, 2.0, 2.0]));
        let block2 = Block {
            header: BHead {
                code: BlockCode::TEST,
                sdna_index: 0,
                old: crate::pointer::OldPtr::Null,
                len: bytes2.len() as i64,
                count: 1,
                kind: BHeadKind::LargeBHead8,
            },
            data: bytes2.into(),
        };
        let view2 = StructView::new(&sdna, &block2, Endian::Little, PtrWidth::P64).unwrap();
        let t2 = extract_transform(&view2);
        assert_eq!(t2.rot_euler, Some([0.1, 0.2, 0.3]));
        assert_eq!(t2.scale, Some([2.0, 2.0, 2.0]));
        assert_eq!(t2.loc, Some([0.0, 0.0, 0.0]));
        assert_eq!(t2.quat, Some([0.0, 0.0, 0.0, 0.0]));
    }
}
