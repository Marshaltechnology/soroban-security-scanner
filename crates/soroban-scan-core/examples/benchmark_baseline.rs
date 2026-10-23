//! Dependency-free benchmark for large-baseline parsing and comparison (#159).
//!
//! Run with:
//!
//! ```text
//! cargo run --release -p soroban-scan-core --example benchmark_baseline
//! ```
//!
//! Synthetic baselines are generated in memory; no fixture files are written.
//! Peak resident memory is sampled from `/proc/self/status` (`VmHWM`) on Linux
//! and printed as `n/a` elsewhere.

use std::time::Instant;

use soroban_scan_core::baseline::{compare, Baseline, BaselineEntry, BASELINE_VERSION};
use soroban_scan_core::category::Category;
use soroban_scan_core::confidence::Confidence;
use soroban_scan_core::finding::{Finding, SourceLocation};
use soroban_scan_core::rule::RuleMetadata;
use soroban_scan_core::severity::Severity;

#[cfg(target_os = "linux")]
fn peak_rss_bytes() -> Option<u64> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    for line in status.lines() {
        if let Some(rest) = line.strip_prefix("VmHWM:") {
            let kb: u64 = rest.split_whitespace().next()?.parse().ok()?;
            return Some(kb.saturating_mul(1024));
        }
    }
    None
}

#[cfg(not(target_os = "linux"))]
fn peak_rss_bytes() -> Option<u64> {
    None
}

fn format_bytes(bytes: Option<u64>) -> String {
    match bytes {
        Some(bytes) if bytes >= 1024 * 1024 => {
            format!("{:.1} MiB", bytes as f64 / (1024.0 * 1024.0))
        }
        Some(bytes) => format!("{} KiB", bytes / 1024),
        None => "n/a".to_string(),
    }
}

fn entries(n: usize) -> Vec<BaselineEntry> {
    (0..n)
        .map(|i| BaselineEntry {
            fingerprint: format!("fp-{i:010}"),
            rule_id: "SS-001".to_string(),
            file: format!("src/module_{}/lib.rs", i % 100),
            severity: Severity::High,
            title: "Synthetic finding".to_string(),
        })
        .collect()
}

fn main() {
    let meta = RuleMetadata::new(
        "SS-001",
        "Test",
        "test description long enough for validation purposes",
        Category::Authorization,
        Severity::High,
        Confidence::Medium,
        "fix",
    );
    let template = Finding::new(&meta, SourceLocation::new("src/lib.rs", 1), "evidence");

    println!(
        "{:>10} {:>12} {:>10} {:>10} {:>12} {:>10} {:>10}",
        "entries", "json_bytes", "parse_ms", "index_ms", "compare_ms", "resolved", "peak_rss"
    );

    for n in [10_000usize, 50_000, 200_000] {
        let baseline = Baseline {
            version: BASELINE_VERSION,
            tool: "soroban-scan".to_string(),
            tool_version: "0.0.0".to_string(),
            format: None,
            findings: entries(n),
        };
        let json = baseline.to_json().expect("serialize baseline");

        let start = Instant::now();
        let parsed = Baseline::from_json(&json).expect("parse baseline");
        let parse_ms = start.elapsed().as_secs_f64() * 1000.0;

        let start = Instant::now();
        let index = parsed.index();
        let index_ms = start.elapsed().as_secs_f64() * 1000.0;
        assert_eq!(index.len(), n);

        // Half of the scanned findings are known; the other half are new.
        let findings: Vec<Finding> = (0..n)
            .map(|i| {
                let mut finding = template.clone();
                finding.fingerprint = if i % 2 == 0 {
                    format!("fp-{i:010}")
                } else {
                    format!("new-{i:010}")
                };
                finding
            })
            .collect();

        let start = Instant::now();
        let comparison = compare(&findings, &parsed);
        let compare_ms = start.elapsed().as_secs_f64() * 1000.0;

        println!(
            "{:>10} {:>12} {:>10.1} {:>10.1} {:>12.1} {:>10} {:>10}",
            n,
            json.len(),
            parse_ms,
            index_ms,
            compare_ms,
            comparison.resolved_count(),
            format_bytes(peak_rss_bytes()),
        );
    }
}
