mod commands;

use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};
use commands::{
    check::{CheckCommand, FailOn},
    diff::DiffCommand,
    report::ReportCommand,
};

#[derive(Debug, Parser)]
#[command(name = "leguard", version, about = "Dataset QA for LeRobot datasets")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Check {
        path: PathBuf,
        #[arg(long)]
        config: Option<PathBuf>,
        #[arg(long, value_enum, default_value_t = FailOn::Error)]
        fail_on: FailOn,
    },
    Report {
        path: PathBuf,
        #[arg(long, default_value = "json")]
        format: String,
        #[arg(long)]
        out: PathBuf,
        #[arg(long)]
        config: Option<PathBuf>,
    },
    Diff {
        base_report_json: PathBuf,
        head_report_json: PathBuf,
        #[arg(long, default_value = "text")]
        format: String,
    },
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            std::env::var("LEGUARD_LOG")
                .or_else(|_| std::env::var("RUST_LOG"))
                .unwrap_or_else(|_| "info".to_string()),
        )
        .without_time()
        .init();

    let cli = Cli::parse();
    let exit_code = match cli.command {
        Commands::Check {
            path,
            config,
            fail_on,
        } => commands::check::run(CheckCommand {
            path,
            config,
            fail_on,
        })?,
        Commands::Report {
            path,
            format,
            out,
            config,
        } => commands::report::run(ReportCommand {
            path,
            format,
            out,
            config,
        })?,
        Commands::Diff {
            base_report_json,
            head_report_json,
            format,
        } => commands::diff::run(DiffCommand {
            base: base_report_json,
            head: head_report_json,
            format,
        })?,
    };

    std::process::exit(exit_code);
}
