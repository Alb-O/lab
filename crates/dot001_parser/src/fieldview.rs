//! Zero-copy field view API for efficient DNA data access
//!
//! This module provides high-performance field access using zero-copy operations
//! with bytemuck for safe casting and minimal overhead.

use crate::{BlendFileErrorKind, DnaCollection, Error, Result};
use bytes::Bytes;

#[cfg(feature = "bytemuck")]
use bytemuck::{Pod, Zeroable};

/// Zero-copy view into a blend file block for efficient field access
///
/// FieldView provides fast, type-safe access to structured data in blend file blocks
/// without allocating or copying data. It uses prevalidated offsets from DNA and
/// optimized endianness handling.
pub struct FieldView<'data> {
    data: &'data [u8],
    dna: &'data DnaCollection,
    pointer_size: usize,
    is_little_endian: bool,
}

impl<'data> FieldView<'data> {
    /// Create a new FieldView for the given block data and DNA
    pub fn new(
        data: &'data [u8],
        dna: &'data DnaCollection,
        pointer_size: usize,
        is_little_endian: bool,
    ) -> Self {
        Self {
            data,
            dna,
            pointer_size,
            is_little_endian,
        }
    }

    /// Create a FieldView from a Bytes slice (zero-copy)
    pub fn from_bytes(
        bytes: &'data Bytes,
        dna: &'data DnaCollection,
        pointer_size: usize,
        is_little_endian: bool,
    ) -> Self {
        Self::new(bytes.as_ref(), dna, pointer_size, is_little_endian)
    }

    /// Get the underlying data slice
    pub fn data(&self) -> &[u8] {
        self.data
    }

    /// Get the data length
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Check if the data is empty
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Read a u8 value at the given offset
    pub fn read_u8(&self, offset: usize) -> Result<u8> {
        self.data.get(offset).copied().ok_or_else(|| {
            Error::blend_file(
                format!("Offset {offset} out of bounds for u8 read"),
                BlendFileErrorKind::InvalidField,
            )
        })
    }

    /// Read a u16 value at the given offset
    pub fn read_u16(&self, offset: usize) -> Result<u16> {
        if offset + 2 > self.data.len() {
            return Err(Error::blend_file(
                format!("Offset {offset} out of bounds for u16 read"),
                BlendFileErrorKind::InvalidField,
            ));
        }

        let bytes: [u8; 2] = self.data[offset..offset + 2].try_into().unwrap();
        Ok(if self.is_little_endian {
            u16::from_le_bytes(bytes)
        } else {
            u16::from_be_bytes(bytes)
        })
    }

    /// Read a u32 value at the given offset
    pub fn read_u32(&self, offset: usize) -> Result<u32> {
        if offset + 4 > self.data.len() {
            return Err(Error::blend_file(
                format!("Offset {offset} out of bounds for u32 read"),
                BlendFileErrorKind::InvalidField,
            ));
        }

        let bytes: [u8; 4] = self.data[offset..offset + 4].try_into().unwrap();
        Ok(if self.is_little_endian {
            u32::from_le_bytes(bytes)
        } else {
            u32::from_be_bytes(bytes)
        })
    }

    /// Read a u64 value at the given offset
    pub fn read_u64(&self, offset: usize) -> Result<u64> {
        if offset + 8 > self.data.len() {
            return Err(Error::blend_file(
                format!("Offset {offset} out of bounds for u64 read"),
                BlendFileErrorKind::InvalidField,
            ));
        }

        let bytes: [u8; 8] = self.data[offset..offset + 8].try_into().unwrap();
        Ok(if self.is_little_endian {
            u64::from_le_bytes(bytes)
        } else {
            u64::from_be_bytes(bytes)
        })
    }

    /// Read an i32 value at the given offset
    pub fn read_i32(&self, offset: usize) -> Result<i32> {
        if offset + 4 > self.data.len() {
            return Err(Error::blend_file(
                format!("Offset {offset} out of bounds for i32 read"),
                BlendFileErrorKind::InvalidField,
            ));
        }

        let bytes: [u8; 4] = self.data[offset..offset + 4].try_into().unwrap();
        Ok(if self.is_little_endian {
            i32::from_le_bytes(bytes)
        } else {
            i32::from_be_bytes(bytes)
        })
    }

    /// Read a f32 value at the given offset
    pub fn read_f32(&self, offset: usize) -> Result<f32> {
        if offset + 4 > self.data.len() {
            return Err(Error::blend_file(
                format!("Offset {offset} out of bounds for f32 read"),
                BlendFileErrorKind::InvalidField,
            ));
        }

        let bytes: [u8; 4] = self.data[offset..offset + 4].try_into().unwrap();
        Ok(if self.is_little_endian {
            f32::from_le_bytes(bytes)
        } else {
            f32::from_be_bytes(bytes)
        })
    }

    /// Read a f64 value at the given offset
    pub fn read_f64(&self, offset: usize) -> Result<f64> {
        if offset + 8 > self.data.len() {
            return Err(Error::blend_file(
                format!("Offset {offset} out of bounds for f64 read"),
                BlendFileErrorKind::InvalidField,
            ));
        }

        let bytes: [u8; 8] = self.data[offset..offset + 8].try_into().unwrap();
        Ok(if self.is_little_endian {
            f64::from_le_bytes(bytes)
        } else {
            f64::from_be_bytes(bytes)
        })
    }

    /// Read a pointer value at the given offset
    pub fn read_pointer(&self, offset: usize) -> Result<u64> {
        match self.pointer_size {
            4 => self.read_u32(offset).map(|v| v as u64),
            8 => self.read_u64(offset),
            _ => Err(Error::blend_file(
                format!("Invalid pointer size: {}", self.pointer_size),
                BlendFileErrorKind::InvalidField,
            )),
        }
    }

    /// Get a slice view of data at the given offset and length
    pub fn slice(&self, offset: usize, len: usize) -> Result<&[u8]> {
        if offset + len > self.data.len() {
            return Err(Error::blend_file(
                format!(
                    "Range {}..{} out of bounds for data of length {}",
                    offset,
                    offset + len,
                    self.data.len()
                ),
                BlendFileErrorKind::InvalidField,
            ));
        }

        Ok(&self.data[offset..offset + len])
    }

    /// Read a field by name from a struct
    pub fn read_field_u32(&self, struct_name: &str, field_name: &str) -> Result<u32> {
        let field = self.find_field(struct_name, field_name)?;
        self.read_u32(field.offset)
    }

    /// Read a field by name as a pointer
    pub fn read_field_pointer(&self, struct_name: &str, field_name: &str) -> Result<u64> {
        let field = self.find_field(struct_name, field_name)?;
        self.read_pointer(field.offset)
    }

    /// Read a field by name as an i32
    pub fn read_field_i32(&self, struct_name: &str, field_name: &str) -> Result<i32> {
        let field = self.find_field(struct_name, field_name)?;
        self.read_i32(field.offset)
    }

    /// Read a field by name as an f32
    pub fn read_field_f32(&self, struct_name: &str, field_name: &str) -> Result<f32> {
        let field = self.find_field(struct_name, field_name)?;
        self.read_f32(field.offset)
    }

    /// Read a field by name as a byte slice
    pub fn read_field_bytes(&self, struct_name: &str, field_name: &str) -> Result<&[u8]> {
        let field = self.find_field(struct_name, field_name)?;
        self.slice(field.offset, field.size)
    }

    /// Find a field definition by struct name and field name
    fn find_field(&self, struct_name: &str, field_name: &str) -> Result<&crate::DnaField> {
        let struct_def = self
            .dna
            .structs
            .iter()
            .find(|s| s.type_name == struct_name)
            .ok_or_else(|| {
                Error::blend_file(
                    format!("Struct '{struct_name}' not found in DNA"),
                    BlendFileErrorKind::InvalidField,
                )
            })?;

        struct_def.find_field(field_name).ok_or_else(|| {
            Error::blend_file(
                format!("Field '{field_name}' not found in struct '{struct_name}'"),
                BlendFileErrorKind::InvalidField,
            )
        })
    }
}

/// Optimized field access for known struct layouts using bytemuck
///
/// This provides fast-path access when the struct layout is known and stable,
/// using zero-copy casts where alignment permits.
#[cfg(feature = "bytemuck")]
pub struct TypedFieldView<'data, T: Pod> {
    data: &'data T,
    _phantom: std::marker::PhantomData<&'data ()>,
}

#[cfg(feature = "bytemuck")]
impl<'data, T: Pod> TypedFieldView<'data, T> {
    /// Create a typed view if the data is properly aligned and sized
    pub fn new(data: &'data [u8]) -> Result<Self> {
        if data.len() < std::mem::size_of::<T>() {
            return Err(Error::blend_file(
                format!(
                    "Data too small for type: need {} bytes, have {}",
                    std::mem::size_of::<T>(),
                    data.len()
                ),
                BlendFileErrorKind::InvalidField,
            ));
        }

        let typed_data = bytemuck::try_from_bytes(data).map_err(|e| {
            Error::blend_file(
                format!("Bytemuck cast failed: {e}"),
                BlendFileErrorKind::InvalidField,
            )
        })?;

        Ok(Self {
            data: typed_data,
            _phantom: std::marker::PhantomData,
        })
    }

    /// Get a reference to the typed data
    pub fn data(&self) -> &T {
        self.data
    }
}

/// Helper trait for common field access patterns
pub trait FieldViewExt<'data> {
    /// Create a FieldView from this data source
    fn create_field_view(
        &'data self,
        dna: &'data DnaCollection,
        pointer_size: usize,
        is_little_endian: bool,
    ) -> FieldView<'data>;
}

impl<'data> FieldViewExt<'data> for [u8] {
    fn create_field_view(
        &'data self,
        dna: &'data DnaCollection,
        pointer_size: usize,
        is_little_endian: bool,
    ) -> FieldView<'data> {
        FieldView::new(self, dna, pointer_size, is_little_endian)
    }
}

impl<'data> FieldViewExt<'data> for Bytes {
    fn create_field_view(
        &'data self,
        dna: &'data DnaCollection,
        pointer_size: usize,
        is_little_endian: bool,
    ) -> FieldView<'data> {
        FieldView::from_bytes(self, dna, pointer_size, is_little_endian)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_data() -> Vec<u8> {
        vec![
            0x01, 0x02, 0x03, 0x04, // u32: 0x04030201 (little endian)
            0x05, 0x06, 0x07, 0x08, // u32: 0x08070605 (little endian)
            0x00, 0x00, 0x80, 0x3F, // f32: 1.0 (little endian)
            0x10, 0x20, 0x30, 0x40, // pointer or data
        ]
    }

    #[test]
    fn test_basic_reads() {
        let data = create_test_data();

        // Create a mock DNA (minimal for testing)
        let dna = DnaCollection::new_for_test(vec![], vec![], vec![], vec![]);

        let view = FieldView::new(&data, &dna, 4, true);

        assert_eq!(view.read_u8(0).unwrap(), 0x01);
        assert_eq!(view.read_u32(0).unwrap(), 0x04030201);
        assert_eq!(view.read_u32(4).unwrap(), 0x08070605);
        assert_eq!(view.read_f32(8).unwrap(), 1.0);
    }

    #[test]
    fn test_bounds_checking() {
        let data = vec![1, 2, 3, 4];
        let dna = DnaCollection::new_for_test(vec![], vec![], vec![], vec![]);

        let view = FieldView::new(&data, &dna, 4, true);

        // Should succeed
        assert!(view.read_u8(3).is_ok());
        assert!(view.read_u32(0).is_ok());

        // Should fail
        assert!(view.read_u8(4).is_err());
        assert!(view.read_u32(1).is_err()); // Would read past end
    }

    #[test]
    fn test_endianness() {
        let data = vec![0x01, 0x02, 0x03, 0x04];
        let dna = DnaCollection::new_for_test(vec![], vec![], vec![], vec![]);

        let view_le = FieldView::new(&data, &dna, 4, true);
        let view_be = FieldView::new(&data, &dna, 4, false);

        assert_eq!(view_le.read_u32(0).unwrap(), 0x04030201);
        assert_eq!(view_be.read_u32(0).unwrap(), 0x01020304);
    }
}
