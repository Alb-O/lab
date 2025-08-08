//! Thread-safe expander generation macros
//!
//! This module provides macros for generating thread-safe block expanders
//! that work with BlendFileBuf and FieldView for zero-copy, parallel access.

/// Generate a simple thread-safe block expander
///
/// This creates an expander that uses  PointerTraversal for
/// zero-copy pointer field and array access.
///
/// # Example
/// ```rust
/// use dot001_tracer::simple_expander;
/// simple_expander! {
///      ObjectExpander, b"OB\0\0", "Object" => {
///         single_fields: ["data"],
///         array_fields: [("totcol", "mat")]
///     }
/// }
/// ```
#[macro_export]
macro_rules! simple_expander {
    (
        $expander_name:ident,
        $block_code:expr,
        $struct_name:expr => {
            single_fields: [$($single_field:expr),*],
            array_fields: [$(($count_field:expr, $array_field:expr)),*]
        }
    ) => {
        pub struct $expander_name;

        impl $crate::BlockExpander for $expander_name {
            fn expand_block_threadsafe(
                &self,
                block_index: usize,
                blend_file: &dot001_parser::BlendFileBuf,
            ) -> dot001_events::error::Result<$crate::ExpandResult> {
                let mut dependencies = Vec::new();

                // Add single pointer field dependencies using thread-safe traversal
                $(
                    if let Ok(single_targets) = $crate::PointerTraversal::read_pointer_fields_threadsafe(
                        blend_file,
                        block_index,
                        $struct_name,
                        &[$single_field]
                    ) {
                        dependencies.extend(single_targets);
                    }
                )*

                // Add array field dependencies using thread-safe traversal
                $(
                    if let Ok(array_targets) = $crate::PointerTraversal::read_pointer_array_threadsafe(
                        blend_file,
                        block_index,
                        $struct_name,
                        $count_field,
                        $array_field
                    ) {
                        dependencies.extend(array_targets);
                    }
                )*

                Ok($crate::ExpandResult::new(dependencies))
            }

            fn can_handle(&self, code: &[u8; 4]) -> bool {
                code == $block_code
            }

            fn block_code(&self) -> [u8; 4] {
                *$block_code
            }

            fn expander_name(&self) -> &'static str {
                stringify!($expander_name)
            }
        }
    };
}

/// Generate a custom thread-safe block expander with manual logic
///
/// # Example
/// ```rust
/// use dot001_tracer::custom_expander;
/// custom_expander! {
///      MaterialExpander, b"MA\0\0" => |block_index, blend_file| {
///         let mut dependencies = Vec::new();
///         // Custom zero-copy logic using FieldView...
///         dependencies
///     }
/// }
/// ```
#[macro_export]
macro_rules! custom_expander {
    (
        $expander_name:ident,
        $block_code:expr => |$block_index:ident, $blend_file:ident| $custom_logic:block
    ) => {
        pub struct $expander_name;

        impl $crate::BlockExpander for $expander_name {
            fn expand_block_threadsafe(
                &self,
                $block_index: usize,
                $blend_file: &dot001_parser::BlendFileBuf,
            ) -> dot001_events::error::Result<$crate::ExpandResult> {
                let dependencies: Vec<usize> = $custom_logic;
                Ok($crate::ExpandResult::new(dependencies))
            }

            fn can_handle(&self, code: &[u8; 4]) -> bool {
                code == $block_code
            }

            fn block_code(&self) -> [u8; 4] {
                *$block_code
            }

            fn expander_name(&self) -> &'static str {
                stringify!($expander_name)
            }
        }
    };
}

/// Generate a hybrid thread-safe expander with both simple patterns and custom logic
///
/// # Example
/// ```rust
/// use dot001_tracer::hybrid_expander;
/// hybrid_expander! {
///      MaterialExpander, b"MA\0\0", "Material" => {
///         single_fields: ["nodetree"],
///         array_fields: [],
///         custom: |block_index, blend_file, dependencies| {
///             // Add custom mtex processing using FieldView
///         }
///     }
/// }
/// ```
#[macro_export]
macro_rules! hybrid_expander {
    (
        $expander_name:ident,
        $block_code:expr,
        $struct_name:expr => {
            single_fields: [$($single_field:expr),*],
            array_fields: [$(($count_field:expr, $array_field:expr)),*],
            custom: |$block_index:ident, $blend_file:ident, $dependencies:ident| $custom_logic:block
        }
    ) => {
        pub struct $expander_name;

        impl $crate::BlockExpander for $expander_name {
            fn expand_block_threadsafe(
                &self,
                $block_index: usize,
                $blend_file: &dot001_parser::BlendFileBuf,
            ) -> dot001_events::error::Result<$crate::ExpandResult> {
                let mut $dependencies = Vec::new();

                // Add single pointer field dependencies using thread-safe traversal
                $(
                    if let Ok(single_targets) = $crate::PointerTraversal::read_pointer_fields_threadsafe(
                        $blend_file,
                        $block_index,
                        $struct_name,
                        &[$single_field]
                    ) {
                        $dependencies.extend(single_targets);
                    }
                )*

                // Add array field dependencies using thread-safe traversal
                $(
                    if let Ok(array_targets) = $crate::PointerTraversal::read_pointer_array_threadsafe(
                        $blend_file,
                        $block_index,
                        $struct_name,
                        $count_field,
                        $array_field
                    ) {
                        $dependencies.extend(array_targets);
                    }
                )*

                // Execute custom logic
                $custom_logic

                Ok($crate::ExpandResult::new($dependencies))
            }

            fn can_handle(&self, code: &[u8; 4]) -> bool {
                code == $block_code
            }

            fn block_code(&self) -> [u8; 4] {
                *$block_code
            }

            fn expander_name(&self) -> &'static str {
                stringify!($expander_name)
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use crate::BlockExpander;

    // Test the macro generation at compile time
    simple_expander! {
        TestSimpleExpander, b"TS\0\0", "TestStruct" => {
            single_fields: ["field1", "field2"],
            array_fields: [("count", "array")]
        }
    }

    custom_expander! {
        TestCustomExpander, b"TC\0\0" => |block_index, _blend_file| {
            vec![block_index] // Just return self for testing
        }
    }

    hybrid_expander! {
        TestHybridExpander, b"TH\0\0", "TestHybrid" => {
            single_fields: ["single"],
            array_fields: [],
            custom: |_block_index, _blend_file, deps| {
                deps.push(999); // Add a test dependency
            }
        }
    }

    #[test]
    fn test_expander_generation() {
        let simple = TestSimpleExpander;
        assert_eq!(simple.block_code(), *b"TS\0\0");
        assert_eq!(simple.expander_name(), "TestSimpleExpander");

        let custom = TestCustomExpander;
        assert_eq!(custom.block_code(), *b"TC\0\0");
        assert_eq!(custom.expander_name(), "TestCustomExpander");

        let hybrid = TestHybridExpander;
        assert_eq!(hybrid.block_code(), *b"TH\0\0");
        assert_eq!(hybrid.expander_name(), "TestHybridExpander");
    }
}
