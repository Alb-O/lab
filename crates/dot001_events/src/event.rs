//! Domain event types and severity levels
//!
//! This module defines the strongly-typed event system for the dot001 toolkit,
//! organizing events by domain with optional key-value context for additional
//! ad-hoc information.

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::SystemTime;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::error::Error;

/// Severity levels for events, mapping to CLI verbosity flags
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Severity {
    /// Trace-level events (-vv flag)
    Trace,
    /// Debug-level events (-v flag)
    Debug,
    /// Info-level events (default)
    Info,
    /// Warning events (default)
    Warn,
    /// Error events (all verbosity levels)
    Error,
}

impl Severity {
    /// Check if this severity should be shown at the given minimum level
    pub fn should_show(&self, min_level: Severity) -> bool {
        *self >= min_level
    }
}

/// Key-value context for ad-hoc event metadata
pub type Kv = HashMap<String, String>;

/// Core domain events for fundamental operations
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum CoreEvent {
    /// Application or operation started
    Started {
        component: String,
        version: Option<String>,
    },

    /// Application or operation finished
    Finished {
        component: String,
        duration_ms: Option<u64>,
        success: bool,
    },

    /// Progress update
    Progress {
        operation: String,
        current: usize,
        total: Option<usize>,
        message: Option<String>,
    },

    /// Generic informational message
    Info { message: String },

    /// Warning that doesn't stop operation
    Warning { code: String, message: String },

    /// Error that stops operation
    Error { error: Error },
}

/// Parser domain events for blend file parsing
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum ParserEvent {
    /// Parser started processing a file
    Started {
        input: PathBuf,
        file_size: Option<u64>,
    },

    /// Successfully parsed file header
    HeaderParsed {
        version: String,
        endianness: String,
        pointer_size: u8,
    },

    /// DNA block found and parsed
    DnaParsed {
        struct_count: usize,
        name_count: usize,
    },

    /// Individual block parsed
    BlockParsed {
        index: usize,
        block_type: String,
        size: usize,
    },

    /// Parser warning (non-fatal)
    Warning { code: String, message: String },

    /// Parser error (fatal)
    Error { error: Error },

    /// Parser finished with statistics
    Finished {
        total_blocks: usize,
        total_size: u64,
        duration_ms: u64,
    },
}

/// Tracer domain events for dependency analysis
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum TracerEvent {
    /// Tracer started analyzing dependencies
    Started {
        root_blocks: Vec<String>,
        options: String, // Serialized options for context
    },

    /// Block expansion started
    BlockExpansionStarted {
        block_type: String,
        block_index: usize,
    },

    /// Block expansion completed
    BlockExpanded {
        block_type: String,
        block_index: usize,
        dependencies_found: usize,
    },

    /// Filter applied to trace results
    FilterApplied {
        filter_name: String,
        blocks_before: usize,
        blocks_after: usize,
    },

    /// Name resolution performed
    NameResolved {
        original_name: String,
        resolved_name: String,
        block_type: String,
    },

    /// Tracer warning
    Warning {
        code: String,
        message: String,
        block_index: Option<usize>,
    },

    /// Tracer error
    Error { error: Error },

    /// Tracer finished with statistics
    Finished {
        total_blocks_traced: usize,
        unique_dependencies: usize,
        duration_ms: u64,
    },
}

/// Diff domain events for file comparison
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum DiffEvent {
    /// Diff operation started
    Started {
        lhs: PathBuf,
        rhs: PathBuf,
        diff_type: String,
    },

    /// Policy decision made during diff
    PolicyApplied {
        policy: String,
        blocks_affected: usize,
    },

    /// Mismatch found between files
    Mismatch {
        path: String,
        detail: String,
        severity: String, // "minor", "major", "critical"
    },

    /// Blocks matched successfully
    BlocksMatched { block_type: String, count: usize },

    /// Diff warning
    Warning { code: String, message: String },

    /// Diff error
    Error { error: Error },

    /// Diff completed with summary
    Summary {
        matched_blocks: usize,
        mismatched_blocks: usize,
        total_blocks: usize,
        duration_ms: u64,
    },
}

/// Editor domain events for file editing operations
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum EditorEvent {
    /// Edit operation started
    Started {
        operation: String,
        target_file: PathBuf,
        block_count: Option<usize>,
    },

    /// Block edit operation
    BlockEdited {
        operation: String,
        block_index: usize,
        block_type: String,
        old_value: Option<String>,
        new_value: String,
    },

    /// Batch operation progress
    BatchProgress {
        operation: String,
        completed: usize,
        total: usize,
    },

    /// Edit validation
    ValidationPerformed {
        validator: String,
        passed: bool,
        message: Option<String>,
    },

    /// Editor warning
    Warning {
        code: String,
        message: String,
        block_index: Option<usize>,
    },

    /// Editor error
    Error { error: Error },

    /// Edit operation completed
    Finished {
        operation: String,
        blocks_modified: usize,
        duration_ms: u64,
        success: bool,
    },
}

/// Writer domain events for file writing operations
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum WriterEvent {
    /// Writer operation started
    Started {
        operation: String,
        target_file: PathBuf,
    },

    /// Header generation
    HeaderGenerated { version: String, block_count: usize },

    /// Block injection started
    BlockInjectionStarted { total_blocks: usize },

    /// Individual block written
    BlockWritten {
        block_type: String,
        block_index: usize,
        size: usize,
    },

    /// DNA block updated
    DnaUpdated {
        new_structs: usize,
        new_names: usize,
    },

    /// Write preview generated
    PreviewGenerated {
        preview_path: PathBuf,
        blocks_included: usize,
    },

    /// Writer warning
    Warning { code: String, message: String },

    /// Writer error
    Error { error: Error },

    /// Writer operation completed
    Finished {
        operation: String,
        bytes_written: u64,
        blocks_written: usize,
        duration_ms: u64,
        success: bool,
    },
}

/// Watcher domain events for filesystem monitoring
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum WatcherEvent {
    /// Watcher started monitoring
    Started {
        watch_paths: Vec<PathBuf>,
        recursive: bool,
    },

    /// File system event detected
    FileEvent {
        event_type: String, // "created", "modified", "deleted", "moved"
        path: PathBuf,
        old_path: Option<PathBuf>, // For move events
    },

    /// Directory event detected
    DirectoryEvent { event_type: String, path: PathBuf },

    /// Event processing started
    ProcessingStarted {
        trigger_path: PathBuf,
        action: String,
    },

    /// Event processing completed
    ProcessingCompleted {
        trigger_path: PathBuf,
        action: String,
        success: bool,
        duration_ms: u64,
    },

    /// Watcher warning
    Warning {
        code: String,
        message: String,
        path: Option<PathBuf>,
    },

    /// Watcher error
    Error { error: Error },

    /// Watcher stopped
    Stopped { reason: String },
}

/// CLI domain events for command-line interface operations
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum CliEvent {
    /// Command started
    CommandStarted { command: String, args: Vec<String> },

    /// Subcommand executed
    SubcommandExecuted {
        subcommand: String,
        success: bool,
        duration_ms: u64,
    },

    /// Output formatting
    OutputFormatted {
        format: String, // "pretty", "plain", "json"
        lines: usize,
    },

    /// User input requested
    InputRequested {
        prompt: String,
        input_type: String, // "confirmation", "text", "selection"
    },

    /// User input received
    InputReceived { input_type: String, value: String },

    /// CLI warning
    Warning { code: String, message: String },

    /// CLI error
    Error { error: Error },

    /// Command completed
    CommandCompleted {
        command: String,
        exit_code: i32,
        duration_ms: u64,
    },
}

/// Top-level event enum containing all domain events
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Event {
    Core(CoreEvent),
    Parser(ParserEvent),
    Tracer(TracerEvent),
    Diff(DiffEvent),
    Editor(EditorEvent),
    Writer(WriterEvent),
    Watcher(WatcherEvent),
    Cli(CliEvent),
}

/// Event metadata added by the event bus
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct EventMetadata {
    /// When the event was published
    pub timestamp: SystemTime,
    /// Event severity level
    pub severity: Severity,
    /// Optional key-value context
    pub context: Option<Kv>,
}

/// Complete event with metadata
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct EventWithMetadata {
    /// The event payload
    pub event: Event,
    /// Event metadata
    pub metadata: EventMetadata,
}

impl Event {
    /// Get the default severity for this event
    pub fn default_severity(&self) -> Severity {
        match self {
            Event::Core(core_event) => match core_event {
                CoreEvent::Started { .. } => Severity::Info,
                CoreEvent::Finished { success: true, .. } => Severity::Info,
                CoreEvent::Finished { success: false, .. } => Severity::Error,
                CoreEvent::Progress { .. } => Severity::Debug,
                CoreEvent::Info { .. } => Severity::Info,
                CoreEvent::Warning { .. } => Severity::Warn,
                CoreEvent::Error { .. } => Severity::Error,
            },
            Event::Parser(parser_event) => match parser_event {
                ParserEvent::Started { .. } => Severity::Info,
                ParserEvent::HeaderParsed { .. } => Severity::Debug,
                ParserEvent::DnaParsed { .. } => Severity::Debug,
                ParserEvent::BlockParsed { .. } => Severity::Trace,
                ParserEvent::Warning { .. } => Severity::Warn,
                ParserEvent::Error { .. } => Severity::Error,
                ParserEvent::Finished { .. } => Severity::Info,
            },
            Event::Tracer(tracer_event) => match tracer_event {
                TracerEvent::Started { .. } => Severity::Info,
                TracerEvent::BlockExpansionStarted { .. } => Severity::Debug,
                TracerEvent::BlockExpanded { .. } => Severity::Debug,
                TracerEvent::FilterApplied { .. } => Severity::Debug,
                TracerEvent::NameResolved { .. } => Severity::Trace,
                TracerEvent::Warning { .. } => Severity::Warn,
                TracerEvent::Error { .. } => Severity::Error,
                TracerEvent::Finished { .. } => Severity::Info,
            },
            Event::Diff(diff_event) => match diff_event {
                DiffEvent::Started { .. } => Severity::Info,
                DiffEvent::PolicyApplied { .. } => Severity::Debug,
                DiffEvent::Mismatch { severity, .. } => match severity.as_str() {
                    "critical" => Severity::Error,
                    "major" => Severity::Warn,
                    _ => Severity::Info,
                },
                DiffEvent::BlocksMatched { .. } => Severity::Debug,
                DiffEvent::Warning { .. } => Severity::Warn,
                DiffEvent::Error { .. } => Severity::Error,
                DiffEvent::Summary { .. } => Severity::Info,
            },
            Event::Editor(editor_event) => match editor_event {
                EditorEvent::Started { .. } => Severity::Info,
                EditorEvent::BlockEdited { .. } => Severity::Debug,
                EditorEvent::BatchProgress { .. } => Severity::Debug,
                EditorEvent::ValidationPerformed { passed: true, .. } => Severity::Debug,
                EditorEvent::ValidationPerformed { passed: false, .. } => Severity::Warn,
                EditorEvent::Warning { .. } => Severity::Warn,
                EditorEvent::Error { .. } => Severity::Error,
                EditorEvent::Finished { success: true, .. } => Severity::Info,
                EditorEvent::Finished { success: false, .. } => Severity::Error,
            },
            Event::Writer(writer_event) => match writer_event {
                WriterEvent::Started { .. } => Severity::Info,
                WriterEvent::HeaderGenerated { .. } => Severity::Debug,
                WriterEvent::BlockInjectionStarted { .. } => Severity::Debug,
                WriterEvent::BlockWritten { .. } => Severity::Trace,
                WriterEvent::DnaUpdated { .. } => Severity::Debug,
                WriterEvent::PreviewGenerated { .. } => Severity::Info,
                WriterEvent::Warning { .. } => Severity::Warn,
                WriterEvent::Error { .. } => Severity::Error,
                WriterEvent::Finished { success: true, .. } => Severity::Info,
                WriterEvent::Finished { success: false, .. } => Severity::Error,
            },
            Event::Watcher(watcher_event) => match watcher_event {
                WatcherEvent::Started { .. } => Severity::Info,
                WatcherEvent::FileEvent { .. } => Severity::Debug,
                WatcherEvent::DirectoryEvent { .. } => Severity::Debug,
                WatcherEvent::ProcessingStarted { .. } => Severity::Info,
                WatcherEvent::ProcessingCompleted { success: true, .. } => Severity::Info,
                WatcherEvent::ProcessingCompleted { success: false, .. } => Severity::Error,
                WatcherEvent::Warning { .. } => Severity::Warn,
                WatcherEvent::Error { .. } => Severity::Error,
                WatcherEvent::Stopped { .. } => Severity::Info,
            },
            Event::Cli(cli_event) => match cli_event {
                CliEvent::CommandStarted { .. } => Severity::Debug,
                CliEvent::SubcommandExecuted { success: true, .. } => Severity::Debug,
                CliEvent::SubcommandExecuted { success: false, .. } => Severity::Error,
                CliEvent::OutputFormatted { .. } => Severity::Trace,
                CliEvent::InputRequested { .. } => Severity::Info,
                CliEvent::InputReceived { .. } => Severity::Debug,
                CliEvent::Warning { .. } => Severity::Warn,
                CliEvent::Error { .. } => Severity::Error,
                CliEvent::CommandCompleted { exit_code: 0, .. } => Severity::Info,
                CliEvent::CommandCompleted { .. } => Severity::Error,
            },
        }
    }

    /// Get the domain name for this event
    pub fn domain(&self) -> &'static str {
        match self {
            Event::Core(_) => "core",
            Event::Parser(_) => "parser",
            Event::Tracer(_) => "tracer",
            Event::Diff(_) => "diff",
            Event::Editor(_) => "editor",
            Event::Writer(_) => "writer",
            Event::Watcher(_) => "watcher",
            Event::Cli(_) => "cli",
        }
    }

    /// Get a short name for this event type
    pub fn event_name(&self) -> &'static str {
        match self {
            Event::Core(e) => match e {
                CoreEvent::Started { .. } => "started",
                CoreEvent::Finished { .. } => "finished",
                CoreEvent::Progress { .. } => "progress",
                CoreEvent::Info { .. } => "info",
                CoreEvent::Warning { .. } => "warning",
                CoreEvent::Error { .. } => "error",
            },
            Event::Parser(e) => match e {
                ParserEvent::Started { .. } => "started",
                ParserEvent::HeaderParsed { .. } => "header_parsed",
                ParserEvent::DnaParsed { .. } => "dna_parsed",
                ParserEvent::BlockParsed { .. } => "block_parsed",
                ParserEvent::Warning { .. } => "warning",
                ParserEvent::Error { .. } => "error",
                ParserEvent::Finished { .. } => "finished",
            },
            Event::Tracer(e) => match e {
                TracerEvent::Started { .. } => "started",
                TracerEvent::BlockExpansionStarted { .. } => "block_expansion_started",
                TracerEvent::BlockExpanded { .. } => "block_expanded",
                TracerEvent::FilterApplied { .. } => "filter_applied",
                TracerEvent::NameResolved { .. } => "name_resolved",
                TracerEvent::Warning { .. } => "warning",
                TracerEvent::Error { .. } => "error",
                TracerEvent::Finished { .. } => "finished",
            },
            Event::Diff(e) => match e {
                DiffEvent::Started { .. } => "started",
                DiffEvent::PolicyApplied { .. } => "policy_applied",
                DiffEvent::Mismatch { .. } => "mismatch",
                DiffEvent::BlocksMatched { .. } => "blocks_matched",
                DiffEvent::Warning { .. } => "warning",
                DiffEvent::Error { .. } => "error",
                DiffEvent::Summary { .. } => "summary",
            },
            Event::Editor(e) => match e {
                EditorEvent::Started { .. } => "started",
                EditorEvent::BlockEdited { .. } => "block_edited",
                EditorEvent::BatchProgress { .. } => "batch_progress",
                EditorEvent::ValidationPerformed { .. } => "validation_performed",
                EditorEvent::Warning { .. } => "warning",
                EditorEvent::Error { .. } => "error",
                EditorEvent::Finished { .. } => "finished",
            },
            Event::Writer(e) => match e {
                WriterEvent::Started { .. } => "started",
                WriterEvent::HeaderGenerated { .. } => "header_generated",
                WriterEvent::BlockInjectionStarted { .. } => "block_injection_started",
                WriterEvent::BlockWritten { .. } => "block_written",
                WriterEvent::DnaUpdated { .. } => "dna_updated",
                WriterEvent::PreviewGenerated { .. } => "preview_generated",
                WriterEvent::Warning { .. } => "warning",
                WriterEvent::Error { .. } => "error",
                WriterEvent::Finished { .. } => "finished",
            },
            Event::Watcher(e) => match e {
                WatcherEvent::Started { .. } => "started",
                WatcherEvent::FileEvent { .. } => "file_event",
                WatcherEvent::DirectoryEvent { .. } => "directory_event",
                WatcherEvent::ProcessingStarted { .. } => "processing_started",
                WatcherEvent::ProcessingCompleted { .. } => "processing_completed",
                WatcherEvent::Warning { .. } => "warning",
                WatcherEvent::Error { .. } => "error",
                WatcherEvent::Stopped { .. } => "stopped",
            },
            Event::Cli(e) => match e {
                CliEvent::CommandStarted { .. } => "command_started",
                CliEvent::SubcommandExecuted { .. } => "subcommand_executed",
                CliEvent::OutputFormatted { .. } => "output_formatted",
                CliEvent::InputRequested { .. } => "input_requested",
                CliEvent::InputReceived { .. } => "input_received",
                CliEvent::Warning { .. } => "warning",
                CliEvent::Error { .. } => "error",
                CliEvent::CommandCompleted { .. } => "command_completed",
            },
        }
    }
}

impl EventWithMetadata {
    /// Create a new event with metadata
    pub fn new(event: Event, severity: Severity, context: Option<Kv>) -> Self {
        Self {
            event,
            metadata: EventMetadata {
                timestamp: SystemTime::now(),
                severity,
                context,
            },
        }
    }

    /// Create a new event with default severity
    pub fn with_default_severity(event: Event, context: Option<Kv>) -> Self {
        let severity = event.default_severity();
        Self::new(event, severity, context)
    }
}
