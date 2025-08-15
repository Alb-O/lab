/// Core dependency tracing functionality
pub mod options;
pub mod tracer;
pub mod tree;

// PRIMARY APIs - Zero-copy Architecture
pub use options::TracerOptions;
pub use tracer::DependencyTracer;
pub use tree::{DependencyNode, DependencyTree};
