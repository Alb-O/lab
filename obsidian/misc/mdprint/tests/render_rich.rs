use paper_terminal::media::RasteroidBackend;
use paper_terminal::{BufferSink, Cells, ColorThemeName, Config, GlyphThemeName, Renderer};

fn strip_ansi(s: &str) -> String {
    console::strip_ansi_codes(s).to_string()
}

fn make_cfg(width: usize, ascii: bool) -> Config {
    Config {
        width: Cells(width),
        tab_length: 4,
        hide_urls: true,
        no_images: true,
        syncat: false,
        dev: false,
        glyph_theme: if ascii {
            GlyphThemeName::Ascii
        } else {
            GlyphThemeName::Unicode
        },
        color_theme: ColorThemeName::None,
        code_theme: paper_terminal::code_theme::CodeThemeSetting::Auto,
        wrap_mode: paper_terminal::config::WrapMode::Greedy,
    }
}

#[test]
fn table_alignments_are_respected_ascii_borders() {
    let cfg = make_cfg(60, true);
    let sink = BufferSink::default();
    let mut renderer: Renderer<RasteroidBackend, BufferSink> =
        Renderer::with_sink(cfg, sink.clone());
    let md = r#"| LeftCol | CenterCol | RightCol |
| :------ | :------: | -------: |
| a       |    b     |        c |
| aaa     |   bbb    |      ccc |
"#;
    let _ = renderer.render_markdown(md, None);
    let lines: Vec<String> = sink
        .snapshot()
        .into_iter()
        .map(|l| strip_ansi(&l))
        .filter(|l| l.contains('|') && !l.contains("-+-"))
        .collect();
    // Extract the two data lines
    let mut data_lines: Vec<String> = lines
        .into_iter()
        .filter(|l| l.trim().starts_with('|') && (l.contains(" a ") || l.contains(" aaa ")))
        .collect();
    assert_eq!(data_lines.len(), 2, "expected two data rows");
    let line1 = data_lines.remove(0);
    let line2 = data_lines.remove(0);

    let bar_positions = |line: &str| {
        line.char_indices()
            .filter_map(|(i, ch)| if ch == '|' { Some(i) } else { None })
            .collect::<Vec<_>>()
    };
    let bars1 = bar_positions(&line1);
    let bars2 = bar_positions(&line2);
    assert_eq!(bars1.len(), bars2.len());

    let cell_bounds = |bars: &Vec<usize>, idx: usize| (bars[idx] + 1, bars[idx + 1] - 1);
    let first_non_ws = |s: &str, (l, r): (usize, usize)| {
        s[l..=r]
            .char_indices()
            .find(|(_, ch)| !ch.is_whitespace())
            .map(|(i, _)| l + i)
            .unwrap()
    };
    let last_non_ws = |s: &str, (l, r): (usize, usize)| {
        let seg = &s[l..=r];
        let off = seg.chars().rev().take_while(|c| c.is_whitespace()).count();
        r - off
    };

    // For left column (idx 0), first non-whitespace should be equal across rows
    let b1 = cell_bounds(&bars1, 0);
    let b2 = cell_bounds(&bars2, 0);
    let l_start1 = first_non_ws(&line1, b1);
    let l_start2 = first_non_ws(&line2, b2);
    assert_eq!(
        l_start1, l_start2,
        "left column not aligned to left: \n{line1}\n{line2}"
    );

    // For center column (idx 1), midpoints should be approximately equal
    let b1c = cell_bounds(&bars1, 1);
    let b2c = cell_bounds(&bars2, 1);
    let c_start1 = first_non_ws(&line1, b1c);
    let c_end1 = last_non_ws(&line1, b1c);
    let c_mid1 = (c_start1 + c_end1) / 2;
    let c_start2 = first_non_ws(&line2, b2c);
    let c_end2 = last_non_ws(&line2, b2c);
    let c_mid2 = (c_start2 + c_end2) / 2;
    assert!(
        (c_mid1 as isize - c_mid2 as isize).abs() <= 1,
        "center column not centered across rows: \n{line1}\n{line2}"
    );

    // For right column (idx 2), last non-whitespace should be equal across rows
    let b1r = cell_bounds(&bars1, 2);
    let b2r = cell_bounds(&bars2, 2);
    let r_end1 = last_non_ws(&line1, b1r);
    let r_end2 = last_non_ws(&line2, b2r);
    assert_eq!(
        r_end1, r_end2,
        "right column not aligned to right: \n{line1}\n{line2}"
    );
}

#[test]
fn nested_blockquotes_prefix_depth_ascii() {
    let cfg = make_cfg(80, true);
    let sink = BufferSink::default();
    let mut renderer: Renderer<RasteroidBackend, BufferSink> =
        Renderer::with_sink(cfg, sink.clone());
    let md = "> Outer\n> > Inner level 2\n> > > Inner level 3";
    let _ = renderer.render_markdown(md, None);
    let out = sink.snapshot();
    let rows: Vec<String> = out
        .into_iter()
        .filter(|l| !l.is_empty())
        .map(|l| strip_ansi(&l))
        .collect();
    // Find lines starting with repeated '>' prefixes and a space. Allow leading spaces from indent.
    let has_lvl1 = rows.iter().any(|l| l.trim_start().starts_with("> "));
    let has_lvl2 = rows.iter().any(|l| l.trim_start().starts_with(">> "));
    let has_lvl3 = rows.iter().any(|l| l.trim_start().starts_with(">>> "));
    assert!(
        has_lvl1 && has_lvl2 && has_lvl3,
        "missing expected quote prefixes: {rows:?}"
    );
}

#[test]
fn strikethrough_emits_crossedout_ansi() {
    let cfg = make_cfg(80, false);
    let sink = BufferSink::default();
    let mut renderer: Renderer<RasteroidBackend, BufferSink> =
        Renderer::with_sink(cfg, sink.clone());
    let md = "This has ~~strike~~ text.";
    let _ = renderer.render_markdown(md, None);
    let rows = sink.snapshot();
    // Search for a line that contains the word and ANSI CrossedOut. ESC[9m is the SGR for crossed-out.
    let crossed = rows
        .iter()
        .any(|l| l.contains("strike") && l.contains("[9m"));
    assert!(
        crossed,
        "expected crossed-out ANSI for strikethrough; output: {rows:?}"
    );
}
