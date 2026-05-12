---
title: LeGuard
short_description: Run diagnostics on LeRobot datasets from Hugging Face.
colorFrom: yellow
colorTo: yellow
sdk: docker
app_port: 7860
pinned: false
license: mit
---

# LeGuard Space

LeGuard is a QA and CI toolkit for LeRobot datasets.

This Space can:

- run diagnostics on a dataset from the Hugging Face Hub (HF repo id only),
- run `leguard check`,
- generate JSON and HTML reports.

## Notes

- Space storage is ephemeral unless a persistent storage bucket is attached.
- For private datasets, set an `HF_TOKEN` secret in the Space settings.
- Local filesystem paths are intentionally disabled in the UI for security and reproducibility.
- Space publishing is manual (no GitHub Actions sync workflow).
