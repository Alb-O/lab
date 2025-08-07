use crate::error::{Error, Result};
use crate::header::BlendFileHeader;
use std::collections::HashMap;
use std::io::{Read, Seek, SeekFrom};

#[derive(Debug, Clone)]
pub struct DnaName {
    pub name_full: String,
    pub name_only: String,
    pub is_pointer: bool,
    pub is_method_pointer: bool,
    pub array_size: usize,
}

#[derive(Debug, Clone)]
pub struct DnaField {
    pub type_name: String,
    pub name: DnaName,
    pub size: usize,
    pub offset: usize,
}

#[derive(Debug, Clone)]
pub struct DnaStruct {
    pub type_name: String,
    pub size: usize,
    pub fields: Vec<DnaField>,
    fields_by_name: HashMap<String, usize>,
}

#[derive(Debug)]
pub struct DnaCollection {
    pub structs: Vec<DnaStruct>,
    struct_index: HashMap<String, usize>,
    pub types: Vec<String>,
    pub names: Vec<DnaName>,
    pub type_sizes: Vec<u16>,
}

impl DnaName {
    fn new(name_full: String) -> Self {
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

        let end = if is_method_pointer {
            bytes.iter().position(|&b| b == b')').unwrap_or(bytes.len())
        } else {
            bytes
                .iter()
                .position(|&b| b == b'[' || b == b'(')
                .unwrap_or(bytes.len())
        };
        let end = if end < start { bytes.len() } else { end };
        let name_only = String::from_utf8_lossy(&bytes[start..end]).into_owned();

        let array_size = Self::calc_array_size_fast(bytes);

        DnaName {
            name_full,
            name_only,
            is_pointer,
            is_method_pointer,
            array_size,
        }
    }

    fn calc_array_size_fast(bytes: &[u8]) -> usize {
        let mut result = 1;
        let mut i = 0;

        while i < bytes.len() {
            if bytes[i] == b'[' {
                i += 1;
                let start = i;
                while i < bytes.len() && bytes[i] != b']' {
                    i += 1;
                }
                if i < bytes.len() {
                    if let Ok(size_str) = std::str::from_utf8(&bytes[start..i]) {
                        if let Ok(size) = size_str.parse::<usize>() {
                            result *= size;
                        }
                    }
                }
            }
            i += 1;
        }

        result
    }
}

impl DnaStruct {
    fn new(type_name: String, size: usize) -> Self {
        DnaStruct {
            type_name,
            size,
            fields: Vec::new(),
            fields_by_name: HashMap::new(),
        }
    }

    /// Create a new DnaStruct for testing purposes
    #[cfg(test)]
    pub fn new_for_test(type_name: String, size: usize, fields: Vec<DnaField>) -> Self {
        let mut fields_by_name = HashMap::new();
        for (i, field) in fields.iter().enumerate() {
            fields_by_name.insert(field.name.name_only.clone(), i);
        }

        DnaStruct {
            type_name,
            size,
            fields,
            fields_by_name,
        }
    }

    fn add_field(&mut self, field: DnaField) {
        let field_index = self.fields.len();
        let field_name = field.name.name_only.clone();
        self.fields.push(field);
        self.fields_by_name.insert(field_name, field_index);
    }

    pub fn find_field(&self, name: &str) -> Option<&DnaField> {
        self.fields_by_name
            .get(name)
            .map(|&index| &self.fields[index])
    }
}

impl DnaCollection {
    /// Create a new DnaCollection for testing purposes
    #[cfg(test)]
    pub fn new_for_test(
        structs: Vec<DnaStruct>,
        types: Vec<String>,
        names: Vec<DnaName>,
        type_sizes: Vec<u16>,
    ) -> Self {
        let mut struct_index = HashMap::new();
        for (i, struct_def) in structs.iter().enumerate() {
            struct_index.insert(struct_def.type_name.clone(), i);
        }

        DnaCollection {
            structs,
            struct_index,
            types,
            names,
            type_sizes,
        }
    }
    pub fn read<R: Read + Seek>(reader: &mut R, header: &BlendFileHeader) -> Result<Self> {
        let mut sdna_marker = [0u8; 4];
        reader.read_exact(&mut sdna_marker)?;
        if &sdna_marker != b"SDNA" {
            return Err(Error::blend_file(
                format!("Expected SDNA marker, got: {sdna_marker:?}"),
                crate::error::BlendFileErrorKind::DnaError,
            ));
        }

        let names = Self::read_names_section(reader, header)?;
        let types = Self::read_types_section(reader, header)?;
        let type_sizes = Self::read_type_lengths_section(reader, header, types.len())?;
        let structs = Self::read_structures_section(reader, header, &names, &types, &type_sizes)?;

        let mut struct_index = HashMap::new();
        for (i, struct_def) in structs.iter().enumerate() {
            struct_index.insert(struct_def.type_name.clone(), i);
        }

        Ok(DnaCollection {
            structs,
            struct_index,
            types,
            names,
            type_sizes,
        })
    }

    fn read_names_section<R: Read + Seek>(
        reader: &mut R,
        header: &BlendFileHeader,
    ) -> Result<Vec<DnaName>> {
        let mut name_marker = [0u8; 4];
        reader.read_exact(&mut name_marker)?;
        if &name_marker != b"NAME" {
            return Err(Error::blend_file(
                format!("Expected NAME marker, got: {name_marker:?}"),
                crate::error::BlendFileErrorKind::DnaError,
            ));
        }

        let names_count = read_u32(reader, header.is_little_endian)?;
        // Validate count to prevent excessive memory allocation
        if names_count > 1_000_000 {
            return Err(Error::blend_file(
                format!("Unreasonably large names count: {names_count}"),
                crate::error::BlendFileErrorKind::DnaError,
            ));
        }
        let mut names = Vec::with_capacity(names_count as usize);

        for _ in 0..names_count {
            let name_str = read_null_terminated_string(reader)?;
            names.push(DnaName::new(name_str));
        }

        Ok(names)
    }

    fn read_types_section<R: Read + Seek>(
        reader: &mut R,
        header: &BlendFileHeader,
    ) -> Result<Vec<String>> {
        find_and_seek_to_marker(reader, b"TYPE", "names section")?;

        let types_count = read_u32(reader, header.is_little_endian)?;
        // Validate count to prevent excessive memory allocation
        if types_count > 1_000_000 {
            return Err(Error::blend_file(
                format!("Unreasonably large types count: {types_count}"),
                crate::error::BlendFileErrorKind::DnaError,
            ));
        }
        let mut types = Vec::with_capacity(types_count as usize);

        for _ in 0..types_count {
            let type_str = read_null_terminated_string(reader)?;
            types.push(type_str);
        }

        Ok(types)
    }

    fn read_type_lengths_section<R: Read + Seek>(
        reader: &mut R,
        header: &BlendFileHeader,
        type_count: usize,
    ) -> Result<Vec<u16>> {
        find_and_seek_to_marker(reader, b"TLEN", "types section")?;

        let mut type_sizes = Vec::with_capacity(type_count);
        for _ in 0..type_count {
            let size = read_u16(reader, header.is_little_endian)?;
            type_sizes.push(size);
        }

        Ok(type_sizes)
    }

    fn read_structures_section<R: Read + Seek>(
        reader: &mut R,
        header: &BlendFileHeader,
        names: &[DnaName],
        types: &[String],
        type_sizes: &[u16],
    ) -> Result<Vec<DnaStruct>> {
        find_and_seek_to_marker(reader, b"STRC", "type lengths section")?;

        let struct_count = read_u32(reader, header.is_little_endian)?;
        // Validate count to prevent excessive memory allocation
        if struct_count > 100_000 {
            return Err(Error::blend_file(
                format!("Unreasonably large struct count: {struct_count}"),
                crate::error::BlendFileErrorKind::DnaError,
            ));
        }
        let mut structs = Vec::with_capacity(struct_count as usize);

        for _ in 0..struct_count {
            let struct_type_index = read_u16(reader, header.is_little_endian)? as usize;
            let field_count = read_u16(reader, header.is_little_endian)? as usize;

            if struct_type_index >= types.len() {
                return Err(Error::blend_file(
                    format!("Invalid struct type index: {struct_type_index}"),
                    crate::error::BlendFileErrorKind::DnaError,
                ));
            }

            let type_name = types[struct_type_index].clone();
            let struct_size = type_sizes[struct_type_index] as usize;
            let mut dna_struct = DnaStruct::new(type_name, struct_size);

            let mut field_offset = 0;

            for _ in 0..field_count {
                let field_type_index = read_u16(reader, header.is_little_endian)? as usize;
                let field_name_index = read_u16(reader, header.is_little_endian)? as usize;

                if field_type_index >= types.len() {
                    return Err(Error::blend_file(
                        format!("Invalid field type index: {field_type_index}"),
                        crate::error::BlendFileErrorKind::DnaError,
                    ));
                }

                if field_name_index >= names.len() {
                    return Err(Error::blend_file(
                        format!("Invalid field name index: {field_name_index}"),
                        crate::error::BlendFileErrorKind::DnaError,
                    ));
                }

                let field_type_name = types[field_type_index].clone();
                let field_name = names[field_name_index].clone();

                let field_size = if field_name.is_pointer || field_name.is_method_pointer {
                    header.pointer_size as usize * field_name.array_size
                } else {
                    type_sizes[field_type_index] as usize * field_name.array_size
                };

                let field = DnaField {
                    type_name: field_type_name,
                    name: field_name,
                    size: field_size,
                    offset: field_offset,
                };

                field_offset += field_size;
                dna_struct.add_field(field);
            }

            structs.push(dna_struct);
        }

        Ok(structs)
    }

    pub fn get_struct(&self, index: usize) -> Option<&DnaStruct> {
        self.structs.get(index)
    }

    pub fn find_struct(&self, name: &str) -> Option<&DnaStruct> {
        self.struct_index
            .get(name)
            .and_then(|&index| self.structs.get(index))
    }
}

fn read_u16<R: Read>(reader: &mut R, is_little_endian: bool) -> Result<u16> {
    let mut buf = [0u8; 2];
    reader.read_exact(&mut buf)?;
    Ok(if is_little_endian {
        u16::from_le_bytes(buf)
    } else {
        u16::from_be_bytes(buf)
    })
}

fn read_u32<R: Read>(reader: &mut R, is_little_endian: bool) -> Result<u32> {
    let mut buf = [0u8; 4];
    reader.read_exact(&mut buf)?;
    Ok(if is_little_endian {
        u32::from_le_bytes(buf)
    } else {
        u32::from_be_bytes(buf)
    })
}

fn read_null_terminated_string<R: Read>(reader: &mut R) -> Result<String> {
    let mut bytes = Vec::with_capacity(32);
    loop {
        let mut byte = [0u8; 1];
        reader.read_exact(&mut byte)?;
        if byte[0] == 0 {
            break;
        }
        bytes.push(byte[0]);
    }

    String::from_utf8(bytes).map_err(|_| {
        Error::blend_file(
            "Invalid UTF-8 in DNA string",
            crate::error::BlendFileErrorKind::InvalidData,
        )
    })
}

fn find_and_seek_to_marker<R: Read + Seek>(
    reader: &mut R,
    marker: &[u8; 4],
    section_name: &str,
) -> Result<()> {
    let start_pos = reader.stream_position()?;

    let mut search_buffer = [0u8; 11];
    let bytes_read = reader.read(&mut search_buffer).unwrap_or(0);

    for i in 0..=(bytes_read.saturating_sub(4)) {
        if &search_buffer[i..i + 4] == marker {
            reader.seek(SeekFrom::Start(start_pos + i as u64 + 4))?;
            return Ok(());
        }
    }

    for offset in 0..8 {
        reader.seek(SeekFrom::Start(start_pos + offset))?;
        let mut marker_check = [0u8; 4];
        if reader.read_exact(&mut marker_check).is_ok() && &marker_check == marker {
            reader.seek(SeekFrom::Start(start_pos + offset + 4))?;
            return Ok(());
        }
    }

    Err(Error::blend_file(
        format!(
            "{} marker not found after {}",
            String::from_utf8_lossy(marker),
            section_name
        ),
        crate::error::BlendFileErrorKind::DnaError,
    ))
}
