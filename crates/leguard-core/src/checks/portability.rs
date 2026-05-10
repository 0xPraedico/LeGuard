use std::fs;
use std::path::Path;

use serde_json::json;
use walkdir::WalkDir;

use crate::{
    checks::MAX_ISSUES_PER_CHECK,
    dataset::DatasetScan,
    issue::{Issue, IssueCategory, Severity},
};

pub fn run(scan: &DatasetScan) -> Vec<Issue> {
    let mut issues = Vec::new();
    let info = load_info(scan);

    if let Some(info) = info.as_ref() {
        if let Some(data_path) = info.get("data_path").and_then(|v| v.as_str()) {
            if Path::new(data_path).is_absolute() {
                issues.push(Issue::new(
                    Severity::Warning,
                    IssueCategory::Compatibility,
                    "portability.absolute_data_path",
                    "Absolute data_path in metadata",
                    format!("meta/info.json uses an absolute data_path: {}", data_path),
                    Some("Use relative templates for portable datasets.".to_string()),
                    Some("meta/info.json".to_string()),
                    None,
                    None,
                    None,
                    Some("data_path".to_string()),
                    None,
                    Some(json!({ "data_path": data_path })),
                    json!({}),
                ));
            } else if !data_path.contains('{') {
                issues.push(Issue::new(
                    Severity::Warning,
                    IssueCategory::Compatibility,
                    "portability.non_template_data_path",
                    "Non-template data_path",
                    format!(
                        "data_path '{}' is not a template. This can break multi-shard portability.",
                        data_path
                    ),
                    Some("Use a templated data_path (chunk/file placeholders).".to_string()),
                    Some("meta/info.json".to_string()),
                    None,
                    None,
                    None,
                    Some("data_path".to_string()),
                    None,
                    Some(json!({ "data_path": data_path })),
                    json!({}),
                ));
            }
        }

        if let Some(video_path) = info.get("video_path").and_then(|v| v.as_str()) {
            if Path::new(video_path).is_absolute() {
                issues.push(Issue::new(
                    Severity::Warning,
                    IssueCategory::Compatibility,
                    "portability.absolute_video_path",
                    "Absolute video_path in metadata",
                    format!("meta/info.json uses an absolute video_path: {}", video_path),
                    Some("Use relative video templates for HF and CI compatibility.".to_string()),
                    Some("meta/info.json".to_string()),
                    None,
                    None,
                    None,
                    Some("video_path".to_string()),
                    None,
                    Some(json!({ "video_path": video_path })),
                    json!({}),
                ));
            }
        }
    }

    let symlinks = WalkDir::new(&scan.root)
        .follow_links(false)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().is_symlink())
        .map(|entry| {
            entry
                .path()
                .strip_prefix(&scan.root)
                .map(|path| path.to_string_lossy().replace('\\', "/"))
                .unwrap_or_else(|_| entry.path().to_string_lossy().replace('\\', "/"))
        })
        .collect::<Vec<_>>();

    if !symlinks.is_empty() && issues.len() < MAX_ISSUES_PER_CHECK {
        issues.push(Issue::new(
            Severity::Warning,
            IssueCategory::Compatibility,
            "portability.symlinks_present",
            "Symlinks detected in dataset",
            format!(
                "{} symlink(s) found. They may break portability on other machines.",
                symlinks.len()
            ),
            Some("Prefer regular files over symlinks for shared datasets.".to_string()),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            json!({ "examples": symlinks.into_iter().take(8).collect::<Vec<_>>() }),
        ));
    }

    let large_files = scan
        .files
        .iter()
        .filter(|file| file.size_bytes > 5 * 1024 * 1024 * 1024)
        .map(|file| (file.relative_path.clone(), file.size_bytes))
        .collect::<Vec<_>>();
    if !large_files.is_empty() && issues.len() < MAX_ISSUES_PER_CHECK {
        issues.push(Issue::new(
            Severity::Warning,
            IssueCategory::Compatibility,
            "portability.large_files",
            "Large files may hurt HF portability",
            format!(
                "{} file(s) are larger than 5GB and may require special handling.",
                large_files.len()
            ),
            Some("Split/compress large assets or use proper LFS strategy.".to_string()),
            None,
            None,
            None,
            None,
            None,
            Some(json!({ "max_bytes": 5 * 1024 * 1024 * 1024u64 })),
            Some(json!({ "files": large_files })),
            json!({}),
        ));
    }

    let data_dir = scan.root.join("data");
    if data_dir.is_dir() && issues.len() < MAX_ISSUES_PER_CHECK {
        let non_parquet = WalkDir::new(&data_dir)
            .follow_links(false)
            .into_iter()
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.file_type().is_file())
            .filter(|entry| {
                entry.path().extension().and_then(|ext| ext.to_str()) != Some("parquet")
            })
            .map(|entry| {
                entry
                    .path()
                    .strip_prefix(&scan.root)
                    .map(|path| path.to_string_lossy().replace('\\', "/"))
                    .unwrap_or_else(|_| entry.path().to_string_lossy().replace('\\', "/"))
            })
            .take(8)
            .collect::<Vec<_>>();
        if !non_parquet.is_empty() {
            issues.push(Issue::new(
                Severity::Warning,
                IssueCategory::Compatibility,
                "portability.non_parquet_data_files",
                "Non-parquet files detected under data/",
                "Found non-parquet files in data/, which may break downstream loaders.".to_string(),
                Some("Keep raw dataset shards in parquet format under data/.".to_string()),
                Some("data/".to_string()),
                None,
                None,
                None,
                None,
                None,
                None,
                json!({ "files": non_parquet }),
            ));
        }
    }

    issues
}

fn load_info(scan: &DatasetScan) -> Option<serde_json::Value> {
    let info_path = scan.root.join("meta").join("info.json");
    let raw = fs::read_to_string(info_path).ok()?;
    serde_json::from_str::<serde_json::Value>(&raw).ok()
}
