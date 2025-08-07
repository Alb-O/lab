/// Thread-safe block expanders for different Blender data types
///
/// This module contains thread-safe expanders that extract dependencies from specific
/// types of Blender blocks using zero-copy FieldView access for optimal performance.
///
/// The expanders are organized by category:
/// - `basic`: Legacy directory (removed - see thread_safe instead)
/// - `external`: Legacy directory (removed - see thread_safe instead)
/// - `macros`: Macro generators for creating thread-safe expanders
/// - `thread_safe`: Modern zero-copy expanders with parallel processing support
pub mod basic; // Legacy - empty placeholder
pub mod external; // Legacy - empty placeholder
pub mod macros;
pub mod thread_safe;

// PRIMARY EXPORTS - Thread-safe expanders only
pub use thread_safe::{
    ThreadSafeMaterialExpander, ThreadSafeMeshExpander, ThreadSafeObjectExpander,
    ThreadSafeSceneExpander,
};

// REMOVED: Legacy expander exports
// All basic::* and external::* expanders have been completely removed.
// Use the ThreadSafe* variants or create custom expanders with the thread_safe_*_expander! macros.
