/// Modernized Lamp expander using simple_expander macro
use crate::simple_expander;

simple_expander! {
    LampExpander, b"LA\0\0", "Lamp" => {
        single_fields: ["nodetree"],
        array_fields: []
    }
}
