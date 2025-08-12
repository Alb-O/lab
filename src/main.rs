mod reader;

use std::env;
use std::fs::File;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;

fn read_all(path: &Path) -> io::Result<Vec<u8>> {
    let mut f = File::open(path)?;
    let mut buf = Vec::new();
    f.read_to_end(&mut buf)?;
    Ok(buf)
}

fn collect_inputs(args: &[String]) -> (Option<usize>, Vec<PathBuf>) {
    let mut jobs: Option<usize> = None;
    let mut paths: Vec<PathBuf> = Vec::new();

    let mut i = 0;
    while i < args.len() {
        let a = &args[i];
        if a == "-j" || a == "--jobs" {
            if i + 1 >= args.len() {
                eprintln!("--jobs requires a number");
                std::process::exit(2);
            }
            let n = args[i + 1]
                .parse::<usize>()
                .unwrap_or_else(|_| {
                    eprintln!("invalid --jobs value: {}", args[i + 1]);
                    std::process::exit(2);
                });
            jobs = Some(n.max(1));
            i += 2;
            continue;
        }
        paths.push(PathBuf::from(a));
        i += 1;
    }

    (jobs, paths)
}

fn discover_blend_files(inputs: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for p in inputs {
        if p.is_dir() {
            for entry in walkdir::WalkDir::new(&p).into_iter().filter_map(Result::ok) {
                let path = entry.path();
                if path.is_file() {
                    if let Some(ext) = path.extension() {
                        if ext.eq_ignore_ascii_case("blend") {
                            files.push(path.to_path_buf());
                        }
                    }
                }
            }
        } else if p.is_file() {
            files.push(p);
        } else {
            eprintln!("warning: path not found: {}", p.display());
        }
    }
    files
}

fn main() -> io::Result<()> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.is_empty() {
        eprintln!("Usage: blendreader [-j N] <file_or_dir> [...]");
        std::process::exit(2);
    }

    let (jobs, inputs) = collect_inputs(&args);
    let files = discover_blend_files(inputs);
    if files.is_empty() {
        eprintln!("No .blend files found");
        std::process::exit(1);
    }

    let process = |paths: Vec<PathBuf>| {
        use rayon::prelude::*;
        let results: Vec<(String, String)> = paths
            .into_par_iter()
            .map(|path_buf| {
                let path_str = path_buf.to_string_lossy().into_owned();
                let mut out = Vec::new();
                match read_all(&path_buf) {
                    Ok(data) => {
                        writeln!(&mut out, "-- {} --", path_buf.display()).ok();
                        let arc: Arc<[u8]> = data.into_boxed_slice().into();
                        match reader::BlendFile::from_bytes_auto_decompress(arc) {
                            Ok(mut bf) => {
                                let hdr = &bf.header;
                                writeln!(
                                    &mut out,
                                    "Header: ptr_size={} endian={} file_version={} format_version={}",
                                    hdr.pointer_size, hdr.endian as u8, hdr.file_version, hdr.file_format_version
                                )
                                .ok();
                                let mut saw_dna = false;
                                while let Some(bh) = bf.next_block() {
                                    if bh.code == reader::codes::BLO_CODE_DNA1 {
                                        writeln!(
                                            &mut out,
                                            "Found DNA1: len={} SDNAnr={} nr={}",
                                            bh.len, bh.sdn_anr, bh.nr
                                        )
                                        .ok();
                                        match bf.read_dna_block(&bh) {
                                            Ok(info) => {
                                                writeln!(
                                                    &mut out,
                                                    "SDNA: names={} types={} structs={}",
                                                    info.names_len, info.types_len, info.structs_len
                                                )
                                                .ok();
                                            }
                                            Err(e) => {
                                                writeln!(&mut out, "SDNA decode error: {}", e).ok();
                                            }
                                        }
                                        saw_dna = true;
                                        break;
                                    }
                                }
                                if !saw_dna {
                                    writeln!(&mut out, "No DNA1 block found (corrupt or very old file)").ok();
                                }
                            }
                            Err(e) => {
                                writeln!(&mut out, "Error: {}", e).ok();
                            }
                        }
                    }
                    Err(e) => {
                        writeln!(&mut out, "-- {} --", path_buf.display()).ok();
                        writeln!(&mut out, "Error reading file: {}", e).ok();
                    }
                }
                let s = String::from_utf8(out).unwrap_or_else(|_| String::from("<non-utf8 output>"));
                (path_str, s)
            })
            .collect();
        results
    };

    let mut results: Vec<(String, String)> = if let Some(n) = jobs {
        let pool = rayon::ThreadPoolBuilder::new().num_threads(n).build().unwrap();
        pool.install(|| process(files))
    } else {
        process(files)
    };

    results.sort_by(|a, b| a.0.cmp(&b.0));
    for (_path, s) in results.into_iter() {
        print!("{}", s);
    }

    Ok(())
}
