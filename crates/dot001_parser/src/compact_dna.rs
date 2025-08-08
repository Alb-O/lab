//! Compact DNA storage with interned strings and optimized lookup
//!
//! This module provides an optimized DNA representation that uses string interning
//! to reduce memory usage and enables faster lookups through integer-based indexing.

use crate::interner::{DnaNameInterner, StringId, StringInterner};
use ahash::AHashMap;

/// Compact representation of a DNA field using interned strings
#[derive(Debug, Clone)]
pub struct CompactDnaField {
    pub type_id: StringId,
    pub name_id: StringId,
    pub size: usize,
    pub offset: usize,
}

/// Compact representation of a DNA struct with precomputed field lookup
#[derive(Debug, Clone)]
pub struct CompactDnaStruct {
    pub type_id: StringId,
    pub size: usize,
    pub fields: Vec<CompactDnaField>,
    /// Fast lookup table: name_id -> field_index
    fields_by_name: AHashMap<StringId, usize>,
}

/// Optimized DNA collection with string interning and compact storage
#[derive(Debug)]
pub struct CompactDnaCollection {
    /// String interner for all DNA strings
    pub strings: StringInterner,
    /// Name-specific interner with parsed components
    pub names: DnaNameInterner,
    /// Compact struct definitions
    pub structs: Vec<CompactDnaStruct>,
    /// Fast lookup: type_id -> struct_index
    struct_by_type: AHashMap<StringId, usize>,
    /// Original type size information
    pub type_sizes: Vec<u16>,
    /// Mapping from type_id to size index
    type_size_map: AHashMap<StringId, usize>,
}

impl CompactDnaField {
    /// Get the type name string
    pub fn type_name<'a>(&self, strings: &'a StringInterner) -> &'a str {
        strings.get_or_empty(self.type_id)
    }

    /// Get the field name string
    pub fn name<'a>(&self, names: &'a DnaNameInterner) -> &'a str {
        names.interner().get_or_empty(self.name_id)
    }
}

impl CompactDnaStruct {
    /// Create a new compact DNA struct
    pub fn new(type_id: StringId, size: usize) -> Self {
        Self {
            type_id,
            size,
            fields: Vec::new(),
            fields_by_name: AHashMap::new(),
        }
    }

    /// Add a field to this struct
    pub fn add_field(&mut self, field: CompactDnaField) {
        let field_index = self.fields.len();
        self.fields_by_name.insert(field.name_id, field_index);
        self.fields.push(field);
    }

    /// Find a field by name ID
    pub fn find_field_by_id(&self, name_id: StringId) -> Option<&CompactDnaField> {
        self.fields_by_name
            .get(&name_id)
            .map(|&index| &self.fields[index])
    }

    /// Find a field by name string
    pub fn find_field_by_name(
        &self,
        names: &DnaNameInterner,
        name: &str,
    ) -> Option<&CompactDnaField> {
        // This requires a lookup in the name interner, so it's less efficient
        // For hot paths, prefer using name IDs directly
        self.fields
            .iter()
            .find(|&field| names.interner().get(field.name_id) == Some(name))
    }

    /// Get the type name string
    pub fn type_name<'a>(&self, strings: &'a StringInterner) -> &'a str {
        strings.get_or_empty(self.type_id)
    }

    /// Optimize internal storage after all fields are added
    pub fn shrink_to_fit(&mut self) {
        self.fields.shrink_to_fit();
        self.fields_by_name.shrink_to_fit();
    }
}

impl CompactDnaCollection {
    /// Create a new empty compact DNA collection
    pub fn new() -> Self {
        Self {
            strings: StringInterner::new(),
            names: DnaNameInterner::new(),
            structs: Vec::new(),
            struct_by_type: AHashMap::new(),
            type_sizes: Vec::new(),
            type_size_map: AHashMap::new(),
        }
    }

    /// Create from the original DnaCollection (conversion)
    pub fn from_original(original: &crate::dna::DnaCollection) -> Self {
        let mut compact = Self::new();

        // Pre-allocate based on original sizes
        compact.strings = StringInterner::with_capacity(
            original.types.len() + original.names.len() * 2, // Estimate for full and name-only
        );
        compact.names = DnaNameInterner::with_capacity(original.names.len());
        compact.structs.reserve(original.structs.len());
        compact.struct_by_type.reserve(original.structs.len());

        // Intern all type names
        let type_ids: Vec<StringId> = original
            .types
            .iter()
            .map(|type_name| compact.strings.intern(type_name))
            .collect();

        // Intern all DNA names (note: name_ids not used as we access names directly)
        let _name_ids: Vec<StringId> = original
            .names
            .iter()
            .map(|name| compact.names.intern_name(&name.name_full))
            .collect();

        // Copy type sizes and build mapping
        compact.type_sizes = original.type_sizes.clone();
        for (i, &type_id) in type_ids.iter().enumerate() {
            compact.type_size_map.insert(type_id, i);
        }

        // Convert structs
        for original_struct in &original.structs {
            let type_id = compact.strings.intern(&original_struct.type_name);
            let mut compact_struct = CompactDnaStruct::new(type_id, original_struct.size);

            // Convert fields
            for original_field in &original_struct.fields {
                let field_type_id = compact.strings.intern(&original_field.type_name);
                let field_name_id = compact.names.intern_name(&original_field.name.name_full);

                let compact_field = CompactDnaField {
                    type_id: field_type_id,
                    name_id: field_name_id,
                    size: original_field.size,
                    offset: original_field.offset,
                };

                compact_struct.add_field(compact_field);
            }

            compact_struct.shrink_to_fit();

            let struct_index = compact.structs.len();
            compact.struct_by_type.insert(type_id, struct_index);
            compact.structs.push(compact_struct);
        }

        // Optimize storage
        compact.shrink_to_fit();
        compact
    }

    /// Find a struct by type name
    pub fn find_struct_by_name(&self, type_name: &str) -> Option<&CompactDnaStruct> {
        // Find the type ID first (this requires a linear search, so cache if used frequently)
        for (type_id, &struct_index) in &self.struct_by_type {
            if self.strings.get(*type_id) == Some(type_name) {
                return Some(&self.structs[struct_index]);
            }
        }
        None
    }

    /// Find a struct by type ID (faster)
    pub fn find_struct_by_id(&self, type_id: StringId) -> Option<&CompactDnaStruct> {
        self.struct_by_type
            .get(&type_id)
            .map(|&index| &self.structs[index])
    }

    /// Get the size of a type by its ID
    pub fn get_type_size(&self, type_id: StringId) -> Option<u16> {
        self.type_size_map
            .get(&type_id)
            .and_then(|&index| self.type_sizes.get(index).copied())
    }

    /// Intern a type name and return its ID
    pub fn intern_type(&mut self, type_name: &str) -> StringId {
        self.strings.intern(type_name)
    }

    /// Intern a field name and return its ID
    pub fn intern_name(&mut self, name: &str) -> StringId {
        self.names.intern_name(name)
    }

    /// Get memory usage statistics
    pub fn memory_stats(&self) -> CompactDnaStats {
        let string_bytes = self.strings.iter().map(|(_, s)| s.len()).sum::<usize>();

        let name_bytes = self
            .names
            .interner()
            .iter()
            .map(|(_, s)| s.len())
            .sum::<usize>();

        CompactDnaStats {
            total_strings: self.strings.len(),
            total_names: self.names.interner().len(),
            total_structs: self.structs.len(),
            total_fields: self.structs.iter().map(|s| s.fields.len()).sum(),
            string_bytes,
            name_bytes,
            estimated_total_bytes: string_bytes
                + name_bytes
                + self.structs.len() * std::mem::size_of::<CompactDnaStruct>()
                + self
                    .structs
                    .iter()
                    .map(|s| s.fields.len() * std::mem::size_of::<CompactDnaField>())
                    .sum::<usize>(),
        }
    }

    /// Optimize all internal storage
    pub fn shrink_to_fit(&mut self) {
        self.strings.shrink_to_fit();
        self.names.shrink_to_fit();
        self.structs.shrink_to_fit();
        self.struct_by_type.shrink_to_fit();
        self.type_sizes.shrink_to_fit();
        self.type_size_map.shrink_to_fit();

        for struct_def in &mut self.structs {
            struct_def.shrink_to_fit();
        }
    }
}

impl Default for CompactDnaCollection {
    fn default() -> Self {
        Self::new()
    }
}

/// Memory usage statistics for compact DNA
#[derive(Debug, Clone)]
pub struct CompactDnaStats {
    pub total_strings: usize,
    pub total_names: usize,
    pub total_structs: usize,
    pub total_fields: usize,
    pub string_bytes: usize,
    pub name_bytes: usize,
    pub estimated_total_bytes: usize,
}

impl std::fmt::Display for CompactDnaStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "CompactDNA Stats: {} structs, {} fields, {} strings ({} bytes), {} names ({} bytes), ~{} total bytes",
            self.total_structs,
            self.total_fields,
            self.total_strings,
            self.string_bytes,
            self.total_names,
            self.name_bytes,
            self.estimated_total_bytes
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        DnaCollection,
        dna::{DnaField, DnaName, DnaStruct},
    };

    fn create_test_dna() -> DnaCollection {
        // Create test DNA data using test constructors
        let types = vec!["TestStruct".to_string(), "int".to_string()];
        let names = vec![DnaName {
            name_full: "value".to_string(),
            name_only: "value".to_string(),
            is_pointer: false,
            is_method_pointer: false,
            array_size: 1,
        }];
        let type_sizes = vec![8, 4];

        // Create a test struct with proper initialization
        let fields = vec![DnaField {
            type_name: "int".to_string(),
            name: DnaName {
                name_full: "value".to_string(),
                name_only: "value".to_string(),
                is_pointer: false,
                is_method_pointer: false,
                array_size: 1,
            },
            size: 4,
            offset: 0,
        }];

        let test_struct = DnaStruct::new_for_test("TestStruct".to_string(), 8, fields);

        DnaCollection::new_for_test(vec![test_struct], types, names, type_sizes)
    }

    #[test]
    fn test_compact_dna_conversion() {
        let original = create_test_dna();
        let compact = CompactDnaCollection::from_original(&original);

        assert_eq!(compact.structs.len(), 1);
        assert_eq!(compact.strings.len(), 2); // "TestStruct", "int"

        let test_struct = compact.find_struct_by_name("TestStruct").unwrap();
        assert_eq!(test_struct.size, 8);
        assert_eq!(test_struct.fields.len(), 1);

        let field = &test_struct.fields[0];
        assert_eq!(field.type_name(&compact.strings), "int");
        assert_eq!(field.name(&compact.names), "value");
        assert_eq!(field.size, 4);
        assert_eq!(field.offset, 0);
    }

    #[test]
    fn test_memory_stats() {
        let original = create_test_dna();
        let compact = CompactDnaCollection::from_original(&original);

        let stats = compact.memory_stats();
        assert!(stats.total_strings > 0);
        assert!(stats.total_names > 0);
        assert!(stats.total_structs > 0);
        assert!(stats.total_fields > 0);
        assert!(stats.estimated_total_bytes > 0);

        println!("{stats}");
    }
}
