//! Thread-safe Material block expander

use crate::thread_safe_simple_expander;

/// Thread-safe Material expander using zero-copy FieldView access
///
/// This expander handles Material blocks (MA) and traces dependencies to:
/// - nodetree: Node tree for shader nodes
///
/// Note: This is a simplified version. The legacy MaterialExpander had complex
/// mtex array processing that would need custom logic to fully replicate.
thread_safe_simple_expander! {
    ThreadSafeMaterialExpander, b"MA\0\0", "Material" => {
        single_fields: ["nodetree"],
        array_fields: []
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ThreadSafeBlockExpander;

    #[test]
    fn test_material_expander_properties() {
        let expander = ThreadSafeMaterialExpander;
        assert_eq!(expander.block_code(), *b"MA\0\0");
        assert_eq!(expander.expander_name(), "ThreadSafeMaterialExpander");
        assert!(expander.can_handle(b"MA\0\0"));
        assert!(!expander.can_handle(b"OB\0\0"));
    }
}
