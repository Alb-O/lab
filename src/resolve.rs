use std::sync::Arc;

use ahash::AHashSet;

use crate::block::Block;
use crate::endian::{Endian, PtrWidth};
use crate::pointer::{OldPtr, OldPtrKey};
use crate::registry::BlockRegistry;
use crate::sdna::Sdna;
use crate::view::StructView;

#[derive(Clone, Debug)]
pub struct Resolver<'a> {
    pub sdna: &'a Sdna,
    pub registry: &'a BlockRegistry,
    pub endian: Endian,
    pub ptr_width: PtrWidth,
}

impl<'a> Resolver<'a> {
    pub fn view_for_block(&self, block: &'a Block) -> Option<StructView<'a>> {
        StructView::new(self.sdna, block, self.endian, self.ptr_width)
    }

    pub fn view_for_ptr(&self, ptr: OldPtr) -> Option<StructView<'a>> {
        let block = self.registry.get(ptr)?;
        // Safety: Arc<Block> lives beyond view as long as caller retains it. Here we borrow block
        // as &'a via temporary; in practice, parser should hold arcs while iterating.
        // For now, tie the lifetime to self ('a) by leaking the Arc or ensure caller holds Arc.
        // To avoid leaking, expose a variant returning Arc<Block> + view.
        let leaked: &'a Block = unsafe { &*(Arc::as_ptr(&block)) };
        self.view_for_block(leaked)
    }

    /// Traverse a ListBase nested member and collect node views.
    /// - `listbase_field`: name of the ListBase member in `owner`.
    /// - `node_next_field`: typically "next"; used to follow the chain.
    /// - `node_struct_name`: optional asserted node struct type name.
    pub fn listbase_items(
        &self,
        owner: &StructView<'a>,
        listbase_field: &str,
        node_next_field: &str,
        node_struct_name: Option<&str>,
    ) -> Vec<StructView<'a>> {
        let mut out = Vec::new();
        let Some(lb_view) = owner.at_member_struct(listbase_field) else {
            return out;
        };
        let Some(first_ptr) = lb_view.get_ptr("first") else {
            return out;
        };
        self.list_traverse_from_first(first_ptr, node_next_field, node_struct_name, &mut out);
        out
    }

    pub fn list_traverse_from_first(
        &self,
        mut cur_ptr: OldPtr,
        node_next_field: &str,
        node_struct_name: Option<&str>,
        out: &mut Vec<StructView<'a>>,
    ) {
        let mut seen: AHashSet<OldPtrKey> = AHashSet::new();
        while !cur_ptr.is_null() {
            if !seen.insert(OldPtrKey::from(cur_ptr)) {
                break;
            } // cycle protection
            let Some(view) = self.view_for_ptr(cur_ptr) else {
                break;
            };
            if let Some(expect) = node_struct_name {
                // Check struct name matches.
                if let Some(sname) = self
                    .sdna
                    .struct_by_index(view.layout.struct_index)
                    .ok()
                    .and_then(|s| self.sdna.type_name(s.type_index).ok())
                {
                    if sname != expect {
                        break;
                    }
                }
            }
            cur_ptr = view.get_ptr(node_next_field).unwrap_or(OldPtr::Null);
            out.push(view);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bhead::{BHead, BHeadKind, BlockCode};
    use crate::block::Block;
    use crate::registry::BlockRegistry;
    use crate::sdna::{Sdna, StructMember, StructRef};
    use std::collections::HashMap;
    use std::sync::Arc;

    fn mk_sdna() -> Sdna {
        // types: 0=Node, 1=ListBase, 2=Owner
        let types: Arc<[Arc<str>]> = vec!["Node", "ListBase", "Owner"]
            .into_iter()
            .map(Arc::<str>::from)
            .collect::<Vec<_>>()
            .into();
        let types_size: Arc<[u16]> = vec![0u16, 0, 0].into();
        let types_alignment: Arc<[u32]> = vec![1u32, 1, 1].into();
        let members: Arc<[Arc<str>]> = vec!["*next", "*first", "*last", "lb"]
            .into_iter()
            .map(Arc::<str>::from)
            .collect::<Vec<_>>()
            .into();
        let members_array_num: Arc<[u16]> = vec![1u16, 1, 1, 1].into();
        let structs: Arc<[StructRef]> = vec![
            StructRef {
                type_index: 0,
                members: vec![StructMember {
                    type_index: 0,
                    member_index: 0,
                }]
                .into(),
            },
            StructRef {
                type_index: 1,
                members: vec![
                    StructMember {
                        type_index: 0,
                        member_index: 1,
                    },
                    StructMember {
                        type_index: 0,
                        member_index: 2,
                    },
                ]
                .into(),
            },
            StructRef {
                type_index: 2,
                members: vec![StructMember {
                    type_index: 1,
                    member_index: 3,
                }]
                .into(),
            },
        ]
        .into();
        let mut type_to_struct_index: HashMap<Arc<str>, u32> = HashMap::new();
        for (i, s) in structs.iter().enumerate() {
            let name = types[s.type_index as usize].clone();
            type_to_struct_index.insert(name, i as u32);
        }
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

    fn le64(x: u64) -> [u8; 8] {
        x.to_le_bytes()
    }

    #[test]
    fn listbase_traversal_linear_and_cycle_protection() {
        let sdna = mk_sdna();
        let registry = BlockRegistry::default();
        // Create three Node blocks with next pointers.
        let addr1 = OldPtr::Ptr64(0x1000);
        let addr2 = OldPtr::Ptr64(0x2000);
        let addr3 = OldPtr::Ptr64(0x3000);
        let make_node = |old: OldPtr, next: OldPtr| -> Arc<Block> {
            let mut data = vec![0u8; 8];
            match next {
                OldPtr::Ptr64(v) => data[..8].copy_from_slice(&le64(v)),
                OldPtr::Ptr32(v) => data[..4].copy_from_slice(&v.to_le_bytes()),
                OldPtr::Null => {}
            }
            Arc::new(Block {
                header: BHead {
                    code: BlockCode::TEST,
                    sdna_index: 0, // Node struct index
                    old,
                    len: data.len() as i64,
                    count: 1,
                    kind: BHeadKind::LargeBHead8,
                },
                data: data.into(),
            })
        };
        let n1 = make_node(addr1, addr2);
        let n2 = make_node(addr2, addr3);
        let n3 = make_node(addr3, OldPtr::Null);
        registry.insert(n1);
        registry.insert(n2);
        registry.insert(n3.clone());

        // Owner block with ListBase.lb.first = addr1
        let mut owner_bytes = vec![0u8; 8 * 2];
        owner_bytes[0..8].copy_from_slice(&le64(match addr1 {
            OldPtr::Ptr64(v) => v,
            _ => 0,
        }));
        owner_bytes[8..16].copy_from_slice(&le64(match OldPtr::Ptr64(0x3000) {
            OldPtr::Ptr64(v) => v,
            _ => 0,
        }));
        let owner = Block {
            header: BHead {
                code: BlockCode::TEST,
                sdna_index: 2, // Owner struct index
                old: OldPtr::Null,
                len: owner_bytes.len() as i64,
                count: 1,
                kind: BHeadKind::LargeBHead8,
            },
            data: owner_bytes.into(),
        };

        let resolver = Resolver {
            sdna: &sdna,
            registry: &registry,
            endian: Endian::Little,
            ptr_width: PtrWidth::P64,
        };
        let owner_view = resolver.view_for_block(&owner).unwrap();
        let items = resolver.listbase_items(&owner_view, "lb", "next", Some("Node"));
        assert_eq!(items.len(), 3);
        // Now create a cycle: point n3.next back to n2
        let n3_cycled = Arc::new(Block {
            header: BHead {
                code: BlockCode::TEST,
                sdna_index: 0,
                old: addr3,
                len: 8,
                count: 1,
                kind: BHeadKind::LargeBHead8,
            },
            data: Arc::from(le64(match addr2 {
                OldPtr::Ptr64(v) => v,
                _ => 0,
            })),
        });
        registry.insert(n3_cycled);
        let items = resolver.listbase_items(&owner_view, "lb", "next", Some("Node"));
        // Should stop on visiting a repeated node due to cycle protection
        assert!(items.len() >= 2);
    }
}
