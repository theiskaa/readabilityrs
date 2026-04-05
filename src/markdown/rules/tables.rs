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

/// Convert a simple table to pipe-format markdown.
pub fn convert_simple_table(headers: &[String], rows: &[Vec<String>]) -> String {
    if headers.is_empty() && rows.is_empty() {
        return String::new();
    }

    // Calculate column widths
    let num_cols = headers.len().max(rows.iter().map(|r| r.len()).max().unwrap_or(0));
    if num_cols == 0 {
        return String::new();
    }

    // Calculate column widths using escaped text (pipes become \|, adding width)
    let mut col_widths = vec![3usize; num_cols];
    for (i, h) in headers.iter().enumerate() {
        col_widths[i] = col_widths[i].max(escape_pipe(h).len());
    }
    for row in rows {
        for (i, cell) in row.iter().enumerate() {
            if i < num_cols {
                col_widths[i] = col_widths[i].max(escape_pipe(cell).len());
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
}
