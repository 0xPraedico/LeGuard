use std::collections::{BTreeSet, HashMap, HashSet};
use std::fs;

use arrow_array::{
    Array, Int16Array, Int32Array, Int64Array, Int8Array, LargeListArray, ListArray, UInt16Array,
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
struct EpisodeProfile {
    columns: BTreeSet<String>,
    dtypes: HashMap<String, String>,
    sample_shapes: HashMap<String, usize>,
}

pub fn run(scan: &DatasetScan, config: &ValidationConfig) -> Vec<Issue> {
    let mut issues = Vec::new();
    let mut profiles: HashMap<i64, EpisodeProfile> = HashMap::new();

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
        let episode_idx = match schema.index_of("episode_index") {
            Ok(idx) => idx,
            Err(_) => continue,
        };
        let mut reader = match builder.with_batch_size(2048).build() {
            Ok(reader) => reader,
            Err(_) => continue,
        };

        let columns = schema
            .fields()
            .iter()
            .map(|field| field.name().clone())
            .collect::<BTreeSet<_>>();
        let dtypes = schema
            .fields()
            .iter()
            .map(|field| (field.name().clone(), field.data_type().to_string()))
            .collect::<HashMap<_, _>>();

        let mut seen_rows = 0usize;
        for batch in &mut reader {
            let Ok(batch) = batch else { continue };
            for row in 0..batch.num_rows() {
                if seen_rows >= config.max_rows_per_parquet {
                    break;
                }
                seen_rows += 1;

                let Some(episode) = value_as_i64(batch.column(episode_idx).as_ref(), row) else {
                    continue;
                };
                if !is_allowed_episode(config.max_episodes, &profiles, episode) {
                    continue;
                }

                let profile = profiles.entry(episode).or_insert_with(|| EpisodeProfile {
                    columns: columns.clone(),
                    dtypes: dtypes.clone(),
                    sample_shapes: HashMap::new(),
                });

                for (col_idx, field) in schema.fields().iter().enumerate() {
                    let name = field.name();
                    if profile.sample_shapes.contains_key(name) {
                        continue;
                    }
                    if let Some(shape) = infer_shape(batch.column(col_idx).as_ref(), row) {
                        profile.sample_shapes.insert(name.clone(), shape);
                    }
                }
            }
            if seen_rows >= config.max_rows_per_parquet {
                break;
            }
        }
    }

    if profiles.len() < 2 {
        return issues;
    }

    let mut episode_ids = profiles.keys().copied().collect::<Vec<_>>();
    episode_ids.sort_unstable();
    let baseline_episode = episode_ids[0];
    let Some(baseline) = profiles.get(&baseline_episode) else {
        return issues;
    };

    for episode in &episode_ids[1..] {
        if issues.len() >= MAX_ISSUES_PER_CHECK {
            break;
        }
        let Some(profile) = profiles.get(episode) else {
            continue;
        };
        let missing = baseline
            .columns
            .difference(&profile.columns)
            .take(8)
            .cloned()
            .collect::<Vec<_>>();
        if !missing.is_empty() {
            issues.push(Issue::new(
                Severity::Warning,
                IssueCategory::Schema,
                "consistency.cross_episode_missing_columns",
                "Cross-episode missing columns",
                format!(
                    "Episode {} is missing columns present in episode {}: {:?}",
                    episode, baseline_episode, missing
                ),
                Some("Align parquet schemas across episodes/shards.".to_string()),
                None,
                Some(*episode),
                None,
                None,
                None,
                None,
                None,
                json!({ "baseline_episode": baseline_episode, "missing": missing }),
            ));
        }

        let unexpected = profile
            .columns
            .difference(&baseline.columns)
            .take(8)
            .cloned()
            .collect::<Vec<_>>();
        if !unexpected.is_empty() {
            issues.push(Issue::new(
                Severity::Warning,
                IssueCategory::Schema,
                "consistency.cross_episode_unexpected_columns",
                "Cross-episode unexpected columns",
                format!(
                    "Episode {} has extra columns compared to episode {}: {:?}",
                    episode, baseline_episode, unexpected
                ),
                Some("Keep a stable feature schema for every episode.".to_string()),
                None,
                Some(*episode),
                None,
                None,
                None,
                None,
                None,
                json!({ "baseline_episode": baseline_episode, "unexpected": unexpected }),
            ));
        }
    }

    let mut dtypes_by_column: HashMap<String, HashSet<String>> = HashMap::new();
    let mut shapes_by_column: HashMap<String, HashSet<usize>> = HashMap::new();
    for profile in profiles.values() {
        for (column, dtype) in &profile.dtypes {
            dtypes_by_column
                .entry(column.clone())
                .or_default()
                .insert(dtype.clone());
        }
        for (column, shape) in &profile.sample_shapes {
            shapes_by_column
                .entry(column.clone())
                .or_default()
                .insert(*shape);
        }
    }

    for (column, dtypes) in dtypes_by_column {
        if issues.len() >= MAX_ISSUES_PER_CHECK {
            break;
        }
        if dtypes.len() > 1 {
            let variants = dtypes.into_iter().collect::<Vec<_>>();
            issues.push(Issue::new(
                Severity::Warning,
                IssueCategory::Schema,
                "consistency.cross_episode_dtype_mismatch",
                "Cross-episode dtype mismatch",
                format!("Column {} has inconsistent dtypes across episodes", column),
                Some("Unify dtype encoding for this feature across shards.".to_string()),
                None,
                None,
                None,
                None,
                Some(column.clone()),
                None,
                None,
                json!({ "dtypes": variants }),
            ));
        }
    }

    for (column, shapes) in &shapes_by_column {
        if issues.len() >= MAX_ISSUES_PER_CHECK {
            break;
        }
        if shapes.len() > 1 {
            let variants = shapes.iter().copied().collect::<Vec<_>>();
            issues.push(Issue::new(
                Severity::Warning,
                IssueCategory::Schema,
                "consistency.cross_episode_shape_mismatch",
                "Cross-episode shape mismatch",
                format!(
                    "Column {} has inconsistent observed vector lengths across episodes",
                    column
                ),
                Some("Ensure feature tensors keep a fixed shape in every episode.".to_string()),
                None,
                None,
                None,
                None,
                Some(column.clone()),
                None,
                None,
                json!({ "observed_lengths": variants }),
            ));
        }
    }

    let expected_feature_sizes = load_expected_feature_sizes(scan);
    for (feature, expected_len) in expected_feature_sizes {
        if issues.len() >= MAX_ISSUES_PER_CHECK {
            break;
        }
        let Some(observed) = shapes_by_column.get(&feature) else {
            continue;
        };
        if observed.iter().any(|value| *value != expected_len) {
            issues.push(Issue::new(
                Severity::Warning,
                IssueCategory::Schema,
                "consistency.feature_shape_mismatch",
                "Feature shape mismatch against metadata",
                format!(
                    "Feature {} has observed lengths {:?}, expected {} from meta/info.json",
                    feature, observed, expected_len
                ),
                Some("Reconcile feature encoding and metadata shape declarations.".to_string()),
                None,
                None,
                None,
                None,
                Some(feature.clone()),
                Some(json!({ "expected_first_dim": expected_len })),
                Some(json!({ "observed_lengths": observed })),
                json!({}),
            ));
        }
    }

    issues
}

fn is_allowed_episode(
    max_episodes: Option<usize>,
    profiles: &HashMap<i64, EpisodeProfile>,
    episode: i64,
) -> bool {
    match max_episodes {
        Some(limit) if limit > 0 => profiles.contains_key(&episode) || profiles.len() < limit,
        _ => true,
    }
}

fn infer_shape(array: &dyn Array, row: usize) -> Option<usize> {
    if array.is_null(row) {
        return None;
    }
    match array.data_type() {
        DataType::List(_) => array
            .as_any()
            .downcast_ref::<ListArray>()
            .map(|values| values.value(row).len()),
        DataType::LargeList(_) => array
            .as_any()
            .downcast_ref::<LargeListArray>()
            .map(|values| values.value(row).len()),
        DataType::FixedSizeList(_, size) => Some(*size as usize),
        _ => None,
    }
}

fn load_expected_feature_sizes(scan: &DatasetScan) -> HashMap<String, usize> {
    let path = scan.root.join("meta").join("info.json");
    let raw = match fs::read_to_string(path) {
        Ok(raw) => raw,
        Err(_) => return HashMap::new(),
    };
    let value = match serde_json::from_str::<serde_json::Value>(&raw) {
        Ok(value) => value,
        Err(_) => return HashMap::new(),
    };
    let Some(features) = value.get("features").and_then(|v| v.as_object()) else {
        return HashMap::new();
    };

    let mut expected = HashMap::new();
    for (name, spec) in features {
        if spec.get("dtype").and_then(|v| v.as_str()) == Some("video") {
            continue;
        }
        let Some(shape) = spec.get("shape").and_then(|v| v.as_array()) else {
            continue;
        };
        let Some(first_dim) = shape.first().and_then(|v| v.as_u64()) else {
            continue;
        };
        if first_dim > 1 {
            expected.insert(name.clone(), first_dim as usize);
        }
    }
    expected
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
