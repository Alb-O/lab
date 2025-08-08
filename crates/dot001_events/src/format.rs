//! Event formatting for human-readable output
//!
//! This module provides different formatters for events:
//! - PrettyFormatter: Colorized, human-friendly output (default)
//! - PlainFormatter: Single-line, no color output for piping
//! - JsonFormatter: JSON output for machine consumption (feature gated)

use chrono::{DateTime, Utc};
use colored::{ColoredString, Colorize};

use crate::event::{Event, EventWithMetadata, Kv, Severity};

/// Trait for formatting events into strings
pub trait Formatter: Send + Sync {
    /// Format an event into a string for display
    fn format(&self, event: &EventWithMetadata) -> String;
}

/// Pretty formatter with colors and structured output
#[derive(Debug, Clone)]
pub struct PrettyFormatter {
    /// Whether to include timestamps
    pub show_timestamps: bool,
    /// Whether to include domain tags
    pub show_domains: bool,
    /// Whether to show context key-value pairs
    pub show_context: bool,
}

impl Default for PrettyFormatter {
    fn default() -> Self {
        Self {
            show_timestamps: true,
            show_domains: true,
            show_context: true,
        }
    }
}

impl PrettyFormatter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_timestamps(mut self, show: bool) -> Self {
        self.show_timestamps = show;
        self
    }

    pub fn with_domains(mut self, show: bool) -> Self {
        self.show_domains = show;
        self
    }

    pub fn with_context(mut self, show: bool) -> Self {
        self.show_context = show;
        self
    }

    fn format_severity(&self, severity: Severity) -> ColoredString {
        match severity {
            Severity::Error => "ERROR".red().bold(),
            Severity::Warn => "WARN".yellow().bold(),
            Severity::Info => "INFO".cyan(),
            Severity::Debug => "DEBUG".blue(),
            Severity::Trace => "TRACE".bright_black(),
        }
    }

    fn format_domain(&self, domain: &str) -> ColoredString {
        match domain {
            "core" => domain.bright_green(),
            "parser" => domain.bright_blue(),
            "tracer" => domain.bright_magenta(),
            "diff" => domain.bright_yellow(),
            "editor" => domain.bright_cyan(),
            "writer" => domain.bright_red(),
            "watcher" => domain.green(),
            "cli" => domain.white(),
            _ => domain.normal(),
        }
    }

    fn format_timestamp(&self, timestamp: std::time::SystemTime) -> String {
        let datetime: DateTime<Utc> = timestamp.into();
        datetime
            .format("%H:%M:%S%.3f")
            .to_string()
            .bright_black()
            .to_string()
    }

    fn format_event_content(&self, event: &Event) -> String {
        match event {
            Event::Core(core_event) => match core_event {
                crate::event::CoreEvent::Started { component, version } => {
                    if let Some(v) = version {
                        format!("Started {} (v{})", component.bold(), v)
                    } else {
                        format!("Started {}", component.bold())
                    }
                }
                crate::event::CoreEvent::Finished {
                    component,
                    duration_ms,
                    success,
                } => {
                    let status = if *success { "✓".green() } else { "✗".red() };
                    if let Some(duration) = duration_ms {
                        format!("{} Finished {} ({} ms)", status, component.bold(), duration)
                    } else {
                        format!("{} Finished {}", status, component.bold())
                    }
                }
                crate::event::CoreEvent::Progress {
                    operation,
                    current,
                    total,
                    message,
                } => {
                    if let Some(total) = total {
                        let percentage = (*current as f64 / *total as f64) * 100.0;
                        let progress_msg = if let Some(msg) = message {
                            format!(" - {msg}")
                        } else {
                            String::new()
                        };
                        format!("{operation}: [{current}/{total}] {percentage:.1}%{progress_msg}")
                    } else {
                        let progress_msg = if let Some(msg) = message {
                            format!(" - {msg}")
                        } else {
                            String::new()
                        };
                        format!("{operation}: [{current}]{progress_msg}")
                    }
                }
                crate::event::CoreEvent::Info { message } => message.clone(),
                crate::event::CoreEvent::Warning { code, message } => {
                    format!("[{}] {}", code.yellow(), message)
                }
                crate::event::CoreEvent::Error { error } => {
                    format!("{}", error.to_string().red())
                }
            },
            Event::Parser(parser_event) => match parser_event {
                crate::event::ParserEvent::Started { input, file_size } => {
                    if let Some(size) = file_size {
                        format!(
                            "Parsing {} ({} bytes)",
                            input.display().to_string().bold(),
                            size
                        )
                    } else {
                        format!("Parsing {}", input.display().to_string().bold())
                    }
                }
                crate::event::ParserEvent::HeaderParsed {
                    version,
                    endianness,
                    pointer_size,
                } => {
                    format!(
                        "Header: v{} {} {}-bit",
                        version,
                        endianness,
                        pointer_size * 8
                    )
                }
                crate::event::ParserEvent::DnaParsed {
                    struct_count,
                    name_count,
                } => {
                    format!("DNA: {struct_count} structs, {name_count} names")
                }
                crate::event::ParserEvent::BlockParsed {
                    index,
                    block_type,
                    size,
                } => {
                    format!("Block {}: {} ({} bytes)", index, block_type.bold(), size)
                }
                crate::event::ParserEvent::Warning { code, message } => {
                    format!("[{}] {}", code.yellow(), message)
                }
                crate::event::ParserEvent::Error { error } => {
                    format!("{}", error.to_string().red())
                }
                crate::event::ParserEvent::Finished {
                    total_blocks,
                    total_size,
                    duration_ms,
                } => {
                    format!(
                        "✓ Parsed {} blocks ({} bytes) in {} ms",
                        total_blocks.to_string().bold(),
                        total_size,
                        duration_ms
                    )
                }
            },
            Event::Tracer(tracer_event) => match tracer_event {
                crate::event::TracerEvent::Started { root_blocks, .. } => {
                    format!("Tracing from {} root blocks", root_blocks.len())
                }
                crate::event::TracerEvent::BlockExpansionStarted {
                    block_type,
                    block_index,
                } => {
                    format!("Expanding {} #{}", block_type.bold(), block_index)
                }
                crate::event::TracerEvent::BlockExpanded {
                    block_type,
                    block_index,
                    dependencies_found,
                } => {
                    format!(
                        "✓ {} #{}: {} dependencies",
                        block_type.bold(),
                        block_index,
                        dependencies_found
                    )
                }
                crate::event::TracerEvent::FilterApplied {
                    filter_name,
                    blocks_before,
                    blocks_after,
                } => {
                    format!(
                        "Filter '{}': {} → {} blocks",
                        filter_name.bold(),
                        blocks_before,
                        blocks_after
                    )
                }
                crate::event::TracerEvent::NameResolved {
                    original_name,
                    resolved_name,
                    ..
                } => {
                    format!(
                        "Name resolved: {} → {}",
                        original_name,
                        resolved_name.bold()
                    )
                }
                crate::event::TracerEvent::Warning { code, message, .. } => {
                    format!("[{}] {}", code.yellow(), message)
                }
                crate::event::TracerEvent::Error { error } => {
                    format!("{}", error.to_string().red())
                }
                crate::event::TracerEvent::Finished {
                    total_blocks_traced,
                    unique_dependencies,
                    duration_ms,
                } => {
                    format!(
                        "✓ Traced {} blocks ({} unique) in {} ms",
                        total_blocks_traced.to_string().bold(),
                        unique_dependencies,
                        duration_ms
                    )
                }
            },
            Event::Diff(diff_event) => match diff_event {
                crate::event::DiffEvent::Started {
                    lhs,
                    rhs,
                    diff_type,
                } => {
                    format!(
                        "{} diff: {} vs {}",
                        diff_type.to_uppercase(),
                        lhs.display().to_string().bold(),
                        rhs.display().to_string().bold()
                    )
                }
                crate::event::DiffEvent::PolicyApplied {
                    policy,
                    blocks_affected,
                } => {
                    format!(
                        "Policy '{}' applied to {} blocks",
                        policy.bold(),
                        blocks_affected
                    )
                }
                crate::event::DiffEvent::Mismatch {
                    path,
                    detail,
                    severity,
                } => {
                    let severity_color = match severity.as_str() {
                        "critical" => detail.red(),
                        "major" => detail.yellow(),
                        _ => detail.normal(),
                    };
                    format!("Mismatch at {}: {}", path.bold(), severity_color)
                }
                crate::event::DiffEvent::BlocksMatched { block_type, count } => {
                    format!("✓ {count} {block_type} blocks matched")
                }
                crate::event::DiffEvent::Warning { code, message } => {
                    format!("[{}] {}", code.yellow(), message)
                }
                crate::event::DiffEvent::Error { error } => {
                    format!("{}", error.to_string().red())
                }
                crate::event::DiffEvent::Summary {
                    matched_blocks,
                    mismatched_blocks,
                    total_blocks,
                    duration_ms,
                } => {
                    let status = if *mismatched_blocks == 0 {
                        "✓".green()
                    } else {
                        "!".yellow()
                    };
                    format!(
                        "{} {} matched, {} mismatched of {} total ({} ms)",
                        status,
                        matched_blocks.to_string().green(),
                        mismatched_blocks.to_string().red(),
                        total_blocks,
                        duration_ms
                    )
                }
            },
            Event::Editor(editor_event) => match editor_event {
                crate::event::EditorEvent::Started {
                    operation,
                    target_file,
                    block_count,
                } => {
                    if let Some(count) = block_count {
                        format!(
                            "{} on {} ({} blocks)",
                            operation.bold(),
                            target_file.display().to_string().bold(),
                            count
                        )
                    } else {
                        format!(
                            "{} on {}",
                            operation.bold(),
                            target_file.display().to_string().bold()
                        )
                    }
                }
                crate::event::EditorEvent::BlockEdited {
                    operation,
                    block_index,
                    block_type,
                    old_value,
                    new_value,
                } => {
                    if let Some(old) = old_value {
                        format!(
                            "{} {} #{}: {} → {}",
                            operation,
                            block_type.bold(),
                            block_index,
                            old.red().strikethrough(),
                            new_value.green()
                        )
                    } else {
                        format!(
                            "{} {} #{}: {}",
                            operation,
                            block_type.bold(),
                            block_index,
                            new_value.green()
                        )
                    }
                }
                crate::event::EditorEvent::BatchProgress {
                    operation,
                    completed,
                    total,
                } => {
                    let percentage = (*completed as f64 / *total as f64) * 100.0;
                    format!("{operation}: [{completed}/{total}] {percentage:.1}%")
                }
                crate::event::EditorEvent::ValidationPerformed {
                    validator,
                    passed,
                    message,
                } => {
                    let status = if *passed { "✓".green() } else { "✗".red() };
                    let msg_part = if let Some(msg) = message {
                        format!(": {msg}")
                    } else {
                        String::new()
                    };
                    format!("{} Validation '{}'{}", status, validator.bold(), msg_part)
                }
                crate::event::EditorEvent::Warning { code, message, .. } => {
                    format!("[{}] {}", code.yellow(), message)
                }
                crate::event::EditorEvent::Error { error } => {
                    format!("{}", error.to_string().red())
                }
                crate::event::EditorEvent::Finished {
                    operation,
                    blocks_modified,
                    duration_ms,
                    success,
                } => {
                    let status = if *success { "✓".green() } else { "✗".red() };
                    format!(
                        "{} {} completed: {} blocks in {} ms",
                        status,
                        operation.bold(),
                        blocks_modified,
                        duration_ms
                    )
                }
            },
            Event::Writer(writer_event) => match writer_event {
                crate::event::WriterEvent::Started {
                    operation,
                    target_file,
                } => {
                    format!(
                        "{} to {}",
                        operation.bold(),
                        target_file.display().to_string().bold()
                    )
                }
                crate::event::WriterEvent::HeaderGenerated {
                    version,
                    block_count,
                } => {
                    format!("Header: v{version} with {block_count} blocks")
                }
                crate::event::WriterEvent::BlockInjectionStarted { total_blocks } => {
                    format!("Injecting {total_blocks} blocks")
                }
                crate::event::WriterEvent::BlockWritten {
                    block_type,
                    block_index,
                    size,
                } => {
                    format!(
                        "Block #{}: {} ({} bytes)",
                        block_index,
                        block_type.bold(),
                        size
                    )
                }
                crate::event::WriterEvent::DnaUpdated {
                    new_structs,
                    new_names,
                } => {
                    format!("DNA updated: +{new_structs} structs, +{new_names} names")
                }
                crate::event::WriterEvent::PreviewGenerated {
                    preview_path,
                    blocks_included,
                } => {
                    format!(
                        "Preview: {} ({} blocks)",
                        preview_path.display().to_string().bold(),
                        blocks_included
                    )
                }
                crate::event::WriterEvent::Warning { code, message } => {
                    format!("[{}] {}", code.yellow(), message)
                }
                crate::event::WriterEvent::Error { error } => {
                    format!("{}", error.to_string().red())
                }
                crate::event::WriterEvent::Finished {
                    operation,
                    bytes_written,
                    blocks_written,
                    duration_ms,
                    success,
                } => {
                    let status = if *success { "✓".green() } else { "✗".red() };
                    format!(
                        "{} {} completed: {} blocks ({} bytes) in {} ms",
                        status,
                        operation.bold(),
                        blocks_written,
                        bytes_written,
                        duration_ms
                    )
                }
            },
            Event::Watcher(watcher_event) => match watcher_event {
                crate::event::WatcherEvent::Started {
                    watch_paths,
                    recursive,
                } => {
                    let recursive_str = if *recursive { " (recursive)" } else { "" };
                    format!("Watching {} paths{}", watch_paths.len(), recursive_str)
                }
                crate::event::WatcherEvent::FileEvent {
                    event_type,
                    path,
                    old_path,
                } => {
                    if let Some(old) = old_path {
                        format!(
                            "{}: {} → {}",
                            event_type.to_uppercase().bold(),
                            old.display(),
                            path.display().to_string().bold()
                        )
                    } else {
                        format!(
                            "{}: {}",
                            event_type.to_uppercase().bold(),
                            path.display().to_string().bold()
                        )
                    }
                }
                crate::event::WatcherEvent::DirectoryEvent { event_type, path } => {
                    format!(
                        "{}: {}/",
                        event_type.to_uppercase().bold(),
                        path.display().to_string().bold()
                    )
                }
                crate::event::WatcherEvent::ProcessingStarted {
                    trigger_path,
                    action,
                } => {
                    format!(
                        "Processing {} → {}",
                        trigger_path.display().to_string().bold(),
                        action
                    )
                }
                crate::event::WatcherEvent::ProcessingCompleted {
                    trigger_path,
                    action,
                    success,
                    duration_ms,
                } => {
                    let status = if *success { "✓".green() } else { "✗".red() };
                    format!(
                        "{} {} → {} completed in {} ms",
                        status,
                        trigger_path.display().to_string().bold(),
                        action,
                        duration_ms
                    )
                }
                crate::event::WatcherEvent::Warning { code, message, .. } => {
                    format!("[{}] {}", code.yellow(), message)
                }
                crate::event::WatcherEvent::Error { error } => {
                    format!("{}", error.to_string().red())
                }
                crate::event::WatcherEvent::Stopped { reason } => {
                    format!("Stopped: {reason}")
                }
            },
            Event::Cli(cli_event) => match cli_event {
                crate::event::CliEvent::CommandStarted { command, args } => {
                    if args.is_empty() {
                        format!("$ {}", command.bold())
                    } else {
                        format!("$ {} {}", command.bold(), args.join(" "))
                    }
                }
                crate::event::CliEvent::SubcommandExecuted {
                    subcommand,
                    success,
                    duration_ms,
                } => {
                    let status = if *success { "✓".green() } else { "✗".red() };
                    format!("{} {} ({} ms)", status, subcommand.bold(), duration_ms)
                }
                crate::event::CliEvent::OutputFormatted { format, lines } => {
                    format!("Output formatted as {} ({} lines)", format.bold(), lines)
                }
                crate::event::CliEvent::InputRequested { prompt, input_type } => {
                    format!("Input requested ({input_type}): {prompt}")
                }
                crate::event::CliEvent::InputReceived { input_type, value } => {
                    format!("Input received ({}): {}", input_type, value.bold())
                }
                crate::event::CliEvent::Warning { code, message } => {
                    format!("[{}] {}", code.yellow(), message)
                }
                crate::event::CliEvent::Error { error } => {
                    format!("{}", error.to_string().red())
                }
                crate::event::CliEvent::CommandCompleted {
                    command,
                    exit_code,
                    duration_ms,
                } => {
                    let status = if *exit_code == 0 {
                        "✓".green()
                    } else {
                        "✗".red()
                    };
                    format!(
                        "{} {} exited {} in {} ms",
                        status,
                        command.bold(),
                        exit_code,
                        duration_ms
                    )
                }
            },
        }
    }

    fn format_context(&self, context: &Option<Kv>) -> String {
        if let Some(kv) = context {
            if kv.is_empty() {
                return String::new();
            }
            let pairs: Vec<String> = kv
                .iter()
                .map(|(k, v)| format!("{}={}", k.bright_black(), v))
                .collect();
            format!(" {{{}}}", pairs.join(", "))
                .bright_black()
                .to_string()
        } else {
            String::new()
        }
    }
}

impl Formatter for PrettyFormatter {
    fn format(&self, event: &EventWithMetadata) -> String {
        let mut parts = Vec::new();

        // Timestamp
        if self.show_timestamps {
            parts.push(self.format_timestamp(event.metadata.timestamp));
        }

        // Severity
        parts.push(self.format_severity(event.metadata.severity).to_string());

        // Domain
        if self.show_domains {
            parts.push(format!("[{}]", self.format_domain(event.event.domain())));
        }

        // Event content
        parts.push(self.format_event_content(&event.event));

        // Context
        if self.show_context {
            let context_str = self.format_context(&event.metadata.context);
            if !context_str.is_empty() {
                parts.push(context_str);
            }
        }

        parts.join(" ")
    }
}

/// Plain formatter for simple, non-colored output
#[derive(Debug, Clone, Default)]
pub struct PlainFormatter {
    /// Whether to include timestamps
    pub show_timestamps: bool,
    /// Whether to include domain tags
    pub show_domains: bool,
}

impl PlainFormatter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_timestamps(mut self, show: bool) -> Self {
        self.show_timestamps = show;
        self
    }

    pub fn with_domains(mut self, show: bool) -> Self {
        self.show_domains = show;
        self
    }

    fn format_severity(&self, severity: Severity) -> &'static str {
        match severity {
            Severity::Error => "ERROR",
            Severity::Warn => "WARN",
            Severity::Info => "INFO",
            Severity::Debug => "DEBUG",
            Severity::Trace => "TRACE",
        }
    }

    fn format_timestamp(&self, timestamp: std::time::SystemTime) -> String {
        let datetime: DateTime<Utc> = timestamp.into();
        datetime.format("%H:%M:%S%.3f").to_string()
    }

    fn format_event_simple(&self, event: &Event) -> String {
        // Simplified formatting without colors
        format!("{}: {}", event.domain(), event.event_name())
    }
}

impl Formatter for PlainFormatter {
    fn format(&self, event: &EventWithMetadata) -> String {
        let mut parts = Vec::new();

        // Timestamp
        if self.show_timestamps {
            parts.push(self.format_timestamp(event.metadata.timestamp));
        }

        // Severity
        parts.push(self.format_severity(event.metadata.severity).to_string());

        // Domain
        if self.show_domains {
            parts.push(format!("[{}]", event.event.domain()));
        }

        // Simple event description
        parts.push(self.format_event_simple(&event.event));

        parts.join(" ")
    }
}

/// JSON formatter for machine consumption (requires 'json' feature)
#[cfg(feature = "json")]
#[derive(Debug, Clone, Default)]
pub struct JsonFormatter {
    /// Whether to pretty-print JSON
    pub pretty: bool,
}

#[cfg(feature = "json")]
impl JsonFormatter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn pretty(mut self, pretty: bool) -> Self {
        self.pretty = pretty;
        self
    }
}

#[cfg(feature = "json")]
impl Formatter for JsonFormatter {
    fn format(&self, event: &EventWithMetadata) -> String {
        let mut json_obj = serde_json::Map::new();

        // Metadata
        json_obj.insert(
            "timestamp".to_string(),
            serde_json::Value::String(
                chrono::DateTime::<Utc>::from(event.metadata.timestamp).to_rfc3339(),
            ),
        );
        json_obj.insert(
            "severity".to_string(),
            serde_json::Value::String(
                match event.metadata.severity {
                    Severity::Trace => "trace",
                    Severity::Debug => "debug",
                    Severity::Info => "info",
                    Severity::Warn => "warn",
                    Severity::Error => "error",
                }
                .to_string(),
            ),
        );
        json_obj.insert(
            "domain".to_string(),
            serde_json::Value::String(event.event.domain().to_string()),
        );
        json_obj.insert(
            "event_name".to_string(),
            serde_json::Value::String(event.event.event_name().to_string()),
        );

        // Context
        if let Some(ref context) = event.metadata.context {
            let context_obj: serde_json::Map<String, serde_json::Value> = context
                .iter()
                .map(|(k, v)| (k.clone(), serde_json::Value::String(v.clone())))
                .collect();
            json_obj.insert(
                "context".to_string(),
                serde_json::Value::Object(context_obj),
            );
        }

        // Event data (simplified for now - could be expanded to full serialization)
        let event_data = match serde_json::to_value(&event.event) {
            Ok(value) => value,
            Err(_) => serde_json::Value::String(format!("{:?}", event.event)),
        };
        json_obj.insert("data".to_string(), event_data);

        let json_value = serde_json::Value::Object(json_obj);

        if self.pretty {
            serde_json::to_string_pretty(&json_value).unwrap_or_else(|_| "{}".to_string())
        } else {
            serde_json::to_string(&json_value).unwrap_or_else(|_| "{}".to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{CoreEvent, Event, Severity};

    fn strip_ansi(input: &str) -> String {
        let mut out = String::with_capacity(input.len());
        let bytes = input.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == 0x1B && i + 1 < bytes.len() && bytes[i + 1] == b'[' {
                // Skip until 'm' or end
                i += 2;
                while i < bytes.len() && bytes[i] != b'm' {
                    i += 1;
                }
                // Skip the 'm' if present
                if i < bytes.len() {
                    i += 1;
                }
            } else {
                out.push(bytes[i] as char);
                i += 1;
            }
        }
        out
    }

    #[test]
    fn test_pretty_formatter() {
        let formatter = PrettyFormatter::new();
        let event = EventWithMetadata::new(
            Event::Core(CoreEvent::Info {
                message: "Test message".to_string(),
            }),
            Severity::Info,
            None,
        );

        let formatted = formatter.format(&event);
        let formatted_clean = strip_ansi(&formatted);
        assert!(formatted_clean.contains("INFO"));
        assert!(formatted_clean.contains("[core]"));
        assert!(formatted_clean.contains("Test message"));
    }

    #[test]
    fn test_plain_formatter() {
        let formatter = PlainFormatter::new().with_domains(true);
        let event = EventWithMetadata::new(
            Event::Core(CoreEvent::Warning {
                code: "TEST_WARN".to_string(),
                message: "Test warning".to_string(),
            }),
            Severity::Warn,
            None,
        );

        let formatted = formatter.format(&event);
        assert!(formatted.contains("WARN"));
        assert!(formatted.contains("[core]"));
        assert!(formatted.contains("core: warning"));
    }

    #[cfg(feature = "json")]
    #[test]
    fn test_json_formatter() {
        let formatter = JsonFormatter::new();
        let event = EventWithMetadata::new(
            Event::Core(CoreEvent::Error {
                error: crate::error::Error::io("Test error"),
            }),
            Severity::Error,
            None,
        );

        let formatted = formatter.format(&event);
        let parsed: serde_json::Value = serde_json::from_str(&formatted).unwrap();

        assert_eq!(parsed["severity"], "error");
        assert_eq!(parsed["domain"], "core");
        assert_eq!(parsed["event_name"], "error");
    }
}
