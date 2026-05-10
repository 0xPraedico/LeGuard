# LeGuard GitHub Action

Composite action to run LeGuard quality checks in CI.

## Inputs

- `dataset-path` (required): path to the dataset root to validate.
- `fail-on` (optional): `error` or `warning` (default: `error`).
- `report-format` (optional): `json`, `html`, `markdown`, `junit` (default: `junit`).
- `report-out` (optional): output path for the report artifact (default: `leguard-report.junit.xml`).

## Example

```yaml
name: Dataset Quality
on:
  pull_request:
  push:
    branches: [main]

jobs:
  quality:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Run LeGuard
        uses: ./packages/github-action
        with:
          dataset-path: examples/fixtures/dataset-clean
          fail-on: error
          report-format: junit
          report-out: leguard-report.junit.xml
```

For external usage (different repository), reference the action with:

```yaml
uses: praedico/LeGuard/packages/github-action@v1
```
