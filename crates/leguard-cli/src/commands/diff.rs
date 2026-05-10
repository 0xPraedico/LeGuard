use std::{fs, path::PathBuf};

use anyhow::{Context, Result};
use leguard_core::{diff::diff_reports, report::ValidationReport};

#[derive(Debug, Clone)]
pub struct DiffCommand {
    pub base: PathBuf,
    pub head: PathBuf,
    pub format: String,
}

pub fn run(command: DiffCommand) -> Result<i32> {
    let base = load_report(&command.base)?;
    let head = load_report(&command.head)?;
    let diff = diff_reports(&base, &head);

    if command.format.eq_ignore_ascii_case("json") {
        println!("{}", serde_json::to_string_pretty(&diff)?);
    } else {
        println!(
            "new issues: {} | fixed issues: {} | persistent issues: {}",
            diff.new_issues.len(),
            diff.fixed_issues.len(),
            diff.persistent_issues.len()
        );
        println!(
            "error delta: {} | warning delta: {}",
            diff.error_delta, diff.warning_delta
        );
    }

    Ok(0)
}

fn load_report(path: &PathBuf) -> Result<ValidationReport> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("failed to read report {}", path.display()))?;
    serde_json::from_str(&raw).with_context(|| format!("invalid report json {}", path.display()))
}
