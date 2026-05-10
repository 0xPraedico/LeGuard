use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::{
    issue::{Issue, IssueCategory, Severity},
    report::ValidationReport,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffReport {
    pub error_delta: i64,
    pub warning_delta: i64,
    pub new_issues: Vec<Issue>,
    pub fixed_issues: Vec<Issue>,
    pub persistent_issues: Vec<Issue>,
    pub category_deltas: HashMap<String, i64>,
}

pub fn diff_reports(base: &ValidationReport, head: &ValidationReport) -> DiffReport {
    let base_map: HashMap<&str, &Issue> = base
        .issues
        .iter()
        .map(|issue| (issue.fingerprint.as_str(), issue))
        .collect();
    let head_map: HashMap<&str, &Issue> = head
        .issues
        .iter()
        .map(|issue| (issue.fingerprint.as_str(), issue))
        .collect();

    let base_set: HashSet<&str> = base_map.keys().copied().collect();
    let head_set: HashSet<&str> = head_map.keys().copied().collect();

    let new_issues = head_set
        .difference(&base_set)
        .filter_map(|fingerprint| head_map.get(fingerprint).copied().cloned())
        .collect::<Vec<_>>();

    let fixed_issues = base_set
        .difference(&head_set)
        .filter_map(|fingerprint| base_map.get(fingerprint).copied().cloned())
        .collect::<Vec<_>>();

    let persistent_issues = base_set
        .intersection(&head_set)
        .filter_map(|fingerprint| head_map.get(fingerprint).copied().cloned())
        .collect::<Vec<_>>();

    let mut category_deltas = HashMap::new();
    for category in [
        IssueCategory::Structure,
        IssueCategory::Schema,
        IssueCategory::Temporal,
        IssueCategory::Video,
        IssueCategory::Numerical,
        IssueCategory::Annotation,
        IssueCategory::Diff,
        IssueCategory::Compatibility,
        IssueCategory::Performance,
        IssueCategory::Unknown,
    ] {
        let key = format!("{category:?}").to_lowercase();
        let base_count = base
            .issues
            .iter()
            .filter(|i| i.category == category)
            .count() as i64;
        let head_count = head
            .issues
            .iter()
            .filter(|i| i.category == category)
            .count() as i64;
        category_deltas.insert(key, head_count - base_count);
    }

    DiffReport {
        error_delta: severity_count(&head.issues, Severity::Error)
            - severity_count(&base.issues, Severity::Error),
        warning_delta: severity_count(&head.issues, Severity::Warning)
            - severity_count(&base.issues, Severity::Warning),
        new_issues,
        fixed_issues,
        persistent_issues,
        category_deltas,
    }
}

fn severity_count(issues: &[Issue], severity: Severity) -> i64 {
    issues
        .iter()
        .filter(|issue| issue.severity == severity)
        .count() as i64
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use serde_json::json;

    use super::diff_reports;
    use crate::{
        issue::{Issue, IssueCategory, Severity},
        report::{DatasetSummary, IssueCounts, RunMetadata, ValidationReport},
    };

    #[test]
    fn computes_new_and_fixed_issues() {
        let base_issue = Issue::new(
            Severity::Warning,
            IssueCategory::Schema,
            "schema.missing",
            "Missing",
            "base",
            None,
            Some("a.parquet".to_string()),
            None,
            None,
            None,
            None,
            None,
            None,
            json!({}),
        );
        let head_issue = Issue::new(
            Severity::Error,
            IssueCategory::Temporal,
            "temporal.non_monotonic_timestamp",
            "Temporal",
            "head",
            None,
            Some("b.parquet".to_string()),
            None,
            None,
            None,
            None,
            None,
            None,
            json!({}),
        );
        let base = report(vec![base_issue.clone()]);
        let head = report(vec![head_issue.clone()]);
        let diff = diff_reports(&base, &head);
        assert_eq!(diff.new_issues.len(), 1);
        assert_eq!(diff.fixed_issues.len(), 1);
        assert_eq!(diff.persistent_issues.len(), 0);
    }

    fn report(issues: Vec<Issue>) -> ValidationReport {
        ValidationReport {
            schema_version: "1.0.0".to_string(),
            leguard_version: "0.1.0".to_string(),
            dataset: DatasetSummary::default(),
            run: RunMetadata {
                source: "test".to_string(),
                git_commit: None,
                git_branch: None,
                git_repo: None,
                dataset_revision: None,
                started_at: Utc::now(),
                finished_at: Utc::now(),
            },
            issue_counts: IssueCounts::from_issues(&issues),
            issues,
            generated_at: Utc::now(),
        }
    }
}
