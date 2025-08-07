/// Modernized Object expander using the simple_expander macro
use crate::simple_expander;

simple_expander! {
    ObjectExpander, b"OB\0\0", "Object" => {
        single_fields: ["data"],
        array_fields: [("totcol", "mat")]
    }
}
