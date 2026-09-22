# Compress embedded OpenAPI

## Participants

- amkisko

## Decisions

- Have `timely_lib` build.rs write `openapi.json.gz` into OUT_DIR with flate2.
- Runtime loads via `include_bytes!` + GzDecoder, cached in OnceLock.
- Keep source spec at `tmp/openapi/openapi.json` for update-openapi.rb and api_methods codegen.

## Effects

- Unit tests for embedded OpenAPI shape and operation set pass.
- Release binary (arm64 macOS) measured at 6.0 MB before and 5.5 MB after (~0.56 MB / ~9%).
- Build emits 44 KB `openapi.json.gz` into OUT_DIR from the 654 KB source spec.

## Next

- Feature-gate Memory/rusqlite for smaller default installs.
- Consider Tokio current_thread for one-shot API commands.

## Source

- Resource profile / binary size follow-up after thin LTO release
