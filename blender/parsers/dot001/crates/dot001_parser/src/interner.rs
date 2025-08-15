//! String interner for compact DNA storage
//!
//! This module provides a compact string interner that reduces memory usage
//! and enables fast integer-based lookups for DNA names and type names.

use ahash::AHashMap;
use std::sync::Arc;

/// A unique identifier for an interned string
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct StringId(u32);

impl StringId {
    /// Create a new StringId from a u32 value
    pub fn from_raw(id: u32) -> Self {
        StringId(id)
    }

    /// Get the raw u32 value of this StringId
    pub fn to_raw(self) -> u32 {
        self.0
    }

    /// The invalid/null string ID
    pub const INVALID: StringId = StringId(u32::MAX);
}

impl std::fmt::Display for StringId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "StringId({})", self.0)
    }
}

/// Compact string interner optimized for DNA parsing
///
/// This interner deduplicates strings and provides O(1) lookup by integer ID.
/// It's optimized for the typical DNA parsing pattern where many strings are repeated.
#[derive(Debug, Clone)]
pub struct StringInterner {
    /// Vector of interned strings, indexed by StringId
    strings: Vec<Arc<str>>,
    /// Map from string content to StringId for deduplication
    string_to_id: AHashMap<Arc<str>, StringId>,
}

impl StringInterner {
    /// Create a new empty string interner
    pub fn new() -> Self {
        Self {
            strings: Vec::new(),
            string_to_id: AHashMap::new(),
        }
    }

    /// Create a new string interner with the specified capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            strings: Vec::with_capacity(capacity),
            string_to_id: AHashMap::with_capacity(capacity),
        }
    }

    /// Intern a string, returning its StringId
    ///
    /// If the string is already interned, returns the existing ID.
    /// Otherwise, creates a new ID and stores the string.
    pub fn intern(&mut self, s: &str) -> StringId {
        // Check if we already have this string
        let arc_str: Arc<str> = Arc::from(s);

        if let Some(&id) = self.string_to_id.get(&arc_str) {
            return id;
        }

        // Add new string
        let id = StringId(self.strings.len() as u32);
        self.strings.push(arc_str.clone());
        self.string_to_id.insert(arc_str, id);

        id
    }

    /// Intern a string from an owned String, returning its StringId
    pub fn intern_owned(&mut self, s: String) -> StringId {
        let arc_str: Arc<str> = Arc::from(s);

        if let Some(&id) = self.string_to_id.get(&arc_str) {
            return id;
        }

        let id = StringId(self.strings.len() as u32);
        self.strings.push(arc_str.clone());
        self.string_to_id.insert(arc_str, id);

        id
    }

    /// Get the string associated with a StringId
    ///
    /// Returns None if the ID is invalid.
    pub fn get(&self, id: StringId) -> Option<&str> {
        self.strings.get(id.0 as usize).map(|s| s.as_ref())
    }

    /// Get the string associated with a StringId, returning an empty string if invalid
    pub fn get_or_empty(&self, id: StringId) -> &str {
        self.get(id).unwrap_or("")
    }

    /// Get an Arc<str> for the given StringId (zero-copy sharing)
    pub fn get_arc(&self, id: StringId) -> Option<Arc<str>> {
        self.strings.get(id.0 as usize).cloned()
    }

    /// Get the number of interned strings
    pub fn len(&self) -> usize {
        self.strings.len()
    }

    /// Check if the interner is empty
    pub fn is_empty(&self) -> bool {
        self.strings.is_empty()
    }

    /// Clear all interned strings
    pub fn clear(&mut self) {
        self.strings.clear();
        self.string_to_id.clear();
    }

    /// Shrink the internal storage to fit the current number of strings
    pub fn shrink_to_fit(&mut self) {
        self.strings.shrink_to_fit();
        self.string_to_id.shrink_to_fit();
    }

    /// Iterate over all interned strings with their IDs
    pub fn iter(&self) -> impl Iterator<Item = (StringId, &str)> {
        self.strings
            .iter()
            .enumerate()
            .map(|(i, s)| (StringId(i as u32), s.as_ref()))
    }
}

impl Default for StringInterner {
    fn default() -> Self {
        Self::new()
    }
}

/// A name interner specifically optimized for DNA parsing
///
/// This provides additional functionality beyond basic string interning,
/// such as parsing name components and caching parsed results.
#[derive(Debug, Clone)]
pub struct DnaNameInterner {
    interner: StringInterner,
    /// Cache of parsed name components to avoid re-parsing
    name_info_cache: AHashMap<StringId, DnaNameInfo>,
}

/// Parsed information about a DNA name
#[derive(Debug, Clone)]
pub struct DnaNameInfo {
    pub full_name_id: StringId,
    pub name_only_id: StringId,
    pub is_pointer: bool,
    pub is_method_pointer: bool,
    pub array_size: usize,
}

impl DnaNameInterner {
    /// Create a new DNA name interner
    pub fn new() -> Self {
        Self {
            interner: StringInterner::new(),
            name_info_cache: AHashMap::new(),
        }
    }

    /// Create a new DNA name interner with the specified capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            interner: StringInterner::with_capacity(capacity),
            name_info_cache: AHashMap::with_capacity(capacity),
        }
    }

    /// Intern a DNA name and parse its components
    pub fn intern_name(&mut self, name_full: &str) -> StringId {
        let full_id = self.interner.intern(name_full);

        // Check if we've already parsed this name
        if self.name_info_cache.contains_key(&full_id) {
            return full_id;
        }

        // Parse the name components
        let name_info = self.parse_name_components(name_full, full_id);
        self.name_info_cache.insert(full_id, name_info);

        full_id
    }

    /// Get the parsed information for a DNA name
    pub fn get_name_info(&self, id: StringId) -> Option<&DnaNameInfo> {
        self.name_info_cache.get(&id)
    }

    /// Get the underlying string interner
    pub fn interner(&self) -> &StringInterner {
        &self.interner
    }

    /// Get the underlying string interner mutably
    pub fn interner_mut(&mut self) -> &mut StringInterner {
        &mut self.interner
    }

    /// Shrink all internal storage to fit
    pub fn shrink_to_fit(&mut self) {
        self.interner.shrink_to_fit();
        self.name_info_cache.shrink_to_fit();
    }

    /// Parse name components (similar to the original DnaName::new logic)
    fn parse_name_components(&mut self, name_full: &str, full_id: StringId) -> DnaNameInfo {
        let bytes = name_full.as_bytes();
        let is_pointer = bytes.contains(&b'*');
        let is_method_pointer = name_full.contains("(*");

        let start = if is_pointer {
            bytes
                .iter()
                .rposition(|&b| b == b'*')
                .map(|i| i + 1)
                .unwrap_or(0)
        } else {
            0
        };

        let end = bytes
            .iter()
            .position(|&b| b == b'[')
            .unwrap_or(name_full.len());

        let name_only = &name_full[start..end];
        let name_only_id = self.interner.intern(name_only);

        // Parse array size
        let array_size = if let Some(bracket_pos) = name_full.find('[') {
            if let Some(close_pos) = name_full.find(']') {
                if close_pos > bracket_pos + 1 {
                    let size_str = &name_full[bracket_pos + 1..close_pos];
                    size_str.parse().unwrap_or(1)
                } else {
                    1
                }
            } else {
                1
            }
        } else {
            1
        };

        DnaNameInfo {
            full_name_id: full_id,
            name_only_id,
            is_pointer,
            is_method_pointer,
            array_size,
        }
    }
}

impl Default for DnaNameInterner {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_interner_basic() {
        let mut interner = StringInterner::new();

        let id1 = interner.intern("hello");
        let id2 = interner.intern("world");
        let id3 = interner.intern("hello"); // Should reuse id1

        assert_eq!(interner.len(), 2);
        assert_eq!(id1, id3);
        assert_ne!(id1, id2);

        assert_eq!(interner.get(id1), Some("hello"));
        assert_eq!(interner.get(id2), Some("world"));
    }

    #[test]
    fn test_dna_name_interner() {
        let mut interner = DnaNameInterner::new();

        let id = interner.intern_name("*next");
        let info = interner.get_name_info(id).unwrap();

        assert!(info.is_pointer);
        assert!(!info.is_method_pointer);
        assert_eq!(info.array_size, 1);
        assert_eq!(interner.interner().get(info.name_only_id), Some("next"));
    }

    #[test]
    fn test_dna_name_array_parsing() {
        let mut interner = DnaNameInterner::new();

        let id = interner.intern_name("data[4]");
        let info = interner.get_name_info(id).unwrap();

        assert!(!info.is_pointer);
        assert_eq!(info.array_size, 4);
        assert_eq!(interner.interner().get(info.name_only_id), Some("data"));
    }

    #[test]
    fn test_dna_name_method_pointer() {
        let mut interner = DnaNameInterner::new();

        let id = interner.intern_name("(*func)(void)");
        let info = interner.get_name_info(id).unwrap();

        assert!(info.is_pointer);
        assert!(info.is_method_pointer);
        assert_eq!(
            interner.interner().get(info.name_only_id),
            Some("func)(void)")
        );
    }
}
