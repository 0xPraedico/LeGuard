.PHONY: build test lint fmt quickstart hf-download hf-check hf-report hf-validate

HF_DATASET ?= praedico/SO101_pillbox_vita
HF_LOCAL_DIR ?= hf-datasets/SO101_pillbox_vita
HF_FAIL_ON ?= error
HF_REPORT_JSON ?= hf-report.json
HF_REPORT_HTML ?= hf-report.html

build:
	cargo build --workspace

test:
	cargo test --workspace

lint:
	cargo clippy --workspace --all-targets -- -D warnings

fmt:
	cargo fmt --all

quickstart:
	cargo run -p leguard-cli -- check examples/fixtures/dataset-clean --fail-on error
	cargo run -p leguard-cli -- report examples/fixtures/dataset-clean --format json --out clean-report.json
	cargo run -p leguard-cli -- report examples/fixtures/dataset-broken --format json --out broken-report.json
	cargo run -p leguard-cli -- diff clean-report.json broken-report.json --format json

hf-download:
	@command -v hf >/dev/null 2>&1 || (echo "Error: 'hf' CLI is not installed."; exit 1)
	mkdir -p "$(HF_LOCAL_DIR)"
	hf download "$(HF_DATASET)" --repo-type dataset --local-dir "$(HF_LOCAL_DIR)"

hf-check:
	cargo run -p leguard-cli -- check "$(HF_LOCAL_DIR)" --fail-on "$(HF_FAIL_ON)"

hf-report:
	cargo run -p leguard-cli -- report "$(HF_LOCAL_DIR)" --format json --out "$(HF_REPORT_JSON)"
	cargo run -p leguard-cli -- report "$(HF_LOCAL_DIR)" --format html --out "$(HF_REPORT_HTML)"

hf-validate: hf-download hf-check hf-report
