//! Thread-safe Mesh block expander

use crate::thread_safe_simple_expander;

/// Thread-safe Mesh expander using zero-copy FieldView access
///
/// This expander handles Mesh blocks (ME) and traces dependencies to:
/// - mat: Array of materials assigned to the mesh (based on totcol count)
thread_safe_simple_expander! {
    ThreadSafeMeshExpander, b"ME\0\0", "Mesh" => {
        single_fields: [],
        array_fields: [("totcol", "mat")]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ThreadSafeBlockExpander;

    #[test]
    fn test_mesh_expander_properties() {
        let expander = ThreadSafeMeshExpander;
        assert_eq!(expander.block_code(), *b"ME\0\0");
        assert_eq!(expander.expander_name(), "ThreadSafeMeshExpander");
        assert!(expander.can_handle(b"ME\0\0"));
        assert!(!expander.can_handle(b"OB\0\0"));
    }
}
