//! Tokio-based async event bus implementation
//!
//! This module provides the primary EventBus implementation using Tokio's
//! broadcast channel for fan-out distribution to multiple subscribers.

use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};
use tokio::sync::broadcast;

use crate::bus::{async_trait, BusStats, EventBus, EventFilter, Subscription};
use crate::event::{Event, EventWithMetadata, Kv, Severity};

/// Tokio-based event bus implementation
pub struct TokioEventBus {
    /// Broadcast sender for fan-out to subscribers
    sender: broadcast::Sender<Arc<EventWithMetadata>>,
    /// Channel capacity
    capacity: usize,
    /// Statistics tracking
    stats: TokioEventBusStats,
}

/// Statistics for the Tokio event bus
struct TokioEventBusStats {
    events_published: AtomicU64,
    events_dropped: AtomicU64,
}

impl TokioEventBus {
    /// Create a new Tokio event bus with the specified capacity
    pub fn new(capacity: usize) -> Self {
        let (sender, _receiver) = broadcast::channel(capacity);

        Self {
            sender,
            capacity,
            stats: TokioEventBusStats {
                events_published: AtomicU64::new(0),
                events_dropped: AtomicU64::new(0),
            },
        }
    }

    /// Create a new event bus with default capacity (1024)
    pub fn with_default_capacity() -> Self {
        Self::new(1024)
    }

    /// Get the current capacity of the broadcast channel
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Get the number of active receivers
    pub fn receiver_count(&self) -> usize {
        self.sender.receiver_count()
    }
}

#[async_trait]
impl EventBus for TokioEventBus {
    async fn publish(&self, event: Event) {
        let severity = event.default_severity();
        self.publish_with_metadata(event, severity, None).await;
    }

    async fn publish_with_metadata(&self, event: Event, severity: Severity, context: Option<Kv>) {
        let event_with_metadata = Arc::new(EventWithMetadata::new(event, severity, context));

        // Optional tracing integration
        #[cfg(feature = "tracing")]
        {
            let domain = event_with_metadata.event.domain();
            let event_name = event_with_metadata.event.event_name();

            match severity {
                Severity::Error => {
                    tracing::error!(domain = domain, event = event_name, "Event published")
                }
                Severity::Warn => {
                    tracing::warn!(domain = domain, event = event_name, "Event published")
                }
                Severity::Info => {
                    tracing::info!(domain = domain, event = event_name, "Event published")
                }
                Severity::Debug => {
                    tracing::debug!(domain = domain, event = event_name, "Event published")
                }
                Severity::Trace => {
                    tracing::trace!(domain = domain, event = event_name, "Event published")
                }
            }
        }

        // Publish to broadcast channel
        match self.sender.send(event_with_metadata) {
            Ok(_) => {
                self.stats.events_published.fetch_add(1, Ordering::Relaxed);
            }
            Err(_) => {
                // No receivers, which is fine - increment published count anyway
                self.stats.events_published.fetch_add(1, Ordering::Relaxed);
            }
        }
    }

    fn subscribe(&self, filter: EventFilter) -> Subscription {
        let receiver = self.sender.subscribe();
        Subscription::new(receiver, filter)
    }

    fn stats(&self) -> Option<BusStats> {
        Some(BusStats {
            events_published: self.stats.events_published.load(Ordering::Relaxed),
            active_subscriptions: self.receiver_count(),
            channel_capacity: self.capacity(),
            events_dropped: self.stats.events_dropped.load(Ordering::Relaxed),
        })
    }
}

impl Clone for TokioEventBus {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
            capacity: self.capacity,
            stats: TokioEventBusStats {
                events_published: AtomicU64::new(0), // New instance gets fresh stats
                events_dropped: AtomicU64::new(0),
            },
        }
    }
}

/// Spawn a task to handle events from a subscription with a subscriber
pub fn spawn_subscriber_task<S>(
    mut subscription: Subscription,
    subscriber: Arc<S>,
) -> tokio::task::JoinHandle<()>
where
    S: crate::bus::Subscriber + 'static,
{
    tokio::spawn(async move {
        loop {
            match subscription.recv().await {
                Ok(event) => {
                    // Call the subscriber handler
                    subscriber.on_event(&event).await;
                }
                Err(broadcast::error::RecvError::Closed) => {
                    // Bus was closed, exit gracefully
                    break;
                }
                Err(broadcast::error::RecvError::Lagged(_)) => {
                    // Some events were missed due to slow processing
                    // Could emit a warning event here if we had access to bus
                    continue;
                }
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bus::{EventFilter, Subscriber};
    use crate::event::{CoreEvent, Event};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tokio::time::{timeout, Duration};

    struct TestSubscriber {
        event_count: AtomicUsize,
    }

    impl TestSubscriber {
        fn new() -> Self {
            Self {
                event_count: AtomicUsize::new(0),
            }
        }

        fn event_count(&self) -> usize {
            self.event_count.load(Ordering::Relaxed)
        }
    }

    #[async_trait]
    impl Subscriber for TestSubscriber {
        async fn on_event(&self, _event: &EventWithMetadata) {
            self.event_count.fetch_add(1, Ordering::Relaxed);
        }
    }

    #[tokio::test]
    async fn test_basic_publish_subscribe() {
        let bus = TokioEventBus::with_default_capacity();
        let filter = EventFilter::new();
        let mut subscription = bus.subscribe(filter);

        // Publish an event
        let event = Event::Core(CoreEvent::Info {
            message: "test message".to_string(),
        });
        bus.publish(event).await;

        // Receive the event
        let received = timeout(Duration::from_millis(100), subscription.recv())
            .await
            .expect("Should receive event within timeout")
            .expect("Should successfully receive event");

        assert_eq!(received.event.domain(), "core");
        assert_eq!(received.event.event_name(), "info");
    }

    #[tokio::test]
    async fn test_filtering_by_severity() {
        let bus = TokioEventBus::with_default_capacity();
        let filter = EventFilter::new().min_severity(Severity::Warn);
        let mut subscription = bus.subscribe(filter);

        // Publish an info event (should be filtered out)
        let info_event = Event::Core(CoreEvent::Info {
            message: "info message".to_string(),
        });
        bus.publish(info_event).await;

        // Publish a warning event (should be received)
        let warn_event = Event::Core(CoreEvent::Warning {
            code: "TEST_WARN".to_string(),
            message: "warning message".to_string(),
        });
        bus.publish(warn_event).await;

        // Should receive only the warning event
        let received = timeout(Duration::from_millis(100), subscription.recv())
            .await
            .expect("Should receive warning event within timeout")
            .expect("Should successfully receive warning event");

        assert_eq!(received.metadata.severity, Severity::Warn);
        assert_eq!(received.event.event_name(), "warning");
    }

    #[tokio::test]
    async fn test_multiple_subscribers() {
        let bus = Arc::new(TokioEventBus::with_default_capacity());

        let subscriber1 = Arc::new(TestSubscriber::new());
        let subscriber2 = Arc::new(TestSubscriber::new());

        let filter = EventFilter::new();

        // Start subscriber tasks
        let sub1_task = spawn_subscriber_task(bus.subscribe(filter.clone()), subscriber1.clone());

        let sub2_task = spawn_subscriber_task(bus.subscribe(filter), subscriber2.clone());

        // Publish some events
        for i in 0..5 {
            let event = Event::Core(CoreEvent::Info {
                message: format!("message {i}"),
            });
            bus.publish(event).await;
        }

        // Give subscribers time to process
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Both subscribers should have received all events
        assert_eq!(subscriber1.event_count(), 5);
        assert_eq!(subscriber2.event_count(), 5);

        // Clean up
        sub1_task.abort();
        sub2_task.abort();
    }

    #[tokio::test]
    async fn test_stats() {
        let bus = TokioEventBus::with_default_capacity();

        // Initially no events
        let stats = bus.stats().unwrap();
        assert_eq!(stats.events_published, 0);

        // Publish an event
        let event = Event::Core(CoreEvent::Info {
            message: "test".to_string(),
        });
        bus.publish(event).await;

        // Stats should be updated
        let stats = bus.stats().unwrap();
        assert_eq!(stats.events_published, 1);
        assert_eq!(stats.channel_capacity, 1024);
    }
}
