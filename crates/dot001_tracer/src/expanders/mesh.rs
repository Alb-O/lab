//! Thread-safe Mesh block expander
//!
//! This expander handles Mesh blocks (ME) and traces dependencies to:
//! - mat: Array of materials assigned to the mesh (based on totcol count)

use crate::simple_expander;
simple_expander! {
    MeshExpander, b"ME\0\0", "Mesh" => {
        single_fields: [],
        array_fields: [("totcol", "mat")]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::BlockExpander;

    #[test]
    fn test_mesh_expander_properties() {
        let expander = MeshExpander;
        assert_eq!(expander.block_code(), *b"ME\0\0");
        assert_eq!(expander.expander_name(), "MeshExpander");
        assert!(expander.can_handle(b"ME\0\0"));
        assert!(!expander.can_handle(b"OB\0\0"));
    }
}
