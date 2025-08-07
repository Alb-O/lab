Current state summary grounded in code
What’s implemented
- Parser crate:
  - Synchronous, cursor-based parsing pipeline in [`crates/dot001_parser/src/lib.rs`](crates/dot001_parser/src/lib.rs:1): header -> block headers enumeration -> DNA -> indices.
  - Input handling supports raw files and zstd with multi-backend decompression in [`compression.DecompressionPolicy`](crates/dot001_parser/src/compression.rs:1) and [`compression.open_source()`](crates/dot001_parser/src/compression.rs:321); backends include in-memory Vec, temp file, optional temp mmap (feature-guarded).
  - Reader abstraction converted to Read+Seek via [`compression.create_reader()`](crates/dot001_parser/src/compression.rs:341) with wrappers [`MemoryCursor`](crates/dot001_parser/src/compression.rs:70) and optional [`MmapTempFile`](crates/dot001_parser/src/compression.rs:352).
  - Block scanning is sequential reading of headers and seeking past bodies; block metadata is stored in [`BlendFileBlock`](crates/dot001_parser/src/block.rs:14) with both header_offset and data_offset captured.
  - DNA parsing implemented in a robust and bounds-checked way in [`dna.rs`](crates/dot001_parser/src/dna.rs:1), with internal indices for names/types and a struct name index; DnaStruct tracks fields and precomputes offsets/array sizes.
  - Bound checks and safety limits: e.g., names/types/structs count caps in [`dna.read_*`](crates/dot001_parser/src/dna.rs:171) and per-block size cap via [`DEFAULT_MAX_BLOCK_SIZE`](crates/dot001_parser/src/lib.rs:66).
  - Parser events emitted via dot001_events during header, blocks, DNA, and finalization paths in [`BlendFile::new()`](crates/dot001_parser/src/lib.rs:106) and [`parse_from_*`](crates/dot001_parser/src/lib.rs:371).
  - Convenience APIs: [`from_path()`](crates/dot001_parser/src/lib.rs:340), [`parse_from_path()`](crates/dot001_parser/src/lib.rs:371), [`parse_from_reader()`](crates/dot001_parser/src/lib.rs:415), and a content hasher [`block_content_hash()`](crates/dot001_parser/src/lib.rs:251).
- Tracer crate:
  - Deterministic, breadth-first dependency traversal with filters in [`core/tracer.DependencyTracer`](crates/dot001_tracer/src/core/tracer.rs:39). Uses a queue, visited/visiting sets, optional allowed set, depth limiting via [`TracerOptions`](crates/dot001_tracer/src/core/options.rs:1).
  - Block expanders registry and default registrations in [`with_default_expanders()`](crates/dot001_tracer/src/core/tracer.rs:128) using simple_expander macros for common Blender blocks; example object expander in [`expanders/basic/object.rs`](crates/dot001_tracer/src/expanders/basic/object.rs:1).
  - Deterministic address remapping via Determinizer integration and event emissions throughout tracing lifecycle, plus a hierarchical DependencyTree builder in [`trace_dependency_tree()`](crates/dot001_tracer/src/core/tracer.rs:298).
- Events ecosystem:
  - Parser and tracer emit detailed events through dot001_events with sync emission macros already wired (async bus exists for future multi-file workflows).

Gaps and performance pain points
- IO and memory:
  - Current parse_from_path pulls uncompressed files via standard File -> BufReader and compressed via Vec or temp file/mmap, but the parsing core uses Read+Seek, which forces copy into Vec for block data in hot path [`read_block_data()`](crates/dot001_parser/src/lib.rs:273).
  - No true zero-copy views over the file: data is re-read and allocated per call; no reuse via Bytes or direct &[u8] views over a stable backing.
- Concurrency:
  - Parsing pipeline is strictly single-threaded. Decompression can be offloaded to zstd internals but parsing blocks and building indices is sequential.
  - Tracer expanders run strictly single-threaded BFS, performing expand_block calls serially.
- Binary decode:
  - DNA field reading likely performed by a higher-level fields module, but we don’t see zero-copy typed views; reads likely do per-field operations. No bytemuck/zerocopy or bytes-based slicing used.
- Caching:
  - Indices use HashMap with default hasher; address_index/block_index are built sequentially but lookup hot paths could benefit from ahash. No concurrent maps for parallel phases.
- Observability:
  - Good event coverage exists, but no sampling of timings per stage, no reduced-overhead tracing spans with fields for hot paths, no built-in flamegraph hooks. Criterion benches exist but likely not isolating micro-stages.

Overhaul plan for blazing fast concurrent, async-ready parsing and tracing
Guiding constraints: MSRV 1.76+, Linux/macOS on stable. Optimize throughput with peak memory under 1.5x file size. Allowed deps: bytes, memmap2, zerocopy/bytemuck, rayon, dashmap, ahash, crossbeam, tokio-util, tracing. Optimize on blendfiles/many_cubes_bench and shaderball.

1) Input and buffer architecture: zero-copy first, async-capable fallback
- Introduce a unified immutable buffer abstraction BlendBuf that can represent:
  - Mapped file region (memmap2 Mmap) for uncompressed or temp decompressed files.
  - Arc<Vec<u8>> for in-memory buffers.
  - Bytes for ref-counted slicing with cheap subviews.
- API shape:
  - BlendSource enum { Mmap(Arc<Mmap>), ArcBuf(Arc<Vec<u8>>), Bytes(Bytes) }.
  - Provide BlendSlice = Bytes which can be cheaply sliced to represent block data without allocation.
- Integrate into parser:
  - In parse_from_path, after create_reader/open_source, attempt to resolve to a backing BlendSource:
    - For BlendRead::File: on Unix/macOS, map file with memmap2 if not compressed and file is regular; else fall back to reading into a single Vec once.
    - For BlendRead::TempMmap: already have mmap.
    - For BlendRead::Memory: already have Arc<Vec<u8>>.
    - For BlendRead::TempFile: prefer mmap if possible; else keep BufReader but also optionally read entire file once into Vec if file size <= policy.max_in_memory_bytes.
  - Maintain a BlendBuf on BlendFile to enable direct slicing of block bytes. Keep Read+Seek path only as a fallback for streaming or very large files that can’t be mapped/read fully under memory budget.

2) Parser data model refactor for zero-copy block access
- Change BlendFile<R: Read+Seek> to BlendFile where R is no longer generic in the primary fast path. Provide two types:
  - BlendFileBuf { buf: BlendBuf, header: BlendFileHeader, blocks: Vec<BlendFileBlock>, … }
  - BlendFileStream<R: Read+Seek> kept for legacy/streaming; functional but not the fast path.
- During read_blocks, parse headers from the backing buffer with pointer arithmetic rather than Read+Seek when BlendFileBuf is used:
  - Read header once to determine header_size, pointer size, endianness.
  - Iterate block headers by reading fixed-size headers from the backing slice directly. For legacy vs v1 header sizes, use minimal copying; if alignment helps, use bytemuck::try_from_bytes for fixed fields where safe.
  - Store data_offset and header_offset as indices into the backing buffer.
- read_block_data returns Bytes:
  - Create a Bytes slice referencing [data_offset .. data_offset+size] and return it, avoiding allocation and copying.
  - Keep a fall-back path that copies for the streaming variant.
- Index maps:
  - Switch HashMap to AHashMap via ahash for block_index and address_index.
  - Optional: add perfect-hash at build time for well-known block codes, but AHash is likely sufficient.

3) DNA parsing: fast-path and compact structures
- Use a two-phase approach:
  - Phase A: fast scan of SDNA layout building raw tables into compact Vecs using Bytes slices for strings to avoid intermediate allocations. Strings can be stored as indices/offsets into a string arena or as Arc<str> if necessary; but since backing file can be dropped after parse, store owned compact copies but deduplicate via string interner or fxhash-based interner. Consider lasso or string_cache if acceptable; otherwise a custom small interner keyed by Bytes hash with ahash.
  - Phase B: precompute for each struct a small table of fields with offset and size, with frequently accessed field name mapping stored in a small, inline capacity map (e.g., index by precomputed name_id). Replace HashMap<String, usize> per-struct with:
    - fields_by_name_id: Vec<Option<usize>> sized to names.len() only if acceptable memory wise, or
    - A small AHashMap<NameId, usize> with reserve_exact(field_count) and shrink_to_fit.
- Provide APIs to get field by NameId without string comparisons in hot loops.

4) Concurrency: block header enumeration, compression, and tracing
- Parsing:
  - Header and DNA must be sequential. Block header scanning is sequential due to file layout; keep it single-threaded but micro-optimized (direct slice parsing).
  - Data decoding: reading block bodies is zero-copy; but certain derived indices or per-block validations can be parallelized:
    - Parallel build of block_index and address_index: after blocks Vec is collected, use rayon::into_par_iter to partition and build local maps, then merge; or compute in single pass and then parallelize any subsequent precomputations (e.g., hashing or schema-specific preprocess).
  - Compression:
    - For zstd -> temp file approach, keep current path. Optionally adopt zstdmt or rayon-enabled decompression if the zstd crate supports multithreaded decoding on large files; configure threads based on available CPUs with bounds.
- Tracing:
  - Replace single-thread BFS with a concurrent work-stealing traversal using rayon’s thread pool for CPU-bound expansion:
    - Maintain a concurrent visited set (DashMap<usize, ()> or bitset if block count known and reasonable).
    - Use crossbeam deque or rayon par_bridge style to process frontier in parallel batches while respecting max_depth.
    - Determinism: accumulate results per depth as small vectors, sort by block index per layer before merging to ensure stable output. Determinizer for addresses remains as-is.
  - For expanders that only compute index lookups and follow pointers, ensure they only read immutable data and return dependency indices without mutation; this makes them thread-safe without locks.
  - Provide two modes:
    - Tracer::trace_dependencies_parallel for throughput.
    - Tracer::trace_dependencies for compatibility and deterministic reproduction with identical ordering.

5) Binary decoding of fields: zero-copy and typed views
- Introduce a FieldView API:
  - Given a Bytes block slice, and a DnaStruct descriptor, provide unchecked-get methods that rely on prevalidated offsets/sizes and file endianness to read scalars, arrays, and pointer arrays using bytemuck casts where alignment allows, or byteorder reads otherwise.
  - For pointer fields, represent as u32/u64 depending on header.pointer_size; avoid widening until needed.
  - For strings and arrays of bytes, return subslices as Bytes::slice for zero-copy.
- For hot structures we routinely inspect (Object, Mesh, Material), add thin typed accessors based on zerocopy derive where layout is stable for given SDNA; guard with DNA sdna_index match and fall back to generic reader if mismatch. This is an opt-in fast path.

6) Public async interfaces and feature flags
- Crate features:
  - default = ["zstd"] optional = ["mmap", "rayon", "tracing", "simd"]
- Async APIs:
  - async fn parse_from_path_async(path) -> Result<(BlendFileBuf, DecompressionMode)>
    - Uses tokio::fs for file open and metadata, but ultimately maps or reads to a single buffer then drops async; bounded memory policy enforced.
  - Tracer remains CPU-bound; offer async wrappers that spawn blocking in a tokio task if consumers are async.
- Maintain existing sync APIs for CLI and benches.

7) Observability, metrics, benchmarking
- Add tracing spans:
  - parser:header, parser:blocks_scan, parser:dna, parser:index_build, parser:hash
  - tracer:queue_layer, tracer:expand_block, tracer:merge_layer
  - Include fields: counts, durations, bytes, block codes.
- Criterion micro-benchmarks:
  - Bench header parse, block header scan over large files, DNA parse, block indexing, expanders for OB/ME/MA on representative blocks.
  - Scenario benches using many_cubes_bench and shaderball. Record wall-clock and throughput MB/s.
- Flamegraph and dhat/rust-alloc toggles in benches to validate memory ceilings.

8) Memory budget strategy to stay under 1.5x file size
- Prefer mmap for uncompressed and temp decompressed data; this avoids double buffers.
- If in-memory decompression is used, ensure only one owned buffer is retained; all block access uses Bytes slices.
- Avoid per-call allocations: expanders should reuse scratch structures or return small Vecs with pre-allocated capacities; consider smallvec for small dependency lists.

Concrete design changes per crate
dot001_parser
- New modules:
  - buf.rs: BlendBuf, BlendSource, traits for zero-copy slicing, conversion from BlendRead.
  - scan.rs: slice-based block header scanning with minimal copies; returns Vec<BlendFileBlock>.
  - index.rs: index builders with ahash, optional rayon parallel build.
  - fieldview.rs: zero-copy FieldView reading utils leveraging bytemuck when possible.
- BlendFile types:
  - BlendFileBuf: primary type with buf: BlendBuf. Methods: header(), blocks(), read_block_slice(index) -> Bytes, dna(), blocks_by_type_iter(), find_block_by_address(), create_field_view(slice).
  - BlendFileStream<R>: legacy, minimal changes.
- Compression integration:
  - Prefer mapping for BlendRead::File and TempFile if feature mmap enabled; otherwise, read into Vec once under policy thresholds.
  - Keep current decompressor; consider zstdmt config if acceptable.
- Hashing:
  - block_content_hash to accept a Bytes slice version to avoid copying.

dot001_tracer
- Core API additions:
  - trace_dependencies_parallel(start, &BlendFileBuf, options) -> Vec<usize>
  - trace_dependency_tree_parallel(...)
- Internals:
  - Use DashMap or lock-free bitset for visited; store allowed set as a fixed BitSet when derived from FilterEngine to allow O(1) checks without hashing.
  - Process BFS by depth layers:
    - For each depth layer, par_iter over frontier, expand dependencies independently into thread-local Vecs.
    - Merge per-thread results, dedup with a temporary bitset against visited, sort for determinism, then proceed to next layer.
- Expanders:
  - Ensure expand_block never mutates shared state; return ExpandResult built from immutable FieldView access.
  - Provide fast-path expanders for common block codes that leverage typed field reads when sdna matches expected.

dot001_events
- Keep as-is for now. Add tracing feature integration so spans emit to the same backend; provide lightweight sampling to avoid overhead in tight loops.

Compatibility, features, and migration
- Maintain current public APIs; add Buf variants alongside. Mark legacy R: Read+Seek path as slower but supported.
- Feature defaults:
  - default features: ["zstd"].
  - Recommend enabling ["mmap", "rayon", "tracing"] in CLI and benches on Linux/macOS CI.
- CI:
  - Add benches that run with and without mmap/rayon to ensure portability.

Measurable targets
- Single large file parsing throughput:
  - many_cubes_bench: 2-4x speedup vs current on same machine by eliminating copies and switching to zero-copy block access.
  - shaderball: 1.5-3x depending on DNA complexity.
- Memory ceiling:
  - For uncompressed/mmap path: peak ~0.6x file size (indices and DNA tables).
  - For compressed with in-memory buffer: peak <= 1.3-1.5x file size including decoded buffer plus indices.
- Tracer expansion:
  - Parallel tracing: 3-6x faster on 8-core CPUs for complex graphs with many independent edges; stable deterministic output per depth-layer sort.

Phased delivery plan
Phase 1: Zero-copy foundation and API
- Implement BlendBuf and Bytes-backed block slices.
- Switch parser to buffer-backed fast path; keep streaming fallback.
- Return Bytes from read_block_data and introduce read_block_slice.
- AHash maps for indices.
- Bench improvements for parse stages.

Phase 2: DNA and FieldView optimization
- Compact DNA storage with NameId/type interning.
- FieldView with zero-copy scalar and array reads; update FieldReader usage to route via FieldView.

Phase 3: Parallel tracer
- Layered BFS with rayon, visited bitset, deterministic merge.
- Migrate default expanders to pure, immutable, zero-copy access using FieldView.

Phase 4: Observability and polish
- tracing spans in hot paths with fields; flamegraph integration; criterion microbenches.
- Feature flags, async wrappers, doc updates, and examples.

Risks and mitigations
- Safety of zero-copy typed views: only enable bytemuck/zerocopy paths when alignment and sdna layout guarantees are validated; otherwise fall back to byteorder reads.
- Determinism in parallel tracing: enforce per-depth stable merges; keep original single-thread path for full reproduction.
- mmap portability: guard with feature flags and fall back to Vec on unsupported platforms; ensure temp file cleanup correctness (already present).
- Memory spikes from interning: prefer compact integer IDs, reserve capacities, and shrink_to_fit after parse.

What remains already solid
- Header parsing robust for legacy and v1 headers in [`header.read`](crates/dot001_parser/src/header.rs:13).
- Block header decoding handling both pointer sizes and v1 layout in [`block.read`](crates/dot001_parser/src/block.rs:24).
- DNA parsing correctness and bounds checks in [`dna.rs`](crates/dot001_parser/src/dna.rs:128).
- Event emission hooks for parser and tracer.

Next actions aligned to the todo list
- Finalize the IO/buffer abstraction design and propose exact public types and signatures.
- Specify the concurrency model for tracer with a minimal deterministic algorithm sketch.
- Detail FieldView APIs with example code snippets for Object/Mesh accessors.

This plan aligns with the constraints and performance goals and provides a staged path to achieve zero-copy, parallel parsing and tracing with modern, idiomatic Rust.
