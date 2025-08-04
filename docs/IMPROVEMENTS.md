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
- **Leverage `thiserror` context**: Use `#[from]` and `#[source]` attributes for cleaner error chaining, especially for `io::Error`. This would make the `From<std::io::Error>` implementation more idiomatic and remove the need for `source_message`.
- **Implement `Display`**: Add a `impl Display for Dot001Error` that calls `user_message()`. This allows for direct, user-friendly printing of errors (e.g., `println!("Error: {}", my_error);`).
- **Add `From<Dot001Error>` conversions**: Or newtype wrappers per crate if you want crate-specific `Result<T>` but still unify at the CLI boundary. Currently many crates re-export `dot001_error::Result` which is fine.
- **Provide helper constructors**: For common error patterns, create constructors in a small extension trait per domain, living in `dot001-error` to avoid repetition of message strings across crates.
- **Consider an `ErrorContext` struct**: For shared fields (file_path, block_index, command, operation), an `ErrorContext` struct could reduce enum payload variability and provide consistent accessor methods.

B) dot001-parser
Findings:
- `BlendFile::new` checks for zstd magic bytes; `parse_from_*` handles decompression policy correctly.
- The public `BlendFile` struct exposes its `reader` and all other fields, which, while convenient, leaks implementation details.
- The DNA is always parsed, which might be unnecessary for commands that only need header info.

Suggestions:
- **Encapsulate `BlendFile` fields**: Expose functionality through public methods to strengthen invariants. For example, provide an iterator over blocks of a certain type instead of direct access to the `blocks` vector.
- **Lazy-load DNA**: Modify `BlendFile` to parse the DNA block only when it's first accessed (e.g., via a `dna()` method that caches the result). This would speed up simple commands like `info`.
- **Refine `DnaName` parsing**: The `DnaName::new` and `calc_array_size_fast` methods could be simplified and made more robust, perhaps by using a small, dedicated parser function for the name string.
- **Introduce a `BlockReader`**: Create a `BlockReader<'a>` struct that takes ownership of a block's data (`Vec<u8>`) and a reference to the `DnaCollection`. This would encapsulate the logic for reading data from a single block and improve ergonomics over the current `FieldReader`.
- **Offer borrow-safe methods**: Return lightweight structs with header snapshots to prevent callers from maintaining references into internal `Vec` while mutating.
- **Consider zero-copy options**: For block data with an internal buffer pool or guarded slice if using mmap, to reduce allocations for repeated reads. Currently `read_block_data` allocates a new `Vec` each time.
- **Unify compression detection**: The magic byte check in `BlendFile::new` is redundant given the logic in `parse_from_path`. Centralize this detection to avoid drift.

C) dot001-tracer
Findings:
- `DependencyTracer` uses a robust BFS approach with good protection against cycles.
- The `FilterEngine`'s `pointer_targets` heuristics overlap with `BlockExpander` logic, creating a maintenance risk.
- `NameResolver` is a static utility struct with methods that require a mutable `BlendFile` reference, which can be awkward to use.

Suggestions:
- **Refactor pointer traversal**: Consolidate pointer traversal logic into a shared utility or trait in `dot001-parser` to be used by both `FilterEngine` and the expanders.
- **Decouple `NameResolver`**: Make `NameResolver` an instantiable struct that holds an immutable reference to `BlendFile`, improving its ergonomics and removing the need for `&mut` borrows in display logic.
- **Richer `ExpandResult`**: Enhance `ExpandResult` to include a `DependencyKind` enum (`Direct`, `Array`, `Node`, etc.) to provide more context on *why* a block is a dependency.
- **Centralize `BlockExpander` registration**: Add a `register_default_expanders` method to `DependencyTracer` to simplify the setup in `dot001-cli`.
- **Introduce a `Determinizer`**: Create a utility to apply address remapping and stable sorting for any outputs, centralizing deterministic output generation.
- **Enhance `BlockExpander::expand_block`**: Optionally return metadata (e.g., external file refs) via a richer `ExpandResult`, unblocking `ImageExpander`’s TODO for file paths.

D) dot001-cli
Findings:
- The command structure is modular, but some command implementations are becoming large and monolithic.
- The `load_blend_file` function in `util.rs` has a surprising type coupling based on the "trace" feature.
- Error handling is inconsistent; some commands print to `eprintln!` and return `Ok(())`.

Suggestions:
- **Introduce a `CliContext` struct**: Create a `CliContext` to hold shared state like `ParseOptions` and `no_auto_decompress`, and pass it to command functions instead of multiple individual arguments.
- **Refactor commands into modules**: Organize each command into its own submodule (e.g., `commands::info`, `commands::dependencies`) to improve structure and maintainability.
- **Break down large commands**: Refactor long command functions like `cmd_filter` into smaller, more focused functions for parsing, filtering, and formatting.
- **Standardize on a single `BlendFile` type**: The `util::load_blend_file` function should always return a `dot001_parser::BlendFile`. The tracer can then be instantiated with a reference to this file.
- **Normalize error returns**: Ensure all commands return a `Result<(), Dot001Error>` and let `main` handle the rendering of errors to the user.
- **Extract output formatting**: Create a small presenter module for tree/flat/json formatting so it can be reused by `dependencies`, `diff`, and future commands.

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
1) ✅ **COMPLETED** Consolidate `BlendFile` type usage and decompression handling
- ✅ Refactored `dot001-cli/src/util.rs` to always return `dot001_parser::BlendFile`.
- ✅ Removed the feature-gated alternate return types.
- ✅ Risk: low; code touch in CLI and tracer imports. Benefit: simplifies mental model.

2) ✅ **COMPLETED** Encapsulate `BlendFile` fields and provide accessors
- ✅ Made fields private in `dot001-parser/src/lib.rs`. Added public, read-only accessors and iterator-based helpers.
- ✅ Migrated internal code across crates to use the new accessors.
- ✅ Risk: medium; touches many call sites. Benefit: stronger invariants, easier refactors.

3) ✅ **COMPLETED** Create shared pointer traversal utilities
- ✅ Implemented new `reflect.rs` module in `dot001-parser` with `PointerTraversal` utility struct.
- ✅ Added DNA-based `find_pointer_targets` for generic pointer field discovery.  
- ✅ Added `read_pointer_array` helper for materials arrays and similar patterns.
- ✅ Added `read_pointer_fields` helper for multiple single pointer fields.
- ✅ Updated `FilterEngine::pointer_targets` to use shared utilities with specialized heuristics.
- ✅ Refactored `ObjectExpander` and `MeshExpander` to use shared utilities.
- Risk: medium; required careful DNA reading and borrowing patterns. Benefit: eliminates duplication and future drift.

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