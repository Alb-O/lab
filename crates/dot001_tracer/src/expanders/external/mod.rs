// REMOVED: All legacy external expanders have been completely removed
//
// The old external expanders have been replaced with thread-safe implementations:
//
// Legacy -> Thread-Safe Replacement
// - ImageExpander -> Use thread_safe_custom_expander! with external file handling
// - SoundExpander -> Use thread_safe_custom_expander! with external file handling
// - LibraryExpander -> Use thread_safe_custom_expander! with external file handling
// - CacheFileExpander -> Use thread_safe_custom_expander! with external file handling
//
// External expanders require custom logic for:
// - Reading filepath fields from blend data
// - Checking packed file status
// - Converting BlendPath to PathBuf
// - Zero-copy access with FieldView
//
// See src/expanders/thread_safe/ for examples of the new pattern.
