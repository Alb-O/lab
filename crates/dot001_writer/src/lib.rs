//! # dot001_writer
//!
//! **WARNING: This library is experimental and not suitable for production use.**
//!
//! A library for creating synthetic .blend files and injecting blocks from existing files.
//! Block injection is complex due to Blender's internal pointer systems and is prone to crashes.
//!
//! ## Current Status
//!
//! - **Simple Materials**: Basic functionality works
//! - **Complex Materials with NodeTrees**: Partially working, may have empty node trees
//! - **Objects and Meshes**: Frequently crash due to unresolved pointer issues
//! - **Collections**: Limited testing, basic cases may work
//!
//! ## Available Approaches
//!
//! ### Safe Injection
//! Attempts to prevent crashes by sanitizing dangerous pointers.
//! ```rust,no_run
//! use dot001_writer::{SeedDnaProvider, BlendWriter, WriteTemplate, SafeBlockInjection};
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! #     let mut seed = SeedDnaProvider::from_seed_path("source.blend")?;
//! #     let injection = SafeBlockInjection::from_block_indices_with_safe_handling(
//! #         &mut seed,
//! #         &[1223, 1225] // Material + NodeTree blocks
//! #     )?;
//! #     Ok(())
//! # }
//! ```
//!
//! ### Exhaustive Injection (Highly Experimental)
//! Attempts to trace complete dependency trees but is unstable.
//! ```rust,no_run
//! use dot001_writer::{SeedDnaProvider, ExhaustivePointerTracer};
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! #     let mut seed = SeedDnaProvider::from_seed_path("source.blend")?;
//! #     let injection = ExhaustivePointerTracer::trace_complete_dependencies(&mut seed, &[1223])?;
//! #     Ok(())
//! # }
//! ```
//!
//! ## Known Issues
//!
//! - Crashes with Object/Mesh combinations
//! - NodeTrees may load empty even when dependencies are included  
//! - Complex internal pointer structures not fully understood
//! - Limited testing across different .blend file versions

pub mod dna_provider;
pub mod emitter;
pub mod exhaustive_tracer;
pub mod expanded_injection;
pub mod header_writer;
pub mod safe_injection;

pub use dna_provider::SeedDnaProvider;
pub use emitter::{BlendWriter, BlockInjection, InjectableBlock, WriteTemplate};
pub use exhaustive_tracer::ExhaustivePointerTracer;
pub use expanded_injection::ExpandedBlockInjection;
pub use safe_injection::SafeBlockInjection;

// Event imports added to individual modules as needed
