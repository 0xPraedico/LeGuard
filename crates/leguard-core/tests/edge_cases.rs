use std::path::PathBuf;

use leguard_core::{config::ValidationConfig, run_validation};
use tempfile::TempDir;

#[test]
fn missing_path_returns_structure_error() {
    let temp = TempDir::new().expect("tempdir");
    let missing = temp.path().join("does-not-exist");

    let report = run_validation(&missing, ValidationConfig::default()).expect("validation report");

    assert!(report
        .issues
        .iter()
        .any(|issue| issue.check_id == "structure.path_not_found"));
    assert!(report.issue_counts.errors >= 1);
}

#[test]
fn empty_dataset_reports_missing_content() {
    let temp = TempDir::new().expect("tempdir");

    let report =
        run_validation(temp.path(), ValidationConfig::default()).expect("validation report");

    assert!(report
        .issues
        .iter()
        .any(|issue| issue.check_id == "structure.no_metadata_files"));
    assert!(report
        .issues
        .iter()
        .any(|issue| issue.check_id == "structure.no_parquet_files"));
}

#[test]
fn broken_parquet_fixture_is_detected() {
    let fixture = fixture_path("examples/fixtures/dataset-broken");
    let report = run_validation(&fixture, ValidationConfig::default()).expect("validation report");

    assert!(report
        .issues
        .iter()
        .any(|issue| issue.check_id == "schema.unreadable_parquet"));
}

fn fixture_path(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative)
}
