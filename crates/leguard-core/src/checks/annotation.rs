use std::{collections::HashMap, fs};

use serde_json::json;

use crate::{
    checks::MAX_ISSUES_PER_CHECK,
    dataset::DatasetScan,
    issue::{Issue, IssueCategory, Severity},
};

#[derive(Debug, Clone)]
struct Segment {
    start: f64,
    end: f64,
    label: String,
}

pub fn run(scan: &DatasetScan) -> Vec<Issue> {
    let mut issues = Vec::new();
    let mut segments_by_episode: HashMap<i64, Vec<Segment>> = HashMap::new();

    let candidates = scan
        .files
        .iter()
        .filter(|file| {
            let name = file
                .path
                .file_name()
                .map(|n| n.to_string_lossy().to_lowercase())
                .unwrap_or_default();
            let rel = file.relative_path.to_lowercase();
            name == "annotations.json"
                || name == "subtasks.json"
                || rel == "meta/episodes.jsonl"
                || rel.contains("annotation")
                || rel.contains("subtask")
        })
        .cloned()
        .collect::<Vec<_>>();

    for file in candidates {
        if issues.len() >= MAX_ISSUES_PER_CHECK {
            break;
        }

        let ext = file
            .path
            .extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .unwrap_or_default();

        let mut records = Vec::new();
        match ext.as_str() {
            "json" => match fs::read_to_string(&file.path)
                .ok()
                .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok())
            {
                Some(serde_json::Value::Array(items)) => records.extend(items),
                Some(obj @ serde_json::Value::Object(_)) => records.push(obj),
                Some(_) => {}
                None => {
                    issues.push(Issue::new(
                        Severity::Warning,
                        IssueCategory::Annotation,
                        "annotation.unreadable_file",
                        "Unreadable annotation file",
                        format!("Unable to parse annotation file {}", file.relative_path),
                        Some("Fix JSON syntax or regenerate annotation export.".to_string()),
                        Some(file.relative_path.clone()),
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        json!({}),
                    ));
                    continue;
                }
            },
            "jsonl" => {
                let raw = match fs::read_to_string(&file.path) {
                    Ok(raw) => raw,
                    Err(_) => {
                        issues.push(Issue::new(
                            Severity::Warning,
                            IssueCategory::Annotation,
                            "annotation.unreadable_file",
                            "Unreadable annotation file",
                            format!("Unable to read annotation file {}", file.relative_path),
                            Some("Check file permissions and encoding.".to_string()),
                            Some(file.relative_path.clone()),
                            None,
                            None,
                            None,
                            None,
                            None,
                            None,
                            json!({}),
                        ));
                        continue;
                    }
                };
                for (line_index, line) in raw.lines().enumerate() {
                    if line.trim().is_empty() {
                        continue;
                    }
                    match serde_json::from_str::<serde_json::Value>(line) {
                        Ok(value) => records.push(value),
                        Err(error) => {
                            issues.push(Issue::new(
                                Severity::Warning,
                                IssueCategory::Annotation,
                                "annotation.unreadable_file",
                                "Invalid JSONL annotation line",
                                format!(
                                    "Invalid JSONL at {} line {}: {}",
                                    file.relative_path,
                                    line_index + 1,
                                    error
                                ),
                                Some("Fix malformed JSON line in annotation file.".to_string()),
                                Some(file.relative_path.clone()),
                                None,
                                None,
                                None,
                                None,
                                None,
                                None,
                                json!({}),
                            ));
                        }
                    }
                }
            }
            _ => continue,
        }

        for record in records {
            if issues.len() >= MAX_ISSUES_PER_CHECK {
                break;
            }
            let Some(obj) = record.as_object() else {
                continue;
            };

            let label = obj
                .get("label")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            if obj.contains_key("label") && label.trim().is_empty() {
                issues.push(Issue::new(
                    Severity::Warning,
                    IssueCategory::Annotation,
                    "annotation.missing_label",
                    "Missing annotation label",
                    format!("Empty label found in {}", file.relative_path),
                    Some("Populate label with a non-empty string.".to_string()),
                    Some(file.relative_path.clone()),
                    obj.get("episode_index").and_then(|v| v.as_i64()),
                    None,
                    None,
                    Some("label".to_string()),
                    Some(json!({ "label": "non-empty string" })),
                    Some(json!({ "label": "" })),
                    json!({}),
                ));
            }

            let episode = obj
                .get("episode_index")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            let start = extract_numeric_field(obj, &["start", "start_sec", "start_frame"]);
            let end = extract_numeric_field(obj, &["end", "end_sec", "end_frame"]);

            if let (Some(start), Some(end)) = (start, end) {
                if end <= start {
                    issues.push(Issue::new(
                        Severity::Warning,
                        IssueCategory::Annotation,
                        "annotation.invalid_segment_duration",
                        "Invalid segment duration",
                        format!(
                            "{} contains segment with end <= start ({end} <= {start})",
                            file.relative_path
                        ),
                        Some("Ensure each annotation segment has positive duration.".to_string()),
                        Some(file.relative_path.clone()),
                        Some(episode),
                        None,
                        None,
                        Some("segment".to_string()),
                        Some(json!({ "end_gt_start": true })),
                        Some(json!({ "start": start, "end": end })),
                        json!({ "label": label }),
                    ));
                } else {
                    segments_by_episode
                        .entry(episode)
                        .or_default()
                        .push(Segment { start, end, label });
                }

                if start < 0.0 || end < 0.0 {
                    issues.push(Issue::new(
                        Severity::Warning,
                        IssueCategory::Annotation,
                        "annotation.negative_timestamp_or_frame",
                        "Negative segment boundary",
                        format!(
                            "{} has negative annotation boundary start={} end={}",
                            file.relative_path, start, end
                        ),
                        Some("Clamp boundaries to valid timestamp/frame values.".to_string()),
                        Some(file.relative_path.clone()),
                        Some(episode),
                        None,
                        None,
                        Some("segment".to_string()),
                        Some(json!({ "min_value": 0 })),
                        Some(json!({ "start": start, "end": end })),
                        json!({}),
                    ));
                }
            }
        }
    }

    for (episode, segments) in segments_by_episode {
        let mut sorted = segments;
        sorted.sort_by(|a, b| a.start.total_cmp(&b.start));
        for window in sorted.windows(2) {
            let left = &window[0];
            let right = &window[1];
            if right.start < left.end {
                issues.push(Issue::new(
                    Severity::Warning,
                    IssueCategory::Annotation,
                    "annotation.overlapping_segments",
                    "Overlapping annotation segments",
                    format!(
                        "episode {episode} has overlapping segments {} and {}",
                        left.label, right.label
                    ),
                    Some("Fix segment boundaries to avoid overlaps.".to_string()),
                    None,
                    Some(episode),
                    None,
                    None,
                    Some("segment".to_string()),
                    Some(json!({ "overlap": false })),
                    Some(json!({
                        "overlap": true,
                        "left": { "start": left.start, "end": left.end, "label": left.label },
                        "right": { "start": right.start, "end": right.end, "label": right.label }
                    })),
                    json!({}),
                ));
                if issues.len() >= MAX_ISSUES_PER_CHECK {
                    break;
                }
            }
        }
    }

    issues
}

fn extract_numeric_field(
    obj: &serde_json::Map<String, serde_json::Value>,
    keys: &[&str],
) -> Option<f64> {
    for key in keys {
        if let Some(value) = obj.get(*key) {
            if let Some(n) = value.as_f64() {
                return Some(n);
            }
            if let Some(n) = value.as_i64() {
                return Some(n as f64);
            }
            if let Some(n) = value.as_u64() {
                return Some(n as f64);
            }
            if let Some(raw) = value.as_str() {
                if let Ok(parsed) = raw.parse::<f64>() {
                    return Some(parsed);
                }
            }
        }
    }
    None
}
