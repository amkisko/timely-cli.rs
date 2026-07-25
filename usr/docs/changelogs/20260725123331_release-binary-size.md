# Release binary size

## Participants

- amkisko

## Decisions

- Add workspace `[profile.release]` with thin LTO, codegen-units = 1, strip, and panic = abort.
- Leave Memory SQLite and Tokio runtime feature cuts for a later pass.

## Effects

- Release builds strip symbols and apply thin LTO by default.
- Measured arm64 macOS release size: 8.3 MB before, 6.0 MB after (-24%).

## Next

- Optional: feature-gate bundled rusqlite Memory reader.

## Source

- Binary size analysis session (timely, scout, status, pray)
