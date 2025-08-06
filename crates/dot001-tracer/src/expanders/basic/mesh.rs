/// Modernized Mesh expander using the simple_expander macro
use crate::simple_expander;

simple_expander! {
    MeshExpander, b"ME\0\0", "Mesh" => {
        single_fields: [
            "vert", "edge", "poly", "loop",
            "vert_normals", "poly_normals", "loop_normals", "face_sets"
        ],
        array_fields: [("totcol", "mat")]
    }
}
