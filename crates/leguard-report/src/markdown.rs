use leguard_core::report::ValidationReport;

pub fn render_markdown(report: &ValidationReport) -> String {
    let mut output = String::new();
    output.push_str("# LeGuard report\n\n");
    output.push_str(&format!("Dataset: `{}`\n\n", report.dataset.name));
    output.push_str(&format!("Generated at: `{}`\n\n", report.generated_at));

    output.push_str("## Issue counts\n\n");
    output.push_str(&format!("- Total: {}\n", report.issue_counts.total));
    output.push_str(&format!("- Errors: {}\n", report.issue_counts.errors));
    output.push_str(&format!("- Warnings: {}\n", report.issue_counts.warnings));
    output.push_str(&format!("- Infos: {}\n\n", report.issue_counts.infos));

    output.push_str("## Issues\n\n");
    if report.issues.is_empty() {
        output.push_str("No issues detected.\n");
        return output;
    }

    for issue in &report.issues {
        output.push_str(&format!(
            "- **{}** `{}` (`{}`): {}\n",
            format!("{:?}", issue.severity).to_uppercase(),
            issue.check_id,
            format!("{:?}", issue.category).to_lowercase(),
            issue.message
        ));
    }

    output
}
