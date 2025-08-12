use crate::Error;
use std::sync::Arc;

pub fn maybe_decompress(raw: Arc<[u8]>) -> Result<Arc<[u8]>, Error> {
    if raw.len() >= 4 && raw[0..4] == [0x28, 0xB5, 0x2F, 0xFD] {
        let mut decoder = zstd::stream::read::Decoder::new(&*raw)
            .map_err(|e| Error::Decode(format!("zstd init: {e}")))?;
        let mut buf = Vec::new();
        use std::io::Read;
        decoder
            .read_to_end(&mut buf)
            .map_err(|e| Error::Decode(format!("zstd decode: {e}")))?;
        return Ok(buf.into_boxed_slice().into());
    }
    if raw.len() >= 2 {
        let m0 = raw[0];
        let m1 = raw[1];
        let is_gzip = m0 == 0x1F && m1 == 0x8B;
        let is_zlib = m0 == 0x78 && matches!(m1, 0x01 | 0x5E | 0x9C | 0xDA);
        if is_gzip || is_zlib {
            use flate2::read::{GzDecoder, ZlibDecoder};
            use std::io::Read;
            let mut buf = Vec::new();
            if is_gzip {
                let mut dec = GzDecoder::new(&*raw);
                dec.read_to_end(&mut buf)
                    .map_err(|e| Error::Decode(format!("gzip decode: {e}")))?;
            } else {
                let mut dec = ZlibDecoder::new(&*raw);
                dec.read_to_end(&mut buf)
                    .map_err(|e| Error::Decode(format!("zlib decode: {e}")))?;
            }
            return Ok(buf.into_boxed_slice().into());
        }
    }
    Ok(raw)
}
