# timely-cli project notes

Terminal UI for Timely. Shared agent guidance is managed through `Prayfile` and `AGENTS.md`.

## Terminal UI changes must check

- keyboard and mouse usability
- discoverability via tooltips, menus, docs
- light and dark mode
- hover, focus, and active states
- keyboard-only navigation
- narrow and short pane behavior
- Retina and high-DPI behavior
- Windows, Linux, and macOS consistency
- instant feedback and performance; frame budget target is 8ms / 120fps
- clear text, no insider jargon
- happy path, error path, offline and online, authenticated and unauthenticated states
- actionable error messages

## Collaboration workflow

- keep backlog, priority, status, and ownership in discussion; do not recreate workflow columns or status folders in the repo
- for design work, keep Figma as source of truth and store intent, summary, decisions, constraints, and relevant links in the repo
