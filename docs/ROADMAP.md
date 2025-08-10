Roadmap
=======

Phase 1 — Parser Skeleton (Read-Only)
-------------------------------------
- Header decode: detect pointer size, endianness, low-level file format (0/1).
- BHead normalization: support BHead4, SmallBHead8, LargeBHead8 into runtime `BHead`.
- DNA1 loader: decode SDNA into `Sdna` (types, members, structs, alignments, maps).
- Block ingestion: stream blocks, store `Block { header, data }`, register by `OldPtr`.

Phase 2 — Rich Typed Reads
--------------------------
- Path navigation: dotted paths (done), add array index syntax (e.g. `verts[12].co`).
- Typed math accessors: f64 variants and typed matrices/vectors across member bases.
- Nested struct helpers: `at_member_struct` (done), add `try_at_path_struct` with better errors.
- ListBase iterators: typed traversal for common lists (constraints, modifiers, etc.).

Phase 3 — Safe Edits & Transactions
-----------------------------------
- Typed setters mirroring getters (endianness & bounds aware).
- Mutability model: snapshot + transactional edits; validation hooks.
- Pointer rewrites: relocate/fix-up `OldPtr` references and dependent blocks.
- ID operations: rename, relink, ensure name uniqueness per ID type (Main indexing).

Phase 4 — Append / Inject
-------------------------
- Import IDs from another file: copy ID block + direct data, rebuild pointers.
- Name conflict policy (rename or replace) with dependency tracing.
- Asset metadata preservation (where applicable) and library linking semantics.

Phase 5 — CLI
-------------
- Query/inspect: list IDs, print dependencies, show field values by path.
- Edit/apply: set fields by path, rename IDs, remap users.
- Append/extract: copy IDs across files from command line.

Phase 6 — Performance
---------------------
- Zero-copy views over file-mapped data when possible.
- Parallel block ingestion and indexing.
- Zstd/zlib stream support via pluggable readers.

Phase 7 — Compatibility & Robustness
------------------------------------
- Cross-version matrices of features with fixtures from 2.7x → 5.x.
- Big-endian (legacy) handling where still relevant.
- Extensive error types with context for all parsing/reading paths.

Phase 8 — Ecosystem & API Polish
---------------------------------
- Serde support (feature) for exporting model states.
- Documentation with examples for common tasks.
- MSRV policy, CI for linting/format/testing across OSes.

