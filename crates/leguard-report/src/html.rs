use leguard_core::report::ValidationReport;

pub fn render_html(report: &ValidationReport) -> String {
    let issue_rows = report
        .issues
        .iter()
        .map(|issue| {
            format!(
                "<tr data-severity=\"{severity}\" data-category=\"{category}\">\
                    <td>{severity}</td>\
                    <td>{category}</td>\
                    <td>{check_id}</td>\
                    <td>{title}</td>\
                    <td>{message}</td>\
                    <td>{file_path}</td>\
                </tr>",
                severity = format!("{:?}", issue.severity).to_lowercase(),
                category = format!("{:?}", issue.category).to_lowercase(),
                check_id = escape_html(&issue.check_id),
                title = escape_html(&issue.title),
                message = escape_html(&issue.message),
                file_path = escape_html(issue.file_path.as_deref().unwrap_or("-")),
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>LeGuard report</title>
  <style>
    body {{ font-family: Inter, Segoe UI, Arial, sans-serif; margin: 24px; background: #0b1020; color: #e5e7eb; }}
    h1, h2 {{ margin-bottom: 8px; }}
    .grid {{ display: grid; grid-template-columns: repeat(3, minmax(180px, 1fr)); gap: 12px; margin-bottom: 20px; }}
    .card {{ background: #111827; border: 1px solid #1f2937; border-radius: 8px; padding: 12px; }}
    .label {{ color: #9ca3af; font-size: 12px; text-transform: uppercase; }}
    .value {{ font-size: 22px; font-weight: 600; margin-top: 4px; }}
    table {{ width: 100%; border-collapse: collapse; background: #111827; border-radius: 8px; overflow: hidden; }}
    th, td {{ text-align: left; border-bottom: 1px solid #1f2937; padding: 8px; font-size: 13px; vertical-align: top; }}
    th {{ background: #0f172a; color: #cbd5e1; }}
    .toolbar {{ display: flex; gap: 10px; margin-bottom: 12px; }}
    input, select {{ background: #0f172a; color: #e5e7eb; border: 1px solid #334155; border-radius: 6px; padding: 6px 10px; }}
  </style>
</head>
<body>
  <h1>LeGuard report</h1>
  <p>Dataset: <strong>{dataset_name}</strong> | Generated: {generated_at}</p>
  <div class="grid">
    <div class="card"><div class="label">Status</div><div class="value">{status}</div></div>
    <div class="card"><div class="label">Issues</div><div class="value">{total}</div></div>
    <div class="card"><div class="label">Errors</div><div class="value">{errors}</div></div>
    <div class="card"><div class="label">Warnings</div><div class="value">{warnings}</div></div>
    <div class="card"><div class="label">Infos</div><div class="value">{infos}</div></div>
  </div>

  <h2>Issues</h2>
  <div class="toolbar">
    <select id="severity">
      <option value="">All severities</option>
      <option value="error">Error</option>
      <option value="warning">Warning</option>
      <option value="info">Info</option>
    </select>
    <select id="category">
      <option value="">All categories</option>
      <option value="structure">Structure</option>
      <option value="schema">Schema</option>
      <option value="temporal">Temporal</option>
      <option value="video">Video</option>
      <option value="numerical">Numerical</option>
      <option value="annotation">Annotation</option>
    </select>
    <input id="search" type="text" placeholder="Search check/message/file...">
  </div>
  <table>
    <thead>
      <tr><th>Severity</th><th>Category</th><th>Check</th><th>Title</th><th>Message</th><th>File</th></tr>
    </thead>
    <tbody id="issues-body">
      {issue_rows}
    </tbody>
  </table>

  <script>
    const severityEl = document.getElementById('severity');
    const categoryEl = document.getElementById('category');
    const searchEl = document.getElementById('search');
    const rows = Array.from(document.querySelectorAll('#issues-body tr'));
    function applyFilters() {{
      const severity = severityEl.value;
      const category = categoryEl.value;
      const query = searchEl.value.toLowerCase();
      rows.forEach((row) => {{
        const matchesSeverity = !severity || row.dataset.severity === severity;
        const matchesCategory = !category || row.dataset.category === category;
        const matchesQuery = !query || row.textContent.toLowerCase().includes(query);
        row.style.display = matchesSeverity && matchesCategory && matchesQuery ? '' : 'none';
      }});
    }}
    severityEl.addEventListener('change', applyFilters);
    categoryEl.addEventListener('change', applyFilters);
    searchEl.addEventListener('input', applyFilters);
  </script>
</body>
</html>"#,
        dataset_name = escape_html(&report.dataset.name),
        generated_at = report.generated_at,
        status = format!("{:?}", report.status()).to_uppercase(),
        total = report.issue_counts.total,
        errors = report.issue_counts.errors,
        warnings = report.issue_counts.warnings,
        infos = report.issue_counts.infos,
        issue_rows = issue_rows
    )
}

fn escape_html(raw: &str) -> String {
    raw.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}
