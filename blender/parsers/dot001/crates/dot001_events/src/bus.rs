//! Event bus traits and filtering
//!
//! This module defines the core abstractions for the async event system:
//! EventBus for publishing and subscribing, EventFilter for selective subscriptions,
//! and related types for managing event flow.

use std::sync::Arc;
use tokio::sync::broadcast;

use crate::event::{Event, EventWithMetadata, Severity};

/// Type alias for event predicate functions to reduce complexity
pub type EventPredicate = Arc<dyn Fn(&EventWithMetadata) -> bool + Send + Sync>;

/// Trait for event bus implementations
#[async_trait::async_trait]
pub trait EventBus: Send + Sync {
    /// Publish an event to all subscribers
    async fn publish(&self, event: Event);

    /// Publish an event with explicit severity and context
    async fn publish_with_metadata(
        &self,
        event: Event,
        severity: Severity,
        context: Option<crate::event::Kv>,
    );

    /// Subscribe to events with a filter
    fn subscribe(&self, filter: EventFilter) -> Subscription;

    /// Get bus statistics (optional)
    fn stats(&self) -> Option<BusStats> {
        None
    }
}

/// Trait for event subscribers
#[async_trait::async_trait]
pub trait Subscriber: Send + Sync {
    /// Handle an incoming event
    async fn on_event(&self, event: &EventWithMetadata);
}

/// Event filter for selective subscriptions
#[derive(Clone)]
pub struct EventFilter {
    /// Minimum severity level to receive
    pub min_severity: Severity,
    /// Allowed domains (None means all domains)
    pub domains: Option<Vec<String>>,
    /// Custom predicate function
    pub predicate: Option<EventPredicate>,
}

impl std::fmt::Debug for EventFilter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EventFilter")
            .field("min_severity", &self.min_severity)
            .field("domains", &self.domains)
            .field("predicate", &self.predicate.as_ref().map(|_| "<predicate>"))
            .finish()
    }
}

impl EventFilter {
    /// Create a new filter with minimum severity
    pub fn new() -> Self {
        Self {
            min_severity: Severity::Info,
            domains: None,
            predicate: None,
        }
    }

    /// Set minimum severity level
    pub fn min_severity(mut self, severity: Severity) -> Self {
        self.min_severity = severity;
        self
    }

    /// Set allowed domains
    pub fn domains<I, S>(mut self, domains: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.domains = Some(domains.into_iter().map(|s| s.into()).collect());
        self
    }

    /// Add a custom predicate
    pub fn with_predicate<F>(mut self, predicate: F) -> Self
    where
        F: Fn(&EventWithMetadata) -> bool + Send + Sync + 'static,
    {
        self.predicate = Some(Arc::new(predicate));
        self
    }

    /// Check if an event passes this filter
    pub fn matches(&self, event: &EventWithMetadata) -> bool {
        // Check severity
        if !event.metadata.severity.should_show(self.min_severity) {
            return false;
        }

        // Check domains
        if let Some(ref allowed_domains) = self.domains {
            if !allowed_domains.contains(&event.event.domain().to_string()) {
                return false;
            }
        }

        // Check custom predicate
        if let Some(ref predicate) = self.predicate {
            if !predicate(event) {
                return false;
            }
        }

        true
    }
}

impl Default for EventFilter {
    fn default() -> Self {
        Self::new()
    }
}

/// Event subscription handle
pub struct Subscription {
    /// Receiver for events
    pub receiver: broadcast::Receiver<Arc<EventWithMetadata>>,
    /// Filter used for this subscription
    pub filter: EventFilter,
}

impl Subscription {
    /// Create a new subscription
    pub fn new(receiver: broadcast::Receiver<Arc<EventWithMetadata>>, filter: EventFilter) -> Self {
        Self { receiver, filter }
    }

    /// Try to receive the next event that matches the filter
    pub async fn recv(&mut self) -> Result<Arc<EventWithMetadata>, broadcast::error::RecvError> {
        loop {
            let event = self.receiver.recv().await?;
            if self.filter.matches(&event) {
                return Ok(event);
            }
            // Continue to next event if this one doesn't match
        }
    }

    /// Try to receive the next event without blocking
    pub fn try_recv(&mut self) -> Result<Arc<EventWithMetadata>, broadcast::error::TryRecvError> {
        loop {
            let event = self.receiver.try_recv()?;
            if self.filter.matches(&event) {
                return Ok(event);
            }
            // Continue to next event if this one doesn't match
        }
    }
}

/// Bus statistics for monitoring
#[derive(Debug, Clone)]
pub struct BusStats {
    /// Total events published
    pub events_published: u64,
    /// Active subscriptions
    pub active_subscriptions: usize,
    /// Current channel capacity
    pub channel_capacity: usize,
    /// Events dropped due to full channels
    pub events_dropped: u64,
}

/// A subscriber implementation that calls a closure for each event
pub struct ClosureSubscriber<F> {
    handler: F,
}

impl<F> ClosureSubscriber<F>
where
    F: Fn(&EventWithMetadata) -> BoxFuture<'_, ()> + Send + Sync,
{
    pub fn new(handler: F) -> Self {
        Self { handler }
    }
}

use std::future::Future;
use std::pin::Pin;

type BoxFuture<'a, T = ()> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

#[async_trait::async_trait]
impl<F> Subscriber for ClosureSubscriber<F>
where
    F: Fn(&EventWithMetadata) -> BoxFuture<'_, ()> + Send + Sync,
{
    async fn on_event(&self, event: &EventWithMetadata) {
        (self.handler)(event).await
    }
}

/// A simple subscriber that prints events
pub struct PrintSubscriber;

#[async_trait::async_trait]
impl Subscriber for PrintSubscriber {
    async fn on_event(&self, event: &EventWithMetadata) {
        println!(
            "[{}] {}: {}",
            format_severity(event.metadata.severity),
            event.event.domain(),
            event.event.event_name()
        );
    }
}

fn format_severity(severity: Severity) -> &'static str {
    match severity {
        Severity::Trace => "TRACE",
        Severity::Debug => "DEBUG",
        Severity::Info => "INFO",
        Severity::Warn => "WARN",
        Severity::Error => "ERROR",
    }
}

/// Global event bus accessor using OnceCell
use once_cell::sync::OnceCell;

static GLOBAL_BUS: OnceCell<Arc<dyn EventBus>> = OnceCell::new();

/// Initialize the global event bus (should be called once by the CLI)
pub fn init_global_bus(bus: Arc<dyn EventBus>) -> Result<(), Arc<dyn EventBus>> {
    GLOBAL_BUS.set(bus)
}

/// Get the global event bus (panics if not initialized)
pub fn get_global_bus() -> Arc<dyn EventBus> {
    GLOBAL_BUS
        .get()
        .expect("Global event bus not initialized. Call init_global_bus() first.")
        .clone()
}

/// Try to get the global event bus (returns None if not initialized)
pub fn try_get_global_bus() -> Option<Arc<dyn EventBus>> {
    GLOBAL_BUS.get().cloned()
}

// Re-export async_trait for implementations
pub use async_trait::async_trait;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{CoreEvent, Event, Severity};

    #[test]
    fn test_event_filter_severity() {
        let filter = EventFilter::new().min_severity(Severity::Warn);

        let info_event = EventWithMetadata::new(
            Event::Core(CoreEvent::Info {
                message: "test".to_string(),
            }),
            Severity::Info,
            None,
        );

        let warn_event = EventWithMetadata::new(
            Event::Core(CoreEvent::Warning {
                code: "TEST".to_string(),
                message: "test".to_string(),
            }),
            Severity::Warn,
            None,
        );

        assert!(!filter.matches(&info_event));
        assert!(filter.matches(&warn_event));
    }

    #[test]
    fn test_event_filter_domains() {
        let filter = EventFilter::new().domains(vec!["core", "parser"]);

        let core_event = EventWithMetadata::new(
            Event::Core(CoreEvent::Info {
                message: "test".to_string(),
            }),
            Severity::Info,
            None,
        );

        let diff_event = EventWithMetadata::new(
            Event::Diff(crate::event::DiffEvent::Started {
                lhs: "/tmp/a.blend".into(),
                rhs: "/tmp/b.blend".into(),
                diff_type: "full".to_string(),
            }),
            Severity::Info,
            None,
        );

        assert!(filter.matches(&core_event));
        assert!(!filter.matches(&diff_event));
    }
}
