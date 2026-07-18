# timely-cli.rs

[![Test Status][test-badge]][test-workflow]

[test-badge]: https://github.com/amkisko/timely-cli.rs/actions/workflows/test.yml/badge.svg
[test-workflow]: https://github.com/amkisko/timely-cli.rs/actions/workflows/test.yml

Rust CLI and MCP server for the Timely API, with a local Memory reader for
macOS.

## Install

Cargo (from source):

```sh
cargo install --path timely --locked
# or from git
cargo install --git https://github.com/amkisko/timely-cli.rs --package timely-cli --locked
```

[bmx](https://github.com/amkisko/bmx.rs) installs, builds, and runs software
from source repositories.

```sh
bmx install amkisko/timely-cli.rs
```

## QA

```sh
make qa
# or: scripts/qa.sh
```

The QA test enforces first-party file limits: fewer than 300 lines, less than
32 KB, and no line longer than 120 characters.

## Home config (`TIMELY_HOME`)

By default the CLI reads non-secret defaults from
`~/.config/timely/config.env` (XDG), or legacy `~/.timely/config.env` when that
directory already exists and the XDG path does not. Set `TIMELY_HOME` to use
another directory. Optional `config.local.env` in the same directory overrides
`config.env`. Project-level `.timely.env` or `.env` in the current working
directory can also supply allowlisted `TIMELY_*` keys.

Configuration precedence (highest first):

1. Command-line flags
2. Process environment variables
3. `config.local.env` in the Timely config directory
4. `config.env` in the Timely config directory
5. `.timely.env` or `.env` in the current working directory

Copy [config.env.example](config.env.example) as a starting point:

```sh
mkdir -p ~/.config/timely
cp config.env.example ~/.config/timely/config.env
# edit ~/.config/timely/config.env
```

```sh
timely config path
timely config list
timely config get oauth.client_id
timely config set oauth.client_id YOUR_CLIENT_ID
timely config unset oauth.client_id
```

Friendly keys map to env vars:

- `oauth.client_id` → `TIMELY_CLIENT_ID`
- `oauth.redirect_uri` → `TIMELY_REDIRECT_URI`
- `account.id` → `TIMELY_ACCOUNT_ID`
- `api.base_url` → `TIMELY_BASE_URL`
- `http.timeout` → `TIMELY_TIMEOUT`
- `output` → `TIMELY_OUTPUT`
- `profile` → `TIMELY_PROFILE`
- `debug` → `TIMELY_DEBUG`
- `no_color` → `TIMELY_NO_COLOR`
- `memory.db` → `TIMELY_MEMORY_DB`

`timely config set` writes to `config.env` only. Tokens, refresh tokens, and
OAuth client secrets are rejected; store those with `auth` or a password
manager.

## Auth

Prefer a password manager or a token file so the secret stays out of shell
history and process listings:

```sh
timely auth source onepassword --reference op://Vault/Timely/token
timely auth source bitwarden --item timely-api-token
timely auth source keepass --database ~/Secrets.kdbx --item Timely --field Password
timely auth token --token-file ~/.config/timely/token
printf '%s' "$TIMELY_TOKEN" | timely auth token --token-file -
```

Inline `--token` still works when needed:

```sh
timely auth token --token "$TIMELY_TOKEN"
```

Run OAuth authorization-code flow:

```sh
timely auth oauth \
  --client-id "$TIMELY_CLIENT_ID" \
  --client-secret-file ~/.config/timely/client-secret
```

The CLI defaults to Timely's out-of-band redirect URI,
`urn:ietf:wg:oauth:2.0:oob`.
It prints the authorization URL, then prompts you to paste the code that Timely shows after approval.
Pass `--open` if you want it to launch your browser for you.
If you prefer a local callback listener, override `--redirect-uri`
with a loopback HTTP URL such as `http://127.0.0.1:8765/callback`.
OAuth credentials are stored in the platform keyring and refresh automatically.
`account_id` is resolved automatically when possible and is written to `.env`
or `TIMELY_ENV_FILE` when either file is available.

Export a redacted snapshot of local auth state (keyring fingerprints, env key
presence, process cache). Secrets are never printed:

```sh
timely auth export
timely auth export --file ~/timely-auth-export.json
```

## API command templates

Use the curated `api` surface for day-to-day work:

```sh
timely api me
timely api clients list --query limit=20
timely api teams list --query limit=20
timely api teams search --query engineering
timely api teams create --name "Platform" --user-id 1 --lead-user-id 1
timely api projects list --query limit=20
timely api projects create --name "Website Redesign" --rate-type project \
  --team-id 3 --required-label-id 8 --user-rate 12=150
timely api projects archive 42 --yes
timely api users get 123
timely api labels list
timely api tasks get 456
timely api permissions current
timely api reports summary --since 2026-05-01 --until 2026-05-31
timely api reports events --since 2026-05-01 --until 2026-05-31
timely api reports by-team --since 2026-05-01 --until 2026-05-31
```

Time entries:

```sh
timely api time-entries list --day 2026-05-28
timely api time-entries create --project-id 456 --day 2026-05-28 --hours 1.5
timely api time-entries update 789 --note "Code review" --minutes 45
timely api time-entries start 789
timely api time-entries stop 789
timely api time-entries delete 789 --yes
timely api time-entries delete 789 --dry-run
```

On a TTY, delete and archive prompt for confirmation unless `--yes` is passed.
When stdin is not a TTY, pass `--yes` (or `--dry-run` to preview).

Experimental private Memory API mode is also available on a best-effort basis:

```sh
timely api experimental memory accounts
timely api experimental memory identity
timely api experimental memory linked-entries --day 2026-05-28
timely api experimental memory request get /1.1/identity.json
```

These endpoints are undocumented and may break without notice.

Raw requests are still available:

```sh
timely api raw get /1.1/{account_id}/projects --query page=1 --query per_page=100
```

## Memory integration

Inspect the local Memory database:

```sh
timely memory status
timely memory apps --limit 20
timely memory recent --limit 10
timely memory search "README.md" --limit 10 --include-details
timely memory export --limit 1000 --include-details
timely memory export --app Code --since 2026-01-01T00:00:00Z --file ~/memory.json
```

Override the default Memory database path with `--db-path` or
`TIMELY_MEMORY_DB`. Export allows up to 10000 rows (default 1000) and writes
pretty JSON to stdout, or to `--file` (`-` means stdout).

## Output

Interactive terminals default to human-readable tables. Piped or scripted
invocations default to compact JSON. Force a mode with:

```sh
timely api me --json
timely api me -o json
timely api me -o plain
timely api me --plain
```

## Environment

- `TIMELY_HOME` — config directory (default `~/.config/timely`)
- `TIMELY_TOKEN` — bearer token for `auth token`
- `TIMELY_CLIENT_ID` — OAuth client ID
- `TIMELY_CLIENT_SECRET` — OAuth client secret
- `TIMELY_REDIRECT_URI` — OAuth redirect URI
- `TIMELY_REFRESH_TOKEN` — OAuth refresh token (runtime)
- `TIMELY_ACCOUNT_ID` — default account ID
- `TIMELY_PROFILE` — credential profile name
- `TIMELY_BASE_URL` — API base URL
- `TIMELY_OUTPUT` — `auto`, `plain`, or `json`
- `TIMELY_TIMEOUT` — HTTP timeout in seconds
- `TIMELY_DEBUG` — enable debug details
- `TIMELY_NO_COLOR` — disable color for this CLI
- `TIMELY_ENV_FILE` — alternate `.env` path for auth persistence
- `TIMELY_MEMORY_DB` — local Memory database path
- `NO_COLOR` — disable color (standard)
- `FORCE_COLOR` — force color when set
- `PAGER` — pager for long human output
- `COLUMNS` / `LINES` — terminal size hints

## MCP

Run the MCP server over stdio:

```sh
timely mcp serve
```

The server exposes:

- `timely_request` for raw authenticated Timely API calls
- curated `timely_*` tools for clients, teams, projects, users, labels,
  tasks, time entries, timers, permissions, reports, and experimental
  private Memory access
- `memory_*` tools for local Memory queries and `memory_export_entries`
- generated `timely_openapi_<operationId>` tools when a vendored OpenAPI file is present

## Development

- Format: `cargo fmt --all`
- Lint: `make lint` (fmt check, clippy, OpenAPI script syntax)
- Tests: `cargo test --workspace`
- Full gate: `make qa` (or `scripts/qa.sh`)

## Repository layout

- `timely_lib` — Timely API client, auth, home config, Memory DB, OpenAPI helpers
- `timely` — CLI binary (`timely`) and MCP server
- `config.env.example` — template for `TIMELY_HOME/config.env`
- `scripts/` — QA gate and OpenAPI update
- `.github/workflows` — CI (test, Scorecard)

## Contributing

Bug reports and pull requests are welcome on GitHub at
https://github.com/amkisko/timely-cli.rs

See [CONTRIBUTING.md](CONTRIBUTING.md) for policy. Pull requests should include
tests for affected behavior and a changelog note when user-facing.

## Security

If you discover a security vulnerability, report it responsibly. Do not open a
public issue. See [SECURITY.md](SECURITY.md).

## Links

- [GitHub](https://github.com/amkisko/timely-cli.rs)
- [GitLab](https://gitlab.com/amkisko/timely-cli.rs)
- [SonarCloud](https://sonarcloud.io/project/overview?id=amkisko_timely-cli.rs)
- [Snyk](https://snyk.io/test/github/amkisko/timely-cli.rs)
- [Codecov](https://app.codecov.io/github/amkisko/timely-cli.rs)
- [OpenSSF Scorecard](https://scorecard.dev/viewer/?uri=github.com/amkisko/timely-cli.rs)

## License

MIT. See [LICENSE.md](LICENSE.md).
