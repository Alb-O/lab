// REMOVED: All legacy expander macros have been completely removed
//
// The old macros have been replaced with thread-safe alternatives:
// - simple_expander! -> thread_safe_simple_expander!
// - custom_expander! -> thread_safe_custom_expander!
// - hybrid_expander! -> thread_safe_hybrid_expander!
// - external_expander! -> thread_safe_custom_expander!
//
// The new thread-safe macros provide:
// - Zero-copy access with BlendFileBuf and FieldView
// - Thread-safe parallel processing capability
// - Better performance through immutable access patterns
// - Deterministic output with stable sorting
//
// See thread_safe_macros.rs for the new macro implementations.
//
// All legacy expander implementations should be converted to use
// the thread-safe versions for optimal performance.
