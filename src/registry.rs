use std::sync::Arc;

use ahash::RandomState;
use dashmap::DashMap;

use crate::block::Block;
use crate::pointer::{OldPtr, OldPtrKey};

/// Handle to a registered block (by allocation identity only).
#[derive(Clone, Debug)]
pub struct BlockHandle(pub Arc<Block>);

/// Global registry of blocks and ID indices, designed for concurrent access.
#[derive(Debug)]
pub struct BlockRegistry {
    by_old_ptr: DashMap<OldPtrKey, Arc<Block>, RandomState>,
}

impl Default for BlockRegistry {
    fn default() -> Self {
        Self {
            by_old_ptr: DashMap::with_hasher(RandomState::default()),
        }
    }
}

impl BlockRegistry {
    pub fn insert(&self, block: Arc<Block>) {
        self.by_old_ptr
            .insert(OldPtrKey::from(block.header.old), block);
    }
    pub fn get(&self, old: OldPtr) -> Option<Arc<Block>> {
        self.by_old_ptr
            .get(&OldPtrKey::from(old))
            .map(|r| r.clone())
    }
    pub fn len(&self) -> usize {
        self.by_old_ptr.len()
    }
    pub fn is_empty(&self) -> bool {
        self.by_old_ptr.is_empty()
    }
}

/// Indexes to resolve and navigate IDs by logical identity (type+name, etc.).
#[derive(Default, Debug)]
pub struct IdIndex {
    // Dual mapping can be added here: (IdKind, name) -> OldPtr, etc.
}
