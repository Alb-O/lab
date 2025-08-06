Audit completed across the Rust workspace. Below is a concise report of inconsistencies, legacy patterns, and refactor opportunities, organized by area with prioritized recommendations.

Workspace and crate roles
- dot001-parser: Core parsing, decompression policy/options, DNA reflection, field reading utilities. Public API is clean, modular, and logging-aware.
- dot001-tracer: Dependency traversal over blocks with a registry of expanders per block type, filter engine integration, deterministic output support (Determinizer), and tree building.
- dot001-diff: Experimental and clearly marked incomplete; uses parser and optional tracer integration. Provides block-level diff with special handling for ME and DATA, and a provenance subsystem scaffold.
- dot001-editor: Experimental mutators for libpath and rename, with warnings and validations.
- dot001-error: Centralized error type with domains and helper modules, serde feature-gated.
- dot001-cli: Feature-gated commands, cohesive output pipeline, utility context for parsing and printing.

Key strengths
- Clear separation of concerns across crates with optional cross-crate coupling via features.
- Error system centralized in dot001-error with domain-specific variants and nice tests.
- Parser code is layered, cautious on block size, and provides ergonomic FieldReader and PointerTraversal.
- Tracer has clean expander registry, BFS traversal with max_depth options, and a path toward determinism and filters.

Findings: inconsistencies and duplication
1) CLI feature gating vs dependency feature sets
- CLI default features: info, blocks, trace; optional: diff, editor.
- dot001-diff default enables tracer_integration; dot001-editor default enables tracer_integration. This implicitly pulls dot001-tracer when those crates are used standalone, but in CLI the features are toggled separately. This can lead to confusing build matrices where enabling diff in CLI pulls tracer twice (via diff feature and diff default). Consider aligning features and defaults to minimize implicit pulls.
Recommendation: Standardize crate features:
  - Make tracer_integration off by default in library crates (dot001-diff, dot001-editor) and enable it via feature in consumers (CLI).
  - Add a workspace-level feature alias to ease common build profiles.

2) Duplicate block metadata and display code
- CLI block_utils::BlockUtils and BlockProcessor duplicate the pattern of extracting code, size, count, address, and resolving names. Similar extraction occurs in commands/filter.rs and likely elsewhere (blocks.rs, diff formatting).
Recommendation: Introduce a shared low-level metadata accessor in parser or a small crate dot001-metadata that returns a neutral struct BlockMeta { index, code, size, count, old_address, header_offset }, so CLI and other crates can share. This reduces repeated code patterns and potential drift.

3) Name resolution
- NameResolver lives in tracer; CLI utilities depend on it for identification and display. Parser does not have name resolution by intention.
Issue: Tightens coupling to tracer for name display even in commands that don’t need traversal, making tracer a transitive requirement for nice CLI UX.
Recommendation: Extract NameResolver into its own small crate or move a minimal read-only name resolution helper into parser.reflect, driven by DNA and FieldReader only. Keep advanced resolution in tracer. This reduces coupling for “read-only list” workflows.

4) Filter logic duplication
- filter.rs builds filter spec via dot001_tracer::filter builder, then separately uses BlockUtils to filter DATA blocks and display formatting, duplicating similar sorting and formatting logic used by blocks.rs.
Recommendation: Encapsulate a reusable filtering pipeline: a function that takes a BlendFile and FilterSpec, returns a stable, pre-sorted set of indices plus metadata. This avoids replication of walk and formatting logic and makes unit testing easier.

5) Block code to string conversions
- Multiple places manually convert header.code bytes to String with trim_end_matches('\0').
- Tracer includes a specialized fast path in DependencyTracer::block_code_to_string for ASCII path.
Recommendation: Create a single utility function in parser or a shared utils module: block_code_to_string([u8;4]) -> String. Replace manual conversions across crates.

6) DATA block handling policies dispersed
- The CLI conditionally hides DATA by default; diff treats DATA blocks specially via size comparison; filter also hides DATA unless show_data is set.
Issue: Policy scattered in CLI and diff; potential inconsistencies when behavior evolves.
Recommendation: Centralize policies for DATA block treatment in a policy module (e.g., in parser or a small shared crate), exposing helpers to decide visibility and comparison modes, driven by config. CLI/diff call into these helpers.

7) Error handling consistency
- dot001-error is well-structured. Most crates return Result aliases correctly and convert io errors. Some code paths in CLI use std::process::exit after logging parse errors for filters instead of returning Dot001Error.
Examples: crates/dot001-cli/src/commands/filter.rs lines ~29-33 exit(1) on expression parse failure rather than propagating a Dot001Error. This bypasses unified error messaging.
Recommendation: Replace process::exit paths with returning Dot001Error::cli and let main() handle it consistently.

8) Decompression and parse options duplication
- CLI constructs ParseOptions and load_blend_file; parser provides parse_from_path and parse_from_reader with options. Good separation, but create_parse_options uses prefer_mmap Option<bool> flags and chooses defaults scattered between CLI and parser::ParseOptions::default.
Recommendation: Define a single authoritative default policy in parser and let CLI only override provided flags. Expose a builder pattern in parser for ParseOptions to reduce direct struct construction outside the crate.

9) Incomplete dot001-diff architecture
- The crate clearly states EXPERIMENTAL. Comparisons mix content-aware and size-based heuristics. Provenance analyzer is present but not integrated fully. Name resolution is TODO. There is no use of tracer filtering when summarizing diffs.
Recommendations:
  - Split diff phases: alignment/correlation of blocks, change classification, and presentation. Introduce traits Analyzer and Correlator for ME and DATA. Move mesh content extraction code into parsable helpers that can be unit-tested.
  - Integrate NameResolver through decoupled interface (see point 3).
  - Provide a stable DiffPolicy for block-specific handling to be configurable.

10) Tracer expanders pattern duplication and opportunity for generics
- ObjectExpander and MeshExpander call PointerTraversal::read_pointer_fields and read_pointer_array with similar structure. Many expanders will follow the same pattern: gather fields list, accumulate targets, return ExpandResult, sometimes with externals.
Recommendation:
  - Introduce a declarative expander builder or a macro that describes: code => array fields and single pointer fields mappings, plus optional externals reader. This reduces boilerplate and makes new expanders safer and faster to implement.
  - Provide a helper for common “read name/path string” extraction, used by ImageExpander and others.

11) Logging consistency
- parser uses debug/trace/warn consistently. CLI formats colored logs via env_logger with custom formatting. Tracer logs debug/trace in registry and traversal.
Recommendation: Standardize log targets/module path or provide a feature-gated compact logger across CLI, or provide log scopes to align trace readability.

12) Use of Option<HashSet<usize>> in tracer filters
- DependencyTracer stores allowed as Option<HashSet<usize>> and checks membership frequently. This is fine, but building allowed set each filter application may be expensive.
Recommendation: Document performance expectations; consider borrowing allowed set or using a BitSet-like structure keyed by block count for speed if this becomes a hot path.

13) Minor API rough edges in parser
- BlendFile::max_block_size returns a constant, while comments mention future ParseOptions-based override. This can confuse consumers.
Recommendation: Add an override in BlendFile or return from ParseOptions stored in BlendFile to align behavior with comments.

14) CLI exit handling and cohesive error propagation
- Most commands return Result and main() prints user_message then exits. Some code paths still perform immediate exit (filter parsing error).
Recommendation: Ensure all CLI commands uniformly propagate errors, avoid std::process::exit in deep command code.

15) Cargo.toml polish
- Workspace sets rust-version = 1.85, edition = 2024: modern. Some crates set default features that increase implicit coupling (see #1). Keywords and categories are good.
Recommendation: Add [workspace.lints] or rustflags for deny(warnings) in CI profile. Ensure resolver = 2 set (already present).

Prioritized refactor plan
1) Error propagation unification in CLI
- Replace std::process::exit in crates/dot001-cli/src/commands/filter.rs with returning Dot001Error::cli.
- Impact: Low; Risk: Low; Effort: Very Low.

2) Centralize block code string conversion
- Add a utility e.g., parser::header::block_code_to_string(code: [u8;4]) or small shared crate.
- Replace across CLI, tracer, diff.
- Impact: Low; Risk: Low; Effort: Low.

3) Consolidate DATA block policy
- Introduce a policy module with constants and helpers: is_data_visible(show_data), data_block_compare(size1,size2).
- Use in CLI, filter, diff for consistent behavior.
- Impact: Medium; Risk: Low; Effort: Low.

4) NameResolver decoupling
- Extract minimal name resolution (for display) from tracer into a small shared helper or parser-based function relying on DNA and standard ID struct fields (e.g., ID.name).
- Update CLI to use the decoupled resolver to avoid tracer dependency for blocks/info.
- Impact: Medium; Risk: Medium; Effort: Medium.

5) Tracer expander abstraction
- Provide a small framework/helper macro to define expanders declaratively (pointer_fields, pointer_arrays, externals).
- Refactor ObjectExpander and MeshExpander to use it. Consider adding an ExternalRefReader trait for cases like images.
- Impact: Medium; Risk: Low; Effort: Medium.

6) dot001-diff modularization
- Introduce DiffPolicy, Analyzer traits per block type, isolate content extraction helpers, integrate optional NameResolver via trait, and prepare for future DNA-level semantics.
- Impact: High; Risk: Medium; Effort: High, but can be incremental.

7) Feature alignment across crates
- Make tracer_integration features opt-in for dot001-editor and dot001-diff. In CLI, tie features to subcommands explicitly.
- Optionally add workspace features profiles for typical builds (trace-only, diff-only, full).
- Impact: Medium; Risk: Low; Effort: Low/Medium.

8) Parser max_block_size and options alignment
- Store ParseOptions or max size in BlendFile and plumb through max_block_size().
- Impact: Low; Risk: Low; Effort: Low.

9) CLI reuse of a filtering pipeline
- Encapsulate the filter + metadata + sorted output as shared helpers, used by blocks and filter commands to minimize duplication.
- Impact: Medium; Risk: Low; Effort: Medium.

Notable legacy or risk-prone patterns
- Immediate process::exit inside command logic (filter.rs). Replace with unified error returns.
- Widespread manual code-to-string conversions of [u8;4].
- Incomplete provenance integration and TODOs in dot001-diff; acceptable given experimental status but should be fenced behind clear feature flags and API stability notes.

Suggested first PRs
- PR-1: CLI error flow unify, block code string util, DATA policy helper. Small, safe changes.
- PR-2: Minimal name resolver extraction for display, update CLI blocks and filter to use it when tracer is not required.
- PR-3: Tracer expander helper to reduce boilerplate and future bug surface.
- PR-4: Diff crate refactor scaffolding: introduce DiffPolicy and Analyzer traits, leave existing behavior defaulted.

Optional diagram: high-level data flow
- CLI -> Parser: load + DNA -> block metadata and optional name resolution
- CLI -> Tracer: optional dependency traversal, expanders gather pointers and externals
- CLI -> Diff: optional comparison using Parser and optional Tracer (provenance)

If you want, I can prepare concrete diffs for PR-1 and PR-2, starting with filter.rs error propagation and a shared block_code_to_string helper, followed by swapping usages in CLI and tracer where applicable.
