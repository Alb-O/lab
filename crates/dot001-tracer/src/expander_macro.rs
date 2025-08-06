/// Enhanced macros to reduce boilerplate in block expanders
///
/// This module provides a unified macro system for creating block expanders
/// with support for pointer fields, arrays, external references, and custom logic.
///
/// The macro system supports four different patterns:
/// - `simple_expander!`: Basic pointer fields and arrays only
/// - `external_expander!`: External file references with optional simple patterns
/// - `hybrid_expander!`: Simple patterns combined with custom logic
/// - `custom_expander!`: Full custom implementation
///
/// # Examples
///
/// Simple expander for basic pointer traversal:
/// ```rust
/// use dot001_tracer::simple_expander;
/// simple_expander! {
///     ObjectExpander, b"OB\0\0", "Object" => {
///         single_fields: ["data"],
///         array_fields: [("totcol", "mat")]
///     }
/// }
/// ```
///
/// External reference expander:
/// ```rust
/// use dot001_tracer::external_expander;
/// external_expander! {
///     SoundExpander, b"SO\0\0", "bSound" => {
///         filepath_field: "filepath",
///         packed_check: "packedfile"
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

        impl<R: std::io::Read + std::io::Seek> $crate::BlockExpander<R> for $expander_name {
            fn expand_block(
                &self,
                block_index: usize,
                blend_file: &mut dot001_parser::BlendFile<R>,
            ) -> dot001_parser::Result<$crate::ExpandResult> {
                let mut dependencies = Vec::new();

                // Add single pointer field dependencies
                $(
                    if let Ok(single_targets) = dot001_parser::PointerTraversal::read_pointer_fields(
                        blend_file,
                        block_index,
                        $struct_name,
                        &[$single_field]
                    ) {
                        dependencies.extend(single_targets);
                    }
                )*

                // Add array field dependencies
                $(
                    if let Ok(array_targets) = dot001_parser::PointerTraversal::read_pointer_array(
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
        }
    };
}

/// Generate a custom block expander with manual dependency logic
///
/// # Example
/// ```rust
/// use dot001_tracer::custom_expander;
/// custom_expander! {
///     MaterialExpander, b"MA\0\0" => |block_index, blend_file| {
///         let mut dependencies = Vec::new();
///         // Custom logic here...
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

        impl<R: std::io::Read + std::io::Seek> $crate::BlockExpander<R> for $expander_name {
            fn expand_block(
                &self,
                $block_index: usize,
                $blend_file: &mut dot001_parser::BlendFile<R>,
            ) -> dot001_parser::Result<$crate::ExpandResult> {
                let dependencies: Vec<usize> = $custom_logic;
                Ok($crate::ExpandResult::new(dependencies))
            }

            fn can_handle(&self, code: &[u8; 4]) -> bool {
                code == $block_code
            }
        }
    };
}

/// Generate a hybrid expander that combines simple patterns with custom logic
///
/// # Example
/// ```rust
/// use dot001_tracer::hybrid_expander;
/// hybrid_expander! {
///     MaterialExpander, b"MA\0\0", "Material" => {
///         single_fields: ["nodetree"],
///         array_fields: [],
///         custom: |block_index, blend_file, dependencies| {
///             // Add custom mtex processing logic
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

        impl<R: std::io::Read + std::io::Seek> $crate::BlockExpander<R> for $expander_name {
            fn expand_block(
                &self,
                $block_index: usize,
                $blend_file: &mut dot001_parser::BlendFile<R>,
            ) -> dot001_parser::Result<$crate::ExpandResult> {
                let mut $dependencies = Vec::new();

                // Add single pointer field dependencies
                $(
                    if let Ok(single_targets) = dot001_parser::PointerTraversal::read_pointer_fields(
                        $blend_file,
                        $block_index,
                        $struct_name,
                        &[$single_field]
                    ) {
                        $dependencies.extend(single_targets);
                    }
                )*

                // Add array field dependencies
                $(
                    if let Ok(array_targets) = dot001_parser::PointerTraversal::read_pointer_array(
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
        }
    };
}

/// Generate an external reference expander for blocks that reference external files
///
/// This macro creates expanders for blocks that primarily reference external files
/// (like sounds, images, libraries) with optional simple pointer field support.
///
/// # Example
/// ```rust
/// use dot001_tracer::external_expander;
/// external_expander! {
///     SoundExpander, b"SO\0\0", "bSound" => {
///         filepath_field: "filepath",
///         packed_check: "packedfile",
///         single_fields: [], // Optional
///         array_fields: []   // Optional
///     }
/// }
/// ```
#[macro_export]
macro_rules! external_expander {
    (
        $expander_name:ident,
        $block_code:expr,
        $struct_name:expr => {
            filepath_field: $filepath_field:expr,
            packed_check: $packed_field:expr
            $(, single_fields: [$($single_field:expr),*])?
            $(, array_fields: [$(($count_field:expr, $array_field:expr)),*])?
        }
    ) => {
        pub struct $expander_name;

        impl<R: std::io::Read + std::io::Seek> $crate::BlockExpander<R> for $expander_name {
            fn expand_block(
                &self,
                block_index: usize,
                blend_file: &mut dot001_parser::BlendFile<R>,
            ) -> dot001_parser::Result<$crate::ExpandResult> {
                let dependencies = Vec::new();
                let mut external_refs = Vec::new();

                // Read the block data for external file handling
                if let Ok(block_data) = blend_file.read_block_data(block_index) {
                    if let Ok(reader) = blend_file.create_field_reader(&block_data) {
                        // Check for external file reference
                        if let Ok(filepath) = reader.read_field_string($struct_name, $filepath_field) {
                            let path_str = filepath.trim_end_matches('\0').trim();
                            if !path_str.is_empty() {
                                let blend_path = $crate::bpath::BlendPath::new(path_str.as_bytes());
                                external_refs.push(blend_path.to_pathbuf_stripped());
                            }
                        }

                        // Check if the file is packed (has internal data)
                        if let Ok(packed_ptr) = reader.read_field_pointer($struct_name, $packed_field) {
                            if packed_ptr != 0 {
                                // File is packed, clear external references
                                external_refs.clear();
                            }
                        }
                    }
                }

                // Add optional single pointer field dependencies
                $($(
                    if let Ok(single_targets) = dot001_parser::PointerTraversal::read_pointer_fields(
                        blend_file,
                        block_index,
                        $struct_name,
                        &[$single_field]
                    ) {
                        dependencies.extend(single_targets);
                    }
                )*)?

                // Add optional array field dependencies
                $($(
                    if let Ok(array_targets) = dot001_parser::PointerTraversal::read_pointer_array(
                        blend_file,
                        block_index,
                        $struct_name,
                        $count_field,
                        $array_field
                    ) {
                        dependencies.extend(array_targets);
                    }
                )*)?

                Ok($crate::ExpandResult::with_externals(dependencies, external_refs))
            }

            fn can_handle(&self, code: &[u8; 4]) -> bool {
                code == $block_code
            }
        }
    };

    // Version with custom external logic
    (
        $expander_name:ident,
        $block_code:expr,
        $struct_name:expr => {
            custom_external: |$block_index:ident, $blend_file:ident, $dependencies:ident, $external_refs:ident| $custom_logic:block
            $(, single_fields: [$($single_field:expr),*])?
            $(, array_fields: [$(($count_field:expr, $array_field:expr)),*])?
        }
    ) => {
        pub struct $expander_name;

        impl<R: std::io::Read + std::io::Seek> $crate::BlockExpander<R> for $expander_name {
            fn expand_block(
                &self,
                $block_index: usize,
                $blend_file: &mut dot001_parser::BlendFile<R>,
            ) -> dot001_parser::Result<$crate::ExpandResult> {
                let $dependencies = Vec::new();
                let mut $external_refs = Vec::new();

                // Execute custom external reference logic
                $custom_logic

                // Add optional single pointer field dependencies
                $($(
                    if let Ok(single_targets) = dot001_parser::PointerTraversal::read_pointer_fields(
                        $blend_file,
                        $block_index,
                        $struct_name,
                        &[$single_field]
                    ) {
                        $dependencies.extend(single_targets);
                    }
                )*)?

                // Add optional array field dependencies
                $($(
                    if let Ok(array_targets) = dot001_parser::PointerTraversal::read_pointer_array(
                        $blend_file,
                        $block_index,
                        $struct_name,
                        $count_field,
                        $array_field
                    ) {
                        $dependencies.extend(array_targets);
                    }
                )*)?

                Ok($crate::ExpandResult::with_externals($dependencies, $external_refs))
            }

            fn can_handle(&self, code: &[u8; 4]) -> bool {
                code == $block_code
            }
        }
    };
}

#[cfg(test)]
mod tests {
    // These are compile-time tests to ensure the macros generate valid code

    simple_expander! {
        TestSimpleExpander, b"TE\0\0", "TestStruct" => {
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
}
