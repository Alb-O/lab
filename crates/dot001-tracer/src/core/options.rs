/// Options to control traversal limits and behavior

/// Options to control traversal limits and behavior.
#[derive(Debug, Clone, Copy)]
pub struct TracerOptions {
    pub max_depth: usize,
}

impl Default for TracerOptions {
    fn default() -> Self {
        Self { max_depth: 10 }
    }
}
