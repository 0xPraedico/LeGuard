import json
import os
import shutil
import subprocess
import uuid
from pathlib import Path

import gradio as gr
from huggingface_hub import snapshot_download

LEGUARD_BIN = os.environ.get("LEGUARD_BIN", "/usr/local/bin/leguard")
WORKDIR = Path("/tmp/leguard-space")
DATASETS_DIR = WORKDIR / "datasets"
RUNS_DIR = WORKDIR / "runs"
CHECK_ORDER = [
    "structure",
    "schema",
    "consistency",
    "episodes",
    "temporal",
    "numerical",
    "video",
    "annotation",
    "training",
    "portability",
]

BANNER_URL = "https://raw.githubusercontent.com/0xPraedico/LeGuard/main/assets/leguard-banner.png"
REPO_URL = "https://github.com/0xPraedico/LeGuard"
ISSUES_URL = f"{REPO_URL}/issues"

BOOTSTRAP_JS = """
() => {
  const root = document.documentElement;

  const applyTheme = (mode) => {
    const normalized = String(mode || "light").toLowerCase();
    const finalMode = normalized === "light" ? "light" : "dark";
    root.setAttribute("data-leguard-theme", finalMode);
  };
  window.__leguardApplyTheme = applyTheme;
  applyTheme("light");

  const localizeFooter = () => {
    const localizedBuiltWith = [67, 114, 233, 233, 32, 97, 118, 101, 99]
      .map((cp) => String.fromCodePoint(cp))
      .join("");
    const localizedSettings = [80, 97, 114, 97, 109, 232, 116, 114, 101, 115]
      .map((cp) => String.fromCodePoint(cp))
      .join("");
    const localizedError = [69, 114, 114, 101, 117, 114]
      .map((cp) => String.fromCodePoint(cp))
      .join("");
    const localizedWarning = [65, 118, 101, 114, 116, 105, 115, 115, 101, 109, 101, 110, 116]
      .map((cp) => String.fromCodePoint(cp))
      .join("");
    const replacer = {
      [localizedBuiltWith]: "Built with",
      [localizedSettings]: "Settings",
      [localizedError]: "Error",
      [localizedWarning]: "Warning",
    };
    const walker = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT);
    let current = walker.nextNode();
    while (current) {
      if (current.nodeValue) {
        Object.entries(replacer).forEach(([from, to]) => {
          if (current.nodeValue.includes(from)) {
            current.nodeValue = current.nodeValue.replace(from, to);
          }
        });
      }
      current = walker.nextNode();
    }
  };

  localizeFooter();
  const observer = new MutationObserver(() => localizeFooter());
  observer.observe(document.body, { childList: true, subtree: true });
}
"""

THEME_CHANGE_JS = """
(mode) => {
  if (window.__leguardApplyTheme) {
    window.__leguardApplyTheme(mode);
  }
  return [];
}
"""

CHECK_LABELS = {
    "structure": "Structure",
    "schema": "Schema",
    "consistency": "Consistency",
    "episodes": "Episodes",
    "temporal": "Temporal",
    "numerical": "Numerical",
    "video": "Video",
    "annotation": "Annotation",
    "training": "Training Readiness",
    "portability": "Portability",
}

CSS = """
:root,
html[data-leguard-theme="light"] {
  --lg-page-bg: #f3f6fb;
  --lg-panel-bg: #ffffff;
  --lg-surface-bg: #ffffff;
  --lg-border: #d8e1ef;
  --lg-text: #111827;
  --lg-muted: #5b6473;
  --lg-accent: #4f46e5;
  --lg-label-color: #5b6473;
  --lg-info-color: #4f46e5;
  --lg-accent-soft: #e0e7ff;
  --lg-input-bg: #ffffff;
  --lg-input-border: #c7d2e5;
  --lg-input-text: #111827;
}

html[data-leguard-theme="dark"] {
  --lg-page-bg: #060c1b;
  --lg-panel-bg: #0a1226;
  --lg-surface-bg: #0d1a34;
  --lg-border: #19345c;
  --lg-text: #e7eefc;
  --lg-muted: #93a9c9;
  --lg-accent: #22d3ee;
  --lg-label-color: #5b6473;
  --lg-info-color: #4f46e5;
  --lg-accent-soft: #14324a;
  --lg-input-bg: #09162d;
  --lg-input-border: #1b3d68;
  --lg-input-text: #dbeafe;
}

.gradio-container {
  background: var(--lg-page-bg) !important;
  color: var(--lg-text) !important;
}

.gradio-container,
.gradio-container * {
  border-color: var(--lg-border);
}

.gradio-container input[type="text"],
.gradio-container input[type="number"],
.gradio-container input[type="search"],
.gradio-container textarea,
.gradio-container .wrap.svelte-1ykxmf5,
.gradio-container .wrap.svelte-1rjryqp {
  background: var(--lg-input-bg) !important;
  color: var(--lg-input-text) !important;
  border-color: var(--lg-input-border) !important;
}

.gradio-container .tabs,
.gradio-container .gradio-accordion,
.gradio-container [role="tablist"],
.gradio-container .form,
.gradio-container .panel {
  background: var(--lg-panel-bg) !important;
  border-color: var(--lg-border) !important;
}

.gradio-container [role="tab"] {
  color: var(--lg-muted) !important;
}

.gradio-container [role="tab"][aria-selected="true"] {
  color: var(--lg-text) !important;
  border-bottom-color: var(--lg-accent) !important;
}

.gradio-container button {
  border-radius: 10px !important;
}

.gradio-container label,
.gradio-container legend,
.gradio-container .gr-form label,
.gradio-container .gr-block label,
.gradio-container .caption,
.gradio-container [data-testid="block-label"],
.gradio-container .svelte-1ipelgc,
.gradio-container [data-testid="block-label"] *,
.gradio-container .svelte-1ipelgc * {
  color: var(--lg-label-color) !important;
  opacity: 1 !important;
}

.gradio-container .prose,
.gradio-container .prose *,
.gradio-container .markdown,
.gradio-container .markdown * {
  color: var(--lg-text) !important;
  opacity: 1 !important;
}

.gradio-container .prose p,
.gradio-container .prose li,
.gradio-container .prose ul,
.gradio-container .prose ol,
.gradio-container .prose span {
  color: var(--lg-text) !important;
}

.gradio-container .prose h1,
.gradio-container .prose h2,
.gradio-container .prose h3,
.gradio-container .prose strong {
  color: var(--lg-text) !important;
}

.gradio-container .prose a,
.gradio-container .markdown a {
  color: var(--lg-accent) !important;
}

.gradio-container [data-testid="block-info"],
.gradio-container .block-info,
.gradio-container .gradio-info,
.gradio-container [data-testid="block-info"] *,
.gradio-container .block-info *,
.gradio-container .gradio-info * {
  color: var(--lg-info-color) !important;
  opacity: 1 !important;
}

.field-help {
  margin-top: 4px;
  font-size: 0.78rem;
  line-height: 1.25;
  color: var(--lg-info-color) !important;
  opacity: 1 !important;
}

.app-container {
  max-width: 1080px;
  margin: 0 auto;
}

.hero-card {
  border: 1px solid var(--lg-border);
  border-radius: 14px;
  padding: 14px 16px 10px 16px;
  background: var(--lg-panel-bg);
  margin-bottom: 14px;
}

.hero-card img {
  width: 100%;
  border-radius: 10px;
}

.hero-card h1 {
  margin: 10px 0 4px 0;
  font-size: 1.35rem;
}

.hero-card p {
  margin: 0;
  color: var(--lg-muted);
}

.hero-links {
  margin-top: 10px;
}

.hero-links a {
  color: var(--lg-accent);
  text-decoration: none;
  font-weight: 600;
}

.hero-links a:hover {
  text-decoration: underline;
}

.run-btn button {
  min-height: 48px !important;
  font-size: 1rem !important;
  font-weight: 700 !important;
  background: linear-gradient(90deg, #4f46e5, #4338ca) !important;
  color: #ffffff !important;
  border: none !important;
}

html[data-leguard-theme="dark"] .run-btn button {
  background: linear-gradient(90deg, #0ea5e9, #06b6d4) !important;
}

.refresh-btn button {
  min-height: 40px !important;
  font-size: 0.88rem !important;
  font-weight: 600 !important;
  background: var(--lg-surface-bg) !important;
  color: var(--lg-text) !important;
  border: 1px solid var(--lg-border) !important;
}

.status-card {
  border-radius: 10px;
  padding: 12px 14px;
  margin-bottom: 12px;
  font-weight: 600;
}

.gradio-container .status-card,
.gradio-container .status-card * {
  color: inherit !important;
  opacity: 1 !important;
}

.status-neutral {
  background: var(--lg-surface-bg);
  color: var(--lg-muted);
  border: 1px solid var(--lg-border);
}

.status-pass {
  background: #ecfdf3 !important;
  color: #166534 !important;
  border: 1px solid #bbf7d0 !important;
}

.status-warn {
  background: #fff7ed !important;
  color: #9a3412 !important;
  border: 1px solid #fed7aa !important;
}

.status-fail {
  background: #fef2f2 !important;
  color: #991b1b !important;
  border: 1px solid #fecaca !important;
}

html[data-leguard-theme="dark"] .status-pass {
  background: #052e1f !important;
  color: #86efac !important;
  border: 1px solid #166534 !important;
}

html[data-leguard-theme="dark"] .status-warn {
  background: #3b2206 !important;
  color: #fdba74 !important;
  border: 1px solid #9a5800 !important;
}

html[data-leguard-theme="dark"] .status-fail {
  background: #3a0b10 !important;
  color: #fca5a5 !important;
  border: 1px solid #b91c1c !important;
}

.metrics-grid {
  display: grid;
  grid-template-columns: repeat(3, minmax(120px, 1fr));
  gap: 10px;
  margin-bottom: 14px;
}

.metric {
  border: 1px solid var(--lg-border);
  border-radius: 10px;
  padding: 10px 12px;
  background: var(--lg-surface-bg);
}

.metric-label {
  font-size: 0.74rem;
  color: var(--lg-muted);
  text-transform: uppercase;
  letter-spacing: 0.04em;
}

.metric-value {
  margin-top: 4px;
  font-size: 1.1rem;
  font-weight: 700;
  color: var(--lg-text) !important;
}

.section-note {
  color: var(--lg-muted);
  font-size: 0.92rem;
}

.issues-card {
  border: 1px solid var(--lg-border);
  border-radius: 10px;
  background: var(--lg-surface-bg);
  padding: 12px;
  margin-top: 4px;
  color: var(--lg-muted);
  font-size: 0.92rem;
}

.issues-card a {
  color: var(--lg-accent);
  text-decoration: none;
  font-weight: 600;
}

.issues-card a:hover {
  text-decoration: underline;
}
"""

for directory in (DATASETS_DIR, RUNS_DIR):
    directory.mkdir(parents=True, exist_ok=True)


def run_command(command):
    completed = subprocess.run(command, capture_output=True, text=True)
    logs = [f"$ {' '.join(command)}"]
    if completed.stdout.strip():
        logs.append(completed.stdout.strip())
    if completed.stderr.strip():
        logs.append(completed.stderr.strip())
    return completed.returncode, "\n\n".join(logs)


def resolve_dataset_path(dataset_repo_id):
    dataset_repo_id = dataset_repo_id.strip()
    if not dataset_repo_id:
        raise ValueError("Enter a Hugging Face dataset id, for example: praedico/SO101_pillbox_vita")

    target_dir = DATASETS_DIR / dataset_repo_id.replace("/", "__")
    if target_dir.exists():
        shutil.rmtree(target_dir)
    token = os.environ.get("HF_TOKEN")
    snapshot_download(
        repo_id=dataset_repo_id,
        repo_type="dataset",
        local_dir=str(target_dir),
        token=token,
    )
    return target_dir


def summarize_report(report, check_exit):
    counts = report["issue_counts"]
    errors = counts["errors"]
    warnings = counts["warnings"]
    if errors > 0 or check_exit != 0:
        status = "FAIL"
        style = "status-fail"
        status_text = "Blocking issues detected. Review and fix before using this dataset in CI/training."
    elif warnings > 0:
        status = "WARN"
        style = "status-warn"
        status_text = "Warnings detected. Dataset is likely usable, but review issues before large runs."
    else:
        status = "PASS"
        style = "status-pass"
        status_text = "No blocking issues detected."

    return (
        f"<div class='status-card {style}'>[{status}] {status_text}</div>",
        style,
    )


def build_metrics_html(report):
    dataset = report["dataset"]["name"]
    counts = report["issue_counts"]
    return (
        "<div class='metrics-grid'>"
        f"<div class='metric'><div class='metric-label'>Dataset</div><div class='metric-value'>{dataset}</div></div>"
        f"<div class='metric'><div class='metric-label'>Total issues</div><div class='metric-value'>{counts['total']}</div></div>"
        f"<div class='metric'><div class='metric-label'>Errors</div><div class='metric-value'>{counts['errors']}</div></div>"
        f"<div class='metric'><div class='metric-label'>Warnings</div><div class='metric-value'>{counts['warnings']}</div></div>"
        f"<div class='metric'><div class='metric-label'>Infos</div><div class='metric-value'>{counts['infos']}</div></div>"
        "</div>"
    )

def empty_status_html():
    return "<div class='status-card status-neutral'>No diagnostics run yet.</div>"


def empty_details_markdown():
    return "Run diagnostics to display issues and recommendations."


def reset_diagnostics_state():
    return (
        empty_status_html(),
        "",
        empty_details_markdown(),
        "",
        None,
        None,
        None,
    )


def build_issue_markdown(report):
    grouped = {name: [] for name in CHECK_ORDER}
    grouped["other"] = []
    for issue in report.get("issues", []):
        check_id = issue.get("check_id", "unknown")
        prefix = check_id.split(".", 1)[0]
        if prefix in grouped:
            grouped[prefix].append(issue)
        else:
            grouped["other"].append(issue)

    sections = []
    for check_name in CHECK_ORDER + ["other"]:
        issues = grouped.get(check_name) or []
        if not issues:
            continue
        title = CHECK_LABELS.get(check_name, "Other")
        sections.append(f"### {title}")
        for issue in issues[:12]:
            severity = str(issue.get("severity", "info")).upper()
            message = issue.get("message", "No message")
            suggestion = issue.get("suggestion")
            line = f"- **[{severity}]** {message}"
            if suggestion:
                line += f"  \n  Suggestion: {suggestion}"
            sections.append(line)
        if len(issues) > 12:
            sections.append(f"- ... {len(issues) - 12} additional issue(s)")
        sections.append("")

    if not sections:
        return "No issues found."
    return "\n".join(sections).strip()


def build_runtime_config(selected_checks, max_episodes):
    if not selected_checks:
        raise ValueError("Select at least one check to run.")
    selected = set(selected_checks)

    lines = ["checks:"]
    for check_name in CHECK_ORDER:
        enabled = "true" if check_name in selected else "false"
        lines.append(f"  {check_name}: {enabled}")

    if max_episodes is not None:
        parsed_max_episodes = int(max_episodes)
        if parsed_max_episodes < 0:
            raise ValueError("Max episodes must be 0 or a positive integer.")
        if parsed_max_episodes > 0:
            lines.append(f"max_episodes: {parsed_max_episodes}")

    return "\n".join(lines) + "\n"


def selected_checks_from_flags(*flags):
    return [name for name, enabled in zip(CHECK_ORDER, flags) if enabled]


def set_check_flags(mode):
    if mode == "all":
        return (True,) * len(CHECK_ORDER)
    if mode == "recommended":
        recommended = {
            "structure",
            "schema",
            "consistency",
            "episodes",
            "temporal",
            "numerical",
            "training",
            "portability",
        }
        return tuple(check in recommended for check in CHECK_ORDER)
    return (False,) * len(CHECK_ORDER)


def run_leguard(
    dataset_repo_id,
    fail_on,
    max_episodes,
    check_structure,
    check_schema,
    check_consistency,
    check_episodes,
    check_temporal,
    check_numerical,
    check_video,
    check_annotation,
    check_training,
    check_portability,
):
    selected_checks = selected_checks_from_flags(
        check_structure,
        check_schema,
        check_consistency,
        check_episodes,
        check_temporal,
        check_numerical,
        check_video,
        check_annotation,
        check_training,
        check_portability,
    )
    try:
        dataset_path = resolve_dataset_path(dataset_repo_id)
        runtime_config = build_runtime_config(selected_checks, max_episodes)
    except Exception as exc:
        return (
            "<div class='status-card status-fail'>[FAIL] Unable to start diagnostics.</div>",
            "",
            f"### Error\n- {exc}",
            str(exc),
            None,
            None,
            None,
        )

    run_id = uuid.uuid4().hex[:8]
    run_dir = RUNS_DIR / run_id
    run_dir.mkdir(parents=True, exist_ok=True)

    runtime_config_path = run_dir / "space-config.yml"
    runtime_config_path.write_text(runtime_config, encoding="utf-8")

    json_report = run_dir / "report.json"
    html_report = run_dir / "report.html"

    check_exit, check_logs = run_command(
        [
            LEGUARD_BIN,
            "check",
            str(dataset_path),
            "--fail-on",
            fail_on,
            "--config",
            str(runtime_config_path),
        ]
    )
    json_exit, json_logs = run_command(
        [
            LEGUARD_BIN,
            "report",
            str(dataset_path),
            "--format",
            "json",
            "--out",
            str(json_report),
            "--config",
            str(runtime_config_path),
        ]
    )
    html_exit, html_logs = run_command(
        [
            LEGUARD_BIN,
            "report",
            str(dataset_path),
            "--format",
            "html",
            "--out",
            str(html_report),
            "--config",
            str(runtime_config_path),
        ]
    )

    all_logs = "\n\n---\n\n".join([check_logs, json_logs, html_logs])

    if json_exit != 0 or html_exit != 0 or not json_report.exists():
        status_html = (
            "<div class='status-card status-fail'>[FAIL] Report generation failed. "
            "Check command logs for details.</div>"
        )
        details_md = (
            "### Run failed\n"
            f"- Dataset path: `{dataset_path}`\n"
            "- `leguard report` did not produce the expected JSON/HTML outputs."
        )
        return status_html, "", details_md, all_logs, None, None, None

    report = json.loads(json_report.read_text(encoding="utf-8"))
    status_html, _ = summarize_report(report, check_exit)
    metrics_html = build_metrics_html(report)
    issues_markdown = build_issue_markdown(report)

    return (
        status_html,
        metrics_html,
        issues_markdown,
        all_logs,
        report,
        str(json_report),
        str(html_report),
    )


with gr.Blocks(
    title="LeGuard Space",
    css=CSS,
    theme=gr.themes.Soft(),
    js=BOOTSTRAP_JS,
) as demo:
    with gr.Column(elem_classes=["app-container"]):
        gr.HTML(
            f"""
            <div class="hero-card">
              <img src="{BANNER_URL}" alt="LeGuard banner" />
              <h1>LeGuard Space</h1>
              <p>Run LeGuard diagnostics on Hugging Face LeRobot datasets and export actionable reports.</p>
              <div class="hero-links"><a href="{REPO_URL}" target="_blank" rel="noopener noreferrer">GitHub</a></div>
            </div>
            """
        )

        with gr.Row():
            dataset_repo_id = gr.Textbox(
                label="Dataset (Hugging Face repo id)",
                placeholder="praedico/SO101_pillbox_vita",
                scale=4,
            )
            with gr.Column(scale=1):
                max_episodes = gr.Number(
                    label="Max episodes (optional)",
                    value=0,
                    precision=0,
                    minimum=0,
                )
                gr.HTML("<div class='field-help'>Set 0 to validate all episodes.</div>")
            theme_mode = gr.Radio(
                [("Light", "light"), ("Dark", "dark")],
                value="light",
                label="Theme mode",
                scale=1,
            )

        with gr.Accordion("Advanced: checks & policy", open=False):
            gr.Markdown("**Checks to run**")
            with gr.Row():
                select_all_btn = gr.Button("Select all", variant="secondary", size="sm")
                select_recommended_btn = gr.Button("Recommended", variant="secondary", size="sm")
                clear_btn = gr.Button("Clear all", variant="secondary", size="sm")

            with gr.Row():
                check_structure = gr.Checkbox(label="Structure", value=True)
                check_schema = gr.Checkbox(label="Schema", value=True)
                check_consistency = gr.Checkbox(label="Consistency", value=True)
                check_episodes = gr.Checkbox(label="Episodes", value=True)
                check_temporal = gr.Checkbox(label="Temporal", value=True)

            with gr.Row():
                check_numerical = gr.Checkbox(label="Numerical", value=True)
                check_video = gr.Checkbox(label="Video", value=True)
                check_annotation = gr.Checkbox(label="Annotation", value=True)
                check_training = gr.Checkbox(label="Training readiness", value=True)
                check_portability = gr.Checkbox(label="Portability", value=True)

            fail_on = gr.Radio(
                ["error", "warning"],
                value="error",
                label="Fail policy",
            )

        check_outputs = [
            check_structure,
            check_schema,
            check_consistency,
            check_episodes,
            check_temporal,
            check_numerical,
            check_video,
            check_annotation,
            check_training,
            check_portability,
        ]
        select_all_btn.click(fn=lambda: set_check_flags("all"), outputs=check_outputs)
        select_recommended_btn.click(
            fn=lambda: set_check_flags("recommended"),
            outputs=check_outputs,
        )
        clear_btn.click(fn=lambda: set_check_flags("none"), outputs=check_outputs)

        with gr.Row():
            run_button = gr.Button(
                "Run LeGuard diagnostics",
                variant="primary",
                scale=6,
                elem_classes=["run-btn"],
            )
            refresh_button = gr.Button(
                "Refresh",
                variant="secondary",
                scale=1,
                min_width=120,
                elem_classes=["refresh-btn"],
            )

        with gr.Row():
            with gr.Column(scale=4):
                with gr.Tabs():
                    with gr.TabItem("Report"):
                        status_output = gr.HTML()
                        metrics_output = gr.HTML()
                        details_output = gr.Markdown()
                    with gr.TabItem("JSON"):
                        json_view = gr.JSON(label="Validation report (JSON)")
            with gr.Column(scale=2, min_width=260):
                gr.HTML(
                    f"""
                    <div class="issues-card">
                      Issues or recommendations? Open an issue on
                      <a href="{ISSUES_URL}" target="_blank" rel="noopener noreferrer">GitHub</a>.
                    </div>
                    """
                )

        with gr.Accordion("Command logs", open=False):
            logs_output = gr.Textbox(lines=16, show_copy_button=True)

        with gr.Row():
            json_file_output = gr.File(label="Download JSON report")
            html_file_output = gr.File(label="Download HTML report")

        run_inputs = [
            dataset_repo_id,
            fail_on,
            max_episodes,
            check_structure,
            check_schema,
            check_consistency,
            check_episodes,
            check_temporal,
            check_numerical,
            check_video,
            check_annotation,
            check_training,
            check_portability,
        ]
        run_outputs = [
            status_output,
            metrics_output,
            details_output,
            logs_output,
            json_view,
            json_file_output,
            html_file_output,
        ]
        run_button.click(
            fn=run_leguard,
            inputs=run_inputs,
            outputs=run_outputs,
        )
        refresh_button.click(
            fn=reset_diagnostics_state,
            inputs=[],
            outputs=run_outputs,
        )
        theme_mode.change(
            fn=lambda _mode: None,
            inputs=[theme_mode],
            outputs=[],
            js=THEME_CHANGE_JS,
        )


if __name__ == "__main__":
    launch_kwargs = {"show_api": False}
    if os.environ.get("SPACE_ID"):
        # Hugging Face Spaces provides the runtime ingress.
        demo.queue().launch(**launch_kwargs)
    else:
        demo.queue().launch(
            server_name="0.0.0.0",
            server_port=int(os.environ.get("PORT", "7860")),
            **launch_kwargs,
        )
