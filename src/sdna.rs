use std::collections::HashMap;
use std::sync::Arc;

use crate::endian::Endian;
use crate::endian::PtrWidth;
use crate::error::{BlendModelError, Result};
use crate::member::MemberNameSpec;

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct StructMember {
    pub type_index: u32,
    pub member_index: u32,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StructRef {
    pub type_index: u32,
    pub members: Arc<[StructMember]>,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub struct Sdna {
    pub pointer_size: PtrWidth,

    // types domain
    pub types: Arc<[Arc<str>]>,
    pub types_size: Arc<[u16]>,
    pub types_alignment: Arc<[u32]>,

    // structs domain
    pub structs: Arc<[StructRef]>,

    // members domain
    pub members: Arc<[Arc<str>]>,
    pub members_array_num: Arc<[u16]>,

    // fast lookups
    pub type_to_struct_index: HashMap<Arc<str>, u32>,
}

impl Sdna {
    pub fn struct_index_by_name(&self, name: &str) -> Option<u32> {
        self.type_to_struct_index.get(name).copied()
    }

    pub fn struct_by_index(&self, idx: u32) -> Result<&StructRef> {
        self.structs
            .get(idx as usize)
            .ok_or(BlendModelError::UnknownStructIndex(idx))
    }

    pub fn type_name(&self, type_index: u32) -> Result<&str> {
        self.types
            .get(type_index as usize)
            .map(|s| s.as_ref())
            .ok_or(BlendModelError::UnknownTypeIndex(type_index))
    }

    pub fn member_name(&self, member_index: u32) -> Result<&str> {
        self.members
            .get(member_index as usize)
            .map(|s| s.as_ref())
            .ok_or(BlendModelError::UnknownMemberIndex(member_index))
    }

    pub fn member_spec(&self, member_index: u32) -> Result<MemberNameSpec> {
        let name = self.member_name(member_index)?;
        MemberNameSpec::parse(name)
    }

    pub fn members_of_struct(&self, struct_index: u32) -> Result<&[StructMember]> {
        Ok(&self.struct_by_index(struct_index)?.members)
    }

    /// Map a type index to its struct index, if that type is a struct with a definition in SDNA.
    pub fn struct_index_for_type_index(&self, type_index: u32) -> Option<u32> {
        let name = self.types.get(type_index as usize)?;
        self.type_to_struct_index.get(name.as_ref()).copied()
    }

    /// Heuristic: returns true if the given struct is an ID-like struct:
    /// contains a first member named "id" whose type resolves to "ID".
    pub fn struct_is_id_like(&self, struct_index: u32) -> bool {
        if let Ok(s) = self.struct_by_index(struct_index) {
            if let Some(first) = s.members.first() {
                if let (Ok(tname), Ok(mname)) = (
                    self.type_name(first.type_index),
                    self.member_name(first.member_index),
                ) {
                    if mname == "id" && tname == "ID" {
                        return true;
                    }
                }
            }
        }
        false
    }
}

impl Sdna {
    pub fn decode_from_dna1(data: &[u8], ptr_width: PtrWidth, endian: Endian) -> Result<Sdna> {
        // Expect "SDNA" header followed by "NAME","TYPE","TLEN","STRC" sections.
        let mut off = 0usize;
        fn expect_bytes(data: &[u8], off: &mut usize, tag: &[u8]) -> Result<()> {
            if data.get(*off..*off + tag.len()) == Some(tag) {
                *off += tag.len();
                Ok(())
            } else {
                Err(BlendModelError::InvalidHeader)
            }
        }
        fn read_u32_at(endian: Endian, data: &[u8], off: &mut usize) -> Result<u32> {
            if *off + 4 > data.len() {
                return Err(BlendModelError::InvalidHeader);
            }
            let v = crate::types::read_u32(endian, &data[*off..*off + 4]);
            *off += 4;
            Ok(v)
        }
        fn read_u16_at(endian: Endian, data: &[u8], off: &mut usize) -> Result<u16> {
            if *off + 2 > data.len() {
                return Err(BlendModelError::InvalidHeader);
            }
            let v = crate::types::read_u16(endian, &data[*off..*off + 2]);
            *off += 2;
            Ok(v)
        }
        fn read_cstring<'a>(data: &'a [u8], off: &mut usize) -> Result<&'a str> {
            let start = *off;
            let mut i = start;
            while i < data.len() {
                if data[i] == 0 {
                    break;
                }
                i += 1;
            }
            if i >= data.len() {
                return Err(BlendModelError::InvalidHeader);
            }
            let s =
                std::str::from_utf8(&data[start..i]).map_err(|_| BlendModelError::InvalidHeader)?;
            *off = i + 1;
            Ok(s)
        }

        expect_bytes(data, &mut off, b"SDNA")?;
        expect_bytes(data, &mut off, b"NAME")?;
        let names_count = read_u32_at(endian, data, &mut off)? as usize;
        let mut names: Vec<Arc<str>> = Vec::with_capacity(names_count);
        for _ in 0..names_count {
            let s = read_cstring(data, &mut off)?;
            names.push(Arc::<str>::from(s));
        }
        off = (off + 3) & !3; // Align to 4

        expect_bytes(data, &mut off, b"TYPE")?;
        let types_count = read_u32_at(endian, data, &mut off)? as usize;
        let mut types_vec: Vec<Arc<str>> = Vec::with_capacity(types_count);
        for _ in 0..types_count {
            let s = read_cstring(data, &mut off)?;
            types_vec.push(Arc::<str>::from(s));
        }
        off = (off + 3) & !3;

        // TLEN section: u16 per type size.
        expect_bytes(data, &mut off, b"TLEN")?;
        let mut types_size: Vec<u16> = Vec::with_capacity(types_count);
        for _ in 0..types_count {
            let sz = read_u16_at(endian, data, &mut off)?;
            types_size.push(sz);
        }
        off = (off + 3) & !3;

        // STRC section
        expect_bytes(data, &mut off, b"STRC")?;
        let struct_count = read_u32_at(endian, data, &mut off)? as usize;
        let members: Vec<Arc<str>> = names; // reuse names vector ref for members domain
        let mut members_array_num: Vec<u16> = vec![1; members.len()];
        // Pre-compute array counts for members by parsing their names.
        for (i, s) in members.iter().enumerate() {
            if let Ok(spec) = MemberNameSpec::parse(s) {
                members_array_num[i] = spec.array.len() as u16;
            }
        }
        let mut structs_v: Vec<StructRef> = Vec::with_capacity(struct_count);
        for _ in 0..struct_count {
            let type_index = read_u16_at(endian, data, &mut off)? as u32;
            let member_count = read_u16_at(endian, data, &mut off)? as usize;
            let mut mlist = Vec::with_capacity(member_count);
            for _ in 0..member_count {
                let t_index = read_u16_at(endian, data, &mut off)? as u32;
                let m_index = read_u16_at(endian, data, &mut off)? as u32;
                mlist.push(StructMember {
                    type_index: t_index,
                    member_index: m_index,
                });
            }
            structs_v.push(StructRef {
                type_index,
                members: mlist.into(),
            });
        }
        // Build type_to_struct_index
        let types: Arc<[Arc<str>]> = types_vec.into();
        let types_alignment: Arc<[u32]> = types
            .iter()
            .enumerate()
            .map(|(i, _)| {
                let sz = *types_size.get(i).unwrap_or(&0) as u32;
                match sz {
                    0 => 1,
                    1 => 1,
                    2 => 2,
                    3 | 4 => 4,
                    5..=8 => 8,
                    _ => 8,
                }
            })
            .collect::<Vec<_>>()
            .into();
        let mut type_to_struct_index = HashMap::new();
        let structs: Arc<[StructRef]> = structs_v.into();
        for (i, s) in structs.iter().enumerate() {
            let name = types[s.type_index as usize].clone();
            type_to_struct_index.insert(name, i as u32);
        }
        Ok(Sdna {
            pointer_size: ptr_width,
            types,
            types_size: types_size.into(),
            types_alignment,
            structs,
            members: members.into(),
            members_array_num: members_array_num.into(),
            type_to_struct_index,
        })
    }
}
