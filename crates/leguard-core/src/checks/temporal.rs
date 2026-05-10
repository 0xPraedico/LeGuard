use std::collections::{HashMap, HashSet};

use arrow_array::{
    Array, Float32Array, Float64Array, Int16Array, Int32Array, Int64Array, Int8Array, UInt16Array,
    UInt32Array, UInt64Array, UInt8Array,
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

#[derive(Debug, Default)]
struct EpisodeState {
    last_timestamp: Option<f64>,
    last_frame_index: Option<i64>,
    deltas_sec: Vec<f64>,
}

pub fn run(scan: &DatasetScan, config: &ValidationConfig) -> Vec<Issue> {
    let mut issues = Vec::new();
    let mut episode_states: HashMap<i64, EpisodeState> = HashMap::new();
    let mut selected_episodes = HashSet::new();

    for parquet_file in &scan.parquet_files {
        if issues.len() >= MAX_ISSUES_PER_CHECK {
            break;
        }

        let file = match std::fs::File::open(&parquet_file.path) {
            Ok(file) => file,
            Err(_) => continue,
        };

        let builder = match ParquetRecordBatchReaderBuilder::try_new(file) {
            Ok(builder) => builder,
            Err(_) => continue,
        };
        let schema = builder.schema().clone();
        let mut reader = match builder.with_batch_size(2048).build() {
            Ok(reader) => reader,
            Err(_) => continue,
        };

        let episode_idx = schema.index_of("episode_index").ok();
        let frame_idx = schema.index_of("frame_index").ok();
        let timestamp_idx = schema
            .index_of("timestamp")
            .ok()
            .or_else(|| schema.index_of("timestamp_sec").ok());

        if frame_idx.is_none() && timestamp_idx.is_none() {
            continue;
        }

        let mut seen_rows = 0usize;
        for batch in &mut reader {
            let Ok(batch) = batch else { continue };
            let row_count = batch.num_rows();
            for row in 0..row_count {
                if seen_rows >= config.max_rows_per_parquet {
                    break;
                }
                seen_rows += 1;

                let episode = episode_idx
                    .and_then(|idx| value_as_i64(batch.column(idx).as_ref(), row))
                    .unwrap_or(0);
                if !allow_episode(config.max_episodes, &mut selected_episodes, episode) {
                    continue;
                }
                let state = episode_states.entry(episode).or_default();

                if let Some(timestamp_col_idx) = timestamp_idx {
                    if let Some(ts) = value_as_f64(batch.column(timestamp_col_idx).as_ref(), row) {
                        if let Some(prev) = state.last_timestamp {
                            if ts + f64::EPSILON < prev {
                                issues.push(Issue::new(
                                    Severity::Error,
                                    IssueCategory::Temporal,
                                    "temporal.non_monotonic_timestamp",
                                    "Non-monotonic timestamp",
                                    format!(
                                        "timestamp decreased in episode {episode} at row {row} ({ts} < {prev})"
                                    ),
                                    Some("Sort rows by timestamp or repair corrupted sequence.".to_string()),
                                    Some(parquet_file.relative_path.clone()),
                                    Some(episode),
                                    None,
                                    Some(ts),
                                    Some("timestamp".to_string()),
                                    Some(json!({ "monotonic": true })),
                                    Some(json!({ "previous": prev, "current": ts })),
                                    json!({}),
                                ));
                            }

                            let delta_sec = ts - prev;
                            if delta_sec > 0.0 {
                                state.deltas_sec.push(delta_sec);
                            }
                            let gap_ms = delta_sec * 1000.0;
                            if let Some(error_gap) = config.max_timestamp_gap_ms_error {
                                if gap_ms > error_gap {
                                    issues.push(Issue::new(
                                        Severity::Error,
                                        IssueCategory::Temporal,
                                        "temporal.large_timestamp_gap",
                                        "Large timestamp gap",
                                        format!(
                                            "episode {episode} has a large timestamp gap of {gap_ms:.2}ms"
                                        ),
                                        Some("Investigate dropped frames or sensor clock jumps.".to_string()),
                                        Some(parquet_file.relative_path.clone()),
                                        Some(episode),
                                        None,
                                        Some(ts),
                                        Some("timestamp".to_string()),
                                        Some(json!({ "max_gap_ms": error_gap })),
                                        Some(json!({ "gap_ms": gap_ms })),
                                        json!({}),
                                    ));
                                }
                            } else if let Some(warn_gap) = config.max_timestamp_gap_ms_warning {
                                if gap_ms > warn_gap {
                                    issues.push(Issue::new(
                                        Severity::Warning,
                                        IssueCategory::Temporal,
                                        "temporal.large_timestamp_gap",
                                        "Large timestamp gap",
                                        format!(
                                            "episode {episode} has a timestamp gap of {gap_ms:.2}ms"
                                        ),
                                        Some(
                                            "Check synchronization quality for this episode."
                                                .to_string(),
                                        ),
                                        Some(parquet_file.relative_path.clone()),
                                        Some(episode),
                                        None,
                                        Some(ts),
                                        Some("timestamp".to_string()),
                                        Some(json!({ "max_gap_ms": warn_gap })),
                                        Some(json!({ "gap_ms": gap_ms })),
                                        json!({}),
                                    ));
                                }
                            }
                        }

                        state.last_timestamp = Some(ts);
                    }
                }

                if let Some(frame_col_idx) = frame_idx {
                    if let Some(frame) = value_as_i64(batch.column(frame_col_idx).as_ref(), row) {
                        if let Some(prev_frame) = state.last_frame_index {
                            if frame < prev_frame {
                                issues.push(Issue::new(
                                    Severity::Error,
                                    IssueCategory::Temporal,
                                    "temporal.non_monotonic_frame_index",
                                    "Non-monotonic frame index",
                                    format!(
                                        "frame_index decreased in episode {episode} at row {row} ({frame} < {prev_frame})"
                                    ),
                                    Some("Repair frame ordering in this parquet shard.".to_string()),
                                    Some(parquet_file.relative_path.clone()),
                                    Some(episode),
                                    Some(frame),
                                    None,
                                    Some("frame_index".to_string()),
                                    Some(json!({ "monotonic": true })),
                                    Some(json!({ "previous": prev_frame, "current": frame })),
                                    json!({}),
                                ));
                            }
                        }
                        state.last_frame_index = Some(frame);
                    }
                }

                if issues.len() >= MAX_ISSUES_PER_CHECK {
                    break;
                }
            }
            if seen_rows >= config.max_rows_per_parquet || issues.len() >= MAX_ISSUES_PER_CHECK {
                break;
            }
        }
    }

    for (episode, state) in episode_states {
        if state.deltas_sec.len() < 5 {
            continue;
        }

        let mean_delta = state.deltas_sec.iter().sum::<f64>() / state.deltas_sec.len() as f64;
        if mean_delta <= 0.0 {
            continue;
        }
        let fps = 1.0 / mean_delta;
        let variance = state
            .deltas_sec
            .iter()
            .map(|d| {
                let delta = d - mean_delta;
                delta * delta
            })
            .sum::<f64>()
            / state.deltas_sec.len() as f64;
        let std_dev = variance.sqrt();
        let cv = std_dev / mean_delta.abs();

        if cv > 0.2 && issues.len() < MAX_ISSUES_PER_CHECK {
            issues.push(Issue::new(
                Severity::Warning,
                IssueCategory::Temporal,
                "temporal.fps_instability",
                "FPS instability detected",
                format!("episode {episode} has unstable frame timing (cv={cv:.3})"),
                Some("Inspect timestamp consistency for this episode.".to_string()),
                None,
                Some(episode),
                None,
                None,
                Some("timestamp".to_string()),
                Some(json!({ "max_cv": 0.2 })),
                Some(json!({ "cv": cv, "estimated_fps": fps })),
                json!({}),
            ));
        }

        if let Some(expected_fps) = config.expected_fps {
            let relative_gap = ((fps - expected_fps) / expected_fps).abs();
            if relative_gap > 0.2 && issues.len() < MAX_ISSUES_PER_CHECK {
                issues.push(Issue::new(
                    Severity::Warning,
                    IssueCategory::Temporal,
                    "temporal.fps_mismatch",
                    "FPS mismatch",
                    format!(
                        "episode {episode} estimated fps ({fps:.2}) differs from expected fps ({expected_fps:.2})"
                    ),
                    Some("Check recorder FPS settings or timestamp generation.".to_string()),
                    None,
                    Some(episode),
                    None,
                    None,
                    Some("timestamp".to_string()),
                    Some(json!({ "expected_fps": expected_fps })),
                    Some(json!({ "estimated_fps": fps })),
                    json!({}),
                ));
            }
        }
    }

    issues
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

fn value_as_f64(array: &dyn Array, row: usize) -> Option<f64> {
    if array.is_null(row) {
        return None;
    }
    match array.data_type() {
        DataType::Float64 => Some(array.as_any().downcast_ref::<Float64Array>()?.value(row)),
        DataType::Float32 => Some(array.as_any().downcast_ref::<Float32Array>()?.value(row) as f64),
        _ => value_as_i64(array, row).map(|v| v as f64),
    }
}

fn allow_episode(max_episodes: Option<usize>, selected: &mut HashSet<i64>, episode: i64) -> bool {
    match max_episodes {
        Some(limit) if limit > 0 => {
            if selected.contains(&episode) {
                true
            } else if selected.len() < limit {
                selected.insert(episode);
                true
            } else {
                false
            }
        }
        _ => true,
    }
}
