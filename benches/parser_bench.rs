use criterion::{black_box, criterion_group, criterion_main, Criterion};
use dot001_parser::BlendFile;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

fn benchmark_parser_load_blend_file(c: &mut Criterion) {
    let test_files = [
        "../tests/test-blendfiles/library.blend",
        "../tests/test-blendfiles/main.blend",
        "../tests/test-blendfiles/compressed.blend",
    ];

    for test_file in &test_files {
        if !Path::new(test_file).exists() {
            continue;
        }

        let file_name = Path::new(test_file)
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string();

        c.bench_function(&format!("parse_blend_file_{file_name}"), |b| {
            b.iter(|| {
                let file = File::open(test_file).expect("Failed to open test file");
                let reader = BufReader::new(file);
                let boxed_reader: Box<dyn dot001_parser::ReadSeekSend> = Box::new(reader);
                let _blend_file =
                    black_box(BlendFile::new(boxed_reader)).expect("Failed to parse blend file");
            })
        });
    }
}

fn benchmark_parser_get_block_operations(c: &mut Criterion) {
    let test_file = "../tests/test-blendfiles/library.blend";
    if !Path::new(test_file).exists() {
        return;
    }

    let file = File::open(test_file).expect("Failed to open test file");
    let reader = BufReader::new(file);
    let boxed_reader: Box<dyn dot001_parser::ReadSeekSend> = Box::new(reader);
    let blend_file = BlendFile::new(boxed_reader).expect("Failed to parse blend file");
    let total_blocks = blend_file.blocks_len();

    c.bench_function("get_block_sequential", |b| {
        b.iter(|| {
            for i in 0..total_blocks.min(10) {
                let _block = black_box(blend_file.get_block(i));
            }
        })
    });

    if total_blocks > 0 {
        c.bench_function("get_block_random_access", |b| {
            b.iter(|| {
                let block_index = black_box(total_blocks / 2);
                let _block = black_box(blend_file.get_block(block_index));
            })
        });
    }
}

fn benchmark_parser_block_count_and_info(c: &mut Criterion) {
    let test_file = "../tests/test-blendfiles/library.blend";
    if !Path::new(test_file).exists() {
        return;
    }

    let file = File::open(test_file).expect("Failed to open test file");
    let reader = BufReader::new(file);
    let boxed_reader: Box<dyn dot001_parser::ReadSeekSend> = Box::new(reader);
    let blend_file = BlendFile::new(boxed_reader).expect("Failed to parse blend file");

    c.bench_function("blocks_len", |b| {
        b.iter(|| {
            let _len = black_box(blend_file.blocks_len());
        })
    });

    c.bench_function("header_info", |b| {
        b.iter(|| {
            let _header = black_box(blend_file.header());
        })
    });
}

fn benchmark_parser_with_different_readers(c: &mut Criterion) {
    let test_file = "../tests/test-blendfiles/library.blend";
    if !Path::new(test_file).exists() {
        return;
    }

    let mut group = c.benchmark_group("reader_types");

    // BufReader
    group.bench_function("bufreader", |b| {
        b.iter(|| {
            let file = File::open(test_file).expect("Failed to open test file");
            let reader = BufReader::new(file);
            let boxed_reader: Box<dyn dot001_parser::ReadSeekSend> = Box::new(reader);
            let _blend_file =
                black_box(BlendFile::new(boxed_reader)).expect("Failed to parse blend file");
        })
    });

    // Direct file
    group.bench_function("direct_file", |b| {
        b.iter(|| {
            let file = File::open(test_file).expect("Failed to open test file");
            let boxed_reader: Box<dyn dot001_parser::ReadSeekSend> = Box::new(file);
            let _blend_file =
                black_box(BlendFile::new(boxed_reader)).expect("Failed to parse blend file");
        })
    });

    group.finish();
}

criterion_group!(
    parser_benches,
    benchmark_parser_load_blend_file,
    benchmark_parser_get_block_operations,
    benchmark_parser_block_count_and_info,
    benchmark_parser_with_different_readers
);
criterion_main!(parser_benches);
