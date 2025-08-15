Testing Strategy
================

Goals
-----
- Ensure layout correctness against SDNA across versions.
- Guarantee safe, endian-correct, pointer-width-correct field access.
- Validate concurrent indexing correctness and determinism.
- Verify editing (when added) preserves invariants and pointers.

Unit Tests (src/*)
------------------
- `member`:
  - Parse names: `"*next"`, `"**parent"`, `"mat[4][4]"`, invalid forms.
  - `ArrayDims` semantics: `len()`, `is_empty()`.
- `types`:
  - Endian-aware readers with fixed byte sequences (both little/big via parameter).
- `sdna` + `layout`:
  - Synthetic SDNA: basic types with sizes/alignments; structs with scalars, arrays, pointers.
  - Check offsets are aligned; total size matches SDNA; pointer members use ptr width.
- `view`:
  - Build mock `Block` payloads and assert `get_f32`, `get_i32`, arrays, pointers.
  - Nested `at_member_struct`, `at_index`, dotted `at_path_struct`.
- `transform`:
  - Presence/absence combinations: only `loc`, only `quat`, both `rot_euler` + `size`, etc.
- `registry`:
  - Multithreaded inserts/gets; ensure no lost updates; `is_empty`/`len` correctness.
- `resolve`:
  - Simulated ListBase chain with blocks and old pointers; cycle protection.

Integration Tests (tests/)
--------------------------
- Parser skeleton (once implemented):
  - Open real `.blend` fixtures from multiple versions.
  - Verify header decode, SDNA load, block counts.
  - Sample fields (Object.loc, Mesh.totvert, Scene.r.cfra) using `StructView`.
- Round-trips (once write/edit exists):
  - Edit numeric fields and write file; reopen and compare.
  - Append IDs from another file and verify dependency graph and names.

Property/Fuzz Testing
---------------------
- `cargo-fuzz` on `MemberNameSpec::parse` to ensure no panics on arbitrary input.
- `proptest` for layout: generate random synthetic type graphs; assert monotonic offsets, size bounds, alignment correctness.
- Fuzz SDNA decoder (when added) with constrained random buffers; assert graceful errors.

Concurrency & Safety
--------------------
- Miri run for UB checks in purely safe code paths.
- Stress test `BlockRegistry` with high-concurrency insert/get under randomized workloads.

Benchmarks (benches/)
---------------------
- Criterion:
  - SDNA layout computation time vs struct complexity.
  - Field access (`get_vec3`, `get_mat4x4`) throughput on large blocks.
  - Pointer resolution throughput with `BlockRegistry`.

CI Plan
-------
- `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test` (all features).
- Matrix for stable and latest toolchain; Linux/macOS/Windows.
- Optional job: coverage via `grcov` or `llvm-cov`.

Fixtures
--------
- Generate minimal `.blend` files headlessly via Blender’s Python API (scripted):
  - Minimal Scene, Object with known transform, Mesh with few verts, a linked ListBase.
  - Store fixtures under `tests/fixtures/` with a README noting Blender version used.

