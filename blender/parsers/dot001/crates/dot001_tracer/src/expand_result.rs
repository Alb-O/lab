use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct ExpandResult {
    pub dependencies: Vec<usize>,
    pub external_refs: Vec<PathBuf>,
    pub debug: Option<String>,
}

impl ExpandResult {
    /// Create a new ExpandResult with only block dependencies
    pub fn new(dependencies: Vec<usize>) -> Self {
        Self {
            dependencies,
            external_refs: Vec::new(),
            debug: None,
        }
    }

    /// Create a new ExpandResult with both block dependencies and external references
    pub fn with_externals(dependencies: Vec<usize>, external_refs: Vec<PathBuf>) -> Self {
        Self {
            dependencies,
            external_refs,
            debug: None,
        }
    }

    /// Create a new ExpandResult with dependencies and debug info
    pub fn with_debug(dependencies: Vec<usize>, debug: String) -> Self {
        Self {
            dependencies,
            external_refs: Vec::new(),
            debug: Some(debug),
        }
    }

    /// Create a full ExpandResult with all fields
    pub fn full(dependencies: Vec<usize>, external_refs: Vec<PathBuf>, debug: String) -> Self {
        Self {
            dependencies,
            external_refs,
            debug: Some(debug),
        }
    }

    /// Add an external file reference
    pub fn add_external_ref(&mut self, path: PathBuf) {
        self.external_refs.push(path);
    }

    /// Add multiple external file references
    pub fn add_external_refs(&mut self, paths: Vec<PathBuf>) {
        self.external_refs.extend(paths);
    }

    /// Check if this result contains any external references
    pub fn has_external_refs(&self) -> bool {
        !self.external_refs.is_empty()
    }
}
