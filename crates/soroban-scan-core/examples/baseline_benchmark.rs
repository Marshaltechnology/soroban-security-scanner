//! Baseline comparison benchmark for large accepted-finding sets (#159).
//!
//! Run with:
//!
//! ```text
//! cargo run --release -p soroban-scan-core --example baseline_benchmark
//! ```
//!
//! This is a contributor measurement, not part of the test gate.

use std::time::Instant;

use soroban_scan_core::baseline::{self, Baseline};
use soroban_scan_core::category::Category;
use soroban_scan_core::confidence::Confidence;
use soroban_scan_core::finding::{Finding, SourceLocation};
use soroban_scan_core::rule::RuleMetadata;
use soroban_scan_core::severity::Severity;

fn main() {
    let meta = RuleMetadata::new(
        "SS-001",
        "Benchmark rule",
        "baseline comparison benchmark",
        Category::Authorization,
        Severity::High,
        Confidence::Medium,
        "fix it",
    );
    let findings: Vec<Finding> = (0..50_000)
        .map(|index| {
            Finding::new(
                &meta,
                SourceLocation::new("src/lib.rs", index + 1),
                format!("evidence-{index}"),
            )
        })
        .collect();
    let baseline = Baseline::from_findings(&findings[..40_000]);

    let start = Instant::now();
    let comparison = baseline::compare(&findings, &baseline);
    let elapsed = start.elapsed();

    println!(
        "findings={} baseline={} new={} existing={} resolved={} elapsed={elapsed:?}",
        findings.len(),
        baseline.findings.len(),
        comparison.new_count(),
        comparison.existing_count(),
        comparison.resolved_count(),
    );
}
