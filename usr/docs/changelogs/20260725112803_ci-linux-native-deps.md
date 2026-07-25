# CI installs Linux native build dependencies

## Participants

- amkisko

## Decisions

- Install libdbus-1-dev and pkg-config in the test workflow before clippy, build, and test.
- Enable rusqlite bundled so SQLite is compiled from source and does not need system headers.
- Document the D-Bus packages in README for Linux source builds.

## Effects

- Fixes test CI failures caused by libdbus-sys failing pkg-config lookup for dbus-1 on ubuntu-latest.
- Removes the system libsqlite3-dev requirement for local and CI builds.

## Next

- Merge and confirm the test workflow is green on main and open Dependabot PRs.

## Source

- Failed runs: https://github.com/amkisko/timely-cli.rs/actions/runs/30143949563
- Failed runs: https://github.com/amkisko/timely-cli.rs/actions/runs/29641173572
