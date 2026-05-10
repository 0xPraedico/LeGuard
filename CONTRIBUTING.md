# Contributing to LeGuard

Thanks for your interest in contributing to LeGuard.

## Ways to contribute

- Report bugs and regressions.
- Suggest new checks, report formats, or CI integrations.
- Improve documentation and examples.
- Submit code changes with tests.

## Development setup

```bash
git clone https://github.com/praedico/LeGuard.git
cd LeGuard
cargo build --workspace
```

## Local validation checklist

Before opening a pull request, run:

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Optional CLI smoke test:

```bash
cargo run -p leguard-cli -- check examples/fixtures/dataset-clean --fail-on error
```

## Pull request guidelines

- Keep PRs focused and reviewable.
- Add or update tests when behavior changes.
- Update docs when CLI flags, report schema, or workflows change.
- Prefer backward-compatible changes for `schema_version = 1.x`.

## Commit messages

Use clear, imperative commit messages that explain intent (the "why"), not only the change (the "what").

## Community standards

By participating, you agree to follow the project Code of Conduct in `CODE_OF_CONDUCT.md`.
