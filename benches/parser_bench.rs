use criterion::{black_box, criterion_group, criterion_main, Criterion};
use dot001_parser::from_path;
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
                let _blend_file =
                    black_box(from_path(test_file)).expect("Failed to parse blend file");
            })
        });
    }
}

fn benchmark_parser_get_block_operations(c: &mut Criterion) {
    let test_file = "../tests/test-blendfiles/library.blend";
    if !Path::new(test_file).exists() {
        return;
    }

    let blend_file = from_path(test_file).expect("Failed to parse blend file");
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

    let blend_file = from_path(test_file).expect("Failed to parse blend file");

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

fn benchmark_parser_from_path(c: &mut Criterion) {
    let test_file = "../tests/test-blendfiles/library.blend";
    if !Path::new(test_file).exists() {
        return;
    }

    c.bench_function("from_path", |b| {
        b.iter(|| {
            let _blend_file = black_box(from_path(test_file)).expect("Failed to parse blend file");
        })
    });
}

criterion_group!(
    parser_benches,
    benchmark_parser_load_blend_file,
    benchmark_parser_get_block_operations,
    benchmark_parser_block_count_and_info,
    benchmark_parser_from_path
);
criterion_main!(parser_benches);
