/// Object expander implemented using the simple_expander macro
///
/// This demonstrates how the macro reduces boilerplate for simple cases
/// where we only need to read pointer fields and arrays.
use crate::simple_expander;

simple_expander! {
    ObjectExpanderMacro, b"OB\0\0", "Object" => {
        single_fields: ["data"],
        array_fields: [("totcol", "mat")]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::BlockExpander;

    #[test]
    fn test_object_expander_macro_can_handle() {
        use std::io::Cursor;
        let expander = ObjectExpanderMacro;
        assert!(
            <ObjectExpanderMacro as BlockExpander<Cursor<Vec<u8>>>>::can_handle(
                &expander, b"OB\0\0"
            )
        );
        assert!(
            !<ObjectExpanderMacro as BlockExpander<Cursor<Vec<u8>>>>::can_handle(
                &expander, b"ME\0\0"
            )
        );
    }
}
