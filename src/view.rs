use std::sync::Arc;

use crate::block::Block;
use crate::endian::{Endian, PtrWidth};
use crate::layout::{MemberLayout, StructLayout, compute_struct_layout};
use crate::pointer::OldPtr;
use crate::sdna::Sdna;
use crate::types::*;

#[derive(Clone, Debug)]
pub struct StructView<'a> {
    pub sdna: &'a Sdna,
    pub layout: Arc<StructLayout>,
    data: Arc<[u8]>,
    base: usize,
    pub endian: Endian,
    pub ptr_width: PtrWidth,
}

fn split_parent_field(path: &str) -> Option<(&str, &str)> {
    let mut it = path.rsplitn(2, '.');
    let field = it.next()?;
    let parent = it.next().unwrap_or("");
    Some((parent, field))
}

impl<'a> StructView<'a> {
    pub fn new(sdna: &'a Sdna, block: &Block, endian: Endian, ptr_width: PtrWidth) -> Option<Self> {
        if block.header.sdna_index < 0 {
            return None;
        }
        let sidx = block.header.sdna_index as u32;
        let layout = compute_struct_layout(sdna, sidx).ok()?;
        Some(Self {
            sdna,
            layout: Arc::new(layout),
            data: block.data.clone(),
            base: 0,
            endian,
            ptr_width,
        })
    }

    pub fn size(&self) -> usize {
        self.layout.size
    }

    pub fn member(&self, name: &str) -> Option<&MemberLayout> {
        self.layout
            .index_by_name
            .get(name)
            .and_then(|&i| self.layout.members.get(i))
    }

    fn slice_for(&self, m: &MemberLayout) -> Option<&[u8]> {
        let start = self.base.checked_add(m.offset)?;
        let end = start.checked_add(m.size)?;
        self.data.get(start..end)
    }

    pub fn get_ptr(&self, name: &str) -> Option<OldPtr> {
        let m = self.member(name)?;
        let bytes = self.slice_for(m)?;
        match self.ptr_width {
            PtrWidth::P32 if m.size >= 4 => Some(OldPtr::Ptr32(read_u32(self.endian, bytes))),
            PtrWidth::P64 if m.size >= 8 => Some(OldPtr::Ptr64(read_u64(self.endian, bytes))),
            _ => None,
        }
    }

    pub fn get_f32(&self, name: &str) -> Option<f32> {
        let m = self.member(name)?;
        if m.size < 4 {
            return None;
        }
        Some(read_f32(self.endian, self.slice_for(m)?))
    }

    pub fn get_f32_array(&self, name: &str) -> Option<Vec<f32>> {
        let m = self.member(name)?;
        if m.size % 4 != 0 {
            return None;
        }
        let mut out = Vec::with_capacity(m.size / 4);
        let mut off = 0usize;
        let bytes = self.slice_for(m)?;
        while off + 4 <= bytes.len() {
            out.push(read_f32(self.endian, &bytes[off..]));
            off += 4;
        }
        Some(out)
    }

    pub fn get_i32(&self, name: &str) -> Option<i32> {
        let m = self.member(name)?;
        if m.size < 4 {
            return None;
        }
        Some(read_i32(self.endian, self.slice_for(m)?))
    }

    // ---------- Vector and matrix typed getters ----------
    pub fn get_vec2(&self, name: &str) -> Option<[f32; 2]> {
        let v = self.get_f32_array(name)?;
        (v.len() >= 2).then(|| [v[0], v[1]])
    }

    pub fn get_vec3(&self, name: &str) -> Option<[f32; 3]> {
        let v = self.get_f32_array(name)?;
        (v.len() >= 3).then(|| [v[0], v[1], v[2]])
    }

    pub fn get_vec4(&self, name: &str) -> Option<[f32; 4]> {
        let v = self.get_f32_array(name)?;
        (v.len() >= 4).then(|| [v[0], v[1], v[2], v[3]])
    }

    pub fn get_mat3x3(&self, name: &str) -> Option<[[f32; 3]; 3]> {
        let v = self.get_f32_array(name)?;
        (v.len() >= 9).then(|| [[v[0], v[1], v[2]], [v[3], v[4], v[5]], [v[6], v[7], v[8]]])
    }

    pub fn get_mat4x4(&self, name: &str) -> Option<[[f32; 4]; 4]> {
        let v = self.get_f32_array(name)?;
        (v.len() >= 16).then(|| {
            [
                [v[0], v[1], v[2], v[3]],
                [v[4], v[5], v[6], v[7]],
                [v[8], v[9], v[10], v[11]],
                [v[12], v[13], v[14], v[15]],
            ]
        })
    }

    // ---------- Dotted-path access ----------
    /// Navigate a dotted path of value-embedded members and return the final StructView.
    /// All path segments must be value members whose types are structs.
    pub fn at_path_struct(&self, path: &str) -> Option<StructView<'a>> {
        let mut cur = self.clone();
        for seg in path.split('.') {
            if seg.is_empty() {
                continue;
            }
            cur = cur.at_member_struct(seg)?;
        }
        Some(cur)
    }

    /// Navigate path and read an f32.
    pub fn get_f32_path(&self, path: &str) -> Option<f32> {
        let (parent_path, field) = split_parent_field(path)?;
        let parent = if parent_path.is_empty() {
            self.clone()
        } else {
            self.at_path_struct(parent_path)?
        };
        parent.get_f32(field)
    }

    /// Navigate path and read an i32.
    pub fn get_i32_path(&self, path: &str) -> Option<i32> {
        let (parent_path, field) = split_parent_field(path)?;
        let parent = if parent_path.is_empty() {
            self.clone()
        } else {
            self.at_path_struct(parent_path)?
        };
        parent.get_i32(field)
    }

    /// Navigate path and read a pointer.
    pub fn get_ptr_path(&self, path: &str) -> Option<OldPtr> {
        let (parent_path, field) = split_parent_field(path)?;
        let parent = if parent_path.is_empty() {
            self.clone()
        } else {
            self.at_path_struct(parent_path)?
        };
        parent.get_ptr(field)
    }

    /// Navigate path and read all floats in a contiguous array.
    pub fn get_f32_array_path(&self, path: &str) -> Option<Vec<f32>> {
        let (parent_path, field) = split_parent_field(path)?;
        let parent = if parent_path.is_empty() {
            self.clone()
        } else {
            self.at_path_struct(parent_path)?
        };
        parent.get_f32_array(field)
    }

    pub fn get_vec3_path(&self, path: &str) -> Option<[f32; 3]> {
        let (parent_path, field) = split_parent_field(path)?;
        let parent = if parent_path.is_empty() {
            self.clone()
        } else {
            self.at_path_struct(parent_path)?
        };
        parent.get_vec3(field)
    }

    pub fn get_mat4x4_path(&self, path: &str) -> Option<[[f32; 4]; 4]> {
        let (parent_path, field) = split_parent_field(path)?;
        let parent = if parent_path.is_empty() {
            self.clone()
        } else {
            self.at_path_struct(parent_path)?
        };
        parent.get_mat4x4(field)
    }

    /// Return a child StructView for a value-embedded struct member.
    pub fn at_member_struct(&self, name: &str) -> Option<StructView<'a>> {
        let m = self.member(name)?;
        // Only value members can be embedded structs.
        if !matches!(m.kind, crate::member::MemberKind::Value) {
            return None;
        }
        let child_struct_index = self.sdna.struct_index_for_type_index(m.type_index)?;
        let child_layout =
            crate::layout::compute_struct_layout(self.sdna, child_struct_index).ok()?;
        Some(StructView {
            sdna: self.sdna,
            layout: Arc::new(child_layout),
            data: self.data.clone(),
            base: self.base + m.offset,
            endian: self.endian,
            ptr_width: self.ptr_width,
        })
    }

    /// For blocks that store an array of this struct (`header.count > 1`),
    /// get a view into the `idx`-th element.
    pub fn at_index(&self, idx: usize) -> Option<StructView<'a>> {
        let start = self.base.checked_add(self.layout.size.checked_mul(idx)?)?;
        Some(StructView {
            sdna: self.sdna,
            layout: self.layout.clone(),
            data: self.data.clone(),
            base: start,
            endian: self.endian,
            ptr_width: self.ptr_width,
        })
    }
}
