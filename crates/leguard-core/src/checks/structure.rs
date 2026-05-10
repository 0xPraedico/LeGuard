use std::path::Path;

use serde_json::json;

use crate::{
    dataset::DatasetScan,
    issue::{Issue, IssueCategory, Severity},
};

pub fn run(path: &Path, scan: Option<&DatasetScan>) -> Vec<Issue> {
    let mut issues = Vec::new();

    if !path.exists() {
        issues.push(Issue::new(
            Severity::Error,
            IssueCategory::Structure,
            "structure.path_not_found",
            "Dataset path not found",
            format!("Dataset path {} does not exist", path.display()),
            Some("Provide a valid dataset directory path.".to_string()),
            None,
            None,
            None,
            None,
            None,
            Some(json!({ "exists": true })),
            Some(json!({ "exists": false })),
            json!({}),
        ));
        return issues;
    }

    if !path.is_dir() {
        issues.push(Issue::new(
            Severity::Error,
            IssueCategory::Structure,
            "structure.path_not_directory",
            "Dataset path is not a directory",
            format!("Dataset path {} is not a directory", path.display()),
            Some("Point LeGuard to the dataset root directory.".to_string()),
            Some(path.to_string_lossy().to_string()),
            None,
            None,
            None,
            None,
            Some(json!({ "is_directory": true })),
            Some(json!({ "is_directory": false })),
            json!({}),
        ));
        return issues;
    }

    let Some(scan) = scan else {
        return issues;
    };

    if !scan.has_meta_dir {
        issues.push(Issue::new(
            Severity::Warning,
            IssueCategory::Structure,
            "structure.missing_meta_dir",
            "Missing meta directory",
            "Expected meta/ directory is missing".to_string(),
            Some("Create a meta/ directory with dataset metadata files.".to_string()),
            Some(path.join("meta").to_string_lossy().to_string()),
            None,
            None,
            None,
            None,
            Some(json!({ "meta_dir": true })),
            Some(json!({ "meta_dir": false })),
            json!({}),
        ));
    }

    if scan.metadata_files.is_empty() {
        issues.push(Issue::new(
            Severity::Warning,
            IssueCategory::Structure,
            "structure.no_metadata_files",
            "No metadata files found",
            "No .json or .jsonl metadata file found in dataset".to_string(),
            Some("Add metadata files in meta/ or dataset root.".to_string()),
            None,
            None,
            None,
            None,
            None,
            Some(json!({ "metadata_files": ">0" })),
            Some(json!({ "metadata_files": 0 })),
            json!({}),
        ));
    }

    if scan.parquet_files.is_empty() {
        issues.push(Issue::new(
            Severity::Warning,
            IssueCategory::Structure,
            "structure.no_parquet_files",
            "No parquet files found",
            "No .parquet files were detected in dataset".to_string(),
            Some("Add parquet shards under data/.".to_string()),
            None,
            None,
            None,
            None,
            None,
            Some(json!({ "parquet_files": ">0" })),
            Some(json!({ "parquet_files": 0 })),
            json!({}),
        ));
    }

    if scan.video_files.is_empty() {
        issues.push(Issue::new(
            Severity::Warning,
            IssueCategory::Structure,
            "structure.no_video_files",
            "No video files found",
            "No video files (.mp4/.avi/.mov/.mkv) were found".to_string(),
            Some("Add camera videos under videos/<camera>/.".to_string()),
            None,
            None,
            None,
            None,
            None,
            Some(json!({ "video_files": ">0" })),
            Some(json!({ "video_files": 0 })),
            json!({}),
        ));
    }

    for empty_file in &scan.empty_files {
        issues.push(Issue::new(
            Severity::Warning,
            IssueCategory::Structure,
            "structure.empty_file",
            "Empty file detected",
            format!("Empty file detected: {}", empty_file.relative_path),
            Some("Remove or regenerate the empty file.".to_string()),
            Some(empty_file.relative_path.clone()),
            None,
            None,
            None,
            None,
            Some(json!({ "size_bytes": ">0" })),
            Some(json!({ "size_bytes": 0 })),
            json!({}),
        ));
    }

    if !scan.has_common_layout() {
        issues.push(Issue::new(
            Severity::Warning,
            IssueCategory::Structure,
            "structure.unrecognized_layout",
            "Unrecognized LeRobot-like layout",
            "Dataset does not match the common layout meta/ + data/ + videos/".to_string(),
            Some("Consider organizing files under meta/, data/, videos/.".to_string()),
            None,
            None,
            None,
            None,
            None,
            Some(json!({ "layout": ["meta", "data", "videos"] })),
            Some(json!({
                "meta": scan.has_meta_dir,
                "data": scan.has_data_dir,
                "videos": scan.has_videos_dir
            })),
            json!({}),
        ));
    }

    issues
}
