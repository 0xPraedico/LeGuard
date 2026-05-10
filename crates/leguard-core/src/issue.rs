use blake3::Hasher;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IssueCategory {
    Structure,
    Schema,
    Temporal,
    Video,
    Numerical,
    Annotation,
    Diff,
    Compatibility,
    Performance,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Issue {
    pub fingerprint: String,
    pub severity: Severity,
    pub category: IssueCategory,
    pub check_id: String,
    pub title: String,
    pub message: String,
    pub suggestion: Option<String>,
    pub file_path: Option<String>,
    pub episode_index: Option<i64>,
    pub frame_index: Option<i64>,
    pub timestamp_sec: Option<f64>,
    pub feature: Option<String>,
    pub expected: Option<serde_json::Value>,
    pub actual: Option<serde_json::Value>,
    #[serde(default)]
    pub metadata: serde_json::Value,
}

impl Issue {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        severity: Severity,
        category: IssueCategory,
        check_id: impl Into<String>,
        title: impl Into<String>,
        message: impl Into<String>,
        suggestion: Option<String>,
        file_path: Option<String>,
        episode_index: Option<i64>,
        frame_index: Option<i64>,
        timestamp_sec: Option<f64>,
        feature: Option<String>,
        expected: Option<serde_json::Value>,
        actual: Option<serde_json::Value>,
        metadata: serde_json::Value,
    ) -> Self {
        let check_id = check_id.into();
        let title = title.into();
        let message = message.into();
        let fingerprint = build_fingerprint(
            &check_id,
            &message,
            file_path.as_deref(),
            episode_index,
            frame_index,
            feature.as_deref(),
        );

        Self {
            fingerprint,
            severity,
            category,
            check_id,
            title,
            message,
            suggestion,
            file_path,
            episode_index,
            frame_index,
            timestamp_sec,
            feature,
            expected,
            actual,
            metadata,
        }
    }
}

pub fn build_fingerprint(
    check_id: &str,
    message: &str,
    file_path: Option<&str>,
    episode_index: Option<i64>,
    frame_index: Option<i64>,
    feature: Option<&str>,
) -> String {
    let mut hasher = Hasher::new();
    hasher.update(check_id.as_bytes());
    hasher.update(message.as_bytes());
    if let Some(path) = file_path {
        hasher.update(path.as_bytes());
    }
    if let Some(ep) = episode_index {
        hasher.update(ep.to_le_bytes().as_ref());
    }
    if let Some(frame) = frame_index {
        hasher.update(frame.to_le_bytes().as_ref());
    }
    if let Some(feature) = feature {
        hasher.update(feature.as_bytes());
    }
    hasher.finalize().to_hex().to_string()
}
