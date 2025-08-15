use crate::OutputFormat;
use crate::util::CommandContext;
use dot001_events::error::Error;
use log::error;
use text_trees::{FormatCharacters, StringTreeNode, TreeFormatting};

/// Utility functions for consistent output formatting across commands
pub struct OutputUtils;

impl OutputUtils {
    /// Print a JSON string directly (for already-serialized JSON)
    pub fn print_json_str(json: &str, ctx: &CommandContext) {
        ctx.output.print_result(json);
    }

    /// Try to serialize data to JSON and print it, with error handling
    pub fn try_print_json<T, F>(data: &T, ctx: &CommandContext, type_name: &str, serializer: F)
    where
        F: FnOnce(&T) -> Result<String, serde_json::Error>,
    {
        match serializer(data) {
            Ok(json) => ctx.output.print_result(&json),
            Err(e) => error!("Failed to serialize {type_name} to JSON: {e}"),
        }
    }

    /// Print a formatted header for command output
    pub fn print_header(ctx: &CommandContext, title: &str) {
        ctx.output.print_info(title);
    }

    /// Print a formatted summary with key-value pairs
    pub fn print_summary(ctx: &CommandContext, items: &[(&str, String)]) {
        for (key, value) in items {
            ctx.output
                .print_result_fmt(format_args!("  {key}: {value}"));
        }
    }

    /// Print a list of items with consistent indentation
    pub fn print_list<T: std::fmt::Display>(ctx: &CommandContext, items: &[T]) {
        for item in items {
            ctx.output.print_result_fmt(format_args!("  {item}"));
        }
    }
}

/// Tree formatter for consistent tree output across commands
pub struct TreeFormatter {
    ascii: bool,
}

impl TreeFormatter {
    pub fn new(ascii: bool) -> Self {
        Self { ascii }
    }

    /// Format a StringTreeNode into a string with consistent styling
    pub fn format_tree(&self, tree: &StringTreeNode) -> Result<String, Box<dyn std::error::Error>> {
        let format_chars = if self.ascii {
            FormatCharacters::ascii()
        } else {
            FormatCharacters::box_chars()
        };
        let formatting = TreeFormatting::dir_tree(format_chars);
        Ok(tree.to_string_with_format(&formatting)?)
    }

    /// Print a tree directly to output with error handling
    pub fn print_tree(&self, tree: &StringTreeNode, ctx: &CommandContext) {
        match self.format_tree(tree) {
            Ok(output) => ctx.output.print_result(output.trim_end()),
            Err(e) => error!("Failed to format tree: {e}"),
        }
    }
}

/// Output format handler trait for consistent format switching
pub trait OutputFormatHandler<T> {
    fn handle_flat(&self, data: &T, ctx: &CommandContext) -> Result<(), Error>;
    fn handle_tree(&self, data: &T, ctx: &CommandContext, ascii: bool) -> Result<(), Error>;
    fn handle_json(&self, data: &T, ctx: &CommandContext) -> Result<(), Error>;

    /// Main entry point that handles format switching
    fn handle_output(
        &self,
        data: &T,
        format: OutputFormat,
        ctx: &CommandContext,
        ascii: bool,
    ) -> Result<(), Error> {
        match format {
            OutputFormat::Flat => self.handle_flat(data, ctx),
            OutputFormat::Tree => self.handle_tree(data, ctx, ascii),
            OutputFormat::Json => self.handle_json(data, ctx),
        }
    }
}

/// Generic output formatter for collections of data
pub struct CollectionFormatter;

impl CollectionFormatter {
    /// Handle flat output for a collection of displayable items
    pub fn print_flat<T: std::fmt::Display>(
        items: &[T],
        ctx: &CommandContext,
    ) -> Result<(), Error> {
        OutputUtils::print_list(ctx, items);
        Ok(())
    }

    // JSON output is handled directly by OutputUtils::print_json

    /// Handle tree output using a provided tree
    pub fn print_tree(
        tree: &StringTreeNode,
        ctx: &CommandContext,
        ascii: bool,
    ) -> Result<(), Error> {
        let formatter = TreeFormatter::new(ascii);
        formatter.print_tree(tree, ctx);
        Ok(())
    }
}

// Removed generic collection handlers to avoid serde trait issues
// Commands can use the specific utilities directly

/// Utility for consistent command summary display
pub struct CommandSummary<'a> {
    title: &'a str,
    items: Vec<(&'a str, String)>,
}

impl<'a> CommandSummary<'a> {
    pub fn new(title: &'a str) -> Self {
        Self {
            title,
            items: Vec::new(),
        }
    }

    pub fn add_item(mut self, key: &'a str, value: String) -> Self {
        self.items.push((key, value));
        self
    }

    pub fn add_count(self, key: &'a str, count: usize) -> Self {
        self.add_item(key, count.to_string())
    }

    pub fn print(self, ctx: &CommandContext) {
        OutputUtils::print_header(ctx, self.title);
        OutputUtils::print_summary(ctx, &self.items);
        ctx.output.print_result("");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_summary_builder() {
        let summary = CommandSummary::new("Test Summary")
            .add_item("File", "test.blend".to_string())
            .add_count("Blocks", 42)
            .add_count("Modified", 3);

        assert_eq!(summary.title, "Test Summary");
        assert_eq!(summary.items.len(), 3);
        assert_eq!(summary.items[0], ("File", "test.blend".to_string()));
        assert_eq!(summary.items[1], ("Blocks", "42".to_string()));
        assert_eq!(summary.items[2], ("Modified", "3".to_string()));
    }
}
