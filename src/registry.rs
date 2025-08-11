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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bhead::{BHead, BHeadKind, BlockCode};
    use std::sync::Arc;
    use std::thread;

    fn mk_block_with_old(old: OldPtr) -> Arc<Block> {
        Arc::new(Block {
            header: BHead {
                code: BlockCode::TEST,
                sdna_index: -1,
                old,
                len: 0,
                count: 1,
                kind: BHeadKind::SmallBHead8,
            },
            data: Arc::from([]),
        })
    }

    #[test]
    fn basic_insert_and_get() {
        let reg = BlockRegistry::default();
        assert!(reg.is_empty());
        let b = mk_block_with_old(OldPtr::Ptr64(0xABCDEF));
        reg.insert(b.clone());
        assert_eq!(reg.len(), 1);
        let got = reg.get(OldPtr::Ptr64(0xABCDEF)).unwrap();
        assert!(Arc::ptr_eq(&got, &b));
    }

    #[test]
    fn concurrent_inserts_no_loss() {
        let reg = Arc::new(BlockRegistry::default());
        let threads = 4;
        let per_thread = 1000;
        let mut handles = Vec::new();
        for t in 0..threads {
            let reg = reg.clone();
            handles.push(thread::spawn(move || {
                for i in 0..per_thread {
                    let addr = ((t as u64) << 32) | i as u64;
                    let b = mk_block_with_old(OldPtr::Ptr64(addr));
                    reg.insert(b);
                }
            }));
        }
        for h in handles {
            h.join().unwrap();
        }
        assert_eq!(reg.len(), threads * per_thread);
        // Spot-check a few
        assert!(reg.get(OldPtr::Ptr64(0)).is_some());
        assert!(
            reg.get(OldPtr::Ptr64(
                ((threads - 1) as u64) << 32 | (per_thread - 1) as u64
            ))
            .is_some()
        );
    }
}
