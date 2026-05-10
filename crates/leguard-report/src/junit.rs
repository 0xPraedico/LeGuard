use leguard_core::{issue::Severity, report::ValidationReport};

pub fn render_junit(report: &ValidationReport) -> String {
    let mut xml = String::new();
    xml.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    xml.push_str(&format!(
        "<testsuite name=\"leguard\" tests=\"{}\" failures=\"{}\" warnings=\"{}\">\n",
        report.issue_counts.total, report.issue_counts.errors, report.issue_counts.warnings
    ));

    for issue in &report.issues {
        xml.push_str(&format!(
            "  <testcase classname=\"{}\" name=\"{}\">\n",
            xml_escape(&format!("{:?}", issue.category).to_lowercase()),
            xml_escape(&issue.check_id),
        ));
        match issue.severity {
            Severity::Error => {
                xml.push_str(&format!(
                    "    <failure message=\"{}\">{}</failure>\n",
                    xml_escape(&issue.title),
                    xml_escape(&issue.message),
                ));
            }
            Severity::Warning => {
                xml.push_str(&format!(
                    "    <system-out>warning: {}</system-out>\n",
                    xml_escape(&issue.message),
                ));
            }
            Severity::Info => {
                xml.push_str(&format!(
                    "    <system-out>info: {}</system-out>\n",
                    xml_escape(&issue.message),
                ));
            }
        }
        xml.push_str("  </testcase>\n");
    }

    xml.push_str("</testsuite>\n");
    xml
}

fn xml_escape(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
