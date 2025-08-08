//! Demo of async watcher integration with dot001_events
//!
//! This example shows how to use the AsyncWatcher to monitor blend files
//! and automatically process them with dependency tracing when they are
//! moved or renamed.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use dot001_events::bus::{EventBus, EventFilter, Subscriber};
use dot001_events::bus_impl::async_tokio::TokioEventBus;
use dot001_events::event::{Event, EventWithMetadata, Severity, WatcherEvent};
use dot001_watcher::async_watcher::{AsyncWatcher, AsyncWatcherConfig};

/// A simple subscriber that logs watcher events
pub struct WatcherSubscriber;

#[async_trait::async_trait]
impl Subscriber for WatcherSubscriber {
    async fn on_event(&self, event: &EventWithMetadata) {
        match &event.event {
            Event::Watcher(watcher_event) => {
                let severity_str = match event.metadata.severity {
                    Severity::Error => "ERROR",
                    Severity::Warn => "WARN",
                    Severity::Info => "INFO",
                    Severity::Debug => "DEBUG",
                    Severity::Trace => "TRACE",
                };

                match watcher_event {
                    WatcherEvent::Started {
                        watch_paths,
                        recursive,
                        debounce_ms,
                    } => {
                        println!(
                            "[{}] Watcher started monitoring {} paths (recursive: {}, debounce: {}ms)",
                            severity_str,
                            watch_paths.len(),
                            recursive,
                            debounce_ms
                        );
                    }
                    WatcherEvent::BlendFileMoved { from, to, filename } => {
                        println!(
                            "[{}] Blend file moved: {} -> {} (filename: {})",
                            severity_str,
                            from.display(),
                            to.display(),
                            filename
                        );
                    }
                    WatcherEvent::BlendFileRenamed {
                        from,
                        to,
                        old_filename,
                        new_filename,
                    } => {
                        println!(
                            "[{}] Blend file renamed: {} -> {} ({} -> {})",
                            severity_str,
                            from.display(),
                            to.display(),
                            old_filename,
                            new_filename
                        );
                    }
                    WatcherEvent::ProcessingStarted {
                        trigger_path,
                        trigger_type,
                        workflow_id,
                    } => {
                        println!(
                            "[{}] Processing started for {} (type: {}, workflow: {})",
                            severity_str,
                            trigger_path.display(),
                            trigger_type,
                            &workflow_id[..8] // Show first 8 chars of UUID
                        );
                    }
                    WatcherEvent::ProcessingStepCompleted {
                        workflow_id,
                        step,
                        success,
                        step_duration_ms,
                    } => {
                        let status = if *success { "✓" } else { "✗" };
                        println!(
                            "[{}] Step {} complete: {} ({}ms, workflow: {})",
                            severity_str,
                            step,
                            status,
                            step_duration_ms,
                            &workflow_id[..8]
                        );
                    }
                    WatcherEvent::ProcessingCompleted {
                        workflow_id,
                        trigger_path,
                        total_steps,
                        total_duration_ms,
                        success,
                        results,
                    } => {
                        let status = if *success { "✓" } else { "✗" };
                        println!(
                            "[{}] Processing {} complete: {} ({} steps, {}ms, workflow: {})",
                            severity_str,
                            trigger_path.display(),
                            status,
                            total_steps,
                            total_duration_ms,
                            &workflow_id[..8]
                        );
                        if let Some(results) = results {
                            println!("    Results: {results}");
                        }
                    }
                    WatcherEvent::Error { error, workflow_id } => match workflow_id {
                        Some(id) => println!(
                            "[{}] Error in workflow {}: {}",
                            severity_str,
                            &id[..8],
                            error
                        ),
                        None => println!("[{severity_str}] Watcher error: {error}"),
                    },
                    WatcherEvent::Stopped { reason } => {
                        println!("[{severity_str}] Watcher stopped: {reason}");
                    }
                    _ => {
                        println!(
                            "[{}] {}: {}",
                            severity_str,
                            event.event.domain(),
                            event.event.event_name()
                        );
                    }
                }
            }
            Event::Tracer(dot001_events::event::TracerEvent::Finished {
                total_blocks_traced,
                unique_dependencies,
                duration_ms,
            }) => {
                println!(
                    "[TRACE] Dependency analysis complete: {unique_dependencies} dependencies, {total_blocks_traced} blocks traced in {duration_ms}ms"
                );
            }
            _ => {}
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::init();

    // Create event bus
    let event_bus: Arc<dyn EventBus> = Arc::new(TokioEventBus::with_default_capacity());

    // Set up subscriber for watcher events
    let subscriber = Arc::new(WatcherSubscriber);
    let filter = EventFilter::new()
        .min_severity(Severity::Debug)
        .domains(vec!["watcher", "tracer"]);

    let mut subscription = event_bus.subscribe(filter);

    // Spawn subscriber task
    let subscriber_task = tokio::spawn(async move {
        while let Ok(event) = subscription.recv().await {
            subscriber.on_event(&event).await;
        }
    });

    // Get watch directory from args or default to current directory
    let watch_dir = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap());

    println!("Starting async watcher demo...");
    println!("Watching directory: {}", watch_dir.display());
    println!("Try moving or renaming .blend files to see events!");
    println!("Press Ctrl+C to stop.");

    // Create and start async watcher
    let config = AsyncWatcherConfig::new(event_bus.clone())
        .watch_root(watch_dir)
        .debounce_ms(200)
        .move_pair_window_ms(2000)
        .auto_process(true)
        .max_concurrent_workflows(5);

    let watcher = AsyncWatcher::start(config)?;

    // Set up graceful shutdown
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();

    tokio::spawn(async move {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to listen for Ctrl+C");
        println!("\nShutdown signal received...");
        shutdown_tx.send(()).ok();
    });

    // Wait for shutdown signal
    shutdown_rx.await.ok();

    // Stop watcher
    watcher.stop().await;

    // Clean up subscriber task
    subscriber_task.abort();

    // Give a moment for final events
    tokio::time::sleep(Duration::from_millis(100)).await;

    println!("Watcher stopped.");
    Ok(())
}
