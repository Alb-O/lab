Blendfile Rust Model
====================

Overview
--------

This crate provides a strongly-typed Rust model of Blender’s `.blend` file internals, designed for
thread-safe, parallel parsers and CLI editors. It centers on:

- SDNA (Struct DNA): types, members, and struct layouts.
- BHead blocks: normalized block headers from Blender’s file format.
- Pointers and IDs: safe wrappers for old addresses and ID-like struct detection.
- Thread-safe registry: concurrent block indexing by old pointer.

Scope
-----

This crate models data; it does not implement a full file parser. A parser can:

1) Read the Blender header (pointer size, endianness, low-level file format version).
2) Read the `DNA1` block and construct an `Sdna`.
3) Normalize each `BHead` and register `Block`s into `BlockRegistry`.

Key Types
---------

- `BlenderHeader`: endianness, pointer width, version, and BHead kind.
- `BHead`/`BlockCode`: normalized block headers.
- `OldPtr`: tagged 32/64-bit old addresses; `OldPtrKey` for hashing.
- `Sdna`: types, members, structs; lookups and ID-like detection.
- `Block`: block header + payload bytes (opaque here).
- `BlockRegistry`: concurrent map from `OldPtr` to `Block`.

ID Detection
------------

By default, a struct is considered ID-like if its first member is named `id` and has type `ID`.
Consumers may extend this heuristic.

Parallel Use
------------

`BlockRegistry` uses `DashMap` for concurrent reads/writes. `Sdna` and `Block` payloads are
reference-counted via `Arc`.

More Docs
---------

- Roadmap: `docs/ROADMAP.md`
- Testing Strategy: `docs/TESTING.md`
