# LeGuard report

Dataset: `pick_place`

Generated at: `2026-05-09 12:03:01 UTC`

## Issue counts

- Total: 5
- Errors: 2
- Warnings: 3
- Infos: 0

## Issues

- **ERROR** `temporal.non_monotonic_timestamp` (`temporal`): episode_000034 has non-monotonic timestamps at frames 812-813
- **ERROR** `video.missing_file` (`video`): Missing video file for episode_000091 / observation.images.wrist
- **WARNING** `schema.missing_common_column` (`schema`): Column observation.state is missing from one parquet shard
- **WARNING** `numerical.nan` (`numerical`): 42 NaN values found in numeric column action
- **WARNING** `annotation.overlapping_segments` (`annotation`): episode_000033 has overlapping subtasks: grasp and lift
