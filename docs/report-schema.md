# `leguard-report.json` Schema (v1.0.0)

The schema contract is versioned through the top-level `schema_version` field.

- Current schema version: `1.0.0`
- Recommended filename: `leguard-report.json`

## Top-level fields

Required:

- `schema_version` (string)
- `leguard_version` (string)
- `dataset` (object)
- `run` (object)
- `issue_counts` (object)
- `issues` (array)
- `generated_at` (RFC3339 datetime)

## `dataset`

Required:

- `name` (string)
- `format` (string)
- `uri` (string)
- `source_type` (string)
- `episodes` (integer)
- `frames` (integer)
- `duration_sec` (number)
- `cameras` (array<string>)
- `parquet_files` (integer)
- `video_files` (integer)
- `metadata_files` (integer)
- `empty_files` (integer)
- `total_files` (integer)

## `run`

Required:

- `source` (string)
- `started_at` (RFC3339 datetime)
- `finished_at` (RFC3339 datetime)

Optional:

- `git_commit` (string | null)
- `git_branch` (string | null)
- `git_repo` (string | null)
- `dataset_revision` (string | null)

## `issue_counts`

Required:

- `total` (integer)
- `errors` (integer)
- `warnings` (integer)
- `infos` (integer)

## `issues[]`

Required:

- `fingerprint` (string)
- `severity` (`error` | `warning` | `info`)
- `category` (`structure` | `schema` | `temporal` | `video` | `numerical` | `annotation` | `diff` | `compatibility` | `performance` | `unknown`)
- `check_id` (string)
- `title` (string)
- `message` (string)
- `metadata` (object, can be empty)

Optional:

- `suggestion` (string | null)
- `file_path` (string | null)
- `episode_index` (integer | null)
- `frame_index` (integer | null)
- `timestamp_sec` (number | null)
- `feature` (string | null)
- `expected` (json | null)
- `actual` (json | null)

## Examples

- Complete sample report: `examples/leguard-report.sample.json`
- Golden HTML/JUnit/Markdown outputs: `crates/leguard-report/tests/golden/`
