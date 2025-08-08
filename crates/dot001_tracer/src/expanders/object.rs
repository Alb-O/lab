//! Thread-safe Object block expander
//!
//! This expander handles Object blocks (OB) and traces dependencies to:
//! - data: The mesh, curve, or other object data
//! - mat: Array of materials (based on totcol count)

use crate::simple_expander;
simple_expander! {
    ObjectExpander, b"OB\0\0", "Object" => {
        single_fields: ["data"],
        array_fields: [("totcol", "mat")]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::BlockExpander;

    #[test]
    fn test_object_expander_properties() {
        let expander = ObjectExpander;
        assert_eq!(expander.block_code(), *b"OB\0\0");
        assert_eq!(expander.expander_name(), "ObjectExpander");
        assert!(expander.can_handle(b"OB\0\0"));
        assert!(!expander.can_handle(b"ME\0\0"));
    }
}
