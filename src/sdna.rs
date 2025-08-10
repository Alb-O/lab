use std::collections::HashMap;
use std::sync::Arc;

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
