use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;
use std::sync::Arc;

use crate::bhead::{BHead, BHeadKind, BlockCode};
use crate::block::Block;
use crate::endian::Endian;
use crate::error::{BlendModelError, Result};
use crate::header::BlenderHeader;
use crate::header_decode::decode_header_bytes;
use crate::pointer::OldPtr;
use crate::registry::BlockRegistry;
use crate::sdna::Sdna;
use crate::types::{read_i32, read_i64, read_u32, read_u64};

fn read_exact_or<T: Read>(r: &mut T, buf: &mut [u8]) -> Result<()> {
    r.read_exact(buf).map_err(|_| BlendModelError::InvalidHeader)?;
    Ok(())
}

fn read_i32_e(endian: Endian, buf: &[u8]) -> i32 {
    read_i32(endian, buf)
}
fn read_u32_e(endian: Endian, buf: &[u8]) -> u32 {
    read_u32(endian, buf)
}
fn read_i64_e(endian: Endian, buf: &[u8]) -> i64 {
    read_i64(endian, buf)
}
fn read_u64_e(endian: Endian, buf: &[u8]) -> u64 {
    read_u64(endian, buf)
}

fn read_header<R: Read + Seek>(r: &mut R) -> Result<BlenderHeader> {
    let mut min = [0u8; 12];
    read_exact_or(r, &mut min)?;
    if &min[..7] != b"BLENDER" {
        return Err(BlendModelError::InvalidHeader);
    }
    // Peek if legacy or new format; if new, we need full 17 bytes.
    let header = if min[7] == b'_' || min[7] == b'-' {
        decode_header_bytes(&min)?
    } else {
        let mut rest = [0u8; 5];
        read_exact_or(r, &mut rest)?;
        let mut all = [0u8; 17];
        all[..12].copy_from_slice(&min);
        all[12..].copy_from_slice(&rest);
        decode_header_bytes(&all)?
    };
    Ok(header)
}

fn align4(pos: i64) -> i64 {
    let rem = pos & 3;
    if rem == 0 { 0 } else { 4 - rem }
}

fn read_next_bhead<R: Read + Seek>(r: &mut R, header: &BlenderHeader) -> Result<Option<BHead>> {
    let endian = header.endian;
    let mut bhead = None;
    if header.ptr_width.bytes() == 4 {
        let mut buf = [0u8; 20];
        if r.read(&mut buf)? == 0 {
            return Ok(None);
        }
        let code = read_u32_e(endian, &buf[0..4]);
        let len = read_i32_e(endian, &buf[4..8]) as i64;
        let old_u32 = read_u32_e(endian, &buf[8..12]);
        let sdna = read_i32_e(endian, &buf[12..16]) as i64;
        let nr = read_i32_e(endian, &buf[16..20]) as i64;
        bhead = Some(BHead {
            code: BlockCode(code),
            sdna_index: sdna,
            old: if old_u32 == 0 { OldPtr::Null } else { OldPtr::Ptr32(old_u32) },
            len,
            count: nr,
            kind: BHeadKind::BHead4,
        });
    } else if !header.bhead_large() {
        let mut buf = [0u8; 32];
        if r.read(&mut buf)? == 0 {
            return Ok(None);
        }
        let code = read_u32_e(endian, &buf[0..4]);
        let len = read_i32_e(endian, &buf[4..8]) as i64;
        let old = read_u64_e(endian, &buf[8..16]);
        let sdna = read_i32_e(endian, &buf[16..20]) as i64;
        let nr = read_i32_e(endian, &buf[20..24]) as i64;
        bhead = Some(BHead {
            code: BlockCode(code),
            sdna_index: sdna,
            old: if old == 0 { OldPtr::Null } else { OldPtr::Ptr64(old) },
            len,
            count: nr,
            kind: BHeadKind::SmallBHead8,
        });
    } else {
        let mut buf = [0u8; 32];
        if r.read(&mut buf)? == 0 {
            return Ok(None);
        }
        let code = read_u32_e(endian, &buf[0..4]);
        let sdna = read_i32_e(endian, &buf[4..8]) as i64;
        let old = read_u64_e(endian, &buf[8..16]);
        let len = read_i64_e(endian, &buf[16..24]);
        let nr = read_i64_e(endian, &buf[24..32]);
        bhead = Some(BHead {
            code: BlockCode(code),
            sdna_index: sdna,
            old: if old == 0 { OldPtr::Null } else { OldPtr::Ptr64(old) },
            len,
            count: nr,
            kind: BHeadKind::LargeBHead8,
        });
    }
    Ok(bhead)
}

/// Parse a .blend file into header, sdna and blocks (registered by OldPtr).
pub fn read_blend_file<P: AsRef<Path>>(path: P) -> Result<(BlenderHeader, Sdna, BlockRegistry)> {
    let mut f = File::open(path)?;
    let header = read_header(&mut f)?;
    let registry = BlockRegistry::default();
    let mut sdna_opt: Option<Sdna> = None;
    loop {
        let here = f.stream_position()? as i64;
        let bh = match read_next_bhead(&mut f, &header)? {
            Some(h) => h,
            None => break,
        };
        if bh.code.is_end() {
            break;
        }
        // Read payload
        let mut data = vec![0u8; bh.len as usize];
        if bh.len > 0 {
            f.read_exact(&mut data)?;
        }
        // Align to 4 bytes
        let consumed = (f.stream_position()? as i64) - here;
        let pad = align4(consumed);
        if pad > 0 {
            f.seek(SeekFrom::Current(pad))?;
        }

        if bh.code.is_dna() {
            // Decode SDNA
            let sdna = Sdna::decode_from_dna1(&data, header.ptr_width, header.endian)?;
            sdna_opt = Some(sdna);
        } else {
            let block = Arc::new(Block {
                header: bh.clone(),
                data: data.into(),
            });
            registry.insert(block);
        }
    }

    let sdna = sdna_opt.ok_or(BlendModelError::InvalidHeader)?;
    Ok((header, sdna, registry))
}

