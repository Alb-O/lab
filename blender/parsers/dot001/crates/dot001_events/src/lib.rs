//! # dot001_events - Async Event and Error Hub
//!
//! This crate provides a unified, async-first event and error system for the dot001 toolkit:
//!
//! - Centralized error taxonomy (ported from dot001_error) and Result aliasing
//! - Domain event types across all crates
//! - Async EventBus abstraction built on Tokio
//! - Subscription and formatting layers for CLI rendering
//! - Integration point for dot001_watcher and automated workflows
//!
//! ## Design Principles
//!
//! - **Tokio-Only**: Single async runtime simplifies the entire stack
//! - **Domain-Oriented**: Strongly-typed events with optional ad-hoc KV context
//! - **CLI-Centric**: Pretty output by default, plus plain and JSON modes
//! - **Ergonomic**: Consumers receive Arc<dyn EventBus>; global accessor available

pub mod bus;
pub mod bus_impl;
pub mod error;
pub mod event;
pub mod format;
pub mod macros;

// Re-export commonly used types
pub use bus::{EventBus, EventFilter, Subscriber, Subscription};
pub use error::{ContextExt, Error, ErrorKind, Result};
pub use event::{Event, Severity};

/// Prelude module for convenient imports
pub mod prelude {
    pub use crate::bus::{EventBus, EventFilter, Subscriber, Subscription};
    pub use crate::error::{ContextExt, Error, ErrorKind, Result};
    pub use crate::event::{Event, Severity};
    pub use crate::{
        context, emit, emit_error, emit_global, emit_global_sync, emit_info, emit_progress,
        emit_sync, emit_warning, try_emit_global,
    };
}
