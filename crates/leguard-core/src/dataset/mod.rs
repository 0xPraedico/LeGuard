use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use walkdir::WalkDir;

use crate::report::DatasetSummary;

#[derive(Debug, Clone)]
pub struct ScannedFile {
    pub path: PathBuf,
    pub relative_path: String,
    pub size_bytes: u64,
}

#[derive(Debug, Clone)]
pub struct DatasetScan {
    pub root: PathBuf,
    pub files: Vec<ScannedFile>,
    pub metadata_files: Vec<ScannedFile>,
    pub parquet_files: Vec<ScannedFile>,
    pub video_files: Vec<ScannedFile>,
    pub empty_files: Vec<ScannedFile>,
    pub has_meta_dir: bool,
    pub has_data_dir: bool,
    pub has_videos_dir: bool,
}

impl DatasetScan {
    pub fn summarize(&self, expected_format: Option<&str>) -> DatasetSummary {
        let mut cameras = BTreeSet::new();
        for file in &self.video_files {
            let parts = file.relative_path.split('/').collect::<Vec<_>>();
            if parts.len() > 2 && parts.first() == Some(&"videos") {
                cameras.insert(parts[1].to_string());
            }
        }

        DatasetSummary {
            name: self
                .root
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "dataset".to_string()),
            format: expected_format.unwrap_or("unknown").to_string(),
            uri: format!("local://{}", self.root.display()),
            source_type: "local".to_string(),
            episodes: 0,
            frames: 0,
            duration_sec: 0.0,
            cameras: cameras.into_iter().collect(),
            parquet_files: self.parquet_files.len() as u64,
            video_files: self.video_files.len() as u64,
            metadata_files: self.metadata_files.len() as u64,
            empty_files: self.empty_files.len() as u64,
            total_files: self.files.len() as u64,
        }
    }

    pub fn has_common_layout(&self) -> bool {
        self.has_meta_dir && self.has_data_dir && self.has_videos_dir
    }
}

pub fn scan_dataset(path: &Path) -> Result<DatasetScan> {
    let mut files = Vec::new();
    let mut metadata_files = Vec::new();
    let mut parquet_files = Vec::new();
    let mut video_files = Vec::new();
    let mut empty_files = Vec::new();

    for entry in WalkDir::new(path).follow_links(false) {
        let entry = entry.with_context(|| format!("failed to traverse {}", path.display()))?;
        if !entry.file_type().is_file() {
            continue;
        }

        let full_path = entry.path().to_path_buf();
        let relative_path = full_path
            .strip_prefix(path)
            .map(|p| p.to_string_lossy().replace('\\', "/"))
            .unwrap_or_else(|_| full_path.to_string_lossy().replace('\\', "/"));
        let metadata = entry
            .metadata()
            .with_context(|| format!("failed to read metadata for {}", full_path.display()))?;

        let file = ScannedFile {
            path: full_path,
            relative_path,
            size_bytes: metadata.len(),
        };

        let extension = file
            .path
            .extension()
            .map(|s| s.to_string_lossy().to_lowercase())
            .unwrap_or_default();

        if file.size_bytes == 0 {
            empty_files.push(file.clone());
        }

        match extension.as_str() {
            "json" | "jsonl" => metadata_files.push(file.clone()),
            "parquet" => parquet_files.push(file.clone()),
            "mp4" | "avi" | "mov" | "mkv" => video_files.push(file.clone()),
            _ => {}
        }

        files.push(file);
    }

    Ok(DatasetScan {
        root: path.to_path_buf(),
        files,
        metadata_files,
        parquet_files,
        video_files,
        empty_files,
        has_meta_dir: path.join("meta").is_dir(),
        has_data_dir: path.join("data").is_dir(),
        has_videos_dir: path.join("videos").is_dir(),
    })
}
