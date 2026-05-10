use std::collections::HashMap;
use std::fs;

use arrow_array::{
    Array, Int16Array, Int32Array, Int64Array, Int8Array, UInt16Array, UInt32Array, UInt64Array,
    UInt8Array,
};
use arrow_schema::DataType;
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use serde_json::json;

use crate::{
    checks::MAX_ISSUES_PER_CHECK,
    config::ValidationConfig,
    dataset::DatasetScan,
    issue::{Issue, IssueCategory, Severity},
};

struct PolicyRequirement {
    name: &'static str,
    chunk_size: usize,
    min_episodes: usize,
    requires_images: bool,
    requires_state: bool,
    requires_language: bool,
}

const POLICIES: [PolicyRequirement; 4] = [
    PolicyRequirement {
        name: "act",
        chunk_size: 100,
        min_episodes: 10,
        requires_images: true,
        requires_state: true,
        requires_language: false,
    },
    PolicyRequirement {
        name: "diffusion",
        chunk_size: 16,
        min_episodes: 10,
        requires_images: true,
        requires_state: true,
        requires_language: false,
    },
    PolicyRequirement {
        name: "smolvla",
        chunk_size: 100,
        min_episodes: 50,
        requires_images: true,
        requires_state: false,
        requires_language: true,
    },
    PolicyRequirement {
        name: "pi0",
        chunk_size: 50,
        min_episodes: 20,
        requires_images: true,
        requires_state: true,
        requires_language: false,
    },
];

pub fn run(scan: &DatasetScan, config: &ValidationConfig) -> Vec<Issue> {
    let mut issues = Vec::new();
    let info_path = scan.root.join("meta").join("info.json");
    let raw = match fs::read_to_string(&info_path) {
        Ok(raw) => raw,
        Err(_) => return issues,
    };
    let info = match serde_json::from_str::<serde_json::Value>(&raw) {
        Ok(info) => info,
        Err(_) => return issues,
    };
    let Some(features) = info.get("features").and_then(|value| value.as_object()) else {
        return issues;
    };

    let has_actions = features.keys().any(|name| name.starts_with("action"));
    let has_state = features.keys().any(|name| name.contains("state"));
    let has_images = features.values().any(|spec| {
        spec.get("dtype")
            .and_then(|value| value.as_str())
            .map(|dtype| dtype == "image" || dtype == "video")
            .unwrap_or(false)
    });
    let has_language = features.keys().any(|name| {
        name.contains("language") || name.contains("instruction") || name.contains("task")
    });

    if !has_actions {
        issues.push(Issue::new(
            Severity::Error,
            IssueCategory::Compatibility,
            "training.missing_action_features",
            "Training blockers: missing action features",
            "No action feature was found in meta/info.json.".to_string(),
            Some("Training policies require at least one action feature.".to_string()),
            Some("meta/info.json".to_string()),
            None,
            None,
            None,
            Some("features".to_string()),
            None,
            None,
            json!({}),
        ));
        return issues;
    }

    let episode_lengths = collect_episode_lengths(scan, config);
    let episode_count = episode_lengths.len();
    for policy in POLICIES {
        if issues.len() >= MAX_ISSUES_PER_CHECK {
            break;
        }

        let mut missing = Vec::new();
        if policy.requires_images && !has_images {
            missing.push("images/video features");
        }
        if policy.requires_state && !has_state {
            missing.push("state features");
        }
        if policy.requires_language && !has_language {
            missing.push("language/task features");
        }
        if !missing.is_empty() {
            issues.push(Issue::new(
                Severity::Warning,
                IssueCategory::Compatibility,
                "training.policy_requirements",
                "Policy feature requirements not satisfied",
                format!(
                    "Policy '{}' is missing required feature groups: {}",
                    policy.name,
                    missing.join(", ")
                ),
                Some("Add missing features or pick a compatible policy.".to_string()),
                Some("meta/info.json".to_string()),
                None,
                None,
                None,
                Some("features".to_string()),
                None,
                None,
                json!({ "policy": policy.name, "missing": missing }),
            ));
        }

        if episode_count > 0 && episode_count < policy.min_episodes {
            issues.push(Issue::new(
                Severity::Warning,
                IssueCategory::Compatibility,
                "training.low_episode_count",
                "Low episode count for target policy",
                format!(
                    "Policy '{}' usually needs >= {} episodes (found {})",
                    policy.name, policy.min_episodes, episode_count
                ),
                Some("Collect more demonstrations or start with a smaller policy.".to_string()),
                None,
                None,
                None,
                None,
                Some("episode_count".to_string()),
                Some(json!({ "min_episodes": policy.min_episodes })),
                Some(json!({ "episode_count": episode_count })),
                json!({ "policy": policy.name }),
            ));
        }

        if !episode_lengths.is_empty() {
            let short_count = episode_lengths
                .values()
                .filter(|length| **length < policy.chunk_size)
                .count();
            let short_ratio = short_count as f64 / episode_lengths.len() as f64;
            if short_ratio > 0.3 {
                issues.push(Issue::new(
                    Severity::Warning,
                    IssueCategory::Compatibility,
                    "training.short_episodes_for_policy",
                    "Many episodes are too short for policy chunk size",
                    format!(
                        "Policy '{}' chunk_size={} is incompatible with {}/{} episodes",
                        policy.name,
                        policy.chunk_size,
                        short_count,
                        episode_lengths.len()
                    ),
                    Some("Trim/drop short episodes or reduce the policy chunk size.".to_string()),
                    None,
                    None,
                    None,
                    None,
                    Some("chunk_size".to_string()),
                    Some(json!({ "max_short_ratio": 0.3 })),
                    Some(json!({ "short_ratio": short_ratio })),
                    json!({ "policy": policy.name }),
                ));
            }
        }
    }

    if issues.len() < MAX_ISSUES_PER_CHECK {
        check_normalization_stats(scan, &mut issues);
    }

    issues
}

fn check_normalization_stats(scan: &DatasetScan, issues: &mut Vec<Issue>) {
    let stats_path = scan.root.join("meta").join("stats.json");
    let raw = match fs::read_to_string(&stats_path) {
        Ok(raw) => raw,
        Err(_) => {
            issues.push(Issue::new(
                Severity::Warning,
                IssueCategory::Compatibility,
                "training.missing_stats",
                "Missing normalization stats",
                "meta/stats.json is missing, normalization checks are limited.".to_string(),
                Some("Generate stats.json before large-scale training.".to_string()),
                Some("meta/stats.json".to_string()),
                None,
                None,
                None,
                None,
                None,
                None,
                json!({}),
            ));
            return;
        }
    };
    let stats = match serde_json::from_str::<serde_json::Value>(&raw) {
        Ok(stats) => stats,
        Err(_) => return,
    };
    let Some(features) = stats.as_object() else {
        return;
    };

    for (feature_name, feature_stats) in features {
        if issues.len() >= MAX_ISSUES_PER_CHECK {
            break;
        }
        if !feature_name.starts_with("action") {
            continue;
        }
        let Some(std_values) = feature_stats.get("std").and_then(|value| value.as_array()) else {
            continue;
        };
        let zero_std = std_values
            .iter()
            .filter_map(|value| value.as_f64())
            .filter(|value| value.abs() < 1e-8)
            .count();
        if zero_std > 0 {
            issues.push(Issue::new(
                Severity::Warning,
                IssueCategory::Compatibility,
                "training.zero_std_dimensions",
                "Zero-std action dimensions in stats",
                format!(
                    "Feature {} has {} dimension(s) with zero std in stats.json",
                    feature_name, zero_std
                ),
                Some("Drop constant dims or regenerate stats after cleaning data.".to_string()),
                Some("meta/stats.json".to_string()),
                None,
                None,
                None,
                Some(feature_name.clone()),
                Some(json!({ "std": ">0" })),
                Some(json!({ "zero_std_dimensions": zero_std })),
                json!({}),
            ));
        }
    }
}

fn collect_episode_lengths(scan: &DatasetScan, config: &ValidationConfig) -> HashMap<i64, usize> {
    let mut lengths = HashMap::new();

    for parquet_file in &scan.parquet_files {
        let file = match std::fs::File::open(&parquet_file.path) {
            Ok(file) => file,
            Err(_) => continue,
        };
        let builder = match ParquetRecordBatchReaderBuilder::try_new(file) {
            Ok(builder) => builder,
            Err(_) => continue,
        };
        let schema = builder.schema().clone();
        let Ok(episode_idx) = schema.index_of("episode_index") else {
            continue;
        };
        let mut reader = match builder.with_batch_size(4096).build() {
            Ok(reader) => reader,
            Err(_) => continue,
        };

        for batch in &mut reader {
            let Ok(batch) = batch else { continue };
            for row in 0..batch.num_rows() {
                let Some(episode) = value_as_i64(batch.column(episode_idx).as_ref(), row) else {
                    continue;
                };
                if !allow_episode(config.max_episodes, &lengths, episode) {
                    continue;
                }
                *lengths.entry(episode).or_default() += 1;
            }
        }
    }

    lengths
}

fn allow_episode(max_episodes: Option<usize>, lengths: &HashMap<i64, usize>, episode: i64) -> bool {
    match max_episodes {
        Some(limit) if limit > 0 => lengths.contains_key(&episode) || lengths.len() < limit,
        _ => true,
    }
}

fn value_as_i64(array: &dyn Array, row: usize) -> Option<i64> {
    if array.is_null(row) {
        return None;
    }
    match array.data_type() {
        DataType::Int64 => Some(array.as_any().downcast_ref::<Int64Array>()?.value(row)),
        DataType::Int32 => Some(array.as_any().downcast_ref::<Int32Array>()?.value(row) as i64),
        DataType::Int16 => Some(array.as_any().downcast_ref::<Int16Array>()?.value(row) as i64),
        DataType::Int8 => Some(array.as_any().downcast_ref::<Int8Array>()?.value(row) as i64),
        DataType::UInt64 => Some(array.as_any().downcast_ref::<UInt64Array>()?.value(row) as i64),
        DataType::UInt32 => Some(array.as_any().downcast_ref::<UInt32Array>()?.value(row) as i64),
        DataType::UInt16 => Some(array.as_any().downcast_ref::<UInt16Array>()?.value(row) as i64),
        DataType::UInt8 => Some(array.as_any().downcast_ref::<UInt8Array>()?.value(row) as i64),
        _ => None,
    }
}
