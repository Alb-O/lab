// bllink-tracer/src/lib.rs

//! # bllink-tracer
//!
//! Dependency tracing engine for Blender .blend files.
//!
//! This crate provides the core dependency tracing functionality with support for
//! sophisticated traversal patterns including linked lists and array dereferencing.
//!
//! ## Key Features
//!
//! - **Dynamic data access**: Block expanders can read additional data on-demand
//! - **Material array dereferencing**: Properly handles objects with multiple materials
//! - **Cross-version compatibility**: Works with Blender 2.79 through 5.0+
//! - **Extensible architecture**: Easy to add new block expanders
//!
//! ## Example
//!
//! ```rust,no_run
//! use bllink_tracer::{BlendFile, DependencyTracer, ObjectExpander};
//! use std::fs::File;
//! use std::io::BufReader;
//!
//! let file = File::open("scene.blend")?;
//! let mut reader = BufReader::new(file);
//! let mut blend_file = BlendFile::new(&mut reader)?;
//!
//! let mut tracer = DependencyTracer::new();
//! tracer.register_expander(*b"OB\0\0", Box::new(ObjectExpander));
//!
//! let deps = tracer.trace_dependencies(object_block_index, &mut blend_file)?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

pub mod expanders;

pub use bllink_parser::{BlendFile, Result};
pub use expanders::{CollectionExpander, MeshExpander, ObjectExpander, SceneExpander};
use std::collections::{HashMap, HashSet, VecDeque};
use std::io::{Read, Seek};
use std::marker::PhantomData;

pub trait BlockExpander<R: Read + Seek> {
    fn expand_block(&self, block_index: usize, blend_file: &mut BlendFile<R>)
        -> Result<Vec<usize>>;

    fn can_handle(&self, code: &[u8; 4]) -> bool;
}

pub struct DependencyTracer<'a, R: Read + Seek> {
    expanders: HashMap<[u8; 4], Box<dyn BlockExpander<R> + 'a>>,
    visited: HashSet<usize>,
    visiting: HashSet<usize>,
    _phantom: PhantomData<&'a R>,
}

impl<'a, R: Read + Seek> Default for DependencyTracer<'a, R> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, R: Read + Seek> DependencyTracer<'a, R> {
    pub fn new() -> Self {
        DependencyTracer {
            expanders: HashMap::new(),
            visited: HashSet::new(),
            visiting: HashSet::new(),
            _phantom: PhantomData,
        }
    }

    pub fn register_expander(&mut self, code: [u8; 4], expander: Box<dyn BlockExpander<R> + 'a>) {
        self.expanders.insert(code, expander);
    }

    pub fn trace_dependencies(
        &mut self,
        start_block_index: usize,
        blend_file: &mut BlendFile<R>,
    ) -> Result<Vec<usize>> {
        self.visited.clear();
        self.visiting.clear();
        let mut result = Vec::new();
        let mut queue = VecDeque::new();
        queue.push_back(start_block_index);

        while let Some(block_index) = queue.pop_front() {
            if self.visited.contains(&block_index) {
                continue;
            }
            if !self.visiting.insert(block_index) {
                continue;
            }

            if let Some(block) = blend_file.blocks.get(block_index) {
                if let Some(expander) = self.expanders.get(&block.header.code) {
                    let deps = expander.expand_block(block_index, blend_file)?;
                    for dep in deps {
                        if !self.visited.contains(&dep) {
                            queue.push_back(dep);
                        }
                    }
                }
            }

            self.visiting.remove(&block_index);
            self.visited.insert(block_index);
            if block_index != start_block_index {
                result.push(block_index);
            }
        }
        Ok(result)
    }
}
