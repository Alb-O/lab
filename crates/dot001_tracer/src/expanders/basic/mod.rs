// REMOVED: All legacy basic expanders have been completely removed
//
// The old expanders have been replaced with thread-safe implementations:
//
// Legacy -> Thread-Safe Replacement
// - ObjectExpander -> thread_safe::ObjectExpander
// - SceneExpander -> thread_safe::SceneExpander
// - MeshExpander -> thread_safe::MeshExpander
// - MaterialExpander -> thread_safe::MaterialExpander
// - CollectionExpander -> Custom implementation needed
// - DataBlockExpander -> Generic replacement needed
// - NodeTreeExpander -> Custom implementation needed
// - LampExpander -> Custom implementation needed
// - TextureExpander -> Custom implementation needed
//
// All thread-safe expanders use zero-copy FieldView access and provide
// better performance through parallel processing capabilities.
//
// See src/expanders/thread_safe/ for the new implementations.
