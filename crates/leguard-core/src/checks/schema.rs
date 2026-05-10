use std::collections::BTreeSet;

use parquet::file::reader::{FileReader, SerializedFileReader};
use serde_json::json;

use crate::{
    checks::MAX_ISSUES_PER_CHECK,
    dataset::DatasetScan,
    issue::{Issue, IssueCategory, Severity},
};

const COMMON_COLUMNS: [&str; 5] = [
    "episode_index",
    "frame_index",
    "timestamp",
    "action",
    "observation.state",
];

pub fn run(scan: &DatasetScan) -> Vec<Issue> {
    let mut issues = Vec::new();
    let mut baseline_columns: Option<BTreeSet<String>> = None;

    for parquet in &scan.parquet_files {
        if issues.len() >= MAX_ISSUES_PER_CHECK {
            break;
        }

        let file = match std::fs::File::open(&parquet.path) {
            Ok(file) => file,
            Err(error) => {
                issues.push(Issue::new(
                    Severity::Error,
                    IssueCategory::Schema,
                    "schema.unreadable_parquet",
                    "Unreadable parquet file",
                    format!(
                        "Unable to open parquet file {}: {error}",
                        parquet.relative_path
                    ),
                    Some("Check path permissions and file integrity.".to_string()),
                    Some(parquet.relative_path.clone()),
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    json!({ "error": error.to_string() }),
                ));
                continue;
            }
        };

        let reader = match SerializedFileReader::new(file) {
            Ok(reader) => reader,
            Err(error) => {
                issues.push(Issue::new(
                    Severity::Error,
                    IssueCategory::Schema,
                    "schema.unreadable_parquet",
                    "Unreadable parquet file",
                    format!(
                        "Unable to read parquet file {}: {error}",
                        parquet.relative_path
                    ),
                    Some("Recreate or repair this parquet shard.".to_string()),
                    Some(parquet.relative_path.clone()),
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    json!({ "error": error.to_string() }),
                ));
                continue;
            }
        };

        let metadata = reader.metadata().file_metadata();
        if metadata.num_rows() == 0 {
            issues.push(Issue::new(
                Severity::Warning,
                IssueCategory::Schema,
                "schema.empty_parquet",
                "Empty parquet shard",
                format!("Parquet file {} contains zero rows", parquet.relative_path),
                Some("Drop empty shard or regenerate it with valid rows.".to_string()),
                Some(parquet.relative_path.clone()),
                None,
                None,
                None,
                None,
                Some(json!({ "rows": ">0" })),
                Some(json!({ "rows": 0 })),
                json!({}),
            ));
        }

        let columns = metadata
            .schema_descr()
            .columns()
            .iter()
            .map(|column| column.path().string())
            .collect::<BTreeSet<_>>();

        if let Some(base) = &baseline_columns {
            if *base != columns {
                let missing = base.difference(&columns).cloned().collect::<Vec<_>>();
                let unexpected = columns.difference(base).cloned().collect::<Vec<_>>();
                issues.push(Issue::new(
                    Severity::Warning,
                    IssueCategory::Schema,
                    "schema.inconsistent_columns",
                    "Inconsistent parquet schema across shards",
                    format!(
                        "Parquet file {} has schema differences compared to first shard",
                        parquet.relative_path
                    ),
                    Some("Align shard schemas before training or release.".to_string()),
                    Some(parquet.relative_path.clone()),
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    json!({ "missing": missing, "unexpected": unexpected }),
                ));
            }
        } else {
            baseline_columns = Some(columns.clone());
        }

        for required in COMMON_COLUMNS {
            if !columns.contains(required) {
                issues.push(Issue::new(
                    Severity::Warning,
                    IssueCategory::Schema,
                    "schema.missing_common_column",
                    "Missing common column",
                    format!(
                        "Column {} is missing from parquet shard {}",
                        required, parquet.relative_path
                    ),
                    Some(
                        "If this is intentional for your LeRobot variant, document the schema."
                            .to_string(),
                    ),
                    Some(parquet.relative_path.clone()),
                    None,
                    None,
                    None,
                    Some(required.to_string()),
                    Some(json!({ "column_present": true })),
                    Some(json!({ "column_present": false })),
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
