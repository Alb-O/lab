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
