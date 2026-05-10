use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::issue::{Issue, Severity};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationReport {
    pub schema_version: String,
    pub leguard_version: String,
    pub dataset: DatasetSummary,
    pub run: RunMetadata,
    pub issue_counts: IssueCounts,
    pub issues: Vec<Issue>,
    pub generated_at: DateTime<Utc>,
}

impl ValidationReport {
    pub fn status(&self) -> RunStatus {
        if self.issue_counts.errors > 0 {
            RunStatus::Failed
        } else if self.issue_counts.warnings > 0 {
            RunStatus::Warning
        } else {
            RunStatus::Passed
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetSummary {
    pub name: String,
    pub format: String,
    pub uri: String,
    pub source_type: String,
    pub episodes: u64,
    pub frames: u64,
    pub duration_sec: f64,
    pub cameras: Vec<String>,
    pub parquet_files: u64,
    pub video_files: u64,
    pub metadata_files: u64,
    pub empty_files: u64,
    pub total_files: u64,
}

impl Default for DatasetSummary {
    fn default() -> Self {
        Self {
            name: "unknown".to_string(),
            format: "unknown".to_string(),
            uri: "local://unknown".to_string(),
            source_type: "local".to_string(),
            episodes: 0,
            frames: 0,
            duration_sec: 0.0,
            cameras: Vec::new(),
            parquet_files: 0,
            video_files: 0,
            metadata_files: 0,
            empty_files: 0,
            total_files: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunMetadata {
    pub source: String,
    pub git_commit: Option<String>,
    pub git_branch: Option<String>,
    pub git_repo: Option<String>,
    pub dataset_revision: Option<String>,
    pub started_at: DateTime<Utc>,
    pub finished_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct IssueCounts {
    pub total: usize,
    pub errors: usize,
    pub warnings: usize,
    pub infos: usize,
}

impl IssueCounts {
    pub fn from_issues(issues: &[Issue]) -> Self {
        let mut counts = IssueCounts {
            total: issues.len(),
            ..IssueCounts::default()
        };
        for issue in issues {
            match issue.severity {
                Severity::Error => counts.errors += 1,
                Severity::Warning => counts.warnings += 1,
                Severity::Info => counts.infos += 1,
            }
        }
        counts
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RunStatus {
    Passed,
    Warning,
    Failed,
}
