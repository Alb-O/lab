//! Macros for ergonomic event publishing
//!
//! This module provides macros to make event publishing more convenient,
//! including both explicit bus passing and global bus access.

/// Emit an event to the specified event bus
///
/// # Examples
///
/// ```rust
/// use dot001_events::prelude::*;
/// use dot001_events::event::{Event, CoreEvent};
/// use std::sync::Arc;
///
/// # async fn example() {
/// let bus: Arc<dyn EventBus> = todo!();
/// let message = "Operation started".to_string();
///
/// // Emit with explicit bus
/// emit!(bus, Event::Core(CoreEvent::Info { message }));
///
/// // Emit with custom severity and context
/// let mut context = std::collections::HashMap::new();
/// context.insert("user".to_string(), "alice".to_string());
/// emit!(bus, Event::Core(CoreEvent::Info { message: "test".to_string() }), Severity::Debug, Some(context));
/// # }
/// ```
#[macro_export]
macro_rules! emit {
    // Basic usage: emit!(bus, event)
    ($bus:expr, $event:expr) => {
        $bus.publish($event).await
    };

    // With custom severity: emit!(bus, event, severity)
    ($bus:expr, $event:expr, $severity:expr) => {
        $bus.publish_with_metadata($event, $severity, None).await
    };

    // With custom severity and context: emit!(bus, event, severity, context)
    ($bus:expr, $event:expr, $severity:expr, $context:expr) => {
        $bus.publish_with_metadata($event, $severity, $context)
            .await
    };
}

/// Emit an event to the global event bus
///
/// This requires the global bus to be initialized first via `init_global_bus()`.
///
/// # Examples
///
/// ```rust
/// use dot001_events::prelude::*;
/// use dot001_events::event::{Event, CoreEvent};
///
/// # async fn example() {
/// // Emit to global bus (panics if not initialized)
/// emit_global!(Event::Core(CoreEvent::Info { message: "test".to_string() }));
///
/// // With custom severity
/// emit_global!(Event::Core(CoreEvent::Info { message: "test".to_string() }), Severity::Debug);
/// # }
/// ```
///
/// # Panics
///
/// Panics if the global event bus has not been initialized.
#[macro_export]
macro_rules! emit_global {
    // Basic usage: emit_global!(event)
    ($event:expr) => {
        $crate::bus::get_global_bus().publish($event).await
    };

    // With custom severity: emit_global!(event, severity)
    ($event:expr, $severity:expr) => {
        $crate::bus::get_global_bus()
            .publish_with_metadata($event, $severity, None)
            .await
    };

    // With custom severity and context: emit_global!(event, severity, context)
    ($event:expr, $severity:expr, $context:expr) => {
        $crate::bus::get_global_bus()
            .publish_with_metadata($event, $severity, $context)
            .await
    };
}

/// Try to emit an event to the global event bus, returning None if not initialized
///
/// This is a non-panicking version of `emit_global!` that returns `Option<()>`.
///
/// # Examples
///
/// ```rust
/// use dot001_events::prelude::*;
/// use dot001_events::event::{Event, CoreEvent};
///
/// # async fn example() {
/// // Try to emit to global bus (returns None if not initialized)
/// if try_emit_global!(Event::Core(CoreEvent::Info { message: "test".to_string() })).is_some() {
///     println!("Event emitted successfully");
/// } else {
///     println!("Global bus not initialized");
/// }
/// # }
/// ```
#[macro_export]
macro_rules! try_emit_global {
    // Basic usage: try_emit_global!(event)
    ($event:expr) => {
        if let Some(bus) = $crate::bus::try_get_global_bus() {
            bus.publish($event).await;
            Some(())
        } else {
            None
        }
    };

    // With custom severity: try_emit_global!(event, severity)
    ($event:expr, $severity:expr) => {
        if let Some(bus) = $crate::bus::try_get_global_bus() {
            bus.publish_with_metadata($event, $severity, None).await;
            Some(())
        } else {
            None
        }
    };

    // With custom severity and context: try_emit_global!(event, severity, context)
    ($event:expr, $severity:expr, $context:expr) => {
        if let Some(bus) = $crate::bus::try_get_global_bus() {
            bus.publish_with_metadata($event, $severity, $context).await;
            Some(())
        } else {
            None
        }
    };
}

/// Emit an event synchronously using tokio::spawn (for sync contexts)
///
/// This macro spawns the event emission in a tokio task, making it usable
/// from synchronous code. Only use when you're sure a tokio runtime is available.
#[macro_export]
macro_rules! emit_sync {
    // Basic usage: emit_sync!(bus, event)
    ($bus:expr, $event:expr) => {{
        let bus = $bus.clone();
        let event = $event;
        tokio::spawn(async move {
            bus.publish(event).await;
        });
    }};

    // With custom severity: emit_sync!(bus, event, severity)
    ($bus:expr, $event:expr, $severity:expr) => {{
        let bus = $bus.clone();
        let event = $event;
        let severity = $severity;
        tokio::spawn(async move {
            bus.publish_with_metadata(event, severity, None).await;
        });
    }};

    // With custom severity and context: emit_sync!(bus, event, severity, context)
    ($bus:expr, $event:expr, $severity:expr, $context:expr) => {{
        let bus = $bus.clone();
        let event = $event;
        let severity = $severity;
        let context = $context;
        tokio::spawn(async move {
            bus.publish_with_metadata(event, severity, context).await;
        });
    }};
}

/// Emit an event synchronously to the global bus using tokio::spawn
#[macro_export]
macro_rules! emit_global_sync {
    // Basic usage: emit_global_sync!(event)
    ($event:expr) => {
        if let Some(bus) = $crate::bus::try_get_global_bus() {
            $crate::emit_sync!(bus, $event);
        }
    };

    // With custom severity: emit_global_sync!(event, severity)
    ($event:expr, $severity:expr) => {
        if let Some(bus) = $crate::bus::try_get_global_bus() {
            $crate::emit_sync!(bus, $event, $severity);
        }
    };

    // With custom severity and context: emit_global_sync!(event, severity, context)
    ($event:expr, $severity:expr, $context:expr) => {
        if let Some(bus) = $crate::bus::try_get_global_bus() {
            $crate::emit_sync!(bus, $event, $severity, $context);
        }
    };
}

/// Create a context HashMap for events
///
/// # Examples
///
/// ```rust
/// use dot001_events::context;
///
/// let ctx = context! {
///     "user" => "alice",
///     "operation" => "parse",
///     "file_size" => "1024"
/// };
/// ```
#[macro_export]
macro_rules! context {
    // Empty context
    {} => {
        None
    };

    // Context with key-value pairs
    { $($key:expr => $value:expr),+ $(,)? } => {
        {
            let mut ctx = std::collections::HashMap::new();
            $(
                ctx.insert($key.to_string(), $value.to_string());
            )+
            Some(ctx)
        }
    };
}

/// Convenience macro for emitting Core events
///
/// # Examples
///
/// ```rust
/// use dot001_events::prelude::*;
/// use std::sync::Arc;
///
/// # async fn example() {
/// let bus: Arc<dyn EventBus> = todo!();
///
/// // Emit info message
/// emit_info!(bus, "Operation completed successfully");
///
/// // Emit warning with code
/// emit_warning!(bus, "DEPRECATED_API", "This API will be removed in v2.0");
///
/// // Emit error
/// emit_error!(bus, Error::io("File not found"));
/// # }
/// ```
#[macro_export]
macro_rules! emit_info {
    ($bus:expr, $message:expr) => {
        $crate::emit!(
            $bus,
            $crate::event::Event::Core($crate::event::CoreEvent::Info {
                message: $message.to_string(),
            })
        )
    };
}

#[macro_export]
macro_rules! emit_warning {
    ($bus:expr, $code:expr, $message:expr) => {
        $crate::emit!(
            $bus,
            $crate::event::Event::Core($crate::event::CoreEvent::Warning {
                code: $code.to_string(),
                message: $message.to_string(),
            })
        )
    };
}

#[macro_export]
macro_rules! emit_error {
    ($bus:expr, $error:expr) => {
        $crate::emit!(
            $bus,
            $crate::event::Event::Core($crate::event::CoreEvent::Error { error: $error })
        )
    };
}

/// Convenience macro for emitting progress events
///
/// # Examples
///
/// ```rust
/// use dot001_events::prelude::*;
/// use std::sync::Arc;
///
/// # async fn example() {
/// let bus: Arc<dyn EventBus> = todo!();
///
/// // Basic progress
/// emit_progress!(bus, "Parsing", 5, 10);
///
/// // Progress with message
/// emit_progress!(bus, "Processing", 3, 8, "Loading block data");
///
/// // Indeterminate progress
/// emit_progress!(bus, "Initializing", 1, None);
/// # }
/// ```
#[macro_export]
macro_rules! emit_progress {
    // Indeterminate progress with message (match this before generic total forms)
    ($bus:expr, $operation:expr, $current:expr, None, $message:expr) => {
        $crate::emit!(
            $bus,
            $crate::event::Event::Core($crate::event::CoreEvent::Progress {
                operation: $operation.to_string(),
                current: $current,
                total: None,
                message: Some($message.to_string()),
            })
        )
    };

    // Indeterminate progress without message (match before generic total forms)
    ($bus:expr, $operation:expr, $current:expr, None) => {
        $crate::emit!(
            $bus,
            $crate::event::Event::Core($crate::event::CoreEvent::Progress {
                operation: $operation.to_string(),
                current: $current,
                total: None,
                message: None,
            })
        )
    };

    // With total and message
    ($bus:expr, $operation:expr, $current:expr, $total:expr, $message:expr) => {
        $crate::emit!(
            $bus,
            $crate::event::Event::Core($crate::event::CoreEvent::Progress {
                operation: $operation.to_string(),
                current: $current,
                total: Some($total),
                message: Some($message.to_string()),
            })
        )
    };

    // With total, no message
    ($bus:expr, $operation:expr, $current:expr, $total:expr) => {
        $crate::emit!(
            $bus,
            $crate::event::Event::Core($crate::event::CoreEvent::Progress {
                operation: $operation.to_string(),
                current: $current,
                total: Some($total),
                message: None,
            })
        )
    };
}

#[cfg(test)]
mod tests {

    use crate::bus::{EventBus, EventFilter};
    use crate::bus_impl::TokioEventBus;
    use crate::event::{CoreEvent, Event};
    use std::sync::Arc;
    use tokio::time::{timeout, Duration};

    #[tokio::test]
    async fn test_emit_macro() {
        let bus: Arc<dyn EventBus> = Arc::new(TokioEventBus::with_default_capacity());
        let filter = EventFilter::new();
        let mut subscription = bus.subscribe(filter);

        // Test basic emit
        let event = Event::Core(CoreEvent::Info {
            message: "test message".to_string(),
        });
        emit!(bus, event);

        // Should receive the event
        let received = timeout(Duration::from_millis(100), subscription.recv())
            .await
            .expect("Should receive event")
            .expect("Should successfully receive event");

        assert_eq!(received.event.domain(), "core");
        assert_eq!(received.event.event_name(), "info");
    }

    #[tokio::test]
    async fn test_context_macro() {
        let ctx = context! {
            "user" => "alice",
            "operation" => "test"
        };

        assert!(ctx.is_some());
        let ctx = ctx.unwrap();
        assert_eq!(ctx.get("user"), Some(&"alice".to_string()));
        assert_eq!(ctx.get("operation"), Some(&"test".to_string()));
    }

    #[tokio::test]
    async fn test_emit_info_macro() {
        let bus: Arc<dyn EventBus> = Arc::new(TokioEventBus::with_default_capacity());
        let filter = EventFilter::new();
        let mut subscription = bus.subscribe(filter);

        emit_info!(bus, "Test info message");

        let received = timeout(Duration::from_millis(100), subscription.recv())
            .await
            .expect("Should receive event")
            .expect("Should successfully receive event");

        if let Event::Core(CoreEvent::Info { message }) = &received.event {
            assert_eq!(message, "Test info message");
        } else {
            panic!("Expected CoreEvent::Info");
        }
    }
}
