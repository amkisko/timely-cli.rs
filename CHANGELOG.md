# CHANGELOG

## 0.1.0 (2026-07-18)

- Add Timely API CLI with curated `api` commands for clients, teams, projects,
  users, labels, tasks, time entries, timers, permissions, and reports.
- Add `auth` for password-manager sources, token files, OAuth, and redacted
  `auth export`.
- Add `TIMELY_HOME` config (`config.env` / `config.local.env`) and
  `timely config` for non-secret defaults such as OAuth client id, account id,
  base URL, timeout, output, and profile.
- Add local Memory reader commands (`memory status`, `apps`, `recent`, `search`,
  `export`) on macOS.
- Add MCP server over stdio (`mcp serve`) with Timely and Memory tools.
- Add `--json` / `--plain` output modes and environment overrides.
- Add shell completions and man page generation.
