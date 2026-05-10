use std::{fs, path::Path};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckToggles {
    pub structure: bool,
    pub schema: bool,
    pub temporal: bool,
    pub video: bool,
    pub numerical: bool,
    pub annotation: bool,
}

impl Default for CheckToggles {
    fn default() -> Self {
        Self {
            structure: true,
            schema: true,
            temporal: true,
            video: true,
            numerical: true,
            annotation: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationConfig {
    pub expected_format: Option<String>,
    pub expected_fps: Option<f64>,
    pub max_timestamp_gap_ms_warning: Option<f64>,
    pub max_timestamp_gap_ms_error: Option<f64>,
    #[serde(default)]
    pub checks: CheckToggles,
    #[serde(default = "default_max_rows")]
    pub max_rows_per_parquet: usize,
}

const fn default_max_rows() -> usize {
    100_000
}

impl Default for ValidationConfig {
    fn default() -> Self {
        Self {
            expected_format: Some("lerobot-v3".to_string()),
            expected_fps: None,
            max_timestamp_gap_ms_warning: Some(200.0),
            max_timestamp_gap_ms_error: Some(500.0),
            checks: CheckToggles::default(),
            max_rows_per_parquet: default_max_rows(),
        }
    }
}

impl ValidationConfig {
    pub fn from_yaml_file(path: &Path) -> Result<Self> {
        let raw = fs::read_to_string(path)
            .with_context(|| format!("unable to read config file {}", path.display()))?;
        let config = serde_yaml::from_str::<ValidationConfig>(&raw)
            .with_context(|| format!("invalid yaml config {}", path.display()))?;
        Ok(config)
    }
}
