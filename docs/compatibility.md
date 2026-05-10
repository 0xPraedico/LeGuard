# Compatibility Policy

This policy defines what can change across LeGuard versions.

## General rules

- `schema_version` defines the JSON contract for `leguard-report.json`.
- New LeGuard patch/minor versions must remain compatible with existing schema versions.
- Any schema compatibility break requires a new major schema version.

## Guarantees for `schema_version = 1.x`

- Required fields documented in `docs/report-schema.md` remain available.
- Existing `severity` and `category` enum values remain valid.
- Core CLI commands (`check`, `report`, `diff`) keep their primary flags.

## Changes allowed without breaking compatibility

- Adding new optional fields.
- Adding new `check_id` values.
- Updating textual messages (`title`, `message`, `suggestion`) without structural impact.
- Refining check heuristics and issue detection volume.

## Breaking changes

- Removing or renaming a required field.
- Changing the type of a required field.
- Changing the backward-compatible meaning of an existing enum value.

## Migration strategy

- Publish the updated schema specification in `docs/report-schema.md`.
- Provide migration notes with before/after examples.
- Keep a schema coexistence window for at least one major release.
