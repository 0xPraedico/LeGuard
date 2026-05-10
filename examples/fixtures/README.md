# Fixtures

- `dataset-clean`: reference dataset for a nominal run.
- `dataset-broken`: intentionally broken dataset to validate checks.
- `dataset-variant`: variant-style dataset (use `examples/variant-config.yml`).

Example:

```bash
cargo run -p leguard-cli -- check examples/fixtures/dataset-clean
cargo run -p leguard-cli -- check examples/fixtures/dataset-broken --fail-on warning
cargo run -p leguard-cli -- check examples/fixtures/dataset-variant --config examples/variant-config.yml
```
