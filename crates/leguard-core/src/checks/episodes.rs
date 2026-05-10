use std::collections::HashMap;
use std::fs;

use arrow_array::{
    Array, Int16Array, Int32Array, Int64Array, Int8Array, UInt16Array, UInt32Array, UInt64Array,
    UInt8Array,
};
use arrow_schema::DataType;
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use serde_json::json;
use walkdir::WalkDir;

use crate::{
    checks::MAX_ISSUES_PER_CHECK,
    config::ValidationConfig,
    dataset::DatasetScan,
    issue::{Issue, IssueCategory, Severity},
};

pub fn run(scan: &DatasetScan, config: &ValidationConfig) -> Vec<Issue> {
    let mut issues = Vec::new();
    let episode_lengths = collect_episode_lengths(scan, config);
    if episode_lengths.is_empty() {
        return issues;
    }

    let mut entries = episode_lengths.into_iter().collect::<Vec<_>>();
    entries.sort_by_key(|(episode, _)| *episode);
    let lengths = entries
        .iter()
        .map(|(_, length)| *length as f64)
        .collect::<Vec<_>>();
    let mean = lengths.iter().sum::<f64>() / lengths.len() as f64;
    let variance = lengths
        .iter()
        .map(|value| {
            let delta = *value - mean;
            delta * delta
        })
        .sum::<f64>()
        / lengths.len() as f64;
    let std_dev = variance.sqrt();

    let short_episodes = entries
        .iter()
        .filter(|(_, length)| *length < 5)
        .map(|(episode, _)| *episode)
        .collect::<Vec<_>>();
    if !short_episodes.is_empty() {
        issues.push(Issue::new(
            Severity::Warning,
            IssueCategory::Performance,
            "episodes.too_short",
            "Very short episodes detected",
            format!(
                "{} episode(s) are shorter than 5 frames",
                short_episodes.len()
            ),
            Some("Filter out extremely short episodes before training.".to_string()),
            None,
            short_episodes.first().copied(),
            None,
            None,
            None,
            Some(json!({ "min_frames": 5 })),
            Some(json!({ "episodes": short_episodes })),
            json!({}),
        ));
    }

    if mean > 0.0 && std_dev / mean > 1.0 {
        issues.push(Issue::new(
            Severity::Warning,
            IssueCategory::Performance,
            "episodes.high_length_variance",
            "High episode length variance",
            format!(
                "Episode lengths show high variance (mean={:.1}, std={:.1})",
                mean, std_dev
            ),
            Some("Normalize episode durations or split very long outliers.".to_string()),
            None,
            None,
            None,
            None,
            None,
            Some(json!({ "max_std_over_mean": 1.0 })),
            Some(json!({ "mean": mean, "std": std_dev })),
            json!({}),
        ));
    }

    if std_dev > 0.0 && entries.len() > 4 {
        let outliers = entries
            .iter()
            .filter_map(|(episode, length)| {
                let z_score = (*length as f64 - mean) / std_dev;
                if z_score.abs() > 3.0 {
                    Some((*episode, *length, z_score))
                } else {
                    None
                }
            })
            .take(10)
            .collect::<Vec<_>>();
        if !outliers.is_empty() {
            issues.push(Issue::new(
                Severity::Warning,
                IssueCategory::Performance,
                "episodes.length_outliers",
                "Episode length outliers detected",
                format!(
                    "{} episode(s) are outliers by length (>3 std dev from mean)",
                    outliers.len()
                ),
                Some("Inspect and trim/drop abnormal episodes.".to_string()),
                None,
                outliers.first().map(|(episode, _, _)| *episode),
                None,
                None,
                None,
                None,
                None,
                json!({ "outliers": outliers }),
            ));
        }
    }

    if config.max_episodes.is_none() {
        compare_info_totals(scan, &entries, &mut issues);
        compare_episode_metadata(scan, &entries, &mut issues);
    }

    issues
}

fn compare_info_totals(scan: &DatasetScan, entries: &[(i64, usize)], issues: &mut Vec<Issue>) {
    if issues.len() >= MAX_ISSUES_PER_CHECK {
        return;
    }
    let info_path = scan.root.join("meta").join("info.json");
    let raw = match fs::read_to_string(info_path) {
        Ok(raw) => raw,
        Err(_) => return,
    };
    let value = match serde_json::from_str::<serde_json::Value>(&raw) {
        Ok(value) => value,
        Err(_) => return,
    };

    let actual_episodes = entries.len() as u64;
    let actual_frames = entries
        .iter()
        .map(|(_, length)| *length as u64)
        .sum::<u64>();

    if let Some(expected_episodes) = value.get("total_episodes").and_then(|v| v.as_u64()) {
        if expected_episodes != actual_episodes && issues.len() < MAX_ISSUES_PER_CHECK {
            issues.push(Issue::new(
                Severity::Error,
                IssueCategory::Compatibility,
                "episodes.total_episodes_mismatch",
                "Metadata/data episode mismatch",
                format!(
                    "meta/info.json reports total_episodes={}, but data has {}",
                    expected_episodes, actual_episodes
                ),
                Some("Rebuild metadata after dataset edits.".to_string()),
                Some("meta/info.json".to_string()),
                None,
                None,
                None,
                Some("total_episodes".to_string()),
                Some(json!({ "total_episodes": expected_episodes })),
                Some(json!({ "total_episodes": actual_episodes })),
                json!({}),
            ));
        }
    }

    if let Some(expected_frames) = value.get("total_frames").and_then(|v| v.as_u64()) {
        if expected_frames != actual_frames && issues.len() < MAX_ISSUES_PER_CHECK {
            issues.push(Issue::new(
                Severity::Error,
                IssueCategory::Compatibility,
                "episodes.total_frames_mismatch",
                "Metadata/data frame mismatch",
                format!(
                    "meta/info.json reports total_frames={}, but data has {}",
                    expected_frames, actual_frames
                ),
                Some("Recompute frame totals and update meta/info.json.".to_string()),
                Some("meta/info.json".to_string()),
                None,
                None,
                None,
                Some("total_frames".to_string()),
                Some(json!({ "total_frames": expected_frames })),
                Some(json!({ "total_frames": actual_frames })),
                json!({}),
            ));
        }
    }
}

fn compare_episode_metadata(scan: &DatasetScan, entries: &[(i64, usize)], issues: &mut Vec<Issue>) {
    if issues.len() >= MAX_ISSUES_PER_CHECK {
        return;
    }
    let metadata_lengths = collect_episode_metadata_lengths(scan);
    if metadata_lengths.is_empty() {
        return;
    }

    for (episode, data_length) in entries {
        if issues.len() >= MAX_ISSUES_PER_CHECK {
            break;
        }
        let Some(meta_length) = metadata_lengths.get(episode) else {
            issues.push(Issue::new(
                Severity::Warning,
                IssueCategory::Compatibility,
                "episodes.missing_meta_entry",
                "Episode missing from metadata parquet",
                format!(
                    "Episode {} exists in data but is missing in meta/episodes parquet files",
                    episode
                ),
                Some("Regenerate meta/episodes metadata.".to_string()),
                Some("meta/episodes".to_string()),
                Some(*episode),
                None,
                None,
                Some("episode_index".to_string()),
                None,
                None,
                json!({}),
            ));
            continue;
        };
        if *meta_length != *data_length {
            issues.push(Issue::new(
                Severity::Error,
                IssueCategory::Compatibility,
                "episodes.meta_length_mismatch",
                "Episode length mismatch between metadata and data",
                format!(
                    "Episode {} length mismatch: meta={} vs data={}",
                    episode, meta_length, data_length
                ),
                Some("Recompute episode lengths in metadata.".to_string()),
                Some("meta/episodes".to_string()),
                Some(*episode),
                None,
                None,
                Some("length".to_string()),
                Some(json!({ "meta_length": meta_length })),
                Some(json!({ "data_length": data_length })),
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

fn collect_episode_metadata_lengths(scan: &DatasetScan) -> HashMap<i64, usize> {
    let mut metadata = HashMap::new();
    let episodes_root = scan.root.join("meta").join("episodes");
    if !episodes_root.is_dir() {
        return metadata;
    }

    for entry in WalkDir::new(episodes_root)
        .follow_links(false)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().is_file())
    {
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("parquet") {
            continue;
        }
        let file = match std::fs::File::open(path) {
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
        let Ok(length_idx) = schema.index_of("length") else {
            continue;
        };
        let mut reader = match builder.with_batch_size(2048).build() {
            Ok(reader) => reader,
            Err(_) => continue,
        };

        for batch in &mut reader {
            let Ok(batch) = batch else { continue };
            for row in 0..batch.num_rows() {
                let Some(episode) = value_as_i64(batch.column(episode_idx).as_ref(), row) else {
                    continue;
                };
                let Some(length) = value_as_i64(batch.column(length_idx).as_ref(), row) else {
                    continue;
                };
                metadata.insert(episode, length.max(0) as usize);
            }
        }
    }

    metadata
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
