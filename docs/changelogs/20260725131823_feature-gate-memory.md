# Feature-gate local Memory

## Participants

amkisko

## Decisions

Keep the `memory` feature enabled by default. Make `rusqlite` and its bundled
SQLite optional, and propagate the CLI feature to `timely_lib`. Keep the
experimental HTTP Memory API available in every build.

## Effects

Builds with `--no-default-features` omit the local Memory command, MCP tools,
and SQLite dependency. Default builds retain the existing local Memory
workflow. Corrected version integration tests to derive the package version
rather than pinning the prior release number.

## Next

No follow-up planned.

## Source

Current repository changes on branch `patch/feature-gate-memory`.
