use scraper::{ElementRef, Selector};

/// Check whether a table is "complex" (has colspan, rowspan, or nested tables).
pub fn is_complex_table(table: &ElementRef) -> bool {
    if let Ok(sel) = Selector::parse("[colspan], [rowspan]") {
        if table.select(&sel).next().is_some() {
            return true;
        }
    }
    if let Ok(sel) = Selector::parse("table table") {
        if table.select(&sel).next().is_some() {
            return true;
        }
    }
    false
}

/// Check if this is a layout table (single column, no `<th>`).
pub fn is_layout_table(table: &ElementRef) -> bool {
    if let Ok(th_sel) = Selector::parse("th") {
        if table.select(&th_sel).next().is_some() {
            return false;
        }
    }

    // Check if all rows have exactly 1 cell
    if let Ok(tr_sel) = Selector::parse("tr") {
        if let Ok(td_sel) = Selector::parse("td, th") {
            for tr in table.select(&tr_sel) {
                let count = tr.select(&td_sel).count();
                if count > 1 {
                    return false;
                }
            }
        }
    }

    true
}

/// Upper bound on markdown table column padding.
///
/// Pipe tables do not need aligned columns to be valid, so padding past this
/// only inflates the output: without a bound, one wide cell pads every other
/// cell in its column, and a column wider than `u16::MAX` panics the runtime
/// formatting machinery outright.
const MAX_COL_WIDTH: usize = 200;

/// Convert a simple table to pipe-format markdown.
pub fn convert_simple_table(headers: &[String], rows: &[Vec<String>]) -> String {
    if headers.is_empty() && rows.is_empty() {
        return String::new();
    }

    // Calculate column widths
    let num_cols = headers
        .len()
        .max(rows.iter().map(|r| r.len()).max().unwrap_or(0));
    if num_cols == 0 {
        return String::new();
    }

    // Calculate column widths using escaped text (pipes become \|, adding width).
    // Widths are clamped to MAX_COL_WIDTH; `{:<width$}` only ever pads, so a cell
    // longer than the cap is emitted at its natural length rather than truncated.
    let mut col_widths = vec![3usize; num_cols];
    for (i, h) in headers.iter().enumerate() {
        col_widths[i] = col_widths[i].max(escape_pipe(h).len()).min(MAX_COL_WIDTH);
    }
    for row in rows {
        for (i, cell) in row.iter().enumerate() {
            if i < num_cols {
                col_widths[i] = col_widths[i]
                    .max(escape_pipe(cell).len())
                    .min(MAX_COL_WIDTH);
            }
        }
    }

    let mut out = String::new();

    // Header row
    out.push('|');
    for (i, w) in col_widths.iter().enumerate() {
        let h = headers.get(i).map(|s| s.as_str()).unwrap_or("");
        out.push_str(&format!(" {:<width$} |", escape_pipe(h), width = w));
    }
    out.push('\n');

    // Separator
    out.push('|');
    for w in &col_widths {
        out.push_str(&format!("-{}-|", "-".repeat(*w)));
    }
    out.push('\n');

    // Data rows
    for row in rows {
        out.push('|');
        for (i, w) in col_widths.iter().enumerate() {
            let cell = row.get(i).map(|s| s.as_str()).unwrap_or("");
            out.push_str(&format!(" {:<width$} |", escape_pipe(cell), width = w));
        }
        out.push('\n');
    }

    format!("\n\n{}\n", out.trim_end())
}

fn escape_pipe(s: &str) -> String {
    s.replace('|', "\\|")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_table() {
        let headers = vec!["Name".to_string(), "Age".to_string()];
        let rows = vec![
            vec!["Alice".to_string(), "30".to_string()],
            vec!["Bob".to_string(), "25".to_string()],
        ];
        let result = convert_simple_table(&headers, &rows);
        assert!(result.contains("| Name"));
        assert!(result.contains("|---"));
        assert!(result.contains("| Alice"));
    }

    #[test]
    fn test_ordinary_table_shape_is_unchanged() {
        let headers = vec!["A".to_string(), "Name".to_string()];
        let rows = vec![vec!["1".to_string(), "Alice".to_string()]];

        let result = convert_simple_table(&headers, &rows);
        let lines: Vec<&str> = result.trim().lines().collect();

        assert_eq!(lines.len(), 3, "header, separator, one data row");
        assert_eq!(lines[0], "| A   | Name  |");
        assert_eq!(lines[1], "|-----|-------|");
        assert_eq!(lines[2], "| 1   | Alice |");

        // Columns start at width 3, so even a one-character column clears the
        // three-dash minimum a pipe-table separator needs.
        for segment in lines[1].split('|').filter(|s| !s.is_empty()) {
            assert!(
                segment.len() >= 3,
                "separator segment {segment:?} is below the three-dash minimum"
            );
        }
    }

    #[test]
    fn test_oversized_cell_does_not_panic() {
        // Rust caps runtime format widths at u16::MAX, so a cell wider than that
        // used to abort formatting rather than render.
        let big_cell = "A".repeat(65_540);
        let headers = vec!["Name".to_string(), "Data".to_string()];
        let rows = vec![vec!["Alice".to_string(), big_cell.clone()]];

        let result = convert_simple_table(&headers, &rows);

        assert!(result.contains(&big_cell), "cell content must not be lost");
    }

    #[test]
    fn test_wide_cell_does_not_amplify_output() {
        // One wide cell used to set the column width for every other row, so 300
        // trivial rows each grew to the width of the widest.
        let headers = vec!["Name".to_string(), "Data".to_string()];
        let mut rows = vec![vec!["Alice".to_string(), "B".repeat(60_000)]];
        rows.extend((0..300).map(|i| vec![format!("row{i}"), "x".to_string()]));

        let input_len: usize = headers.iter().map(|h| h.len()).sum::<usize>()
            + rows
                .iter()
                .flat_map(|r| r.iter().map(|c| c.len()))
                .sum::<usize>();

        let result = convert_simple_table(&headers, &rows);

        assert!(
            result.len() < input_len * 3,
            "output {} bytes for {} bytes of input",
            result.len(),
            input_len
        );
    }
}
