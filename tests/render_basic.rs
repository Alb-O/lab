use paper_terminal::media::RasteroidBackend;
use paper_terminal::{BufferSink, Cells, ColorThemeName, Config, GlyphThemeName, Renderer};

fn strip_ansi(s: &str) -> String {
    console::strip_ansi_codes(s).to_string()
}

#[test]
fn paragraph_wraps_within_width() {
    let cfg = Config {
        width: Cells(20),
        tab_length: 4,
        hide_urls: true,
        no_images: true,
        syncat: false,
        dev: false,
        glyph_theme: GlyphThemeName::Unicode,
        color_theme: ColorThemeName::None,
        code_theme: paper_terminal::code_theme::CodeThemeSetting::Auto,
        wrap_mode: paper_terminal::config::WrapMode::Greedy,
    };
    let sink = BufferSink::default();
    let mut renderer: Renderer<RasteroidBackend, BufferSink> =
        Renderer::with_sink(cfg, sink.clone());
    let md = "This is a small paragraph that should wrap nicely across lines.";
    let _ = renderer.render_markdown(md, None);
    let out = sink.snapshot();
    let lines: Vec<String> = out
        .into_iter()
        .filter(|l| !l.is_empty())
        .map(|l| strip_ansi(&l))
        .collect();
    assert!(lines.len() >= 2);
    for line in &lines {
        assert!(paper_terminal::str_width::str_width(line) <= 20);
    }
}

#[test]
fn unordered_list_has_bullets() {
    let cfg = Config {
        width: Cells(40),
        tab_length: 4,
        hide_urls: true,
        no_images: true,
        syncat: false,
        dev: false,
        glyph_theme: GlyphThemeName::Unicode,
        color_theme: ColorThemeName::None,
        code_theme: paper_terminal::code_theme::CodeThemeSetting::Auto,
        wrap_mode: paper_terminal::config::WrapMode::Greedy,
    };
    let sink = BufferSink::default();
    let mut renderer: Renderer<RasteroidBackend, BufferSink> =
        Renderer::with_sink(cfg, sink.clone());
    let md = "- item one\n- item two";
    let _ = renderer.render_markdown(md, None);
    let out = sink.snapshot();
    let lines: Vec<String> = out
        .into_iter()
        .filter(|l| !l.is_empty())
        .map(|l| strip_ansi(&l))
        .collect();
    // Expect at least one bullet line (allow for indent and padding)
    assert!(
        lines
            .iter()
            .map(|l| l.trim_start())
            .any(|l| l.starts_with("• ")
                || l.starts_with("* ")
                || l.starts_with("•")
                || l.starts_with("*"))
    );
}

#[test]
fn heading_renders_text() {
    let cfg = Config {
        width: Cells(40),
        tab_length: 4,
        hide_urls: true,
        no_images: true,
        syncat: false,
        dev: false,
        glyph_theme: GlyphThemeName::Unicode,
        color_theme: ColorThemeName::None,
        code_theme: paper_terminal::code_theme::CodeThemeSetting::Auto,
        wrap_mode: paper_terminal::config::WrapMode::Greedy,
    };
    let sink = BufferSink::default();
    let mut renderer: Renderer<RasteroidBackend, BufferSink> =
        Renderer::with_sink(cfg, sink.clone());
    let md = "# My Title";
    let _ = renderer.render_markdown(md, None);
    let out = sink.snapshot();
    let lines: Vec<String> = out
        .into_iter()
        .filter(|l| !l.is_empty())
        .map(|l| strip_ansi(&l))
        .collect();
    assert!(lines.iter().any(|l| l.contains("My Title")));
}
