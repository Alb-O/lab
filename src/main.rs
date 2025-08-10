#![allow(dead_code)]

use clap::{ArgAction, CommandFactory, Parser as _};
use clap_complete::Shell;
use std::fs;
use std::io::{self, Read};
use std::path::PathBuf;
use terminal_size::{Width, terminal_size};

// use the library crate API

use paper_terminal::{Cells, ColorThemeName, Config, Renderer, ThemeName};

/// Minimal terminal Markdown renderer with optional inline images
#[derive(clap::Parser, Debug)]
#[clap(
    name = "paper",
    about = "Minimal terminal Markdown renderer with inline images (Kitty/iTerm2/Sixel)",
    rename_all = "kebab-case"
)]
pub struct Opts {
    /// Target width (in terminal cells)
    #[arg(short = 'w', long, default_value_t = 92)]
    pub width: usize,

    /// Print input without Markdown parsing
    #[arg(short = 'p', long, action = ArgAction::SetTrue)]
    pub plain: bool,

    /// The length to consider tabs as.
    #[arg(short, long, default_value_t = 4)]
    pub tab_length: usize,

    /// Hide link URLs
    #[arg(short = 'U', long, action = ArgAction::SetTrue)]
    pub hide_urls: bool,

    /// Disable inline images
    #[arg(short = 'I', long, action = ArgAction::SetTrue)]
    pub no_images: bool,

    /// Use syntax highlighting for fenced code blocks
    /// Backward-compatible alias: --syncat
    #[cfg_attr(
        feature = "syntax-highlighting",
        arg(
            short = 'H',
            long = "highlight",
            visible_alias = "syncat",
            default_value_t = true
        )
    )]
    #[cfg_attr(
        not(feature = "syntax-highlighting"),
        arg(short = 'H', long = "highlight", visible_alias = "syncat", action = ArgAction::SetTrue)
    )]
    pub highlight: bool,

    /// Print parser events (debug)
    #[arg(long, action = ArgAction::SetTrue)]
    pub dev: bool,

    /// Files to print
    #[arg(name = "FILE")]
    pub files: Vec<PathBuf>,

    /// Generate shell completions
    #[arg(long)]
    completions: Option<Shell>,

    /// GlyphTheme for bullets and rules
    #[arg(long, value_enum, default_value = "unicode")]
    pub theme: ThemeName,

    /// Color theme for markdown elements
    #[arg(long, value_enum, default_value = "light")]
    pub color_theme: ColorThemeName,
}

fn normalize(tab_len: usize, source: &str) -> String {
    source
        .lines()
        .map(|line| {
            let mut len = 0;
            if line.contains('\t') {
                line.chars()
                    .flat_map(|ch| {
                        if ch == '\t' {
                            let missing = tab_len - (len % tab_len);
                            len += missing;
                            vec![' '; missing]
                        } else {
                            len += 1;
                            vec![ch]
                        }
                    })
                    .collect::<String>()
            } else {
                line.to_string()
            }
        })
        .map(|line| format!("{line}\n"))
        .collect::<String>()
}

fn print<I>(opts: Opts, sources: I)
where
    I: Iterator<Item = (PathBuf, Result<String, std::io::Error>)>,
{
    let terminal_width = terminal_size()
        .map(|(Width(width), _)| width)
        .unwrap_or(opts.width as u16) as usize;
    let width = usize::min(opts.width, terminal_width.saturating_sub(1));

    let cfg = Config {
        width: Cells(width),
        tab_length: opts.tab_length,
        hide_urls: opts.hide_urls,
        no_images: opts.no_images,
        syncat: opts.highlight,
        dev: opts.dev,
        theme: opts.theme,
        color_theme: opts.color_theme,
    }
    .validate();
    let mut renderer = Renderer::<paper_terminal::media::RasteroidBackend>::new(cfg);
    for (file_path, source) in sources {
        let source = match source {
            Ok(source) => normalize(opts.tab_length, &source),
            Err(error) => {
                println!("{error}");
                continue;
            }
        };
        if opts.plain {
            // Just print normalized text as-is
            print!("{source}");
        } else {
            let _ = renderer.render_markdown(&source, Some(&file_path));
        }
    }
}

fn main() {
    let opts = Opts::parse();

    if opts.completions.is_some() {
        let shell = opts.completions.or_else(Shell::from_env).unwrap();
        let mut opts = Opts::command();
        let name = opts.get_name().to_string();
        clap_complete::generate(shell, &mut opts, name, &mut std::io::stdout());
        std::process::exit(0);
    }

    if opts.files.is_empty() {
        let mut string = String::new();
        io::stdin().read_to_string(&mut string).unwrap();
        print(opts, vec![(PathBuf::new(), Ok(string))].into_iter());
    } else {
        let sources = opts.files.clone().into_iter().map(|path| {
            let source = fs::read_to_string(&path);
            (path, source)
        });
        print(opts, sources);
    }
}
