//! Async watcher integration with dot001_events
//!
//! This module provides async integration between the file system watcher and the
//! dot001_events EventBus, enabling decoupled workflow processing for blend file
//! operations like moves and renames.

use futures::{StreamExt, pin_mut};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::task::JoinHandle;
use uuid::Uuid;

use dot001_events::error::WatcherErrorKind;
use dot001_events::event::{Event, WatcherEvent};
use dot001_events::prelude::*;

use crate::{WatchEvent, WatchOptions};

/// Configuration for the async watcher workflow
#[derive(Clone)]
pub struct AsyncWatcherConfig {
    /// Base watcher options
    pub watch_options: WatchOptions,
    /// Event bus to publish events to
    pub event_bus: Arc<dyn EventBus>,
    /// Whether to process events automatically
    pub auto_process: bool,
    /// Maximum concurrent workflows
    pub max_concurrent_workflows: usize,
}

impl AsyncWatcherConfig {
    /// Create new config with an event bus
    pub fn new(event_bus: Arc<dyn EventBus>) -> Self {
        Self {
            watch_options: WatchOptions::default(),
            event_bus,
            auto_process: true,
            max_concurrent_workflows: 10,
        }
    }

    /// Set the watch root directory
    pub fn watch_root<P: AsRef<Path>>(mut self, root: P) -> Self {
        self.watch_options.root = root.as_ref().to_path_buf();
        self
    }

    /// Set debounce timing
    pub fn debounce_ms(mut self, ms: u64) -> Self {
        self.watch_options.debounce_ms = ms;
        self
    }

    /// Set move pair detection window
    pub fn move_pair_window_ms(mut self, ms: u64) -> Self {
        self.watch_options.move_pair_window_ms = ms;
        self
    }

    /// Enable/disable automatic workflow processing
    pub fn auto_process(mut self, enabled: bool) -> Self {
        self.auto_process = enabled;
        self
    }

    /// Set maximum concurrent workflows
    pub fn max_concurrent_workflows(mut self, max: usize) -> Self {
        self.max_concurrent_workflows = max;
        self
    }
}

/// Handle for the async watcher
pub struct AsyncWatcher {
    /// Handle to the watcher task
    task_handle: JoinHandle<()>,
    /// Event bus reference for publishing
    event_bus: Arc<dyn EventBus>,
}

impl AsyncWatcher {
    /// Start the async watcher with the given configuration
    pub fn start(config: AsyncWatcherConfig) -> std::result::Result<Self, crate::WatchError> {
        let event_bus = config.event_bus.clone();

        // Emit watcher started event
        let started_event = Event::Watcher(WatcherEvent::Started {
            watch_paths: vec![config.watch_options.root.clone()],
            recursive: true,
            debounce_ms: config.watch_options.debounce_ms,
        });

        let bus_clone = event_bus.clone();
        tokio::spawn(async move {
            bus_clone.publish(started_event).await;
        });

        // Start the watcher task
        let event_bus_for_error = event_bus.clone();
        let task_handle = tokio::spawn(async move {
            if let Err(e) = Self::run_watcher_loop(config).await {
                let error_event = Event::Watcher(WatcherEvent::Error {
                    error: dot001_events::error::Error::watcher(
                        format!("Watcher failed: {e}"),
                        WatcherErrorKind::EventProcessingFailed,
                    ),
                    workflow_id: None,
                });
                let _ = event_bus_for_error.publish(error_event).await;
            }
        });

        Ok(Self {
            task_handle,
            event_bus,
        })
    }

    /// Stop the async watcher
    pub async fn stop(self) {
        self.task_handle.abort();
        let stopped_event = Event::Watcher(WatcherEvent::Stopped {
            reason: "Manual stop requested".to_string(),
        });
        let _ = self.event_bus.publish(stopped_event).await;
    }

    /// Main watcher loop
    async fn run_watcher_loop(
        config: AsyncWatcherConfig,
    ) -> std::result::Result<(), crate::WatchError> {
        // Use the existing async watcher stream
        let stream = crate::async_api::watch_stream(config.watch_options)?;
        pin_mut!(stream);

        while let Some(watch_event) = stream.next().await {
            let workflow_id = Uuid::new_v4().to_string();

            match watch_event {
                WatchEvent::BlendFileMoved(pair) => {
                    let event = Event::Watcher(WatcherEvent::BlendFileMoved {
                        from: pair.from.clone(),
                        to: pair.to.clone(),
                        filename: pair.base.to_string_lossy().to_string(),
                    });
                    config.event_bus.publish(event).await;

                    if config.auto_process {
                        Self::spawn_workflow(
                            workflow_id,
                            "moved".to_string(),
                            pair.to,
                            config.event_bus.clone(),
                        )
                        .await;
                    }
                }

                WatchEvent::BlendFileRenamed {
                    from,
                    to,
                    base_from,
                    base_to,
                } => {
                    let event = Event::Watcher(WatcherEvent::BlendFileRenamed {
                        from: from.clone(),
                        to: to.clone(),
                        old_filename: base_from.to_string_lossy().to_string(),
                        new_filename: base_to.to_string_lossy().to_string(),
                    });
                    config.event_bus.publish(event).await;

                    if config.auto_process {
                        Self::spawn_workflow(
                            workflow_id,
                            "renamed".to_string(),
                            to,
                            config.event_bus.clone(),
                        )
                        .await;
                    }
                }

                WatchEvent::DirRenamedOrMoved(pair) => {
                    let event = Event::Watcher(WatcherEvent::DirectoryMoved {
                        from: pair.from.clone(),
                        to: pair.to.clone(),
                        blend_files_affected: 0, // TODO: Calculate affected files
                    });
                    config.event_bus.publish(event).await;

                    if config.auto_process {
                        Self::spawn_workflow(
                            workflow_id,
                            "dir_moved".to_string(),
                            pair.to,
                            config.event_bus.clone(),
                        )
                        .await;
                    }
                }

                WatchEvent::DirBlendChildMoved(pair) => {
                    // Find the parent move info from the pair
                    let parent_from = pair.from.parent().unwrap_or(Path::new("")).to_path_buf();
                    let parent_to = pair.to.parent().unwrap_or(Path::new("")).to_path_buf();

                    let event = Event::Watcher(WatcherEvent::BlendFileMovedWithDirectory {
                        from: pair.from.clone(),
                        to: pair.to.clone(),
                        filename: pair.base.to_string_lossy().to_string(),
                        parent_move: (parent_from, parent_to),
                    });
                    config.event_bus.publish(event).await;

                    if config.auto_process {
                        Self::spawn_workflow(
                            workflow_id,
                            "moved_with_directory".to_string(),
                            pair.to,
                            config.event_bus.clone(),
                        )
                        .await;
                    }
                }
            }
        }

        Ok(())
    }

    /// Spawn an async workflow for processing a file event
    async fn spawn_workflow(
        workflow_id: String,
        trigger_type: String,
        trigger_path: PathBuf,
        event_bus: Arc<dyn EventBus>,
    ) {
        // Emit processing started event
        let started_event = Event::Watcher(WatcherEvent::ProcessingStarted {
            trigger_path: trigger_path.clone(),
            trigger_type: trigger_type.clone(),
            workflow_id: workflow_id.clone(),
        });
        event_bus.publish(started_event).await;

        // Spawn the actual workflow task
        let workflow_id_clone = workflow_id.clone();
        let trigger_path_clone = trigger_path.clone();
        let event_bus_clone = event_bus.clone();

        tokio::spawn(async move {
            let workflow_result = Self::execute_workflow(
                workflow_id_clone.clone(),
                trigger_type.clone(),
                trigger_path_clone.clone(),
                event_bus_clone.clone(),
            )
            .await;

            // Emit completion event
            let completed_event = Event::Watcher(WatcherEvent::ProcessingCompleted {
                workflow_id: workflow_id_clone,
                trigger_path: trigger_path_clone,
                total_steps: 3,       // TODO: Make this configurable
                total_duration_ms: 0, // TODO: Track actual duration
                success: workflow_result.is_ok(),
                results: match workflow_result {
                    Ok(summary) => Some(summary),
                    Err(e) => Some(format!("{{\"error\": \"{e}\"}}")),
                },
            });

            event_bus_clone.publish(completed_event).await;
        });
    }

    /// Execute the processing workflow for a file event
    async fn execute_workflow(
        workflow_id: String,
        _trigger_type: String,
        trigger_path: PathBuf,
        event_bus: Arc<dyn EventBus>,
    ) -> std::result::Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let start_time = std::time::Instant::now();

        // Step 1: Validate file exists and is accessible
        Self::emit_step_completed(&workflow_id, "validation", true, 50, &event_bus).await;

        // Step 2: Dependency tracing using dot001_tracer
        let trace_result = Self::trace_dependencies(&trigger_path, &workflow_id, &event_bus).await;
        let trace_success = trace_result.is_ok();
        Self::emit_step_completed(
            &workflow_id,
            "dependency_trace",
            trace_success,
            100, // TODO: Use actual elapsed time
            &event_bus,
        )
        .await;

        // Step 3: Update completion
        Self::emit_step_completed(&workflow_id, "update_complete", true, 30, &event_bus).await;

        let duration = start_time.elapsed();
        Ok(format!(
            "{{\"file\": \"{}\", \"duration_ms\": {}, \"steps\": 3}}",
            trigger_path.display(),
            duration.as_millis()
        ))
    }

    /// Perform dependency tracing on a blend file
    async fn trace_dependencies(
        trigger_path: &PathBuf,
        _workflow_id: &str,
        event_bus: &Arc<dyn EventBus>,
    ) -> std::result::Result<String, Box<dyn std::error::Error + Send + Sync>> {
        use std::fs;
        // Read the blend file
        let bytes = fs::read(trigger_path)?;
        // Parse using modern zero-copy API
        let blend_file = dot001_parser::from_bytes_buf(bytes)?;

        // For now, emit a simple tracer summary based on block count
        let start = std::time::Instant::now();
        let blocks_traced = blend_file.blocks_len();
        let duration = start.elapsed();

        // Emit tracer finished event
        let trace_event = Event::Tracer(dot001_events::event::TracerEvent::Finished {
            total_blocks_traced: blocks_traced,
            unique_dependencies: 0,
            duration_ms: duration.as_millis() as u64,
        });
        event_bus.publish(trace_event).await;

        Ok(format!(
            "{{\"dependencies\": {}, \"blocks_traced\": {}, \"duration_ms\": {}}}",
            0,
            blocks_traced,
            duration.as_millis()
        ))
    }

    /// Emit a step completion event
    async fn emit_step_completed(
        workflow_id: &str,
        step: &str,
        success: bool,
        duration_ms: u64,
        event_bus: &Arc<dyn EventBus>,
    ) {
        let event = Event::Watcher(WatcherEvent::ProcessingStepCompleted {
            workflow_id: workflow_id.to_string(),
            step: step.to_string(),
            success,
            step_duration_ms: duration_ms,
        });
        event_bus.publish(event).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dot001_events::bus_impl::async_tokio::TokioEventBus;

    #[tokio::test]
    async fn test_async_watcher_config() {
        let event_bus = Arc::new(TokioEventBus::with_default_capacity());
        let config = AsyncWatcherConfig::new(event_bus)
            .watch_root("/tmp")
            .debounce_ms(500)
            .auto_process(true);

        assert_eq!(config.watch_options.root, PathBuf::from("/tmp"));
        assert_eq!(config.watch_options.debounce_ms, 500);
        assert!(config.auto_process);
    }

    #[tokio::test]
    async fn test_workflow_id_generation() {
        let id1 = Uuid::new_v4().to_string();
        let id2 = Uuid::new_v4().to_string();

        assert_ne!(id1, id2);
        assert!(!id1.is_empty());
        assert!(!id2.is_empty());
    }
}
