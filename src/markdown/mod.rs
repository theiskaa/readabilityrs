pub mod converter;
pub mod options;
pub mod rules;
pub mod state;

pub use options::MarkdownOptions;

/// Convert cleaned HTML to markdown.
///
/// This is the main entry point for the markdown conversion module.
/// The HTML should already be standardized via `elements::standardize_all()`.
pub fn html_to_markdown(html: &str, options: &MarkdownOptions) -> String {
    let doc = scraper::Html::parse_fragment(html);
    converter::convert(&doc, options)
}
