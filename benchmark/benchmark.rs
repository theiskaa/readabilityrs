//! Rust benchmark runner that outputs JSON results for comparison
//!
//! This can be run with: cargo run --release --example rust_benchmark
//! Or compiled and run directly.

use readabilityrs::Readability;
use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::time::Instant;

#[derive(Serialize)]
struct BenchmarkResult {
    mean: f64,
    median: f64,
    min: f64,
    max: f64,
    p95: f64,
    iterations: usize,
    size: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    size_category: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    document_count: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    total_size: Option<usize>,
}

#[derive(Serialize)]
struct Results {
    single: HashMap<String, BenchmarkResult>,
    batch: Option<BenchmarkResult>,
    large: HashMap<String, BenchmarkResult>,
}

fn load_test_case(name: &str) -> Option<String> {
    let path = Path::new("tests/test-pages").join(name).join("source.html");
    fs::read_to_string(&path).ok()
}

fn benchmark<F>(f: F, iterations: usize) -> BenchmarkResult
where
    F: Fn(),
{
    for _ in 0..5 {
        f();
    }

    let mut times: Vec<f64> = Vec::with_capacity(iterations);
    for _ in 0..iterations {
        let start = Instant::now();
        f();
        let elapsed = start.elapsed();
        times.push(elapsed.as_secs_f64() * 1000.0); // Convert to ms
    }

    times.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let mean = times.iter().sum::<f64>() / times.len() as f64;
    let median = times[times.len() / 2];
    let min = times[0];
    let max = times[times.len() - 1];
    let p95 = times[(times.len() as f64 * 0.95) as usize];

    BenchmarkResult {
        mean,
        median,
        min,
        max,
        p95,
        iterations,
        size: 0,
        size_category: None,
        document_count: None,
        total_size: None,
    }
}

fn format_ms(ms: f64) -> String {
    if ms < 1.0 {
        format!("{:.2} µs", ms * 1000.0)
    } else {
        format!("{:.2} ms", ms)
    }
}

fn format_bytes(bytes: usize) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.2} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

fn main() {
    // Note: wikipedia-2 is excluded due to its unusual DOM structure causing slow processing
    let test_cases: HashMap<&str, Vec<&str>> = [
        ("small", vec!["001", "002", "aclu"]),
        ("medium", vec!["medium-1", "nytimes-1", "ars-1"]),
        ("large", vec!["guardian-1", "yahoo-2"]),
    ]
    .into_iter()
    .collect();

    let batch_cases = vec![
        "001",
        "002",
        "aclu",
        "ars-1",
        "bbc-1",
        "buzzfeed-1",
        "cnet",
        "cnn",
        "ehow-1",
        "herald-sun-1",
    ];

    let mut results = Results {
        single: HashMap::new(),
        batch: None,
        large: HashMap::new(),
    };

    println!("{}", "=".repeat(70));
    println!("ReadabilityRS Benchmark (Rust)");
    println!("{}", "=".repeat(70));
    println!();

    println!("Single Document Parsing");
    println!("{}", "-".repeat(70));

    for (size_category, cases) in &test_cases {
        for test_case in cases {
            let html = match load_test_case(test_case) {
                Some(h) => h,
                None => {
                    println!("Skipping {}: file not found", test_case);
                    continue;
                }
            };

            let iterations = if *size_category == "large" { 20 } else { 100 };
            let html_clone = html.clone();

            let mut result = benchmark(
                || {
                    let readability = Readability::new(&html_clone, None, None).unwrap();
                    let _ = readability.parse();
                },
                iterations,
            );

            result.size = html.len();
            result.size_category = Some(size_category.to_string());

            println!(
                "{:20} ({:>10}) | mean: {:>12} | median: {:>12} | p95: {:>12}",
                test_case,
                format_bytes(html.len()),
                format_ms(result.mean),
                format_ms(result.median),
                format_ms(result.p95)
            );

            if *size_category == "large" {
                results.large.insert(test_case.to_string(), result);
            } else {
                results.single.insert(test_case.to_string(), result);
            }
        }
    }

    println!();

    println!("Batch Processing (10 documents)");
    println!("{}", "-".repeat(70));

    let batch_docs: Vec<String> = batch_cases
        .iter()
        .filter_map(|name| load_test_case(name))
        .collect();

    if !batch_docs.is_empty() {
        let total_size: usize = batch_docs.iter().map(|h| h.len()).sum();

        let mut result = benchmark(
            || {
                for html in &batch_docs {
                    let readability = Readability::new(html, None, None).unwrap();
                    let _ = readability.parse();
                }
            },
            50,
        );

        result.document_count = Some(batch_docs.len());
        result.total_size = Some(total_size);
        result.size = total_size;

        println!(
            "{} documents ({} total) | mean: {} | per-doc avg: {}",
            batch_docs.len(),
            format_bytes(total_size),
            format_ms(result.mean),
            format_ms(result.mean / batch_docs.len() as f64)
        );

        results.batch = Some(result);
    }

    println!();

    println!("Large Document Parsing");
    println!("{}", "-".repeat(70));

    for test_case in &["guardian-1", "yahoo-2"] {
        if let Some(result) = results.large.get(*test_case) {
            let throughput = result.size as f64 / result.mean / 1024.0;
            println!(
                "{:20} ({:>10}) | mean: {:>12} | throughput: {:.2} KB/ms",
                test_case,
                format_bytes(result.size),
                format_ms(result.mean),
                throughput
            );
        }
    }

    println!();
    println!("{}", "=".repeat(70));

    let json = serde_json::to_string_pretty(&results).unwrap();
    let output_path = "benchmark/rust-results.json";
    fs::write(output_path, &json).unwrap();
    println!("Results saved to: {}", output_path);
}
