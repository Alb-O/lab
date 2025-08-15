use std::sync::Arc;

use crate::bhead::{BHead, BlockCode};

/// A block couples a normalized BHead with its raw payload bytes.
/// The payload is not interpreted here; typed views are provided by users based on SDNA.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub struct Block {
    pub header: BHead,
    pub data: Arc<[u8]>,
}

impl Block {
    pub fn code(&self) -> BlockCode {
        self.header.code
    }
    pub fn len(&self) -> i64 {
        self.header.len
    }
    pub fn is_empty(&self) -> bool {
        self.data.is_empty() || self.header.len == 0
    }
}
