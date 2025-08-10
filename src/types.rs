use crate::endian::Endian;

#[inline]
fn swap_u16(x: u16) -> u16 {
    x.swap_bytes()
}
#[inline]
fn swap_u32(x: u32) -> u32 {
    x.swap_bytes()
}
#[inline]
fn swap_u64(x: u64) -> u64 {
    x.swap_bytes()
}

#[inline]
fn needs_swap(file_endian: Endian) -> bool {
    !matches!(
        (cfg!(target_endian = "little"), file_endian),
        (true, Endian::Little) | (false, Endian::Big)
    )
}

#[inline]
pub fn read_u16(endian: Endian, bytes: &[u8]) -> u16 {
    let mut arr = [0u8; 2];
    arr.copy_from_slice(&bytes[..2]);
    let v = u16::from_ne_bytes(arr);
    if needs_swap(endian) { swap_u16(v) } else { v }
}

#[inline]
pub fn read_i16(endian: Endian, bytes: &[u8]) -> i16 {
    read_u16(endian, bytes) as i16
}

#[inline]
pub fn read_u32(endian: Endian, bytes: &[u8]) -> u32 {
    let mut arr = [0u8; 4];
    arr.copy_from_slice(&bytes[..4]);
    let v = u32::from_ne_bytes(arr);
    if needs_swap(endian) { swap_u32(v) } else { v }
}

#[inline]
pub fn read_i32(endian: Endian, bytes: &[u8]) -> i32 {
    read_u32(endian, bytes) as i32
}

#[inline]
pub fn read_u64(endian: Endian, bytes: &[u8]) -> u64 {
    let mut arr = [0u8; 8];
    arr.copy_from_slice(&bytes[..8]);
    let v = u64::from_ne_bytes(arr);
    if needs_swap(endian) { swap_u64(v) } else { v }
}

#[inline]
pub fn read_i64(endian: Endian, bytes: &[u8]) -> i64 {
    read_u64(endian, bytes) as i64
}

#[inline]
pub fn read_f32(endian: Endian, bytes: &[u8]) -> f32 {
    let bits = read_u32(endian, bytes);
    f32::from_bits(bits)
}

#[inline]
pub fn read_f64(endian: Endian, bytes: &[u8]) -> f64 {
    let bits = read_u64(endian, bytes);
    f64::from_bits(bits)
}
