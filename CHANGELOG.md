# Changelog

All notable changes to this project are documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project aims to follow [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- Hugging Face direct dataset validation commands in `README.md` and `Makefile`.
- Community documentation: `CONTRIBUTING.md`, `CODE_OF_CONDUCT.md`, and `SECURITY.md`.
- Multi-OS CI test coverage (Linux, macOS, Windows).
- Supply-chain hardening via Dependabot, dependency audit job, and release checksums.
- README branding banner and extended project badges.

### Changed

- Removed scoring fields and score-based gating from reports and CLI behavior.
- Replaced repository placeholders with `praedico/LeGuard`.
- Expanded `.gitignore` for local Hugging Face downloads and generated report artifacts.

## [0.1.0] - 2026-05-10

### Added

- Initial open-source release of LeGuard (`leguard-core`, `leguard-cli`, `leguard-report`, and GitHub Action).
