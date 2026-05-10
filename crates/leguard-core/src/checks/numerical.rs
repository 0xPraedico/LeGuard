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
struct ColumnStats {
    observed_rows: usize,
    null_count: usize,
    nan_count: usize,
    inf_count: usize,
    values: Vec<f64>,
}

pub fn run(scan: &DatasetScan, config: &ValidationConfig) -> Vec<Issue> {
    let mut issues = Vec::new();

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
        let episode_col_idx = schema.index_of("episode_index").ok();
        let mut reader = match builder.with_batch_size(2048).build() {
            Ok(reader) => reader,
            Err(_) => continue,
        };

        let numeric_indices = schema
            .fields()
            .iter()
            .enumerate()
            .filter_map(|(idx, field)| {
                if is_numeric_scalar(field.data_type()) {
                    Some((idx, field.name().clone()))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        let mut stats_by_col: HashMap<String, ColumnStats> = HashMap::new();
        let mut seen_rows = 0usize;
        let mut selected_episodes = HashSet::new();

        for batch in &mut reader {
            let Ok(batch) = batch else { continue };
            for row in 0..batch.num_rows() {
                if seen_rows >= config.max_rows_per_parquet {
                    break;
                }

                if let Some(ep_col_idx) = episode_col_idx {
                    if let Some(episode) = value_as_i64(batch.column(ep_col_idx).as_ref(), row) {
                        if !allow_episode(config.max_episodes, &mut selected_episodes, episode) {
                            continue;
                        }
                    }
                }

                seen_rows += 1;

                for (column_idx, column_name) in &numeric_indices {
                    let column = batch.column(*column_idx).as_ref();
                    let stats = stats_by_col.entry(column_name.clone()).or_default();
                    stats.observed_rows += 1;

                    if column.is_null(row) {
                        stats.null_count += 1;
                        continue;
                    }

                    if let Some(value) = value_as_f64(column, row) {
                        if value.is_nan() {
                            stats.nan_count += 1;
                        } else if value.is_infinite() {
                            stats.inf_count += 1;
                        } else if stats.values.len() < 50_000 {
                            stats.values.push(value);
                        }
                    }
                }
            }

            if seen_rows >= config.max_rows_per_parquet {
                break;
            }
        }

        for (column_name, stats) in stats_by_col {
            if issues.len() >= MAX_ISSUES_PER_CHECK {
                break;
            }

            if stats.nan_count > 0 {
                issues.push(Issue::new(
                    Severity::Warning,
                    IssueCategory::Numerical,
                    "numerical.nan",
                    "NaN values detected",
                    format!(
                        "{} NaN values found in numeric column {}",
                        stats.nan_count, column_name
                    ),
                    Some("Impute, filter, or regenerate this column.".to_string()),
                    Some(parquet_file.relative_path.clone()),
                    None,
                    None,
                    None,
                    Some(column_name.clone()),
                    Some(json!({ "nan_count": 0 })),
                    Some(json!({ "nan_count": stats.nan_count })),
                    json!({}),
                ));
            }

            if stats.inf_count > 0 {
                issues.push(Issue::new(
                    Severity::Warning,
                    IssueCategory::Numerical,
                    "numerical.inf",
                    "Infinite values detected",
                    format!(
                        "{} infinite values found in numeric column {}",
                        stats.inf_count, column_name
                    ),
                    Some("Clamp or clean infinity values in this feature.".to_string()),
                    Some(parquet_file.relative_path.clone()),
                    None,
                    None,
                    None,
                    Some(column_name.clone()),
                    Some(json!({ "inf_count": 0 })),
                    Some(json!({ "inf_count": stats.inf_count })),
                    json!({}),
                ));
            }

            if stats.observed_rows > 0 {
                let null_ratio = stats.null_count as f64 / stats.observed_rows as f64;
                if null_ratio > 0.2 {
                    issues.push(Issue::new(
                        Severity::Warning,
                        IssueCategory::Numerical,
                        "numerical.high_null_ratio",
                        "High null ratio in numeric column",
                        format!(
                            "Column {} has a high null ratio ({:.2}%)",
                            column_name,
                            null_ratio * 100.0
                        ),
                        Some("Check feature extraction for missing values.".to_string()),
                        Some(parquet_file.relative_path.clone()),
                        None,
                        None,
                        None,
                        Some(column_name.clone()),
                        Some(json!({ "max_null_ratio": 0.2 })),
                        Some(json!({ "null_ratio": null_ratio })),
                        json!({}),
                    ));
                }
            }

            if stats.values.len() > 1 {
                let mean = stats.values.iter().sum::<f64>() / stats.values.len() as f64;
                let variance = stats
                    .values
                    .iter()
                    .map(|v| {
                        let delta = v - mean;
                        delta * delta
                    })
                    .sum::<f64>()
                    / stats.values.len() as f64;
                let std_dev = variance.sqrt();

                if std_dev < 1e-9 {
                    issues.push(Issue::new(
                        Severity::Warning,
                        IssueCategory::Numerical,
                        "numerical.near_zero_variance",
                        "Near-zero variance column",
                        format!("Column {} has near-zero variance", column_name),
                        Some("Drop or review this constant-like feature.".to_string()),
                        Some(parquet_file.relative_path.clone()),
                        None,
                        None,
                        None,
                        Some(column_name.clone()),
                        Some(json!({ "std_dev": ">1e-9" })),
                        Some(json!({ "std_dev": std_dev })),
                        json!({ "mean": mean }),
                    ));
                } else {
                    let outlier_count = stats
                        .values
                        .iter()
                        .filter(|v| ((**v - mean) / std_dev).abs() > 6.0)
                        .count();
                    if outlier_count > 0 {
                        issues.push(Issue::new(
                            Severity::Warning,
                            IssueCategory::Numerical,
                            "numerical.outlier",
                            "Potential outliers detected",
                            format!(
                                "Column {} has {} extreme values based on z-score",
                                column_name, outlier_count
                            ),
                            Some("Inspect data distribution and clipping policy.".to_string()),
                            Some(parquet_file.relative_path.clone()),
                            None,
                            None,
                            None,
                            Some(column_name.clone()),
                            Some(json!({ "outliers": 0 })),
                            Some(json!({ "outliers": outlier_count })),
                            json!({ "threshold_z": 6.0 }),
                        ));
                    }
                }
            }
        }
    }

    issues
}

fn is_numeric_scalar(data_type: &DataType) -> bool {
    matches!(
        data_type,
        DataType::Int8
            | DataType::Int16
            | DataType::Int32
            | DataType::Int64
            | DataType::UInt8
            | DataType::UInt16
            | DataType::UInt32
            | DataType::UInt64
            | DataType::Float32
            | DataType::Float64
    )
}

fn value_as_f64(array: &dyn Array, row: usize) -> Option<f64> {
    if array.is_null(row) {
        return None;
    }
    match array.data_type() {
        DataType::Float64 => Some(array.as_any().downcast_ref::<Float64Array>()?.value(row)),
        DataType::Float32 => Some(array.as_any().downcast_ref::<Float32Array>()?.value(row) as f64),
        DataType::Int64 => Some(array.as_any().downcast_ref::<Int64Array>()?.value(row) as f64),
        DataType::Int32 => Some(array.as_any().downcast_ref::<Int32Array>()?.value(row) as f64),
        DataType::Int16 => Some(array.as_any().downcast_ref::<Int16Array>()?.value(row) as f64),
        DataType::Int8 => Some(array.as_any().downcast_ref::<Int8Array>()?.value(row) as f64),
        DataType::UInt64 => Some(array.as_any().downcast_ref::<UInt64Array>()?.value(row) as f64),
        DataType::UInt32 => Some(array.as_any().downcast_ref::<UInt32Array>()?.value(row) as f64),
        DataType::UInt16 => Some(array.as_any().downcast_ref::<UInt16Array>()?.value(row) as f64),
        DataType::UInt8 => Some(array.as_any().downcast_ref::<UInt8Array>()?.value(row) as f64),
        _ => None,
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
