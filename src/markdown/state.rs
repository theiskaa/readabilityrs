/// Tracks context during recursive DOM-to-markdown conversion.
#[derive(Debug, Default)]
pub struct ConversionState {
    /// Current list nesting depth (0 = not in a list).
    pub list_depth: usize,
    /// Counter stack for ordered lists at each nesting level.
    pub ordered_list_counters: Vec<usize>,
    /// True when inside a `<pre><code>` block — skip all formatting.
    pub in_code_block: bool,
    /// True when inside a `<table>` — different whitespace rules.
    pub in_table: bool,
    /// Current blockquote nesting depth.
    pub in_blockquote_depth: usize,
    /// Collected link references for reference-style output.
    pub link_references: Vec<(String, String)>,
    /// Collected footnote definitions `(id, content)`.
    pub footnotes: Vec<(String, String)>,
    /// True when inside an `<a>` tag — prevents nested link output.
    pub in_link: bool,
    /// True when inside a heading — prevents nested block elements.
    pub in_heading: bool,
    /// True when inside a `<li>` — suppresses paragraph blank-line wrapping.
    pub in_list_item: bool,
}
