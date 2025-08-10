use crate::types::Column;
use std::io::{self, Write};
use std::sync::{Arc, Mutex};

/// Trait for output destinations in the terminal renderer
pub trait Sink {
    /// Write a line with the specified indentation
    fn write_line(&mut self, line: &str, indent: usize) -> io::Result<()>;

    /// Write a line positioned at an absolute column (0-based) on a new line.
    /// Implementations should ensure the cursor starts at column 0 before applying padding.
    fn write_line_absolute(&mut self, line: &str, column: Column) -> io::Result<()>;

    /// Write a blank line
    fn write_blank_line(&mut self) -> io::Result<()>;

    /// Flush any buffered output
    fn flush(&mut self) -> io::Result<()>;
}

/// Standard output sink that writes directly to stdout
#[derive(Default)]
pub struct StdoutSink;

impl StdoutSink {
    pub fn new() -> Self {
        Self
    }
}

impl Sink for StdoutSink {
    fn write_line(&mut self, line: &str, indent: usize) -> io::Result<()> {
        if line.is_empty() {
            return Ok(());
        }
        let pad = " ".repeat(indent);
        println!("{pad}{line}");
        Ok(())
    }

    fn write_line_absolute(&mut self, line: &str, column: Column) -> io::Result<()> {
        if line.is_empty() {
            return Ok(());
        }
        let pad = " ".repeat(column.0);
        // Move to start of line, then write padding and content, ending with a newline
        print!("\r{pad}{line}\n");
        Ok(())
    }

    fn write_blank_line(&mut self) -> io::Result<()> {
        println!();
        Ok(())
    }

    fn flush(&mut self) -> io::Result<()> {
        io::stdout().flush()
    }
}

/// In-memory sink backed by a shared buffer for tests and examples
#[derive(Clone, Default)]
pub struct BufferSink {
    lines: Arc<Mutex<Vec<String>>>,
}

impl BufferSink {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn snapshot(&self) -> Vec<String> {
        self.lines.lock().unwrap().clone()
    }
}

impl Sink for BufferSink {
    fn write_line(&mut self, line: &str, indent: usize) -> io::Result<()> {
        if line.is_empty() {
            return Ok(());
        }
        let pad = " ".repeat(indent);
        self.lines.lock().unwrap().push(format!("{pad}{line}"));
        Ok(())
    }

    fn write_line_absolute(&mut self, line: &str, column: Column) -> io::Result<()> {
        if line.is_empty() {
            return Ok(());
        }
        let pad = " ".repeat(column.0);
        self.lines.lock().unwrap().push(format!("{pad}{line}"));
        Ok(())
    }

    fn write_blank_line(&mut self) -> io::Result<()> {
        self.lines.lock().unwrap().push(String::new());
        Ok(())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
