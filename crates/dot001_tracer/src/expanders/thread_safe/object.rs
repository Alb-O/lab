//! Thread-safe Object block expander

use crate::thread_safe_simple_expander;

/// Thread-safe Object expander using zero-copy FieldView access
///
/// This expander handles Object blocks (OB) and traces dependencies to:
/// - data: The mesh, curve, or other object data
/// - mat: Array of materials (based on totcol count)
thread_safe_simple_expander! {
    ThreadSafeObjectExpander, b"OB\0\0", "Object" => {
        single_fields: ["data"],
        array_fields: [("totcol", "mat")]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ThreadSafeBlockExpander;

    #[test]
    fn test_object_expander_properties() {
        let expander = ThreadSafeObjectExpander;
        assert_eq!(expander.block_code(), *b"OB\0\0");
        assert_eq!(expander.expander_name(), "ThreadSafeObjectExpander");
        assert!(expander.can_handle(b"OB\0\0"));
        assert!(!expander.can_handle(b"ME\0\0"));
    }
}
