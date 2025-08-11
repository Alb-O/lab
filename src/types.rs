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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endian_readers_u16_u32_u64() {
        // Value 0x1122 in both endian arrangements
        let le16 = [0x22, 0x11];
        let be16 = [0x11, 0x22];
        assert_eq!(read_u16(Endian::Little, &le16), 0x1122);
        assert_eq!(read_u16(Endian::Big, &be16), 0x1122);

        let le32 = [0x78, 0x56, 0x34, 0x12];
        let be32 = [0x12, 0x34, 0x56, 0x78];
        assert_eq!(read_u32(Endian::Little, &le32), 0x12345678);
        assert_eq!(read_u32(Endian::Big, &be32), 0x12345678);

        let le64 = [0xEF, 0xCD, 0xAB, 0x89, 0x67, 0x45, 0x23, 0x01];
        let be64 = [0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF];
        assert_eq!(read_u64(Endian::Little, &le64), 0x0123456789ABCDEF);
        assert_eq!(read_u64(Endian::Big, &be64), 0x0123456789ABCDEF);
    }

    #[test]
    fn endian_readers_i32_f32_f64() {
        let le32 = [0x78, 0x56, 0x34, 0x12];
        let be32 = [0x12, 0x34, 0x56, 0x78];
        assert_eq!(read_i32(Endian::Little, &le32), 0x12345678);
        assert_eq!(read_i32(Endian::Big, &be32), 0x12345678);

        let f32_val: f32 = 123.25;
        let f32_bits = f32_val.to_bits();
        let le = f32_bits.to_le_bytes();
        let be = f32_bits.to_be_bytes();
        assert!((read_f32(Endian::Little, &le) - f32_val).abs() < 1e-6);
        assert!((read_f32(Endian::Big, &be) - f32_val).abs() < 1e-6);

        let f64_val: f64 = -42.625;
        let f64_bits = f64_val.to_bits();
        let le = f64_bits.to_le_bytes();
        let be = f64_bits.to_be_bytes();
        assert!((read_f64(Endian::Little, &le) - f64_val).abs() < 1e-12);
        assert!((read_f64(Endian::Big, &be) - f64_val).abs() < 1e-12);
    }
}
