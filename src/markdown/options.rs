/// Options for controlling markdown output format.
#[derive(Debug, Clone)]
pub struct MarkdownOptions {
    /// Heading style: ATX (`# Heading`) or Setext (underline).
    pub heading_style: HeadingStyle,
    /// Bullet character for unordered lists.
    pub bullet_char: char,
    /// Fence character for code blocks (`` ` `` or `~`).
    pub code_fence: char,
    /// Emphasis delimiter (`*` or `_`).
    pub emphasis_delimiter: char,
    /// Strong delimiter (`**` or `__`).
    pub strong_delimiter: String,
    /// Link style: inline `[text](url)` or reference `[text][ref]`.
    pub link_style: LinkStyle,
    /// Keep complex tables (colspan/rowspan) as raw HTML.
    pub preserve_complex_tables: bool,
}

/// Heading output style.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeadingStyle {
    /// `# Heading` style (default).
    Atx,
    /// Underline style (only for h1/h2).
    Setext,
}

/// Link output style.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkStyle {
    /// `[text](url)` — inline (default).
    Inline,
    /// `[text][ref]` with references collected at end.
    Reference,
}

impl Default for MarkdownOptions {
    fn default() -> Self {
        Self {
            heading_style: HeadingStyle::Atx,
            bullet_char: '-',
            code_fence: '`',
            emphasis_delimiter: '*',
            strong_delimiter: "**".to_string(),
            link_style: LinkStyle::Inline,
            preserve_complex_tables: true,
        }
    }
}
