# 1. Project Overview

bllink is a Rust-based tool for analyzing .blend files, focusing on dependency tracing and data inspection. The workspace consists of two main crates:

- `bllink-core`: The engine for parsing .blend files, handling the low-level file structure, DNA (data structures), and providing the foundational logic for dependency tracing.
- `bllink-cli`: A command-line interface that uses bllink-core to provide user-facing commands for inspecting various aspects of a .blend file.

The project aims to provide functionality similar to Python's blender-asset-tracer, with the performance and type safety of Rust.

# 2. Core Crate Analysis (bllink-core)

## 2.1. Key Modules and Functionality

- `header.rs`: Parses the .blend file header, identifying the Blender version and system architecture.
- `block.rs`: Manages the file blocks, which are the fundamental units of data in a .blend file.
- `dna.rs`: Handles the "DNA" block, which contains the file's data schema (structures, types, and fields).
- `fields.rs`: Provides logic for reading and interpreting data fields within a block, based on the DNA schema.
- `dependencies/`: The core of the dependency tracing logic.
    - `context.rs`: Defines DependencyContext, which holds immutable references to the parsed .blend file data (blocks, DNA, address index). This is passed to expanders.
    - `expanders.rs`: Contains different BlockExpander implementations, each responsible for finding dependencies for a specific block type (e.g., ObjectExpander, SceneExpander).
    - `tracer.rs`: The DependencyTracer orchestrates the traversal, using the expanders to build a dependency tree.
    - `tree.rs`: Defines the DependencyTree data structure used to represent the relationships between blocks.

## 2.2. Architectural Strengths

- Layered design: Separation between CLI and core library.
- Performance: Zero-copy parsing and direct memory mapping (where applicable) for efficient file reading.
- Static analysis: Well-suited for simple, direct dependency links (e.g., Object → Mesh).

## 2.3. Architectural Weaknesses and Issues

The ARCHITECTURE.md document identifies the primary architectural flaw: a borrowing conflict that prevents dynamic data access during dependency tracing.

- The core problem: DependencyContext holds immutable references to the BlendFile's data. To trace complex dependencies (such as linked lists or arrays of pointers), BlockExpanders require mutable access to the BlendFile to read additional data blocks on demand. Rust's borrowing rules prevent this mutable access while immutable references exist.
- Consequences:
    - Incomplete traversals: The system cannot fully traverse linked lists or dereference arrays of pointers.
    - Workarounds: The code uses heuristics and proximity-based guesses to find dependencies, which are brittle and incomplete. For example, the SceneExpander cannot properly iterate through all of a scene's objects.
    - Feature limitations: The tool cannot achieve feature parity with blender-asset-tracer due to these constraints.
- Root cause: The architecture was designed for static analysis of pre-read data chunks, but sophisticated dependency tracing is inherently dynamic.

# 3. CLI Crate Analysis (bllink-cli)

## 3.1. Key Modules and Functionality

- `main.rs`: The entry point, using the clap crate to define the command-line interface and its subcommands.
- `commands/`: Each file in this module corresponds to a CLI subcommand.
    - `info.rs`: Displays header information from the .blend file.
    - `blocks.rs`: Lists all the data blocks in the file.
    - `dna.rs`: Inspects the DNA structures.
    - `fields.rs`: Reads and displays the fields of a specific data block.
    - `dependencies.rs`: Uses the DependencyTracer from bllink-core to build and display a dependency tree.
    - `output.rs`: Handles the formatting of the output (e.g., as a tree).

## 3.2. CLI Strengths

- Well-structured: The use of clap makes the CLI robust and easy to extend.
- Clear separation: Each command is encapsulated in its own module.
- Output: The tree view for dependencies provides a clear visualization.

## 3.3. CLI Weaknesses

- Limited by core: The dependencies command is limited by the architectural constraints of bllink-core. It can only display the incomplete dependency graphs produced by the core.
- Error handling: Basic error handling is present, but could be improved to provide more context, especially during dependency tracing.

# 4. Overall Project Summary and Recommendations

## 4.1. What Works Well

- Parsing and inspecting the basic structure of .blend files (header, blocks, DNA).
- Tracing simple, direct pointer dependencies.
- A well-organized codebase and CLI structure.

## 4.2. Major Issues

- The architectural flaw in bllink-core's dependency tracing prevents the tool from fulfilling its primary purpose for complex scenes.
- The project cannot progress further without a significant architectural refactor.

## 4.3. Recommendations

The ARCHITECTURE.md file outlines several potential solutions. The most viable path forward is an architectural redesign.

1. Acknowledge the limitation: The current state is suitable for simple file inspection, but not for comprehensive dependency tracing.
2. Prioritize a refactor: The recommended solution is to refactor the BlockExpander trait and the DependencyTracer to allow for mutable access to the BlendFile during expansion. This is a significant change but is necessary to address the core problem.

    Proposed new trait signature:
    ```rs
    pub trait BlockExpander {
        fn expand_block<R: Read + Seek>(
            &self,
            block_index: usize,
            blend_file: &mut BlendFile<R>,
        ) -> Result<Vec<usize>>;
    }
    ```
3. Incremental improvements (pre-refactor):
    - Improve the accuracy of existing workaround expanders where possible.
    - Add more detailed logging to clarify incomplete traversals.
    - Enhance CLI output to label dependency chains that are known to be incomplete.
4. Testing: The project requires a more robust testing suite, especially for dependency tracing. Creating test .blend files with complex dependency graphs (e.g., multiple materials, linked collections, drivers) is necessary for verifying current logic and future refactoring. The existing tests/test-blendfiles directory is a starting point but should be expanded.

In summary, bllink provides a foundation for .blend file parsing. To achieve comprehensive dependency tracing, a significant architectural refactor is required to overcome the current borrowing limitations.
