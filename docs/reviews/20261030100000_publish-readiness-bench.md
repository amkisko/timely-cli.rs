# Publish readiness bench

## Decisions

- Keep release profile thin LTO, single codegen unit, strip, and panic abort.
- Compare default Memory-enabled release against `--no-default-features`.
- Time cold CLI startup with Python `time.perf_counter` (10 runs); `hyperfine`
  was not installed. Coarse `/usr/bin/time -p` rounds sub-10ms starts to 0.00s.
- Leave work uncommitted for the parent session to coordinate.

## Effects

- `make qa` passed: fmt check, clippy `-D warnings`, OpenAPI script syntax,
  workspace tests including QA size/width gate.
- Release binary sizes (Mach-O arm64):
  - default (`memory`): 5854992 bytes (5.6 MiB, `ls -lh` 5.6M)
  - `--no-default-features`: 4023552 bytes (3.8 MiB, `ls -lh` 3.8M)
  - delta: about 1.8 MiB smaller without Memory/SQLite
- Startup (10 runs, mean / median / min / max):
  - default `--help`: 8.474 / 6.796 / 5.529 / 19.329 ms
  - default `version`: 6.669 / 6.677 / 4.930 / 8.201 ms
  - no-memory `--help`: 6.912 / 5.647 / 5.120 / 18.907 ms
  - no-memory `version`: 6.322 / 6.276 / 5.377 / 7.394 ms
- Host: macOS 15.8.1 arm64, rustc 1.99.1. Measured 2026-10-30T08:20Z.
- Audit: auth export stays fingerprinted; destructive delete/archive confirm
  still wired on curated `api` paths. Fixed medium: opt-in plain credential
  files now use Unix owner-only modes (`0600` / dir `0700`). Remaining notes
  are low-only (MCP/`call`/`request` skip interactive confirm by design;
  README documents CLI confirm).

## Next

- Parent may commit, tag, and push when ready.
- Optional: install `hyperfine` for repeatable CI-friendly startup benches.
- Hosted GitHub Actions matrix was not re-run from this session.

## Source

- Makefile (`make qa`)
- Cargo.toml release profile
- timely/Cargo.toml features
- docs/changelogs/20260725123331_release-binary-size.md
- docs/changelogs/20260725131823_feature-gate-memory.md
