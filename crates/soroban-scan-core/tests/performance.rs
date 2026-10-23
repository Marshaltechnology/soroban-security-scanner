//! Bounded-performance regression tests for pathological inputs.
//!
//! These guard against algorithmic regressions (for example quadratic helper
//! expansion) and non-termination on adversarial source. They assert generous
//! upper bounds so they are not flaky on slow CI machines.

use std::time::{Duration, Instant};

use soroban_scan_core::baseline::{compare, Baseline, BaselineEntry, BASELINE_VERSION};
use soroban_scan_core::category::Category;
use soroban_scan_core::confidence::Confidence;
use soroban_scan_core::config::ScanConfig;
use soroban_scan_core::engine;
use soroban_scan_core::finding::{Finding, SourceLocation};
use soroban_scan_core::registry::RuleRegistry;
use soroban_scan_core::rule::RuleMetadata;
use soroban_scan_core::severity::Severity;

fn scan(src: &str) -> engine::ScanOutcome {
    let registry = RuleRegistry::with_default_rules();
    engine::scan_source_str(src, "performance.rs", &ScanConfig::default(), &registry)
}

#[test]
fn deeply_chained_helpers_terminate_quickly() {
    // A long call chain exercises intra-file helper summaries. The walk is
    // bounded by depth and a visited set, so this must be fast and terminate.
    let n = 1_000;
    let mut src = String::from("#[contractimpl]\nimpl C {\n");
    src.push_str("    pub fn entry(env: Env) { Self::h0(&env); }\n");
    for i in 0..n {
        if i + 1 < n {
            src.push_str(&format!(
                "    fn h{i}(env: &Env) {{ Self::h{}(env); }}\n",
                i + 1
            ));
        } else {
            src.push_str(&format!("    fn h{i}(_env: &Env) {{ }}\n"));
        }
    }
    src.push_str("}\n");

    let start = Instant::now();
    let _ = scan(&src);
    let elapsed = start.elapsed();
    assert!(
        elapsed < Duration::from_secs(20),
        "chained-helper scan took too long: {elapsed:?}"
    );
}

#[test]
fn many_functions_with_cross_calls_scale() {
    // Guards against recomputing the whole-file summary for every entry point.
    let n = 1_500;
    let mut src = String::from("#[contractimpl]\nimpl C {\n");
    for i in 0..n {
        src.push_str(&format!(
            "    pub fn f{i}(env: Env) {{ Self::helper_{i}(&env); env.storage().persistent().set(&K{i}, &{i}); }}\n"
        ));
        src.push_str(&format!(
            "    fn helper_{i}(env: &Env) {{ let _ = env; }}\n"
        ));
    }
    src.push_str("}\n");

    let start = Instant::now();
    let _ = scan(&src);
    let elapsed = start.elapsed();
    assert!(
        elapsed < Duration::from_secs(30),
        "many-function scan took too long: {elapsed:?}"
    );
}

#[test]
fn baseline_comparison_is_linear_and_reuses_its_index() {
    const N: usize = 100_000;

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

    let baseline = Baseline {
        version: BASELINE_VERSION,
        tool: "soroban-scan".to_string(),
        tool_version: "0.0.0".to_string(),
        format: None,
        findings: (0..N)
            .map(|i| BaselineEntry {
                fingerprint: format!("fp-{i:08}"),
                rule_id: "SS-001".to_string(),
                file: "src/lib.rs".to_string(),
                severity: Severity::High,
                title: "Test".to_string(),
            })
            .collect(),
    };

    // Even indexes are known to the baseline; odd indexes are new.
    let findings: Vec<Finding> = (0..N)
        .map(|i| {
            let mut finding = template.clone();
            finding.fingerprint = if i % 2 == 0 {
                format!("fp-{i:08}")
            } else {
                format!("new-{i:08}")
            };
            finding
        })
        .collect();

    let start = Instant::now();
    let comparison = compare(&findings, &baseline);
    let elapsed = start.elapsed();

    assert_eq!(comparison.existing_count(), N / 2);
    assert_eq!(comparison.new_count(), N / 2);
    assert_eq!(comparison.resolved_count(), N / 2);
    assert!(
        elapsed < Duration::from_secs(30),
        "large baseline comparison took too long: {elapsed:?}"
    );
}
