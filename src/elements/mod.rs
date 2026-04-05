pub mod code_blocks;
pub mod footnotes;
pub mod headings;
pub mod images;
pub mod languages;
pub mod math;

/// Run the full standardization pipeline on HTML content.
///
/// This normalizes vendor-specific HTML (code blocks, headings, images,
/// footnotes, math) into canonical forms before markdown conversion.
pub fn standardize_all(html: &str, title: Option<&str>) -> String {
    let mut result = html.to_string();

    result = code_blocks::standardize_code_blocks(&result);
    result = headings::standardize_headings(&result, title);
    result = images::standardize_images(&result);
    result = footnotes::standardize_footnotes(&result);
    result = math::standardize_math(&result);

    result
}
