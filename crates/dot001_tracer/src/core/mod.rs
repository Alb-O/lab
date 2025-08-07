/// Core dependency tracing functionality
pub mod options;
pub mod parallel_tracer;
pub mod tree;
// REMOVED: tracer module with legacy DependencyTracer

// PRIMARY APIs - Zero-copy Architecture
pub use options::TracerOptions;
pub use parallel_tracer::ParallelDependencyTracer;
pub use tree::{DependencyNode, DependencyTree};

// REMOVED: Legacy DependencyTracer
// Use ParallelDependencyTracer for all dependency tracing operations.
