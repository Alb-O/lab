# Table Rendering Demo

This document exercises various table cases for paper-terminal using comfy-table.

## Basic Table

| Header 1 | Header 2 | Header 3 |
|---------:|:--------:|:---------|
| Right-aligned number: 12345 | center | normal |
| Simple text | multiline line 1\nline 2 | a-very-very-very-long-word-that-should-wrap |

## Inline Styles

| Feature | Example |
|--------:|:-------|
| Italic | This is *italic* inside a cell |
| Bold | This is **bold** inside a cell |
| Strikethrough | This is ~~struck~~ inside a cell |
| Inline Code | `let x = 42;` inside a cell |
| Link | Link text [paper-terminal](https://github.com/foxfriends/paper-terminal) appears with URL suffix |

## Line Breaks

| Title | Content |
|------:|:--------|
| Soft breaks | First line\nsecond line\nthird line |
| Hard break | Use two spaces at line end  \
New line using Markdown hard break |

## Wrapping and Width Pressure

| Left | Middle | Right |
|-----:|:------:|:------|
| A short sentence. | This column has content that should wrap within the available table width and demonstrate dynamic layout. | Another column with an even longer text chunk to test how comfy-table balances columns under width constraints in the terminal. |

## Mixed Inline Elements

| Mixed | Notes |
|-----:|:------|
| Combined styles | Mix of *italic*, **bold**, `code`, and a [link](https://example.com) all together. |
| Emojis and CJK | Emojis 😀 😺 and some CJK: 你好，世界 — wide characters should be measured correctly. |

---

Tip: Run with `--theme ascii` to preview ASCII borders, or default `--theme unicode` to preview UTF‑8 borders. Use `--width` to test wrapping.

