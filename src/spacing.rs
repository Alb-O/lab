#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Block {
    Document,
    Paragraph,
    Heading,
    List,
    ListItem,
    CodeBlock,
    Rule,
    BlockQuote,
    Image,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BlankLines(pub u8);

pub trait SpacingPolicy {
    fn between(&self, prev: Option<Block>, next: Block, in_list: bool) -> BlankLines;
}

#[derive(Default, Clone, Copy)]
pub struct DefaultSpacingPolicy;

impl SpacingPolicy for DefaultSpacingPolicy {
    fn between(&self, prev: Option<Block>, next: Block, in_list: bool) -> BlankLines {
        use Block::*;
        match (prev, next) {
            // Document start: no leading space
            (None, _) => BlankLines(0),

            // List item boundaries: compact by default
            (Some(ListItem), ListItem) => BlankLines(0),
            (Some(ListItem), List) => BlankLines(0), // nested list under item
            (Some(List), ListItem) => BlankLines(0), // first item of a list
            (Some(List), List) => BlankLines(1),     // separate adjacent lists

            // Paragraph groupings
            (Some(Paragraph), Paragraph) => BlankLines(1),
            (Some(Paragraph), Heading) => BlankLines(1),
            (Some(Heading), Paragraph) => BlankLines(1),

            // Rules are strong separators
            (_, Rule) => BlankLines(1),
            (Some(Rule), _) => BlankLines(1),

            // Code blocks: add space around, but inside lists can be tighter before
            (Some(Paragraph), CodeBlock) if in_list => BlankLines(1),
            (Some(Paragraph), CodeBlock) => BlankLines(1),
            (Some(CodeBlock), Paragraph) => BlankLines(1),

            // Block quotes as blocks
            (Some(BlockQuote), BlockQuote) => BlankLines(1),
            (_, BlockQuote) => BlankLines(1),
            (Some(BlockQuote), _) => BlankLines(1),

            // Images: treat like blocks
            (_, Image) => BlankLines(1),
            (Some(Image), _) => BlankLines(1),

            // Lists relative to paragraphs and headings
            (Some(Paragraph), List) => BlankLines(1),
            (Some(List), Paragraph) => BlankLines(1),
            (Some(Heading), List) => BlankLines(1),
            (Some(List), Heading) => BlankLines(1),

            // Default: a single blank line between blocks
            _ => BlankLines(1),
        }
    }
}
