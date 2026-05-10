pub mod checks;
pub mod config;
pub mod dataset;
pub mod diff;
pub mod errors;
pub mod issue;
pub mod report;

use std::path::Path;

use anyhow::Result;
use chrono::Utc;
use config::ValidationConfig;
use dataset::scan_dataset;
use report::{DatasetSummary, IssueCounts, RunMetadata, ValidationReport};

use crate::checks::{
    annotation, consistency, episodes, numerical, portability, schema, structure, temporal,
    training, video,
};

pub const REPORT_SCHEMA_VERSION: &str = "1.0.0";

pub fn run_validation(path: &Path, config: ValidationConfig) -> Result<ValidationReport> {
    let started_at = Utc::now();
    let mut issues = Vec::new();

    let scan = if path.exists() && path.is_dir() {
        Some(scan_dataset(path)?)
    } else {
        None
    };

    issues.extend(structure::run(path, scan.as_ref()));

    if let Some(scan) = scan.as_ref() {
        if config.checks.schema {
            issues.extend(schema::run(scan));
        }
        if config.checks.consistency {
            issues.extend(consistency::run(scan, &config));
        }
        if config.checks.episodes {
            issues.extend(episodes::run(scan, &config));
        }
        if config.checks.temporal {
            issues.extend(temporal::run(scan, &config));
        }
        if config.checks.numerical {
            issues.extend(numerical::run(scan, &config));
        }
        if config.checks.video {
            issues.extend(video::run(scan));
        }
        if config.checks.annotation {
            issues.extend(annotation::run(scan));
        }
        if config.checks.training {
            issues.extend(training::run(scan, &config));
        }
        if config.checks.portability {
            issues.extend(portability::run(scan));
        }
    }

    issues.sort_by_key(|issue| match issue.severity {
        issue::Severity::Error => 0,
        issue::Severity::Warning => 1,
        issue::Severity::Info => 2,
    });

    let issue_counts = IssueCounts::from_issues(&issues);
    let finished_at = Utc::now();

    let dataset = scan
        .as_ref()
        .map(|scan| scan.summarize(config.expected_format.as_deref()))
        .unwrap_or_else(DatasetSummary::default);

    Ok(ValidationReport {
        schema_version: REPORT_SCHEMA_VERSION.to_string(),
        leguard_version: env!("CARGO_PKG_VERSION").to_string(),
        dataset,
        run: RunMetadata {
            source: "cli".to_string(),
            git_commit: std::env::var("GIT_COMMIT").ok(),
            git_branch: std::env::var("GIT_BRANCH").ok(),
            git_repo: std::env::var("GIT_REPO").ok(),
            dataset_revision: std::env::var("DATASET_REVISION").ok(),
            started_at,
            finished_at,
        },
        issue_counts,
        issues,
        generated_at: Utc::now(),
    })
}
