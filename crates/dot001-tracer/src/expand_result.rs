#[derive(Debug, Clone)]
pub struct ExpandResult {
    pub dependencies: Vec<usize>,
    pub debug: Option<String>,
}

impl ExpandResult {
    pub fn new(dependencies: Vec<usize>) -> Self {
        Self {
            dependencies,
            debug: None,
        }
    }
    pub fn with_debug(dependencies: Vec<usize>, debug: String) -> Self {
        Self {
            dependencies,
            debug: Some(debug),
        }
    }
}
