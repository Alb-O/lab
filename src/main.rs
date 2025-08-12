mod reader;

use std::env;
use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

fn read_all(path: &Path) -> io::Result<Vec<u8>> {
    let mut f = File::open(path)?;
    let mut buf = Vec::new();
    f.read_to_end(&mut buf)?;
    Ok(buf)
}

fn main() -> io::Result<()> {
    let mut args = env::args().skip(1).collect::<Vec<_>>();
    if args.is_empty() {
        eprintln!("Usage: blendreader <file.blend> [more.blend ...]");
        std::process::exit(2);
    }

    for path_str in args.drain(..) {
        let path = Path::new(&path_str);
        let data = read_all(path)?;
        // Debug: dump first 32 bytes to inspect header
        let dump_len = data.len().min(32);
        let mut hexline = String::new();
        for (idx, b) in data[..dump_len].iter().enumerate() {
            if idx == 0 {
                hexline.push_str("hdr[0..32]: ");
            }
            hexline.push_str(&format!("{:02X} ", b));
        }
        if !hexline.is_empty() { println!("{}", hexline); }
        println!("-- {} --", path.display());
        match reader::BlendFile::from_bytes_auto_decompress(&data) {
            Ok(mut bf) => {
                let hdr = &bf.header;
                println!(
                    "Header: ptr_size={} endian={} file_version={} format_version={}",
                    hdr.pointer_size, hdr.endian as u8, hdr.file_version, hdr.file_format_version
                );
                // Iterate blocks to find DNA1 and print summary.
                let mut saw_dna = false;
                while let Some(bh) = bf.next_block() {
                    if bh.code == reader::codes::BLO_CODE_DNA1 {
                        println!("Found DNA1: len={} SDNAnr={} nr={}", bh.len, bh.sdn_anr, bh.nr);
                        let info = bf.read_dna_block(&bh).unwrap_or_else(|e| {
                            panic!("Failed to decode SDNA: {}", e);
                        });
                        println!(
                            "SDNA: names={} types={} structs={}",
                            info.names_len, info.types_len, info.structs_len
                        );
                        saw_dna = true;
                        break;
                    }
                }
                if !saw_dna {
                    println!("No DNA1 block found (corrupt or very old file)");
                }
            }
            Err(e) => {
                println!("Error: {}", e);
            }
        }
    }
    Ok(())
}
