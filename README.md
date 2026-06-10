# Focus Planner

Focus Planner is a native desktop application for Pomodoro-based day planning. Built with Tauri v2 (Rust backend) + Vue 3 + TypeScript + SQLite, it lets you manage a backlog of Projects → Goals → Tasks → Microtasks, generate a deterministic daily schedule, and run a Pomodoro timer loop — all offline, with sound and system notifications driven from Rust so they fire reliably even when the window is minimized.

## Current State

**Phase 2 backlog management is live.** You can manage projects → goals → tasks → microtasks with completion roll-up, drag reordering, and pomodoro type presets. Day planning arrives in Phase 3. The legacy vanilla-JS prototype lives in `legacy/` for reference.

Active development is tracked in `docs/superpowers/plans/`.

## Setup & Dev

Install the SQLx CLI once:

```sh
cargo install sqlx-cli --no-default-features --features sqlite
```

Then on each fresh clone:

```sh
./scripts/setup-db.sh   # creates the SQLite dev DB and runs migrations
npm install
npm run tauri dev
```

## Testing

```sh
# Frontend (Vitest)
npm test

# Rust integration tests
cargo test --manifest-path src-tauri/Cargo.toml

# TypeScript typecheck
npx vue-tsc --noEmit
```

CI runs all three automatically on every push/PR (see `.github/workflows/ci.yml`).

## Logs

Structured logs are written to `logs/focus-planner.log.<date>` (daily rolling). The `logs/` directory is gitignored; a `.gitkeep` keeps the folder tracked. Set `RUST_LOG` to control verbosity; both console and file layers share one level (default: `debug` in dev, `info` in release).

## Key Docs

| Path | What it is |
|------|-----------|
| `PHILOSOPHY.md` | Project development standards and operational principles |
| `docs/specs/m1-focus-planner-design.md` | Full M1 architecture + schema + API spec |
| `docs/specs/m1-roadmap.md` | Six-phase M1 roadmap |
| `docs/superpowers/plans/` | Per-phase implementation plans |
| `docs/db-context/` | Plain-language DB schema walk + migration history |
| `docs/lessons/` | Hard-won lessons — one file per category |

## Recommended IDE

VS Code + [Vue - Official](https://marketplace.visualstudio.com/items?itemName=Vue.volar) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)
