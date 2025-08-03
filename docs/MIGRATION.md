# Bllink Migration and Refactoring Plan

This document outlines the current state of the `bllink` project, its architectural limitations, and a proposed plan for migrating to a more capable and robust architecture.

## 1. Project Overview

bllink is a Rust-based tool for analyzing `.blend` files, focusing on dependency tracing and data inspection. The workspace consists of two main crates:

- `bllink-core`: The engine for parsing `.blend` files, handling the low-level file structure, DNA (data structures), and providing the foundational logic for dependency tracing.
- `bllink-cli`: A command-line interface that uses `bllink-core` to provide user-facing commands for inspecting various aspects of a `.blend` file.

The project aims to provide functionality similar to Python's `blender-asset-tracer` with the performance and type safety of Rust.

## 2. Core Crate Analysis (`bllink-core`)

### 2.1. Key Modules and Functionality

- `header.rs`: Parses the `.blend` file header.
- `block.rs`: Manages the file blocks, the fundamental units of data.
- `dna.rs`: Handles the DNA block, which contains the file's data schema.
- `fields.rs`: Provides logic for reading and interpreting data fields within a block.
- `dependencies/`: The core of the dependency tracing logic.
    - `context.rs`: Defines `DependencyContext`, which holds immutable references to the parsed `.blend` file data.
    - `expanders.rs`: Contains `BlockExpander` implementations for specific block types.
    - `tracer.rs`: The `DependencyTracer` orchestrates the traversal.
    - `tree.rs`: Defines the `DependencyTree` data structure.

### 2.2. Architectural Strengths

- Layered design: Separation between CLI and core library.
- Performance: Efficient file reading through zero-copy parsing.
- Static analysis: Well-suited for simple, direct dependency links.

### 2.3. Architectural Weaknesses and Issues

The `ARCHITECTURE.md` document identifies the primary architectural flaw: a borrowing conflict that prevents dynamic data access during dependency tracing.

- The core problem: `DependencyContext` holds immutable references to the `BlendFile`'s data. To trace complex dependencies (such as linked lists or arrays of pointers), `BlockExpander` implementations require mutable access to the `BlendFile` to read additional data blocks on demand. Rust's borrowing rules prevent this.
- Consequences:
    - Incomplete traversals: The system cannot fully traverse linked lists or dereference arrays of pointers.
    - Workarounds: The code uses heuristics and proximity-based guesses to find dependencies.
    - Feature limitations: The tool cannot achieve feature parity with `blender-asset-tracer` due to these constraints.

## 3. CLI Crate Analysis (`bllink-cli`)

The CLI is structured using `clap` but is limited by the capabilities of the `bllink-core` library. The `dependencies` command can only display the incomplete graphs produced by the core.

## 4. Additional Notes and Technical Details

Further analysis of the source code reveals several technical details:

- Fragile proximity heuristics: The `SceneExpander` in `expanders.rs` does not follow the `Scene.base.first` pointer to find objects. Instead, it iterates through the file blocks immediately following the `SC` block, assuming they are `Base` blocks for that scene. This approach is brittle and fails in many `.blend` files.
- Raw pointer scanning: The `DataExpander` acts as a fallback and scans the raw byte data of a block for values resembling pointers. This workaround is inefficient and prone to errors, highlighting the architecture's limitations.
- Incomplete array handling: The `ObjectExpander` finds the primary material link (`id.ma`) but does not handle the `*mats` field, which is a pointer to an array of material pointers. The code contains a `// TODO` comment for this limitation, indicating the need for on-demand data reading.
- Central role of `address_index`: The `DependencyContext` contains an `address_index` (`HashMap<u64, usize>`) mapping a block's memory address to its index in the file's block list. This mechanism is efficient and should be preserved. The issue is not pointer resolution, but the inability to read the data of a block during traversal.
- `DependencyTracer` as the enforcer: The architectural bottleneck is enforced in `tracer.rs`. The `trace_from` method reads the current block's data, creates the `DependencyContext` (placing an immutable borrow on the `BlendFile`), and calls the appropriate `BlockExpander` with the context and pre-read data. No further calls to `blend_file.read_block_data()` are possible due to the active immutable borrow. This is the point that requires refactoring.
- Output formatting is decoupled: The `output.rs` module in the CLI handles printing the `DependencyTree` and is decoupled from the tracing logic. A refactored core producing a more accurate `DependencyTree` should not require significant changes to output formatting.

## 5. Proposed Migration Path

The fundamental issue cannot be addressed with small patches. A deliberate architectural refactoring is required.

Goal: Allow `BlockExpander` implementations to read arbitrary block data from the `BlendFile` during dependency expansion.

Recommended approach:

1. Redefine the `BlockExpander` trait to provide mutable access to the `BlendFile`.

    Current:
    ```rust
    pub trait BlockExpander {
        fn expand_block(
            &self,
            block_index: usize,
            context: &DependencyContext,
            block_data: &[u8],
        ) -> Result<Vec<usize>>;
    }
    ```

    Proposed:
    ```rust
    pub trait BlockExpander<R: Read + Seek> {
        fn expand_block(
            &self,
            block_index: usize,
            context: &DependencyContext, // Context now contains only non-borrowing info
            blend_file: &mut BlendFile<R>, // Pass the file mutably
        ) -> Result<Vec<usize>>;
    }
    ```

2. Refactor `DependencyContext`: The context should not hold direct references to `blend_file` data (such as `&[BlendFileBlock]`). It should contain data that can be cloned or does not borrow, such as the `DnaCollection`. The `address_index` may need to be passed alongside it.

3. Refactor `DependencyTracer`: The tracer will be responsible for passing the mutable `blend_file` reference to the expanders. The core loop will change from "read then trace" to "trace and read on-demand".

4. Rewrite expanders: With the new architecture, workarounds can be removed.
    - `SceneExpander`: Can read `Scene.base.first`, then read the `Base` block, and follow the `next` pointers, reading each subsequent `Base` and `Object` block.
    - `ObjectExpander`: Can read the `totcol` field for materials, read the `*mats` block, and iterate through the array of pointers, resolving each one.
    - `DataExpander`: Can likely be deprecated or removed.

5. Enhance testing: Create new `.blend` files with complex structures (linked lists, pointer arrays, nested collections) to serve as unit tests for the new traversal logic. This is critical for verifying the migration's success.
