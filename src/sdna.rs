use crate::{Error, format::Endian};

#[derive(Clone, Debug)]
pub struct SdnaInfo {
    pub names_len: usize,
    pub types_len: usize,
    pub structs_len: usize,
}

fn align4(idx: usize) -> usize {
    (idx + 3) & !3
}

fn expect_tag(bytes: &[u8], idx: usize, tag: &[u8; 4]) -> Result<(), Error> {
    if bytes.len() < idx + 4 {
        return Err(Error::Eof);
    }
    if &bytes[idx..idx + 4] != tag {
        return Err(Error::Decode(format!(
            "expected {:?} tag",
            std::str::from_utf8(tag).unwrap()
        )));
    }
    Ok(())
}

fn read_u32_le(bytes: &[u8], idx: usize, endian: Endian) -> Result<(u32, usize), Error> {
    if bytes.len() < idx + 4 {
        return Err(Error::Eof);
    }
    let mut val = u32::from_le_bytes(bytes[idx..idx + 4].try_into().unwrap());
    if let Endian::Big = endian {
        val = val.swap_bytes();
    }
    Ok((val, idx + 4))
}

pub fn decode_sdna(bytes: &[u8], endian: Endian) -> Result<SdnaInfo, Error> {
    let mut i = 0usize;
    expect_tag(bytes, i, b"SDNA")?;
    i += 4;
    expect_tag(bytes, i, b"NAME")?;
    i += 4;
    let (names_count, i2) = read_u32_le(bytes, i, endian)?;
    i = i2;
    for _ in 0..names_count {
        let mut j = i;
        while j < bytes.len() && bytes[j] != 0 {
            j += 1;
        }
        if j >= bytes.len() {
            return Err(Error::Eof);
        }
        i = j + 1;
    }
    i = align4(i);

    expect_tag(bytes, i, b"TYPE")?;
    i += 4;
    let (types_count, i3) = read_u32_le(bytes, i, endian)?;
    i = i3;
    for _ in 0..types_count {
        let mut j = i;
        while j < bytes.len() && bytes[j] != 0 {
            j += 1;
        }
        if j >= bytes.len() {
            return Err(Error::Eof);
        }
        i = j + 1;
    }
    i = align4(i);

    expect_tag(bytes, i, b"TLEN")?;
    i += 4;
    let need = (types_count as usize) * 2;
    if bytes.len() < i + need {
        return Err(Error::Eof);
    }
    i += need;
    if (types_count & 1) != 0 {
        i += 2;
    }

    expect_tag(bytes, i, b"STRC")?;
    i += 4;
    let (structs_count, i4) = read_u32_le(bytes, i, endian)?;
    i = i4;
    for _ in 0..structs_count {
        if bytes.len() < i + 4 {
            return Err(Error::Eof);
        }
        i += 4;
        let members_num_raw = u16::from_le_bytes(bytes[i - 2..i].try_into().unwrap());
        let members_num = match endian {
            Endian::Little => members_num_raw,
            Endian::Big => members_num_raw.swap_bytes(),
        } as usize;
        let member_rec_sz: usize = 4;
        let bytes_to_advance = member_rec_sz
            .checked_mul(members_num)
            .ok_or_else(|| Error::Decode("overflow".into()))?;
        if bytes.len() < i + bytes_to_advance {
            return Err(Error::Eof);
        }
        i += bytes_to_advance;
    }

    Ok(SdnaInfo {
        names_len: names_count as usize,
        types_len: types_count as usize,
        structs_len: structs_count as usize,
    })
}
