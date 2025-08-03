use crate::error::{BlendError, Result};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Policy controlling how compressed files are handled
#[derive(Debug, Clone)]
pub struct DecompressionPolicy {
    /// Maximum size to decompress into memory (default: 256MB)
    pub max_in_memory_bytes: usize,
    /// Prefer memory-mapped temp files when available (default: true if mmap feature enabled)
    pub prefer_mmap_temp: bool,
    /// Custom temp directory (default: OS temp dir)
    pub temp_dir: Option<PathBuf>,
    /// Allow streaming decompression (default: false, parser requires Read + Seek)
    pub allow_streaming: bool,
}

impl Default for DecompressionPolicy {
    fn default() -> Self {
        Self {
            max_in_memory_bytes: 256 * 1024 * 1024, // 256MB
            prefer_mmap_temp: cfg!(feature = "mmap"),
            temp_dir: None,
            allow_streaming: false,
        }
    }
}

/// Options for parsing operations
#[derive(Debug, Clone, Default)]
pub struct ParseOptions {
    pub decompression_policy: DecompressionPolicy,
}

/// Mode used for decompression
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecompressionMode {
    /// No compression detected
    None,
    /// Zstd decompressed into memory
    ZstdInMemory,
    /// Zstd decompressed to temp file with mmap
    ZstdTempMmap,
    /// Zstd decompressed to temp file
    ZstdTempFile,
}

/// Detected compression type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionKind {
    None,
    Zstd,
}

/// Abstraction over different backing storage for blend file data
pub enum BlendRead {
    /// In-memory data
    Memory(Arc<Vec<u8>>),
    /// Memory-mapped temp file
    #[cfg(feature = "mmap")]
    TempMmap(Arc<memmap2::Mmap>, PathBuf),
    /// Regular temp file
    TempFile(File, PathBuf),
    /// Regular file (uncompressed)
    File(File),
}

/// Reader wrapper that implements Read + Seek for memory-backed data
pub struct MemoryCursor {
    data: Arc<Vec<u8>>,
    position: usize,
}

impl MemoryCursor {
    pub fn new(data: Arc<Vec<u8>>) -> Self {
        Self { data, position: 0 }
    }
}

impl Read for MemoryCursor {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let available = self.data.len().saturating_sub(self.position);
        let to_read = buf.len().min(available);

        if to_read > 0 {
            buf[..to_read].copy_from_slice(&self.data[self.position..self.position + to_read]);
            self.position += to_read;
        }

        Ok(to_read)
    }
}

impl Seek for MemoryCursor {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        let new_pos = match pos {
            SeekFrom::Start(offset) => offset as i64,
            SeekFrom::End(offset) => self.data.len() as i64 + offset,
            SeekFrom::Current(offset) => self.position as i64 + offset,
        };

        if new_pos < 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Seek position cannot be negative",
            ));
        }

        self.position = new_pos as usize;
        Ok(self.position as u64)
    }
}

/// Reader wrapper for memory-mapped files
#[cfg(feature = "mmap")]
pub struct MmapCursor {
    mmap: Arc<memmap2::Mmap>,
    position: usize,
}

#[cfg(feature = "mmap")]
impl MmapCursor {
    pub fn new(mmap: Arc<memmap2::Mmap>) -> Self {
        Self { mmap, position: 0 }
    }
}

#[cfg(feature = "mmap")]
impl Read for MmapCursor {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let available = self.mmap.len().saturating_sub(self.position);
        let to_read = buf.len().min(available);

        if to_read > 0 {
            buf[..to_read].copy_from_slice(&self.mmap[self.position..self.position + to_read]);
            self.position += to_read;
        }

        Ok(to_read)
    }
}

#[cfg(feature = "mmap")]
impl Seek for MmapCursor {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        let new_pos = match pos {
            SeekFrom::Start(offset) => offset as i64,
            SeekFrom::End(offset) => self.mmap.len() as i64 + offset,
            SeekFrom::Current(offset) => self.position as i64 + offset,
        };

        if new_pos < 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Seek position cannot be negative",
            ));
        }

        self.position = new_pos as usize;
        Ok(self.position as u64)
    }
}

/// Detect compression type from reader
pub fn detect_compression<R: Read + Seek>(reader: &mut R) -> Result<CompressionKind> {
    let mut magic = [0u8; 4];
    reader.read_exact(&mut magic)?;
    reader.seek(SeekFrom::Start(0))?;

    // Zstandard magic number: 0x28B52FFD (little endian: FD 2F B5 28)
    if magic == [0x28, 0xB5, 0x2F, 0xFD] {
        Ok(CompressionKind::Zstd)
    } else {
        Ok(CompressionKind::None)
    }
}

/// Trait for decompressing different formats
pub trait Decompressor {
    fn decompress<R: Read + Seek>(
        &self,
        reader: &mut R,
        estimated_size: Option<u64>,
        policy: &DecompressionPolicy,
    ) -> Result<BlendRead>;
}

/// Zstd decompressor implementation
pub struct ZstdDecompressor;

impl Decompressor for ZstdDecompressor {
    fn decompress<R: Read + Seek>(
        &self,
        reader: &mut R,
        estimated_size: Option<u64>,
        policy: &DecompressionPolicy,
    ) -> Result<BlendRead> {
        // First, try to estimate decompressed size
        let should_use_memory = estimated_size
            .map(|size| size <= policy.max_in_memory_bytes as u64)
            .unwrap_or(true); // Default to memory for unknown sizes, will spillover if needed

        if should_use_memory {
            // Try in-memory decompression first
            match self.decompress_to_memory(reader, policy.max_in_memory_bytes) {
                Ok(data) => return Ok(BlendRead::Memory(Arc::new(data))),
                Err(BlendError::SizeLimitExceeded(_)) => {
                    // Spillover to temp file
                    log::debug!("Size limit exceeded, spilling over to temp file");
                    reader.seek(SeekFrom::Start(0))?;
                }
                Err(e) => return Err(e),
            }
        }

        // Use temp file approach
        self.decompress_to_temp(reader, policy)
    }
}

impl ZstdDecompressor {
    fn decompress_to_memory<R: Read>(&self, reader: &mut R, max_size: usize) -> Result<Vec<u8>> {
        #[cfg(feature = "zstd")]
        {
            use std::io::BufReader;

            let mut decoder = zstd::Decoder::new(BufReader::new(reader))?;
            let mut result = Vec::new();
            let mut buffer = [0u8; 8192];

            loop {
                let bytes_read = decoder.read(&mut buffer)?;
                if bytes_read == 0 {
                    break;
                }

                if result.len() + bytes_read > max_size {
                    return Err(BlendError::SizeLimitExceeded(max_size));
                }

                result.extend_from_slice(&buffer[..bytes_read]);
            }

            Ok(result)
        }

        #[cfg(not(feature = "zstd"))]
        {
            Err(BlendError::UnsupportedCompression(
                "Zstd support not compiled in".to_string(),
            ))
        }
    }

    fn decompress_to_temp<R: Read>(
        &self,
        reader: &mut R,
        policy: &DecompressionPolicy,
    ) -> Result<BlendRead> {
        #[cfg(feature = "zstd")]
        {
            use std::io::BufReader;

            // Create temp file
            let default_temp = std::env::temp_dir();
            let temp_dir = policy.temp_dir.as_deref().unwrap_or(default_temp.as_path());
            let temp_file =
                tempfile::NamedTempFile::new_in(temp_dir).map_err(BlendError::TempFileError)?;
            let temp_path = temp_file.path().to_path_buf();

            // Decompress to temp file
            {
                let mut decoder = zstd::Decoder::new(BufReader::new(reader))?;
                let mut writer = temp_file.as_file();
                std::io::copy(&mut decoder, &mut writer)?;
                writer.flush()?;
            }

            let file = temp_file.into_file();

            // Try mmap if preferred and available
            #[cfg(feature = "mmap")]
            if policy.prefer_mmap_temp {
                match unsafe { memmap2::Mmap::map(&file) } {
                    Ok(mmap) => {
                        log::debug!("Successfully memory-mapped temp file: {temp_path:?}");
                        return Ok(BlendRead::TempMmap(Arc::new(mmap), temp_path));
                    }
                    Err(e) => {
                        log::warn!("Failed to mmap temp file, falling back to regular file: {e}");
                    }
                }
            }

            Ok(BlendRead::TempFile(file, temp_path))
        }

        #[cfg(not(feature = "zstd"))]
        {
            Err(BlendError::UnsupportedCompression(
                "Zstd support not compiled in".to_string(),
            ))
        }
    }
}

/// Open a blend file source with decompression handling
pub fn open_source<P: AsRef<Path>>(
    path: P,
    policy: Option<&DecompressionPolicy>,
) -> Result<BlendRead> {
    let default_policy = DecompressionPolicy::default();
    let policy = policy.unwrap_or(&default_policy);
    let mut file = File::open(path.as_ref())?;

    let compression = detect_compression(&mut file)?;

    match compression {
        CompressionKind::None => Ok(BlendRead::File(file)),
        CompressionKind::Zstd => {
            let decompressor = ZstdDecompressor;
            decompressor.decompress(&mut file, None, policy)
        }
    }
}

/// Convert BlendRead to a reader that implements Read + Seek
pub fn create_reader(blend_read: BlendRead) -> Result<Box<dyn crate::ReadSeekSend>> {
    match blend_read {
        BlendRead::Memory(data) => Ok(Box::new(MemoryCursor::new(data))),
        #[cfg(feature = "mmap")]
        BlendRead::TempMmap(mmap, path) => Ok(Box::new(MmapTempFile::new(mmap, path))),
        BlendRead::TempFile(file, path) => Ok(Box::new(OwnedTempFile::new(file, path))),
        BlendRead::File(file) => Ok(Box::new(std::io::BufReader::new(file))),
    }
}

/// A wrapper for memory-mapped temp files that handles cleanup
#[cfg(feature = "mmap")]
pub struct MmapTempFile {
    cursor: MmapCursor,
    temp_path: PathBuf,
}

#[cfg(feature = "mmap")]
impl MmapTempFile {
    pub fn new(mmap: Arc<memmap2::Mmap>, temp_path: PathBuf) -> Self {
        Self {
            cursor: MmapCursor::new(mmap),
            temp_path,
        }
    }
}

#[cfg(feature = "mmap")]
impl Read for MmapTempFile {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.cursor.read(buf)
    }
}

#[cfg(feature = "mmap")]
impl Seek for MmapTempFile {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        self.cursor.seek(pos)
    }
}

#[cfg(feature = "mmap")]
impl Drop for MmapTempFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.temp_path);
    }
}

/// A wrapper that owns a temp file and cleans it up on drop
pub struct OwnedTempFile {
    reader: std::io::BufReader<File>,
    temp_path: PathBuf,
}

impl OwnedTempFile {
    pub fn new(file: File, temp_path: PathBuf) -> Self {
        Self {
            reader: std::io::BufReader::new(file),
            temp_path,
        }
    }
}

impl Read for OwnedTempFile {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.reader.read(buf)
    }
}

impl Seek for OwnedTempFile {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        self.reader.seek(pos)
    }
}

impl Drop for OwnedTempFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.temp_path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_detect_compression_none() {
        let data = b"BLENDER-v300\0\0\0\0";
        let mut cursor = Cursor::new(data);
        let compression = detect_compression(&mut cursor).unwrap();
        assert_eq!(compression, CompressionKind::None);
    }

    #[test]
    fn test_detect_compression_zstd() {
        let data = [0x28, 0xB5, 0x2F, 0xFD, 0x01, 0x02, 0x03, 0x04];
        let mut cursor = Cursor::new(data);
        let compression = detect_compression(&mut cursor).unwrap();
        assert_eq!(compression, CompressionKind::Zstd);
    }

    #[test]
    fn test_decompression_policy_defaults() {
        let policy = DecompressionPolicy::default();
        assert_eq!(policy.max_in_memory_bytes, 256 * 1024 * 1024);
        assert!(!policy.allow_streaming);
        assert_eq!(policy.temp_dir, None);
    }

    #[test]
    fn test_parse_options_defaults() {
        let options = ParseOptions::default();
        assert_eq!(
            options.decompression_policy.max_in_memory_bytes,
            256 * 1024 * 1024
        );
    }

    #[test]
    fn test_memory_cursor() {
        let data = Arc::new(vec![1, 2, 3, 4, 5]);
        let mut cursor = MemoryCursor::new(data);

        let mut buf = [0u8; 3];
        let n = cursor.read(&mut buf).unwrap();
        assert_eq!(n, 3);
        assert_eq!(&buf, &[1, 2, 3]);

        let pos = cursor.seek(SeekFrom::Start(1)).unwrap();
        assert_eq!(pos, 1);

        let n = cursor.read(&mut buf).unwrap();
        assert_eq!(n, 3);
        assert_eq!(&buf, &[2, 3, 4]);
    }
}
