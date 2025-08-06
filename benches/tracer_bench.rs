use criterion::{black_box, criterion_group, criterion_main, Criterion};
use dot001_parser::BlendFile;
use dot001_tracer::DependencyTracer;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

fn find_block_by_name(blend_file: &mut BlendFile<Box<dyn dot001_parser::ReadSeekSend>>, _name: &str) -> Option<usize> {
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

    let file = File::open(test_file).expect("Failed to open test file");
    let reader = BufReader::new(file);
    let boxed_reader: Box<dyn dot001_parser::ReadSeekSend> = Box::new(reader);
    let mut blend_file = BlendFile::new(boxed_reader).expect("Failed to parse blend file");
    
    // Try to find Collection block or use first block
    let target_block = find_block_by_name(&mut blend_file, "Collection")
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
            let file = File::open(test_file).expect("Failed to open test file");
            let reader = BufReader::new(file);
            let boxed_reader: Box<dyn dot001_parser::ReadSeekSend> = Box::new(reader);
            let mut blend_file = BlendFile::new(boxed_reader).expect("Failed to parse blend file");
            
            let mut tracer = DependencyTracer::new().with_default_expanders();
            let deps = black_box(tracer.trace_dependencies(target_block, &mut blend_file))
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

    let file = File::open(test_file).expect("Failed to open test file");
    let reader = BufReader::new(file);
    let boxed_reader: Box<dyn dot001_parser::ReadSeekSend> = Box::new(reader);
    let mut blend_file = BlendFile::new(boxed_reader).expect("Failed to parse blend file");
    
    let target_block = find_block_by_name(&mut blend_file, "Collection")
        .unwrap_or(0);

    c.bench_function("trace_dependencies_tree", |b| {
        b.iter(|| {
            let file = File::open(test_file).expect("Failed to open test file");
            let reader = BufReader::new(file);
            let boxed_reader: Box<dyn dot001_parser::ReadSeekSend> = Box::new(reader);
            let mut blend_file = BlendFile::new(boxed_reader).expect("Failed to parse blend file");
            
            let mut tracer = DependencyTracer::new().with_default_expanders();
            let tree = black_box(tracer.trace_dependency_tree(target_block, &mut blend_file))
                .expect("Failed to trace dependency tree");
            black_box(tree);
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
            let file = File::open(test_file).expect("Failed to open test file");
            let reader = BufReader::new(file);
            let boxed_reader: Box<dyn dot001_parser::ReadSeekSend> = Box::new(reader);
            let mut blend_file = BlendFile::new(boxed_reader).expect("Failed to parse blend file");
            
            let mut tracer = black_box(DependencyTracer::new());
            if blend_file.blocks_len() > 0 {
                let _deps = tracer.trace_dependencies(0, &mut blend_file);
            }
        })
    });

    c.bench_function("tracer_with_default_expanders_and_trace", |b| {
        b.iter(|| {
            let file = File::open(test_file).expect("Failed to open test file");
            let reader = BufReader::new(file);
            let boxed_reader: Box<dyn dot001_parser::ReadSeekSend> = Box::new(reader);
            let mut blend_file = BlendFile::new(boxed_reader).expect("Failed to parse blend file");
            
            let mut tracer = black_box(DependencyTracer::new().with_default_expanders());
            if blend_file.blocks_len() > 0 {
                let _deps = tracer.trace_dependencies(0, &mut blend_file);
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
        let test_path = format!("../tests/test-blendfiles/{}", file_name);
        if !Path::new(&test_path).exists() {
            continue;
        }

        group.bench_function(&format!("trace_{}", file_name.replace(".blend", "")), |b| {
            b.iter(|| {
                let file = File::open(&test_path).expect("Failed to open test file");
                let reader = BufReader::new(file);
                let boxed_reader: Box<dyn dot001_parser::ReadSeekSend> = Box::new(reader);
                let mut blend_file = BlendFile::new(boxed_reader).expect("Failed to parse blend file");
                
                let target_block = find_block_by_name(&mut blend_file, target_name)
                    .unwrap_or(0);
                
                let mut tracer = DependencyTracer::new().with_default_expanders();
                let deps = black_box(tracer.trace_dependencies(target_block, &mut blend_file))
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

    let file = File::open(test_file).expect("Failed to open test file");
    let reader = BufReader::new(file);
    let boxed_reader: Box<dyn dot001_parser::ReadSeekSend> = Box::new(reader);
    let mut blend_file = BlendFile::new(boxed_reader).expect("Failed to parse blend file");
    let target_block = find_block_by_name(&mut blend_file, "Collection").unwrap_or(0);

    let mut group = c.benchmark_group("tracer_configurations");

    group.bench_function("no_expanders", |b| {
        b.iter(|| {
            let file = File::open(test_file).expect("Failed to open test file");
            let reader = BufReader::new(file);
            let boxed_reader: Box<dyn dot001_parser::ReadSeekSend> = Box::new(reader);
            let mut blend_file = BlendFile::new(boxed_reader).expect("Failed to parse blend file");
            
            let mut tracer = DependencyTracer::new(); // No expanders
            let deps = black_box(tracer.trace_dependencies(target_block, &mut blend_file))
                .expect("Failed to trace dependencies");
            black_box(deps);
        })
    });

    group.bench_function("with_default_expanders", |b| {
        b.iter(|| {
            let file = File::open(test_file).expect("Failed to open test file");
            let reader = BufReader::new(file);
            let boxed_reader: Box<dyn dot001_parser::ReadSeekSend> = Box::new(reader);
            let mut blend_file = BlendFile::new(boxed_reader).expect("Failed to parse blend file");
            
            let mut tracer = DependencyTracer::new().with_default_expanders();
            let deps = black_box(tracer.trace_dependencies(target_block, &mut blend_file))
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