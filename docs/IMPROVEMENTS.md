Constructive codebase critique and improvement plan for dot001 crates

Scope reviewed
- Workspace layout: crates/dot001-error, dot001-parser, dot001-tracer, dot001-cli, dot001-diff, dot001-editor, dot001-checkpoint.
- Deep dive files: [crates/dot001-error/src/lib.rs](crates/dot001-error/src/lib.rs), [crates/dot001-parser/src/lib.rs](crates/dot001-parser/src/lib.rs), [crates/dot001-parser/src/block.rs](crates/dot001-parser/src/block.rs), [crates/dot001-parser/src/header.rs](crates/dot001-parser/src/header.rs), [crates/dot001-tracer/src/lib.rs](crates/dot001-tracer/src/lib.rs), [crates/dot001-tracer/src/filter.rs](crates/dot001-tracer/src/filter.rs), [crates/dot001-tracer/src/expanders/mod.rs](crates/dot001-tracer/src/expanders/mod.rs), [crates/dot001-tracer/src/expanders/image.rs](crates/dot001-tracer/src/expanders/image.rs), [crates/dot001-cli/src/main.rs](crates/dot001-cli/src/main.rs), [crates/dot001-cli/src/commands/mod.rs](crates/dot001-cli/src/commands/mod.rs), [crates/dot001-cli/src/commands/dependencies.rs](crates/dot001-cli/src/commands/dependencies.rs), [crates/dot001-cli/src/util.rs](crates/dot001-cli/src/util.rs), [crates/dot001-diff/src/lib.rs](crates/dot001-diff/src/lib.rs).

High-level assessment
1) Architecture and cohesion
- Crate boundaries are logical and clean. Parser is focused on IO/DNA, tracer on traversal/expanders, diff on comparison, CLI orchestrates, and error crate unifies errors.
- dot001-error is well-structured and centralizes error taxonomy with useful helpers and user/debug messages. Clone strategy stores source_message string for cloneability which is pragmatic.
- dot001-parser exposes a BlendFile with indices, address map, and helper for FieldReader. API is intentionally minimal and avoids holding long borrows. Good attention to endianness, legacy vs new header formats, and compression handling via options/policy.
- dot001-tracer provides a clear BlockExpander trait and registry, BFS traversal with visited/visiting, depth limiting, and filter integration. The tree-building function is careful to copy header info before mutable borrow usage.
- dot001-diff is marked experimental with honest limitations and a staged design for content-aware types.
- CLI structure is modularized with commands; util encapsulates parse options and file loading including no-auto-decompress mode.

2) Code quality
- Borrowing discipline is strong: code consistently avoids holding immutable borrows across calls that require &mut BlendFile by copying header fields early. This is visible in tracer and diff code paths.
- IO safety and error mapping are consistently applied; parser returns unified errors and appropriately checks magic/version bytes.
- Naming and doc comments are clear, with module- and function-level explanations.
- Feature flags are used to compile CLI commands conditionally. However, some re-exports and generics introduce incidental complexity.

3) DRY and cross-crate cohesion
- Some duplication in address remapping, name resolution concepts, and repeated pointer traversal patterns across tracer modules and diff analyzer. The FilterEngine includes pointer_targets heuristics that overlap with expander logic.
- Both tracer and diff rely on ad hoc name resolution or omit it; name resolution lives in tracer, but diff could benefit from a shared NameResolver crate API.
- Compression handling logic is duplicated between parse_from_path/reader and CLI util load_blend_file with slight divergence of types (tracer vs parser BlendFile type usage).
- Error helper methods exist in multiple crates (e.g., tracer::DependencyTracer helpers and diff::BlendDiffer helpers) that construct Dot001Error variants; this is fine, but some could be centralized in dot001-error as smart constructors or extension traits to keep semantics uniform.

4) API ergonomics
- dot001-parser.BlendFile has public fields. While convenient internally, for external consumers a more opaque API with accessors could prevent misuse and enable invariants. Blocks Vec is accessed directly in callers across crates.
- BlockExpander trait returns Result<ExpandResult> which is good; however ExpandResult seems trivial (dependencies vector). Consider returning Vec<usize> or an enum that can evolve to record typed edges or external assets.
- Tracer options contain only max_depth; future-proof with a builder style and additional options (e.g., include_self, detect_cycles_strategy, traversal_order).
- FilterSpec is powerful yet stringly typed; good first step. Consider a typed filter mini-DSL or schema once stabilized.

5) Testing strategy
- dot001-error has unit tests. Parser, tracer, diff area lacks visible tests in the examined files, aside from tracer/tests directory which exists, though not reviewed fully here. Consider adding:
  - Golden tests for dependency outputs on small fixture .blend files (redacted or synthetic).
  - Property tests for header parsing and block index/address map invariants.
  - Round-trip tests for filter rules to ensure include/exclude and recursion semantics hold.
  - Integration tests for CLI that assert formatted output (with determinism via address remap).

Detailed findings and suggestions

A) dot001-error
Findings:
- Comprehensive error variants with contextual fields, user_message/debug_message/summary helpers improve DX/UX.
- Dot001Error::with_file_path takes a path and blindly sets Some(path) across variants that have file context, but unwrap_or_default patterns elsewhere can hide missing context.

Suggestions:
- Add From<Dot001Error> conversions or newtype wrappers per crate if you want crate-specific Result<T> but still unify at the CLI boundary. Currently many crates re-export dot001_error::Result which is fine.
- Provide helper constructors for common patterns in a small extension trait per domain, living in dot001-error to avoid repetition of message strings across crates.
- Consider an ErrorContext struct for shared fields (file_path, block_index, command, operation) to reduce enum payload variability and give consistent accessor methods.

B) dot001-parser
Findings:
- BlendFile::new checks zstd via magic; parse_from_* handle decompression policy correctly.
- Public struct BlendFile exposes reader and all fields. This eases internal coordination but leaks internals to API consumers.
- read_blocks/read_dna/build_block_index do a single pass. The DNA not found error is consistent.
- block_content_hash uses twox-hash including header fields and data; good determinism.
- ReadSeekSend trait is blanket implemented for any Read+Seek+Send; helpful for trait object.

Suggestions:
- Encapsulate `BlendFile` fields. Expose functionality through public methods to strengthen invariants. For example, provide an iterator over blocks of a certain type instead of direct access to the `blocks` vector.
- Offer borrow-safe methods that return lightweight structs with header snapshots to prevent callers from maintaining references into internal Vec while mutating.
- Consider zero-copy options for block data with an internal buffer pool or guarded slice if using mmap, to reduce allocations for repeated reads. Currently read_block_data allocates Vec new each time.
- Unify compression detection path: You check magic twice (in new and in parse_from_*). Document or centralize detection to one path to avoid drift. Optionally gate BlendFile::new to require already-decompressed streams and keep auto-detection only in parse_* APIs.

C) dot001-tracer
Findings:
- DependencyTracer uses BFS with visited/visiting sets and depth limit, plus optional allowed set precomputed by FilterEngine. Good protections against cycles and explosions.
- build_dependency_node duplicates filtering logic; both trace_dependencies and tree path re-check allowed sets. Acceptable for now but consider centralizing.
- Error mapping function to_tracer_error unwrap_or_default when adding file_path may insert empty PathBuf. This can hide missing context.
- FilterEngine implements pointer_targets heuristics for multiple block types (OB, ME, GR/DATA, NT/DATA), partially overlapping expander logic. This is a DRY hotspot and a maintenance risk if expander logic evolves differently.
- The `NameResolver` is a static utility struct and its methods require a mutable `BlendFile` reference, which can be ergonomically challenging.

Suggestions:
- Refactor pointer traversal into shared utilities/traits. Two approaches:
  1) Promote a SharedReflect module in dot001-parser with reflective helpers over DNA, providing generic traversal of pointer fields and arrays. Then both FilterEngine and expanders can rely on it.
  2) Or, route filter recursion through expanders instead of separate heuristics, e.g., FilterEngine requests DependencyTracer to expand “one step” for a given index. This consolidates traversal knowledge.
- Enhance BlockExpander::expand_block to optionally return metadata (e.g., external file refs) via a richer ExpandResult, unblocking ImageExpander’s TODO for file paths.
- Provide a NameResolver trait in dot001-tracer with a default implementation; allow crates to add resolvers per block type. Currently NameResolver is present but tightly coupled in tracer; define a formal interface and allow DI.
- Introduce a Determinizer that applies address remapping and stable sorting in one place for any outputs (flat lists, trees), used by CLI and diff when needed. This centralizes determinism handling.

D) dot001-cli
Findings:
- Commands are split and re-exported; main.rs is thin and uses util for parse options and file loading.
- load_blend_file has two implementations depending on feature "trace", returning different BlendFile types (tracer vs parser module). This is a type coupling that can be surprising.

Suggestions:
- Standardize on a single BlendFile type alias across crates via re-export in dot001-parser, and keep tracer consuming parser::BlendFile generically. In CLI util, always return dot001_parser::BlendFile<Box<dyn ReadSeekSend>>; let commands pass it to tracer or diff. Avoid returning tracer::BlendFile constructor, since tracer re-exports parser::BlendFile anyway.
- Extract output formatting (tree/flat/json) into a small presenter module so commands/dependencies.rs is more testable and formatting can be reused by dot001-diff and future commands.
- Normalize error returns: commands sometimes print to `eprintln!` and return `Ok(())` (e.g., `cmd_dependencies` for an out-of-range index). Prefer returning a proper error using `dot001_error` to allow for consistent CLI exit codes and centralized error handling in `main`.

E) dot001-diff
Findings:
- Clear documentation of limitations. Good split of provenance module and main engine. Mesh content-aware path is implemented; structural/DATA heuristics are conservative.
- Name resolution is TODO; currently get_block_name returns None.

Suggestions:
- Add a DiffContext that injects NameResolver and a determinizer for addresses. Allow callers to customize. This keeps BlendDiffer pure and testable.
- Factor binary_compare and extract_mesh_content into reusable utilities, consider a CompareStrategy trait per block kind similar to BlockExpander to align architecture between tracer and diff. This simplifies adding semantic diffs incrementally.
- Integrate DNA-aware field diffing: expose a helper in parser to enumerate fields of a struct instance, enabling selective comparison ignoring pointer fields. Start with OB/GR/NT IDs to reduce false positives.

Key DRY improvements to target
1) Pointer traversal consolidation
- Today: duplicated logic between FilterEngine and expanders.
- Plan: create a shared traversal abstraction:
  - A trait PointerWalker with methods to enumerate pointer fields of a block via DNA schema, with optional type hints. Implement default reflection-based walker in parser using FieldReader and DnaCollection.
  - Expanders can still add domain-specific logic, but general pointer following for arrays/listbases lives in one place.

2) Name resolution central service
- Extract NameResolver into a small module with a trait, re-export from a new crate or from tracer with a stable interface. Let diff and CLI depend on it. Provide a fallback “code#index” formatter.

3) Deterministic output utilities
- Single module to remap addresses, normalize codes, and stable-sort outputs for CLI and JSON generation. Remove per-module ad hoc mapping fields.

4) Compression/decompression policy handling
- Unify parsing entrypoints so CLI util does not need to know about tracer::BlendFile; always return parser::BlendFile. Keep decompression logic centralized in parser::parse_from_path/reader.

5) Error helpers standardization
- Move common “domain error” constructors into dot001-error as free fns or traits: tracer_error(kind, msg), diff_error(kind, msg), parser_error(kind, msg). This reduces per-crate helper duplication and ensures consistent user/debug messages.

Public API refinements
- Make BlendFile fields private; expose:
  - fn header(&self) -> &BlendFileHeader
  - fn blocks_len(&self) -> usize
  - fn block_header(&self, index) -> Option<BlockHeaderSnapshot>
  - fn read_block_data(&mut self, index) -> Result<Vec<u8>>
  - fn find_block_by_address(&self, addr) -> Option<usize>
  - fn blocks_by_type_iter(&self, code) -> impl Iterator<Item=usize>
- For tracer:
  - Provide a TracerBuilder with methods register_default_expanders(), with_options(), with_filter_spec(), with_address_map().
  - Separate one-step expansion API: fn expand_once(&mut self, index, blend) to aid FilterEngine reuse.

Testing strategy recommendations
- Parser:
  - Property tests for header read: random endian/pointer_size combos generate bytes, roundtrip parse asserts fields.
  - Unit tests for block header read with v0/v1 formats.
  - Tests for block_content_hash stability across runs.
- Tracer:
  - Unit tests on synthetic BlendFile with small fabricated blocks to exercise cycles and depth limits.
  - Tests for FilterEngine rule ordering, include recursion, and exclude short-circuiting.
- Diff:
  - Golden tests comparing known .blend fixtures focusing on ME, DATA, OB changes, asserting summary counts and selected block statuses.
  - If distributing fixtures is problematic, build in-memory readers with mocked DNA and minimal slices simulating blocks.
- CLI:
  - Snapshot tests for text tree and flat outputs using text_trees with address mapping to ensure determinism.
  - Argument parsing tests via clap’s test harness.

Prioritized improvement plan
1) Consolidate `BlendFile` type usage and decompression handling
- Refactor `dot001-cli/src/util.rs` to always return `dot001_parser::BlendFile`.
- Remove the feature-gated alternate return types.
- Risk: low; code touch in CLI and tracer imports. Benefit: simplifies mental model.

2) Encapsulate `BlendFile` fields and provide accessors
- Make fields private in `dot001-parser/src/lib.rs`. Add public, read-only accessors and iterator-based helpers.
- Migrate internal code across crates to use the new accessors.
- Risk: medium; touches many call sites. Benefit: stronger invariants, easier refactors.

3) Create shared pointer traversal utilities
- Implement a new module in `dot001-parser` (e.g., `reflect.rs`) for generic pointer field enumeration via DNA.
- Update `FilterEngine::pointer_targets` and key expanders to use this shared utility.
- Risk: medium; requires careful DNA reading and offset calculations. Benefit: eliminates duplication and future drift.

4) Introduce `NameResolver` trait and `Determinizer`
- Define a `NameResolver` trait and a default implementation in `dot001-tracer` or a new shared crate.
- Implement a `Determinizer` for stable output generation in the CLI and diff tools.
- Risk: low. Benefit: consistent outputs and better UX.

5) Expand `BlockExpander`/`ExpandResult` for external asset tracking
- Add a field like `external_refs: Vec<PathBuf>` to `ExpandResult`.
- Update `ImageExpander` to populate this field for non-packed images.
- Risk: low; additive change. Benefit: unlocks file dependency reporting.

6) Standardize error helpers
- In `dot001-error`, add functions to build common errors with context.
- Replace ad hoc error creation in `dot001-tracer` and `dot001-diff`.
- Risk: low. Benefit: consistent messages and reduced duplication.

7) Testing investments and fixtures
- Add synthetic `BlendFile` builder helpers for tests that craft blocks, headers, and DNA fragments.
- Create minimal fixtures checked into repo or generated on-the-fly for deterministic tests.

8) Diff strategy refactor
- Extract block-type strategies into traits (e.g., `CompareStrategy`) similar to `BlockExpander`.
- Integrate `NameResolver` and `Determinizer` into the diffing process.
- Risk: medium. Benefit: modular growth path for semantic diffs.

Potential API example adjustments
- Accessor pattern:
  - dot001_parser.BlendFile::get_block_header(index) -> Option<BlockHeaderSnapshot> shown as [`fn get_block(&self, index: usize) -> Option<&BlendFileBlock>`](crates/dot001-parser/src/lib.rs:207).
- Tracer builder:
  - Example additions adjacent to [`impl<'a, R: Read + Seek> DependencyTracer<'a, R>`](crates/dot001-tracer/src/lib.rs:127).
- Shared traversal hook usage in FilterEngine to replace bespoke logic:
  - Replace bespoke code starting at [`fn pointer_targets`](crates/dot001-tracer/src/filter.rs:270) with a call to a parser::reflect::enumerate_pointers(blend, index).

Conclusion
The codebase demonstrates solid Rust practices, clear separation of concerns, and thoughtful ergonomics. The most impactful improvements are consolidation of pointer traversal, encapsulation of parser internals, unified decompression and type usage in CLI, and establishing shared services for name resolution and deterministic outputs. These changes reduce duplication, harden invariants, and make future features (external asset tracking and semantic diffs) straightforward to implement.