//! dot001 - Blender file format parser and analyzer
//!
//! This is a convenience crate that re-exports the main functionality
//! from the dot001 ecosystem for benchmarking and integration.

pub use dot001_events::error::*;
pub use dot001_parser::{self, BlendFile};
pub use dot001_tracer::{self, DependencyTree, ParallelDependencyTracer};
