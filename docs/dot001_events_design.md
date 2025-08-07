# dot001_events: Async Event and Error Hub (Tokio-Only)

Status: Design approved
Owner: dot001 maintainers
Last updated: 2025-08-07

## Overview

We are replacing the dot001_error crate with dot001_events, a centralized, async-first event and error hub for the entire workspace. This crate will:

- Provide a unified error taxonomy (ported from dot001_error) and Result aliasing.
- Define domain event types across all crates.
- Expose an async EventBus abstraction built on Tokio (async-only).
- Offer subscription and formatting layers — the CLI becomes a primary subscriber that renders human-friendly outputs.
- Serve as the future integration point for dot001_watcher and automated workflows.

This is a breaking change across the workspace by design: adopting Tokio as the only concurrency model simplifies APIs, usage patterns, and future integrations.

## Goals

- Single, consistent async event pipeline using Tokio.
- Strongly-typed, domain-oriented events with optional ad-hoc KV context.
- CLI-centric formatting with pretty output by default, plus plain and JSON modes.
- Migration path: re-export shim in dot001_error for one release cycle, then removal.
- Ergonomic but explicit: consumers receive Arc<dyn EventBus> where practical; a global accessor is available for ergonomics where needed.

## Non-Goals

- No sync bus implementation.
- No non-Tokio runtime support in v1 (can be reconsidered later).
- Not a generic logging framework replacement; it complements crates like tracing but focuses on domain events and CLI UX.

---

## Crate Layout

Directory: crates/dot001_events

- src/lib.rs: top-level features, re-exports, prelude
- src/error.rs: ported error taxonomy and Result alias
- src/event.rs: domain event enums, Severity, KV context, metadata
- src/bus.rs: traits: EventBus, Subscriber, Subscription, EventFilter
- src/bus_impl/async_tokio.rs: Tokio-based event bus and subscription implementation
- src/format.rs: Formatter trait; Pretty, Plain; JSON (serde) formatter
- src/macros.rs: emit! and helpers for ergonomic publishing
- features:
  - json: enable serde-based formatter
  - tracing: mirror events to tracing::event for external observability

### Public API Sketch

- dot001_events::prelude
  - Result, Error, ErrorKind
  - Event, Severity
  - EventBus, Subscriber, EventFilter
  - emit!
- dot001_events::error::{Error, ErrorKind, Result, ContextExt}
- dot001_events::event::{Event, CoreEvent, ParserEvent, TracerEvent, DiffEvent, EditorEvent, WriterEvent, WatcherEvent, CliEvent, Severity, Kv}
- dot001_events::bus::{EventBus, Subscriber, EventFilter, Subscription}
- dot001_events::format::{Formatter, PrettyFormatter, PlainFormatter, JsonFormatter}
- dot001_events::bus_impl::async_tokio::{TokioEventBus}

---

## Events

### Domain Structure

- Event::Core(CoreEvent)
- Event::Parser(ParserEvent)
- Event::Tracer(TracerEvent)
- Event::Diff(DiffEvent)
- Event::Editor(EditorEvent)
- Event::Writer(WriterEvent)
- Event::Watcher(WatcherEvent)
- Event::Cli(CliEvent)

Each domain enum contains focused, strongly-typed variants for key lifecycle moments, metrics, and diagnostics. Examples (illustrative, not exhaustive):

- ParserEvent
  - Started { input: PathBuf }
  - BlockParsed { id: u64, kind: String }
  - Warning { code: static str, message: String, ctx: Option<Kv> }
  - Error { error: Error }
  - Finished { stats: ParserStats }

- DiffEvent
  - Started { lhs: PathBuf, rhs: PathBuf }
  - Mismatch { path: String, detail: String }
  - Summary { matched: usize, mismatched: usize }
  - Error { error: Error }

Common metadata:

- Severity: Trace | Debug | Info | Warn | Error
- Timestamping: provided by the bus when publishing.
- KV context bag (Kv): lightweight key-value for ad-hoc context without breaking the typed payload.

### Severity and CLI Verbosity Mapping

- -q: Warn and Error
- default: Info, Warn, Error
- -v: Debug, Info, Warn, Error
- -vv: Trace, Debug, Info, Warn, Error

---

## Errors

dot001_events::error will port dot001_error’s taxonomy:

- Error, ErrorKind, Result<T>, ContextExt for context (e.g., .context("reading header"))
- From conversions and Display semantics maintained to minimize churn
- One release-cycle re-export shim from dot001_error to dot001_events::error with deprecation notes

---

## Event Bus: Tokio-Only

### Rationale

- Unified async runtime simplifies the entire stack.
- dot001_watcher and future orchestration are inherently async.
- Reduced combinatorial complexity and features.

### API

- trait EventBus: Send + Sync
  - async fn publish(&self, event: Event)
  - fn subscribe(&self, filter: EventFilter) -> Subscription
  - Optional: fn channel_sizes(&self) -> BusStats

- struct Subscription (Drop detaches)
  - Receiver or stream-like interface for consuming events
  - spawn handler tasks for Subscribers

- trait Subscriber
  - async fn on_event(&self, event: &Event)

- struct EventFilter
  - by severity minimum
  - by domain allowlist
  - optional predicate: Fn(&Event) -> bool + Send + Sync + 'static

### Implementation Notes

- TokioEventBus
  - Internals: tokio::sync::broadcast for fan-out plus optional mpsc for heavy handlers
  - Backpressure: for slow subscribers, allow bounded channels; drop strategy or backpressure signals configurable
  - Publish path timestamps events and runs optional tracing mirror when feature enabled
  - Subscribers register with a filter; dispatcher enforces filter before sending

### Global Accessor (Hybrid)

- once_cell::sync::OnceCell<Arc<dyn EventBus>> optional global
- get_bus() panics if not initialized (CLI must init)
- Prefer explicit Arc<dyn EventBus> passing across APIs; global exists for ergonomics and legacy integration

---

## Formatting

- trait Formatter
  - fn format(&self, event: &Event) -> String

- PrettyFormatter (default)
  - Colorized severity and compact structured fields
  - Domain tags, timestamps, indentation for multi-line payloads

- PlainFormatter
  - Single-line, no color, stable for piping

- JsonFormatter (feature json)
  - serde_json::Value encoding
  - Suitable for log ingestion and testing snapshots

CLI chooses formatter via flags; pretty is default.

---

## CLI Integration

- dot001_cli main:
  - Initialize Tokio runtime (if not already active)
  - Create Arc<TokioEventBus> and set global accessor
  - Determine verbosity and formatter from args
  - Register a CLI subscriber task:
    - Consumes events using chosen filter
    - Formats and writes to stdout/stderr
  - Commands receive Arc<dyn EventBus> and emit events

- emit! macro ergonomics:
  - emit!(bus, Event::Parser(ParserEvent::Started { input }));
  - Optionally, emit_global!(Event::...) if global accessor is initialized

---

## Cross-Crate Integration

- dot001_parser: emits start, per-block, warnings, errors, finish; uses dot001_events::error::Result
- dot001_tracer: emits expansion steps, filters applied, stats
- dot001_diff: emits policy decisions, mismatches, summaries
- dot001_editor, dot001_writer: emits operations, previews, writes
- dot001_watcher: emits filesystem events; triggers orchestrations via subscribers
- dot001_checkpoint: emits save/load events

All human-facing output routes through CLI subscriber formatting.

---

## Migration Plan

1) Create dot001_events with modules: error, event, bus, bus_impl/async_tokio.rs, format, macros
2) Port dot001_error types into dot001_events::error (keep APIs)
3) Make dot001_error re-export from dot001_events::error and mark deprecated
4) Define domain Event enums, Severity, and EventFilter
5) Implement TokioEventBus with subscribe, publish, filters, timestamping
6) Implement Formatter trait and Pretty (default), Plain, JSON (feature)
7) Wire dot001_cli: initialize bus, register subscriber, add flags for format and verbosity
8) Migrate dot001_parser to emit events and use new Result
9) Migrate dot001_diff, dot001_tracer, dot001_editor, dot001_writer sequentially
10) Integrate dot001_watcher events and orchestration
11) Add unit tests for bus, filters, formatters, error conversions
12) Add integration tests capturing event sequences across parser/diff flows
13) Remove dot001_error after all crates compile against dot001_events

---

## Risks and Mitigations

- Breaking Change Scope
  - Risk: Wide impact across crates due to async-only switch.
  - Mitigation: Stage migrations by crate; provide a temporary re-export shim; keep error APIs stable.

- Tokio Runtime Contention
  - Risk: Multiple runtimes or blocking operations inside async handlers.
  - Mitigation: Single runtime created by CLI; avoid blocking in subscribers; use spawn_blocking for heavy CPU tasks.

- Backpressure and Event Loss
  - Risk: Slow subscribers causing buffer overflow.
  - Mitigation: Bounded channels with strategy options: drop oldest, drop newest, or block. Default to dropping oldest with metrics.

- Subscriber Panics
  - Risk: Handler panics terminate tasks and reduce observability.
  - Mitigation: Guard subscriber handlers; catch_unwind where necessary; emit an internal error event on subscriber failure.

- Formatting Overhead
  - Risk: Pretty/JSON formatting may be expensive on hot paths.
  - Mitigation: Lazy formatting; filter early by severity/domain to avoid unnecessary formatting; optional JSON only when requested.

- Global Accessor Abuse
  - Risk: Hidden coupling and testing complexity.
  - Mitigation: Document explicit-bus as preferred; keep global for CLI ergonomics; provide test utilities to inject local buses.

- Feature Creep into Logger Territory
  - Risk: Overlap with tracing/logging usage.
  - Mitigation: Keep domain events distinct and business-focused; optional tracing bridge for correlation.

---

## Open Questions

- Event Schema Stability: which event payloads are considered stable for tooling?
- JSON Schema Versioning: do we need a versioned schema for machine consumption?
- Orchestrator Placement: should orchestration be a separate crate or live in CLI?

---

## Mermaid Diagrams

Note: Avoiding quotes inside brackets to prevent parsing errors.

### High-level Event Flow

flowchart TD
  CLI[dot001_cli main] -->|Arc EventBus| Commands[cli commands]
  Commands --> Parser[dot001_parser]
  Commands --> Tracer[dot001_tracer]
  Commands --> Diff[dot001_diff]
  Commands --> Editor[dot001_editor]
  Commands --> Writer[dot001_writer]
  Watcher[dot001_watcher] -->|publish WatcherEvent| EventBus[dot001_events Tokio bus]
  Parser -->|publish ParserEvent| EventBus
  Tracer -->|publish TracerEvent| EventBus
  Diff -->|publish DiffEvent| EventBus
  Editor -->|publish EditorEvent| EventBus
  Writer -->|publish WriterEvent| EventBus
  CLI_Sub[cli subscriber with formatter] -->|subscribe| EventBus
  EventBus -->|deliver formatted output| CLI_Sub
  Checkpoint[dot001_checkpoint] -->|publish CoreEvent| EventBus

### Tokio Bus Internals

flowchart LR
  Pub[publish] --> TS[Timestamp]
  TS --> Filter[Apply filters]
  Filter --> Fan[Fan out via broadcast]
  Fan --> Sub1[subscriber 1]
  Fan --> Sub2[subscriber 2]
  Fan --> SubN[subscriber N]
  Sub1 --> H1[handler task]
  Sub2 --> H2[handler task]
  SubN --> HN[handler task]

---

## Example API Sketch

- Example emit from parser:

```rust
// emit!(...) macro usage
emit!(bus, Event::Parser(ParserEvent::Started { input }));
emit!(bus, Event::Parser(ParserEvent::BlockParsed { id, kind }));
emit!(bus, Event::Parser(ParserEvent::Finished { stats }));
```

- CLI subscriber loop (conceptual):

```rust
let sub = bus.subscribe(EventFilter::new().min_severity(min).domains(domains));
tokio::spawn(async move {
    while let Some(event) = sub.next().await {
        if let Some(line) = formatter.format(&event).into() {
            println!("{}", line);
        }
    }
});
```

- Error usage:

```rust
use dot001_events::error::{Result, ContextExt};

fn parse_block(...) -> Result<Block> {
    let data = read(...) .context("reading block bytes")?;
    decode(data).context("decoding block")
}
```

---

## Implementation Checklist

- ✅ Crate scaffold with modules and public API
- ✅ Error taxonomy port; shim in dot001_error
- ✅ Event enums per domain; Severity; KV context
- ✅ TokioEventBus; filters; timestamp; backpressure policy
- ✅ Formatters: Pretty (default), Plain, JSON (feature)
- ✅ Emit! macro for ergonomic event publishing
- ✅ CLI wiring; flags for formatter and verbosity
- ✅ Parser migration with events and sync macros
- ✅ Diff migration with events and comprehensive policy tracking  
- ✅ Tracer migration with comprehensive tracing events including filters and block expansion
- ✅ Editor migration with validation, field modification, and operation tracking
- ✅ Writer migration with template generation, block injection, and dependency tracing
- ✅ All core crate migrations completed successfully
- ✅ Remove dot001_error crate and update all references throughout workspace
- ⏳ Unit and integration tests for bus, filters, formatters, error conversions
- ⏳ Integration tests capturing event sequences across parser/diff flows

---

## Appendix: Configuration Surface

- CLI flags
  - --format [pretty|plain|json]
  - --verbosity [-q | none | -v | -vv]
  - --events [domains comma-separated] future
- Environment
  - DOT001_FORMAT, DOT001_VERBOSITY future
