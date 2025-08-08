//! Thread-safe Lamp block expander
//!
//! This expander handles Lamp blocks (LA) and traces dependencies to:
//! - nodetree: Node tree for shader nodes

use crate::simple_expander;
simple_expander! {
    LampExpander, b"LA\0\0", "Lamp" => {
        single_fields: ["nodetree"],
        array_fields: []
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::BlockExpander;

    #[test]
    fn test_lamp_expander_properties() {
        let expander = LampExpander;
        assert_eq!(expander.block_code(), *b"LA\0\0");
        assert_eq!(expander.expander_name(), "LampExpander");
        assert!(expander.can_handle(b"LA\0\0"));
        assert!(!expander.can_handle(b"OB\0\0"));
    }
}
