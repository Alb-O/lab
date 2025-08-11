//! Strongly-typed .blend file data model for thread-safe parsers/editors.
//!
//! This crate models Blender's .blend internals around SDNA, BHead blocks, pointers and IDs.
//! It focuses on a safe, opinionated Rust API that can back a parallel parser and CLI editor.

pub mod bhead;
pub mod block;
pub mod endian;
pub mod error;
pub mod header;
pub mod header_decode;
pub mod id;
pub mod index;
pub mod layout;
pub mod member;
pub mod pointer;
pub mod registry;
pub mod resolve;
pub mod sdna;
pub mod transform;
pub mod types;
pub mod view;

pub use bhead::{BHead, BHeadKind, BlockCode};
pub use block::Block;
pub use endian::{Endian, PtrWidth};
pub use error::{BlendModelError, Result};
pub use header::BlenderHeader;
pub use id::{IdClass, IdKind, IsId};
pub use member::{ArrayDims, MemberKind, MemberNameSpec};
pub use pointer::{OldPtr, OldPtrKey};
pub use registry::{BlockHandle, BlockRegistry, IdIndex};
pub use resolve::Resolver;
pub use sdna::{Sdna, StructMember, StructRef};
pub use transform::{Transform, extract_transform};
pub use view::StructView;
