//! Machine-readable baseline status in JSON and SARIF (#155).

use std::path::PathBuf;

use serde_json::Value;

use soroban_scan_core::baseline::{self, Baseline, BaselineEntry, BaselineReport};
use soroban_scan_core::config::ScanConfig;
use soroban_scan_core::engine;
use soroban_scan_core::registry::RuleRegistry;
use soroban_scan_core::report;
use soroban_scan_core::severity::Severity;

fn fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/corpus/SS-001/positive.rs")
}

#[test]
fn json_and_sarif_carry_baseline_summary_and_per_finding_status() {
    let source = std::fs::read_to_string(fixture()).expect("positive fixture");
    let registry = RuleRegistry::with_default_rules();
    let config = ScanConfig::default();
    let mut outcome = engine::scan_source_str(&source, "SS-001/positive.rs", &config, &registry);
    assert!(
        outcome.findings.len() >= 2,
        "fixture must produce at least two findings"
    );

    let mut baseline = Baseline::from_findings(&outcome.findings[..1]);
    baseline.findings.push(BaselineEntry {
        fingerprint: "resolved-only-fingerprint".to_string(),
        rule_id: "SS-001".to_string(),
        file: "SS-001/positive.rs".to_string(),
        severity: Severity::High,
        title: "Resolved finding".to_string(),
    });
    let comparison = baseline::compare(&outcome.findings, &baseline);
    assert_eq!(comparison.new_count(), outcome.findings.len() - 1);
    assert_eq!(comparison.existing_count(), 1);
    assert_eq!(comparison.resolved_count(), 1);
    outcome.baseline = Some(BaselineReport::from_comparison(
        &comparison,
        &outcome.findings,
    ));

    let json: Value = serde_json::from_str(&report::render_json(&outcome).unwrap()).unwrap();
    assert_eq!(json["baseline"]["new"], outcome.findings.len() - 1);
    assert_eq!(json["baseline"]["existing"], 1);
    assert_eq!(json["baseline"]["resolved"], 1);
    let statuses: Vec<&str> = json["findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|finding| finding["baseline_status"].as_str().unwrap())
        .collect();
    assert_eq!(
        statuses.iter().filter(|status| **status == "new").count(),
        outcome.findings.len() - 1
    );
    assert_eq!(
        statuses
            .iter()
            .filter(|status| **status == "existing")
            .count(),
        1
    );

    let sarif: Value =
        serde_json::from_str(&report::render_sarif(&outcome, &registry)).expect("SARIF JSON");
    assert_eq!(sarif["runs"][0]["properties"]["baseline"]["resolved"], 1);
    let result_statuses: Vec<&str> = sarif["runs"][0]["results"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|result| result["properties"]["baselineStatus"].as_str())
        .collect();
    assert!(result_statuses.contains(&"existing"));
    assert!(result_statuses.contains(&"new"));
}
