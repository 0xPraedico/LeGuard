use std::process::Command;

use serde_json::json;

use crate::{
    checks::MAX_ISSUES_PER_CHECK,
    dataset::DatasetScan,
    issue::{Issue, IssueCategory, Severity},
};

pub fn run(scan: &DatasetScan) -> Vec<Issue> {
    let mut issues = Vec::new();

    for video in &scan.video_files {
        if video.size_bytes == 0 {
            issues.push(Issue::new(
                Severity::Error,
                IssueCategory::Video,
                "video.empty_file",
                "Empty video file",
                format!("Video file {} has zero size", video.relative_path),
                Some("Regenerate or restore the missing video content.".to_string()),
                Some(video.relative_path.clone()),
                None,
                None,
                None,
                None,
                Some(json!({ "size_bytes": ">0" })),
                Some(json!({ "size_bytes": 0 })),
                json!({}),
            ));
        }
    }

    let ffprobe_available = Command::new("ffprobe")
        .arg("-version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false);

    if !ffprobe_available {
        issues.push(Issue::new(
            Severity::Info,
            IssueCategory::Video,
            "video.ffprobe_unavailable",
            "ffprobe unavailable",
            "ffprobe is not available, advanced video integrity checks were skipped".to_string(),
            Some("Install ffmpeg/ffprobe to enable video metadata checks.".to_string()),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            json!({}),
        ));
        return issues;
    }

    for video in &scan.video_files {
        if issues.len() >= MAX_ISSUES_PER_CHECK {
            break;
        }
        if video.size_bytes == 0 {
            continue;
        }

        let output = Command::new("ffprobe")
            .arg("-v")
            .arg("error")
            .arg("-select_streams")
            .arg("v:0")
            .arg("-show_entries")
            .arg("stream=width,height,r_frame_rate")
            .arg("-show_entries")
            .arg("format=duration")
            .arg("-of")
            .arg("json")
            .arg(&video.path)
            .output();

        let output = match output {
            Ok(output) => output,
            Err(error) => {
                issues.push(Issue::new(
                    Severity::Warning,
                    IssueCategory::Video,
                    "video.unreadable",
                    "Video probe failed",
                    format!("Failed to run ffprobe on {}: {error}", video.relative_path),
                    Some("Check file permissions and ffmpeg installation.".to_string()),
                    Some(video.relative_path.clone()),
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

        if !output.status.success() {
            issues.push(Issue::new(
                Severity::Warning,
                IssueCategory::Video,
                "video.unreadable",
                "Unreadable video file",
                format!("ffprobe failed for {}", video.relative_path),
                Some("Re-encode or replace this video file.".to_string()),
                Some(video.relative_path.clone()),
                None,
                None,
                None,
                None,
                None,
                None,
                json!({ "stderr": String::from_utf8_lossy(&output.stderr) }),
            ));
            continue;
        }

        let parsed = serde_json::from_slice::<serde_json::Value>(&output.stdout);
        let Ok(parsed) = parsed else {
            issues.push(Issue::new(
                Severity::Warning,
                IssueCategory::Video,
                "video.unreadable",
                "Invalid ffprobe output",
                format!("ffprobe returned invalid JSON for {}", video.relative_path),
                Some("Run ffprobe manually and verify the file integrity.".to_string()),
                Some(video.relative_path.clone()),
                None,
                None,
                None,
                None,
                None,
                None,
                json!({}),
            ));
            continue;
        };

        let duration = parsed
            .get("format")
            .and_then(|v| v.get("duration"))
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(0.0);

        let stream = parsed
            .get("streams")
            .and_then(|v| v.as_array())
            .and_then(|arr| arr.first())
            .cloned()
            .unwrap_or_else(|| json!({}));

        let width = stream.get("width").and_then(|v| v.as_i64()).unwrap_or(0);
        let height = stream.get("height").and_then(|v| v.as_i64()).unwrap_or(0);
        let fps = stream
            .get("r_frame_rate")
            .and_then(|v| v.as_str())
            .and_then(parse_ffprobe_fraction)
            .unwrap_or(0.0);

        if duration <= 0.0 {
            issues.push(Issue::new(
                Severity::Warning,
                IssueCategory::Video,
                "video.zero_duration",
                "Video has zero duration",
                format!(
                    "Video {} has invalid duration {}",
                    video.relative_path, duration
                ),
                Some("Re-encode this video to repair container metadata.".to_string()),
                Some(video.relative_path.clone()),
                None,
                None,
                None,
                None,
                Some(json!({ "duration_sec": ">0" })),
                Some(json!({ "duration_sec": duration })),
                json!({}),
            ));
        }

        if fps <= 0.0 {
            issues.push(Issue::new(
                Severity::Warning,
                IssueCategory::Video,
                "video.invalid_fps",
                "Video has invalid FPS",
                format!("Video {} has invalid FPS {}", video.relative_path, fps),
                Some("Inspect recording/export settings.".to_string()),
                Some(video.relative_path.clone()),
                None,
                None,
                None,
                None,
                Some(json!({ "fps": ">0" })),
                Some(json!({ "fps": fps })),
                json!({}),
            ));
        }

        if width <= 0 || height <= 0 {
            issues.push(Issue::new(
                Severity::Warning,
                IssueCategory::Video,
                "video.invalid_resolution",
                "Video has invalid resolution",
                format!(
                    "Video {} has invalid dimensions {}x{}",
                    video.relative_path, width, height
                ),
                Some("Verify source videos were encoded correctly.".to_string()),
                Some(video.relative_path.clone()),
                None,
                None,
                None,
                None,
                Some(json!({ "width": ">0", "height": ">0" })),
                Some(json!({ "width": width, "height": height })),
                json!({}),
            ));
        }
    }

    issues
}

fn parse_ffprobe_fraction(raw: &str) -> Option<f64> {
    if let Ok(v) = raw.parse::<f64>() {
        return Some(v);
    }
    let (num, den) = raw.split_once('/')?;
    let num = num.parse::<f64>().ok()?;
    let den = den.parse::<f64>().ok()?;
    if den == 0.0 {
        return None;
    }
    Some(num / den)
}
