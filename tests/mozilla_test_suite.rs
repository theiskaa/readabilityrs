//! Integration tests using Mozilla's official Readability test suite
//!
//! This test harness loads test cases from readability/test/test-pages/
//! and compares our output with Mozilla's expected results.

use readabilityrs::{Readability, ReadabilityOptions};
use scraper::Html;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Cases where readabilityrs intentionally diverges from Mozilla's expected
/// metadata (better byline/excerpt choices — see README "Compatibility").
/// A case listed here is ALLOWED to mismatch; it is still parsed and must
/// not panic. Do not add to this list to silence a new regression.
const KNOWN_METADATA_DIVERGENCES: &[&str] = &[
    "ietf-1",
    "liberation-1",
    "mathjax",
    "mercurial",
    "nytimes-5",
    "replace-brs",
    "salon-1",
    "seattletimes-1",
    "wikipedia-2",
    "wikipedia-4",
    "wordpress",
];

/// Cases where readabilityrs's extracted body content falls outside the
/// coarse length-similarity band used by `test_mozilla_suite_content`
/// (half to double Mozilla's expected text length). This is a floor check,
/// not parity, so these are logged divergences, not regressions.
/// Do not add to this list to silence a new regression.
const KNOWN_CONTENT_DIVERGENCES: &[&str] = &[
    "archive-of-our-own",
    "bug-1255978",
    "hukumusume",
    "mozilla-1",
    "yahoo-3",
];

/// Expected metadata from Mozilla test cases
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct ExpectedMetadata {
    title: Option<String>,
    byline: Option<String>,
    dir: Option<String>,
    lang: Option<String>,
    excerpt: Option<String>,
    site_name: Option<String>,
    published_time: Option<String>,
    #[serde(default)]
    readerable: bool,
}

/// A single test case from Mozilla's test suite
struct TestCase {
    name: String,
    source_html: String,
    expected_html: Option<String>,
    expected_metadata: ExpectedMetadata,
}

impl TestCase {
    fn load(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or("Invalid test case name")?
            .to_string();

        let source_html = fs::read_to_string(path.join("source.html"))?;

        let expected_html = if path.join("expected.html").exists() {
            Some(fs::read_to_string(path.join("expected.html"))?)
        } else {
            None
        };

        let expected_metadata: ExpectedMetadata =
            serde_json::from_str(&fs::read_to_string(path.join("expected-metadata.json"))?)?;

        Ok(TestCase {
            name,
            source_html,
            expected_html,
            expected_metadata,
        })
    }
}

fn load_test_cases() -> Vec<TestCase> {
    let test_dir = PathBuf::from("tests/test-pages");

    if !test_dir.exists() {
        eprintln!("Warning: Test directory not found at {:?}", test_dir);
        return vec![];
    }

    let mut test_cases = Vec::new();

    if let Ok(entries) = fs::read_dir(&test_dir) {
        for entry in entries.flatten() {
            if entry.path().is_dir() {
                match TestCase::load(&entry.path()) {
                    Ok(test_case) => test_cases.push(test_case),
                    Err(e) => eprintln!("Failed to load test case {:?}: {}", entry.path(), e),
                }
            }
        }
    }

    test_cases.sort_by(|a, b| a.name.cmp(&b.name));
    test_cases
}

/// Compare two optional strings, allowing for minor differences
fn strings_match(actual: &Option<String>, expected: &Option<String>) -> bool {
    match (actual, expected) {
        (None, None) => true,
        (Some(a), Some(e)) => {
            let a_normalized = a.split_whitespace().collect::<Vec<_>>().join(" ");
            let e_normalized = e.split_whitespace().collect::<Vec<_>>().join(" ");
            a_normalized == e_normalized
        }
        _ => false,
    }
}

/// Extract whitespace-normalized text length from an HTML fragment.
///
/// Uses the same extraction approach as the library itself so the length
/// comparison reflects comparable text, not serializer artifacts.
fn normalized_text_len(html: &str) -> usize {
    let doc = Html::parse_fragment(html);
    let text = doc.root_element().text().collect::<String>();
    text.split_whitespace().collect::<Vec<_>>().join(" ").len()
}

#[test]
fn test_mozilla_suite_metadata() {
    let test_cases = load_test_cases();

    assert_eq!(
        test_cases.len(),
        130,
        "expected to load all 130 Mozilla test-page directories, found {}",
        test_cases.len()
    );

    println!("\nRunning Mozilla Readability Test Suite");
    println!("======================================\n");
    println!("Total test cases: {}\n", test_cases.len());

    let mut passed = 0;
    let mut failed = 0;
    let mut failures = Vec::new();

    for test_case in &test_cases {
        let known_divergence = KNOWN_METADATA_DIVERGENCES.contains(&test_case.name.as_str());

        let result = Readability::new(&test_case.source_html, None, None);
        let readability = match result {
            Ok(r) => r,
            Err(e) => {
                let msg = format!(
                    "{}: Failed to create Readability instance: {}",
                    test_case.name, e
                );
                println!("❌ {}", msg);
                failed += 1;
                if !known_divergence {
                    failures.push(msg);
                }
                continue;
            }
        };

        let article = readability.parse();
        if test_case.expected_metadata.readerable && article.is_none() {
            let msg = format!(
                "{}: Expected readerable content but got None",
                test_case.name
            );
            println!("❌ {}", msg);
            failed += 1;
            if !known_divergence {
                failures.push(msg);
            }
            continue;
        }

        let mut metadata_matches = true;
        let mut mismatches = Vec::new();

        if let Some(ref article) = article {
            if !strings_match(&article.title, &test_case.expected_metadata.title) {
                metadata_matches = false;
                mismatches.push(format!(
                    "  - Title: expected {:?}, got {:?}",
                    test_case.expected_metadata.title, article.title
                ));
            }

            if !strings_match(&article.byline, &test_case.expected_metadata.byline) {
                metadata_matches = false;
                mismatches.push(format!(
                    "  - Byline: expected {:?}, got {:?}",
                    test_case.expected_metadata.byline, article.byline
                ));
            }

            if !strings_match(&article.excerpt, &test_case.expected_metadata.excerpt) {
                metadata_matches = false;
                mismatches.push(format!(
                    "  - Excerpt: expected {:?}, got {:?}",
                    test_case.expected_metadata.excerpt, article.excerpt
                ));
            }

            if !strings_match(&article.site_name, &test_case.expected_metadata.site_name) {
                metadata_matches = false;
                mismatches.push(format!(
                    "  - Site Name: expected {:?}, got {:?}",
                    test_case.expected_metadata.site_name, article.site_name
                ));
            }
        }

        if metadata_matches {
            println!("✅ {}", test_case.name);
            passed += 1;
        } else {
            println!("❌ {}: Metadata mismatch", test_case.name);
            for mismatch in &mismatches {
                println!("{}", mismatch);
            }
            failed += 1;
            if !known_divergence {
                failures.push(format!(
                    "{}: Metadata mismatch\n{}",
                    test_case.name,
                    mismatches.join("\n")
                ));
            }
        }
    }

    println!("\n======================================");
    println!("Results: {} passed, {} failed", passed, failed);
    println!(
        "Pass rate: {:.1}%",
        (passed as f64 / test_cases.len() as f64) * 100.0
    );

    assert!(
        failures.is_empty(),
        "{} case(s) regressed:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

#[test]
fn test_mozilla_suite_content() {
    let test_cases = load_test_cases();

    assert_eq!(
        test_cases.len(),
        130,
        "expected to load all 130 Mozilla test-page directories, found {}",
        test_cases.len()
    );

    let mut failures = Vec::new();
    let mut checked = 0;

    for test_case in &test_cases {
        let Some(ref expected_html) = test_case.expected_html else {
            continue;
        };
        if !test_case.expected_metadata.readerable {
            continue;
        }

        checked += 1;

        let readability = match Readability::new(&test_case.source_html, None, None) {
            Ok(r) => r,
            Err(e) => {
                failures.push(format!(
                    "{}: Failed to create Readability instance: {}",
                    test_case.name, e
                ));
                continue;
            }
        };

        let article = readability.parse();
        let Some(article) = article else {
            failures.push(format!(
                "{}: expected article content but got None",
                test_case.name
            ));
            continue;
        };

        let Some(ref content) = article.content else {
            failures.push(format!(
                "{}: article.content was None despite parse succeeding",
                test_case.name
            ));
            continue;
        };

        let actual_len = normalized_text_len(content);
        let expected_len = normalized_text_len(expected_html);

        // Unconditional, and deliberately outside the divergence allowlist: a
        // readerable page that yields an empty body is never an acceptable
        // divergence, and the band below cannot catch it when expected_len <= 1.
        if actual_len == 0 {
            failures.push(format!(
                "{}: extracted content is empty (expected ~{})",
                test_case.name, expected_len
            ));
            continue;
        }

        let within_band = actual_len >= expected_len / 2 && actual_len <= expected_len * 2;

        if !within_band && !KNOWN_CONTENT_DIVERGENCES.contains(&test_case.name.as_str()) {
            failures.push(format!(
                "{}: content length {} outside [{}, {}] band (expected ~{})",
                test_case.name,
                actual_len,
                expected_len / 2,
                expected_len * 2,
                expected_len
            ));
        }
    }

    println!(
        "Checked content length band for {} readerable cases with expected.html",
        checked
    );

    assert!(
        failures.is_empty(),
        "{} case(s) regressed:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

#[test]
#[ignore = "manual debugging helper, prints only"]
fn test_single_case_debug() {
    let test_name =
        std::env::var("MOZ_READABILITY_TEST").unwrap_or_else(|_| "replace-brs".to_string());

    let test_dir = PathBuf::from("tests/test-pages").join(&test_name);

    if !test_dir.exists() {
        println!("Test case '{}' not found", test_name);
        return;
    }

    let test_case = TestCase::load(&test_dir).expect("Failed to load test case");

    println!("\n=== Debugging Test Case: {} ===\n", test_case.name);
    println!("Expected metadata:");
    println!("{:#?}\n", test_case.expected_metadata);

    let readability =
        Readability::new(&test_case.source_html, None, None).expect("Failed to create Readability");

    let article = readability.parse();

    println!("Actual result (default options):");
    if let Some(ref article) = article {
        println!("Title: {:?}", article.title);
        println!("Byline: {:?}", article.byline);
        println!("Excerpt: {:?}", article.excerpt);
        println!("Site Name: {:?}", article.site_name);
        println!("Length: {}", article.length);

        if let Some(ref content) = article.content {
            println!("\nContent preview (first 500 chars):");
            println!("{}", &content.chars().take(500).collect::<String>());
        }
    } else {
        println!("No article extracted");

        println!("\n--- Trying with char_threshold=100 ---\n");
        let options = ReadabilityOptions::builder().char_threshold(100).build();

        let readability2 = Readability::new(&test_case.source_html, None, Some(options))
            .expect("Failed to create Readability");

        let article2 = readability2.parse();

        if let Some(ref art) = article2 {
            println!("SUCCESS with lower threshold!");
            println!("Length: {}", art.length);
            if let Some(ref content) = art.content {
                println!("\nContent preview (first 300 chars):");
                println!("{}", &content.chars().take(300).collect::<String>());
            }
        } else {
            println!("Still no article extracted");
        }
    }

    if let Some(ref expected_html) = test_case.expected_html {
        println!("\nExpected HTML preview (first 500 chars):");
        println!("{}", &expected_html.chars().take(500).collect::<String>());
    }
}
