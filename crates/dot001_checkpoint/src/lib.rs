use std::path::{Path, PathBuf};
use thiserror::Error;
use titor::{Checkpoint, CompressionStrategy, Titor, TitorBuilder};

#[derive(Error, Debug)]
pub enum BlenderCheckpointError {
    #[error("Titor error: {0}")]
    Titor(#[from] titor::TitorError),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Invalid Blender project path: {0}")]
    InvalidProjectPath(String),
}

pub type Result<T> = std::result::Result<T, BlenderCheckpointError>;

pub struct BlenderCheckpointer {
    titor: Titor,
    project_path: PathBuf,
}

impl BlenderCheckpointer {
    pub fn new<P: AsRef<Path>>(project_path: P) -> Result<Self> {
        let project_path = project_path.as_ref().to_path_buf();

        if !project_path.exists() {
            return Err(BlenderCheckpointError::InvalidProjectPath(format!(
                "Project path does not exist: {}",
                project_path.display()
            )));
        }

        let checkpoint_dir = project_path.join(".dot001");

        let titor = TitorBuilder::new()
            .compression_strategy(CompressionStrategy::Adaptive {
                min_size: 4096,
                skip_extensions: vec![
                    "blend1".to_string(),
                    "blend2".to_string(),
                    "blend3".to_string(),
                    "blend4".to_string(),
                    "blend5".to_string(),
                    "blend6".to_string(),
                    "blend7".to_string(),
                    "blend8".to_string(),
                    "blend9".to_string(),
                    "blend10".to_string(),
                    "tmp".to_string(),
                    "cache".to_string(),
                ],
            })
            .ignore_patterns(vec![
                "*.tmp".to_string(),
                "*.cache".to_string(),
                ".DS_Store".to_string(),
                "Thumbs.db".to_string(),
                "__pycache__/**".to_string(),
                ".blend_backup/**".to_string(),
            ])
            .max_file_size(500 * 1024 * 1024) // 500MB limit for individual files
            .follow_symlinks(false)
            .parallel_workers(num_cpus::get())
            .build(project_path.clone(), checkpoint_dir)?;

        Ok(Self {
            titor,
            project_path,
        })
    }

    pub fn checkpoint(&mut self, description: Option<String>) -> Result<Checkpoint> {
        let checkpoint = self.titor.checkpoint(description)?;
        Ok(checkpoint)
    }

    pub fn checkpoint_with_blend_info(
        &mut self,
        blend_file: Option<&str>,
        description: Option<String>,
    ) -> Result<Checkpoint> {
        let full_description = match (blend_file, description) {
            (Some(blend), Some(desc)) => Some(format!("Blend: {blend} - {desc}")),
            (Some(blend), None) => Some(format!("Blend: {blend}")),
            (None, Some(desc)) => Some(desc),
            (None, None) => None,
        };

        self.checkpoint(full_description)
    }

    pub fn restore(&mut self, checkpoint_id: &str) -> Result<titor::types::RestoreResult> {
        let result = self.titor.restore(checkpoint_id)?;
        Ok(result)
    }

    pub fn list_checkpoints(&self) -> Result<Vec<Checkpoint>> {
        let checkpoints = self.titor.list_checkpoints()?;
        Ok(checkpoints)
    }

    pub fn diff(&self, from_id: &str, to_id: &str) -> Result<titor::types::CheckpointDiff> {
        let diff = self.titor.diff(from_id, to_id)?;
        Ok(diff)
    }

    pub fn diff_detailed(
        &self,
        from_id: &str,
        to_id: &str,
    ) -> Result<titor::types::DetailedCheckpointDiff> {
        let options = titor::types::DiffOptions {
            context_lines: 3,
            ignore_whitespace: false,
            show_line_numbers: true,
            max_file_size: 10 * 1024 * 1024, // 10MB
        };

        let diff = self.titor.diff_detailed(from_id, to_id, options)?;
        Ok(diff)
    }

    pub fn gc(&self) -> Result<titor::types::GcStats> {
        let stats = self.titor.gc()?;
        Ok(stats)
    }

    pub fn verify_checkpoint(&self, checkpoint_id: &str) -> Result<bool> {
        let report = self.titor.verify_checkpoint(checkpoint_id)?;
        Ok(report.is_valid())
    }

    pub fn project_path(&self) -> &Path {
        &self.project_path
    }

    pub fn find_blend_files(&self) -> Result<Vec<PathBuf>> {
        let mut blend_files = Vec::new();
        let mut stack = vec![self.project_path.clone()];
        while let Some(dir) = stack.pop() {
            if dir.is_dir() {
                for entry in std::fs::read_dir(&dir)? {
                    let entry = entry?;
                    let path = entry.path();
                    if path.is_dir()
                        && !path
                            .file_name()
                            .unwrap_or_default()
                            .to_str()
                            .unwrap_or("")
                            .starts_with('.')
                    {
                        stack.push(path);
                    } else if path.extension().and_then(|s| s.to_str()) == Some("blend") {
                        blend_files.push(path);
                    }
                }
            }
        }
        Ok(blend_files)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_new_checkpointer() {
        let temp_dir = TempDir::new().unwrap();
        let result = BlenderCheckpointer::new(temp_dir.path());
        assert!(result.is_ok());
    }

    #[test]
    fn test_invalid_project_path() {
        let result = BlenderCheckpointer::new("/nonexistent/path");
        assert!(matches!(
            result,
            Err(BlenderCheckpointError::InvalidProjectPath(_))
        ));
    }

    #[test]
    fn test_find_blend_files() {
        let temp_dir = TempDir::new().unwrap();
        let blend_path = temp_dir.path().join("test.blend");
        fs::write(&blend_path, b"fake blend content").unwrap();

        let checkpointer = BlenderCheckpointer::new(temp_dir.path()).unwrap();
        let blend_files = checkpointer.find_blend_files().unwrap();

        assert_eq!(blend_files.len(), 1);
        assert_eq!(blend_files[0], blend_path);
    }
}
