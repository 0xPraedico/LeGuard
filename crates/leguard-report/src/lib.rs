pub mod html;
pub mod junit;
pub mod markdown;

use anyhow::Result;
use leguard_core::report::ValidationReport;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportFormat {
    Json,
    Html,
    Markdown,
    Junit,
}

impl ReportFormat {
    pub fn parse(value: &str) -> Option<Self> {
        match value.to_lowercase().as_str() {
            "json" => Some(Self::Json),
            "html" => Some(Self::Html),
            "markdown" | "md" => Some(Self::Markdown),
            "junit" | "xml" => Some(Self::Junit),
            _ => None,
        }
    }
}

pub fn render_report(report: &ValidationReport, format: ReportFormat) -> Result<String> {
    let rendered = match format {
        ReportFormat::Json => serde_json::to_string_pretty(report)?,
        ReportFormat::Html => html::render_html(report),
        ReportFormat::Markdown => markdown::render_markdown(report),
        ReportFormat::Junit => junit::render_junit(report),
    };
    Ok(rendered)
}
