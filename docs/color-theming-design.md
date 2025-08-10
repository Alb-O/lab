# Color Theming System Design

## Problem
Currently, the "Theme" concept only handles character display (bullets, horizontal rules, quote prefixes), which is confusing since users expect "theme" to mean colors. We need a proper color/styling system.

## Design Approach

### 1. Naming Refactor
- `Theme` → `GlyphTheme` (characters/glyphs)
- New `ColorTheme` (colors and text styling)
- Keep CLI compatibility with transitional types

### 2. Color Theme Architecture

```rust
// Basic color representation - start simple
#[derive(Debug, Clone, Copy)]
pub enum Color {
    Default,
    Black,
    Red, Green, Blue, Yellow, Magenta, Cyan, White,
    BrightBlack, BrightRed, BrightGreen, BrightBlue,
    BrightYellow, BrightMagenta, BrightCyan, BrightWhite,
    // Future: RGB(u8, u8, u8), Ansi256(u8)
}

#[derive(Debug, Clone, Default)]
pub struct TextStyle {
    pub foreground: Option<Color>,
    pub background: Option<Color>, 
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub dim: bool,
    pub reverse: bool,
}

#[derive(Debug, Clone)]
pub struct ColorTheme {
    pub name: String,
    
    // Text element styles
    pub default_text: TextStyle,
    pub heading: TextStyle,
    pub quote: TextStyle,
    pub inline_code: TextStyle,
    pub code_block: TextStyle,
    pub link: TextStyle,
    pub emphasis: TextStyle,   // italic
    pub strong: TextStyle,     // bold
    
    // UI elements
    pub hr: TextStyle,
}
```

### 3. Built-in Themes

**Light Theme:**
- Default text: black on default background
- Headings: bold black
- Quotes: dim blue
- Code: reverse (traditional terminal look)
- Links: blue, underlined

**Dark Theme:** 
- Default text: white on default background
- Headings: bold white
- Quotes: dim cyan  
- Code: reverse
- Links: cyan, underlined

**No-Color Theme:**
- Everything uses default colors
- Only uses non-color attributes like bold
- For compatibility/accessibility

### 4. Dependencies

**Option A: Stick with ansi_term**
- ✅ Already used, lightweight
- ✅ Familiar API
- ❌ Unmaintained (archived on GitHub)
- ❌ Limited features

**Option B: Switch to crossterm**
- ✅ Actively maintained, full-featured
- ✅ Cross-platform
- ❌ Heavier dependency
- ❌ API changes required

**Option C: termcolor**
- ✅ Lightweight, actively maintained
- ✅ Good Windows support
- ❌ Different API from ansi_term

**Option D: Minimal custom ANSI**
- ✅ No dependencies, full control
- ✅ Exactly what we need
- ❌ More code to maintain

**Decision: Start with ansi_term (minimal change), plan migration path**

### 5. Implementation Strategy

**Phase 1: Structure Setup**
- Create `style` module with Color/TextStyle/ColorTheme types
- Create `Styler` trait for applying styles to text
- Implement basic light/dark/no-color themes

**Phase 2: Integration** 
- Update renderer to use ColorTheme instead of direct ansi_term calls
- Add CLI flags: `--color-theme light|dark|none`
- Keep `--theme` for glyph themes, add `--glyph-theme` alias

**Phase 3: Enhancement**
- Add RGB/256-color support
- Theme loading from files (TOML?)
- More built-in themes

### 6. CLI Interface

```bash
# Current (backward compatible)
paper --theme unicode file.md         # glyph theme
paper --theme ascii file.md

# New color options
paper --color-theme dark file.md      # color theme
paper --color-theme light file.md
paper --color-theme none file.md      # no colors

# Combined
paper --glyph-theme unicode --color-theme dark file.md

# Future: custom theme files
paper --color-theme ~/.config/paper/my-theme.toml file.md
```

### 7. Configuration Approach

For now: Built-in themes only
Future: TOML configuration files

```toml
name = "my-dark-theme"

[default_text]
foreground = "white"

[heading] 
foreground = "bright_yellow"
bold = true

[quote]
foreground = "cyan" 
dim = true
```

## Next Steps

1. Implement basic style module structure
2. Create Styler trait and basic themes
3. Update renderer to use new system
4. Test with sample documents
5. Consider dependency migration later if needed