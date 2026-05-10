use std::{fs, path::PathBuf};

use anyhow::{anyhow, Context, Result};
use leguard_core::run_validation;
use leguard_report::{render_report, ReportFormat};

use crate::commands::check::load_config;

#[derive(Debug, Clone)]
pub struct ReportCommand {
    pub path: PathBuf,
    pub format: String,
    pub out: PathBuf,
    pub config: Option<PathBuf>,
}

pub fn run(command: ReportCommand) -> Result<i32> {
    let config = load_config(command.config.as_deref())?;
    let report = run_validation(&command.path, config)
        .with_context(|| format!("failed to validate dataset {}", command.path.display()))?;
    let format = ReportFormat::parse(&command.format)
        .ok_or_else(|| anyhow!("unsupported report format: {}", command.format))?;

    let rendered = render_report(&report, format)?;
    fs::write(&command.out, rendered)
        .with_context(|| format!("failed to write report to {}", command.out.display()))?;

    println!("report written to {}", command.out.display());
    Ok(0)
}
