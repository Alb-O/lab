use criterion::{black_box, criterion_group, criterion_main, Criterion};
use dot001_parser::from_path;
use dot001_tracer::{ParallelDependencyTracer, TracerOptions};
use std::path::Path;

fn find_block_by_name(blend_file: &dot001_parser::BlendFile, _name: &str) -> Option<usize> {
    // Try to find a block with the given name pattern
    // For now, just return a reasonable block index for testing
    if blend_file.blocks_len() > 5 {
        Some(5) // Assume block 5 is a good test target
    } else if blend_file.blocks_len() > 1 {
        Some(1) // Skip header block
    } else {
        None
    }
}

fn benchmark_trace_dependencies_flat(c: &mut Criterion) {
    let test_file = "../tests/test-blendfiles/library.blend";
    if !Path::new(test_file).exists() {
        return;
    }

    let blend_file = from_path(test_file).expect("Failed to parse blend file");

    // Try to find Collection block or use first block
    let target_block = find_block_by_name(&blend_file, "Collection")
        .or_else(|| {
            // Fallback to any non-header block
            for i in 0..blend_file.blocks_len() {
                if let Some(block) = blend_file.get_block(i) {
                    let code_str = dot001_parser::block_code_to_string(block.header.code);
                    if code_str != "ENDB" && code_str != "REND" {
                        return Some(i);
                    }
                }
            }
            None
        })
        .unwrap_or(0);

    c.bench_function("trace_dependencies_flat", |b| {
        b.iter(|| {
            let blend_file = from_path(test_file).expect("Failed to parse test file");

            let mut tracer = ParallelDependencyTracer::new().with_default_expanders();
            let deps = black_box(tracer.trace_dependencies_parallel(target_block, &blend_file))
                .expect("Failed to trace dependencies");
            black_box(deps);
        })
    });
}

fn benchmark_trace_dependencies_tree(c: &mut Criterion) {
    let test_file = "../tests/test-blendfiles/library.blend";
    if !Path::new(test_file).exists() {
        return;
    }

    let blend_file = from_path(test_file).expect("Failed to parse blend file");

    let target_block = find_block_by_name(&blend_file, "Collection").unwrap_or(0);

    c.bench_function("trace_dependencies_tree", |b| {
        b.iter(|| {
            let blend_file = from_path(test_file).expect("Failed to parse test file");

            let mut tracer = ParallelDependencyTracer::new().with_default_expanders();
            let deps = black_box(tracer.trace_dependencies_parallel(target_block, &blend_file))
                .expect("Failed to trace dependencies");
            black_box(deps);
        })
    });
}

fn benchmark_tracer_initialization(c: &mut Criterion) {
    let test_file = "../tests/test-blendfiles/library.blend";
    if !Path::new(test_file).exists() {
        return;
    }

    c.bench_function("tracer_new_and_trace", |b| {
        b.iter(|| {
            let blend_file = from_path(test_file).expect("Failed to parse test file");

            let mut tracer = black_box(ParallelDependencyTracer::new());
            if blend_file.blocks_len() > 0 {
                let _deps = tracer.trace_dependencies_parallel(0, &blend_file);
            }
        })
    });

    c.bench_function("tracer_with_default_expanders_and_trace", |b| {
        b.iter(|| {
            let blend_file = from_path(test_file).expect("Failed to parse test file");

            let mut tracer = black_box(ParallelDependencyTracer::new().with_default_expanders());
            if blend_file.blocks_len() > 0 {
                let _deps = tracer.trace_dependencies_parallel(0, &blend_file);
            }
        })
    });
}

fn benchmark_different_blend_files(c: &mut Criterion) {
    let test_files = [
        ("library.blend", "Collection"),
        ("main.blend", "Scene"),
        ("compressed.blend", "Collection"),
    ];

    let mut group = c.benchmark_group("trace_by_file");

    for (file_name, target_name) in &test_files {
        let test_path = format!("../tests/test-blendfiles/{file_name}");
        if !Path::new(&test_path).exists() {
            continue;
        }

        group.bench_function(format!("trace_{}", file_name.replace(".blend", "")), |b| {
            b.iter(|| {
                let blend_file = from_path(&test_path).expect("Failed to parse test file");

                let target_block = find_block_by_name(&blend_file, target_name).unwrap_or(0);

                let mut tracer = ParallelDependencyTracer::new().with_default_expanders();
                let deps = black_box(tracer.trace_dependencies_parallel(target_block, &blend_file))
                    .expect("Failed to trace dependencies");
                black_box(deps);
            })
        });
    }

    group.finish();
}

fn benchmark_tracer_with_different_configurations(c: &mut Criterion) {
    let test_file = "../tests/test-blendfiles/library.blend";
    if !Path::new(test_file).exists() {
        return;
    }

    let blend_file = from_path(test_file).expect("Failed to parse blend file");
    let target_block = find_block_by_name(&blend_file, "Collection").unwrap_or(0);

    let mut group = c.benchmark_group("tracer_configurations");

    group.bench_function("no_expanders", |b| {
        b.iter(|| {
            let blend_file = from_path(test_file).expect("Failed to parse test file");

            let mut tracer = ParallelDependencyTracer::new().with_options(TracerOptions {
                max_depth: usize::MAX,
            }); // default options
            let deps = black_box(tracer.trace_dependencies_parallel(target_block, &blend_file))
                .expect("Failed to trace dependencies");
            black_box(deps);
        })
    });

    group.bench_function("with_default_expanders", |b| {
        b.iter(|| {
            let blend_file = from_path(test_file).expect("Failed to parse test file");

            let mut tracer = ParallelDependencyTracer::new().with_default_expanders();
            let deps = black_box(tracer.trace_dependencies_parallel(target_block, &blend_file))
                .expect("Failed to trace dependencies");
            black_box(deps);
        })
    });

    group.finish();
}

criterion_group!(
    tracer_benches,
    benchmark_trace_dependencies_flat,
    benchmark_trace_dependencies_tree,
    benchmark_tracer_initialization,
    benchmark_different_blend_files,
    benchmark_tracer_with_different_configurations
);
criterion_main!(tracer_benches);
