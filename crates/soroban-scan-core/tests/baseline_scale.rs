//! Large-baseline comparison regression (#159).

use std::time::{Duration, Instant};

use soroban_scan_core::baseline::{self, Baseline};
use soroban_scan_core::category::Category;
use soroban_scan_core::confidence::Confidence;
use soroban_scan_core::finding::{Finding, SourceLocation};
use soroban_scan_core::rule::RuleMetadata;
use soroban_scan_core::severity::Severity;

#[test]
fn large_baseline_comparison_stays_linear_and_correct() {
    let meta = RuleMetadata::new(
        "SS-001",
        "Scale rule",
        "large baseline regression",
        Category::Authorization,
        Severity::High,
        Confidence::Medium,
        "fix it",
    );
    let findings: Vec<Finding> = (0..20_000)
        .map(|index| {
            Finding::new(
                &meta,
                SourceLocation::new("src/lib.rs", index + 1),
                format!("evidence-{index}"),
            )
        })
        .collect();
    let baseline = Baseline::from_findings(&findings[..15_000]);

    let start = Instant::now();
    let comparison = baseline::compare(&findings, &baseline);
    let elapsed = start.elapsed();

    assert_eq!(comparison.existing_count(), 15_000);
    assert_eq!(comparison.new_count(), 5_000);
    assert_eq!(comparison.resolved_count(), 0);
    assert!(
        elapsed < Duration::from_secs(10),
        "20k-finding comparison took {elapsed:?}; expected a linear pass"
    );
}
