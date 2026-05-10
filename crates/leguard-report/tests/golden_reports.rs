use std::{fs, path::PathBuf};

use leguard_core::report::ValidationReport;
use leguard_report::{render_report, ReportFormat};

#[test]
fn markdown_report_matches_golden() {
    let report = load_sample_report();
    let rendered = render_report(&report, ReportFormat::Markdown).expect("markdown");
    let expected = fs::read_to_string(golden_path("report.md")).expect("golden markdown");
    assert_eq!(normalize_newlines(&rendered), normalize_newlines(&expected));
}

#[test]
fn junit_report_matches_golden() {
    let report = load_sample_report();
    let rendered = render_report(&report, ReportFormat::Junit).expect("junit");
    let expected = fs::read_to_string(golden_path("report.junit.xml")).expect("golden junit");
    assert_eq!(normalize_newlines(&rendered), normalize_newlines(&expected));
}

#[test]
fn html_report_matches_golden() {
    let report = load_sample_report();
    let rendered = render_report(&report, ReportFormat::Html).expect("html");
    let expected = fs::read_to_string(golden_path("report.html")).expect("golden html");
    assert_eq!(normalize_newlines(&rendered), normalize_newlines(&expected));
}

fn load_sample_report() -> ValidationReport {
    let raw = fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/leguard-report.sample.json"),
    )
    .expect("sample json");
    serde_json::from_str(&raw).expect("valid sample report")
}

fn golden_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden")
        .join(name)
}

fn normalize_newlines(input: &str) -> String {
    input.replace("\r\n", "\n").trim().to_string()
}
