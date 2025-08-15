//! Thread-safe Material block expander
//!
//! This expander handles Material blocks (MA) and traces dependencies to:
//! - nodetree: Node tree for shader nodes
//!
//! Note: This is a simplified version. The legacy MaterialExpander had complex
//! mtex array processing that would need custom logic to fully replicate.

use crate::simple_expander;
simple_expander! {
    MaterialExpander, b"MA\0\0", "Material" => {
        single_fields: ["nodetree"],
        array_fields: []
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::BlockExpander;

    #[test]
    fn test_material_expander_properties() {
        let expander = MaterialExpander;
        assert_eq!(expander.block_code(), *b"MA\0\0");
        assert_eq!(expander.expander_name(), "MaterialExpander");
        assert!(expander.can_handle(b"MA\0\0"));
        assert!(!expander.can_handle(b"OB\0\0"));
    }
}
