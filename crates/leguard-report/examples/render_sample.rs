use std::{fs, path::PathBuf};

use leguard_core::report::ValidationReport;
use leguard_report::{render_report, ReportFormat};

fn main() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let sample = root.join("examples/leguard-report.sample.json");
    let raw = fs::read_to_string(sample).expect("sample");
    let report: ValidationReport = serde_json::from_str(&raw).expect("json");

    let golden_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden");
    fs::create_dir_all(&golden_dir).expect("mkdir");

    let markdown = render_report(&report, ReportFormat::Markdown).expect("markdown");
    let junit = render_report(&report, ReportFormat::Junit).expect("junit");
    let html = render_report(&report, ReportFormat::Html).expect("html");

    fs::write(golden_dir.join("report.md"), markdown).expect("write md");
    fs::write(golden_dir.join("report.junit.xml"), junit).expect("write junit");
    fs::write(golden_dir.join("report.html"), html).expect("write html");
}
