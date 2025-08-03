use crate::{BlendError, DnaCollection, Result};

/// Utilities for reading structured data from block data using DNA information
pub struct FieldReader<'a> {
    pub data: &'a [u8],
    pub dna: &'a DnaCollection,
    pub pointer_size: usize,
    pub is_little_endian: bool,
}

impl<'a> FieldReader<'a> {
    pub fn new(
        data: &'a [u8],
        dna: &'a DnaCollection,
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

    /// Read a pointer value from the given offset
    pub fn read_pointer(&self, offset: usize) -> Result<u64> {
        if self.pointer_size == 8 {
            self.read_u64(offset)
        } else {
            self.read_u32(offset).map(|v| v as u64)
        }
    }

    /// Read a 32-bit unsigned integer
    pub fn read_u32(&self, offset: usize) -> Result<u32> {
        if offset + 4 > self.data.len() {
            return Err(BlendError::InvalidField(format!(
                "Offset {} + 4 exceeds data length {}",
                offset,
                self.data.len()
            )));
        }

        let bytes = [
            self.data[offset],
            self.data[offset + 1],
            self.data[offset + 2],
            self.data[offset + 3],
        ];

        let value = if self.is_little_endian {
            u32::from_le_bytes(bytes)
        } else {
            u32::from_be_bytes(bytes)
        };

        Ok(value)
    }

    /// Read a 64-bit unsigned integer
    pub fn read_u64(&self, offset: usize) -> Result<u64> {
        if offset + 8 > self.data.len() {
            return Err(BlendError::InvalidField(format!(
                "Offset {} + 8 exceeds data length {}",
                offset,
                self.data.len()
            )));
        }

        let mut bytes = [0u8; 8];
        bytes.copy_from_slice(&self.data[offset..offset + 8]);

        let value = if self.is_little_endian {
            u64::from_le_bytes(bytes)
        } else {
            u64::from_be_bytes(bytes)
        };

        Ok(value)
    }

    /// Read a field by name from a struct
    pub fn read_field_pointer(&self, struct_name: &str, field_name: &str) -> Result<u64> {
        let struct_def = self
            .dna
            .structs
            .iter()
            .find(|s| s.type_name == struct_name)
            .ok_or_else(|| BlendError::InvalidField(format!("Struct {struct_name} not found")))?;

        let field = struct_def
            .fields
            .iter()
            .find(|f| f.name.name_only == field_name)
            .ok_or_else(|| {
                BlendError::InvalidField(format!(
                    "Field {field_name} not found in struct {struct_name}"
                ))
            })?;

        self.read_pointer(field.offset)
    }

    /// Read a field value as u32
    pub fn read_field_u32(&self, struct_name: &str, field_name: &str) -> Result<u32> {
        let struct_def = self
            .dna
            .structs
            .iter()
            .find(|s| s.type_name == struct_name)
            .ok_or_else(|| BlendError::InvalidField(format!("Struct {struct_name} not found")))?;

        let field = struct_def
            .fields
            .iter()
            .find(|f| f.name.name_only == field_name)
            .ok_or_else(|| {
                BlendError::InvalidField(format!(
                    "Field {field_name} not found in struct {struct_name}"
                ))
            })?;

        self.read_u32(field.offset)
    }

    /// Read a field as a string (for character arrays like name[66])
    pub fn read_field_string(&self, struct_name: &str, field_name: &str) -> Result<String> {
        let struct_def = self
            .dna
            .structs
            .iter()
            .find(|s| s.type_name == struct_name)
            .ok_or_else(|| BlendError::InvalidField(format!("Struct {struct_name} not found")))?;

        let field = struct_def
            .fields
            .iter()
            .find(|f| f.name.name_only == field_name)
            .ok_or_else(|| {
                BlendError::InvalidField(format!(
                    "Field {field_name} not found in struct {struct_name}"
                ))
            })?;

        // Read the raw bytes from the field
        let start = field.offset;
        let size = field.size;

        if start + size > self.data.len() {
            return Err(BlendError::InvalidField(format!(
                "Field data exceeds block bounds: offset {} + size {} > {}",
                start,
                size,
                self.data.len()
            )));
        }

        let bytes = &self.data[start..start + size];

        // Convert bytes to string, handling null termination
        let string_bytes: Vec<u8> = bytes.iter().take_while(|&&b| b != 0).copied().collect();

        String::from_utf8(string_bytes)
            .map_err(|e| BlendError::InvalidField(format!("Invalid UTF-8 in field: {e}")))
    }
}
