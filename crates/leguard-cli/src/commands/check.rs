use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::ValueEnum;
use leguard_core::{config::ValidationConfig, run_validation};

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum FailOn {
    Error,
    Warning,
}

#[derive(Debug, Clone)]
pub struct CheckCommand {
    pub path: PathBuf,
    pub config: Option<PathBuf>,
    pub fail_on: FailOn,
}

pub fn run(command: CheckCommand) -> Result<i32> {
    let config = load_config(command.config.as_deref())?;
    let report = run_validation(&command.path, config)
        .with_context(|| format!("failed to validate dataset {}", command.path.display()))?;

    println!("leguard check");
    println!("dataset: {}", report.dataset.uri);
    println!("status: {:?}", report.status());
    println!(
        "issues: total={} errors={} warnings={} infos={}",
        report.issue_counts.total,
        report.issue_counts.errors,
        report.issue_counts.warnings,
        report.issue_counts.infos
    );

    let mut should_fail = false;
    match command.fail_on {
        FailOn::Error if report.issue_counts.errors > 0 => should_fail = true,
        FailOn::Warning if report.issue_counts.errors > 0 || report.issue_counts.warnings > 0 => {
            should_fail = true
        }
        _ => {}
    }

    Ok(if should_fail { 1 } else { 0 })
}

pub fn load_config(path: Option<&Path>) -> Result<ValidationConfig> {
    match path {
        Some(path) => ValidationConfig::from_yaml_file(path),
        None => Ok(ValidationConfig::default()),
    }
}
