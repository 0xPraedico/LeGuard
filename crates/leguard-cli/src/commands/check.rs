use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{anyhow, bail, Context, Result};
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
    pub checks: Option<String>,
    pub max_episodes: Option<usize>,
}

pub fn run(command: CheckCommand) -> Result<i32> {
    let mut config = load_config(command.config.as_deref())?;
    apply_cli_overrides(&mut config, &command)?;

    let resolved = resolve_dataset_input(&command.path)?;
    let report_result = run_validation(&resolved.dataset_path, config)
        .with_context(|| format!("failed to validate dataset {}", command.path.display()));
    if let Some(cleanup_dir) = resolved.cleanup_dir.as_ref() {
        let _ = fs::remove_dir_all(cleanup_dir);
    }
    let report = report_result?;

    println!("leguard check");
    if let Some(repo_id) = resolved.repo_id.as_ref() {
        println!("source: hf://{repo_id}");
    }
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

const CHECK_NAMES: [&str; 10] = [
    "structure",
    "schema",
    "temporal",
    "video",
    "numerical",
    "annotation",
    "consistency",
    "episodes",
    "training",
    "portability",
];

#[derive(Debug, Clone)]
struct ResolvedDataset {
    dataset_path: PathBuf,
    cleanup_dir: Option<PathBuf>,
    repo_id: Option<String>,
}

fn apply_cli_overrides(config: &mut ValidationConfig, command: &CheckCommand) -> Result<()> {
    if let Some(max_episodes) = command.max_episodes {
        config.max_episodes = if max_episodes == 0 {
            None
        } else {
            Some(max_episodes)
        };
    }
    if let Some(raw_checks) = command.checks.as_deref() {
        apply_checks_override(config, raw_checks)?;
    }
    Ok(())
}

fn apply_checks_override(config: &mut ValidationConfig, raw_checks: &str) -> Result<()> {
    let mut parsed = raw_checks
        .split(',')
        .map(|value| value.trim().to_lowercase())
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    parsed.sort();
    parsed.dedup();

    if parsed.is_empty() {
        bail!(
            "--checks requires a comma-separated list (available: {})",
            CHECK_NAMES.join(",")
        );
    }

    config.checks.structure = false;
    config.checks.schema = false;
    config.checks.temporal = false;
    config.checks.video = false;
    config.checks.numerical = false;
    config.checks.annotation = false;
    config.checks.consistency = false;
    config.checks.episodes = false;
    config.checks.training = false;
    config.checks.portability = false;

    for check in parsed {
        match check.as_str() {
            "structure" => config.checks.structure = true,
            "schema" => config.checks.schema = true,
            "temporal" => config.checks.temporal = true,
            "video" => config.checks.video = true,
            "numerical" => config.checks.numerical = true,
            "annotation" => config.checks.annotation = true,
            "consistency" => config.checks.consistency = true,
            "episodes" => config.checks.episodes = true,
            "training" => config.checks.training = true,
            "portability" => config.checks.portability = true,
            _ => {
                bail!(
                    "unknown check '{}'. Available checks: {}",
                    check,
                    CHECK_NAMES.join(",")
                );
            }
        }
    }

    Ok(())
}

fn resolve_dataset_input(path_or_repo: &Path) -> Result<ResolvedDataset> {
    if path_or_repo.exists() {
        if !path_or_repo.is_dir() {
            bail!(
                "dataset path '{}' exists but is not a directory",
                path_or_repo.display()
            );
        }
        return Ok(ResolvedDataset {
            dataset_path: path_or_repo.to_path_buf(),
            cleanup_dir: None,
            repo_id: None,
        });
    }

    let repo_id = path_or_repo.to_string_lossy().trim().to_string();
    if !looks_like_hf_repo_id(&repo_id) {
        bail!(
            "dataset path '{}' does not exist and is not a valid Hugging Face repo_id",
            path_or_repo.display()
        );
    }

    let local_dir = download_hf_dataset(&repo_id)?;
    Ok(ResolvedDataset {
        dataset_path: local_dir.clone(),
        cleanup_dir: Some(local_dir),
        repo_id: Some(repo_id),
    })
}

fn looks_like_hf_repo_id(value: &str) -> bool {
    !value.is_empty() && value.contains('/') && !value.contains('\\') && !value.contains(' ')
}

fn download_hf_dataset(repo_id: &str) -> Result<PathBuf> {
    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0);
    let local_dir =
        std::env::temp_dir().join(format!("leguard-hf-{now_ms}-{}", std::process::id()));
    fs::create_dir_all(&local_dir).with_context(|| {
        format!(
            "failed to create temp download directory {}",
            local_dir.display()
        )
    })?;

    let output = Command::new("hf")
        .args([
            "download",
            repo_id,
            "--repo-type",
            "dataset",
            "--local-dir",
            &local_dir.to_string_lossy(),
        ])
        .output()
        .map_err(|error| {
            let _ = fs::remove_dir_all(&local_dir);
            anyhow!(
                "failed to run 'hf download' for {}. Ensure the Hugging Face CLI is installed: {}",
                repo_id,
                error
            )
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let _ = fs::remove_dir_all(&local_dir);
        bail!(
            "hf download failed for {}:\n{}\n{}",
            repo_id,
            stdout.trim(),
            stderr.trim()
        );
    }

    Ok(local_dir)
}
