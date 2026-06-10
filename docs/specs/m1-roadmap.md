# M1 Roadmap — Phase Breakdown & Planning Conventions

M1 (the standalone desktop app) is too large to plan and build as one feature. It is broken
into **6 sequential phases**. Each phase ends with working, manually-testable software: the
app launches and the phase's surface is usable end-to-end. All six phase plans are written
**upfront** (decided 2026-06-09) so the team can execute them in sequence and arrive at the
first running version; when an earlier phase teaches us something, the later plans get
amended and the lesson recorded in `docs/lessons/`.

Source documents:
- `PHILOSOPHY.md` — how we work; every spec and plan aligns with it.
- `ArchitecturePlanning.md` — authoritative command vocabulary (Section 4 is the contract).
- `docs/specs/m1-focus-planner-design.md` — schema, planner algorithm, runtime state machine, observability rules (§7).

---

## Philosophy alignment (PHILOSOPHY.md)

This project is explicitly on the **Full path**: TDD, formal specs and plans, layered
modeling, schema-change management via SQLx migrations.

- **Logging is non-negotiable.** Spec §7 defines the rules. Phase 1 builds the
  infrastructure (tracing + exposed `logs/` folder); every later phase has explicit logging
  deliverables, and a phase is not done until its flow reads as a narrative in the log file.
- **TDD with intent.** Tests first, designed deliberately — they encode what the code is
  for. The role split below concentrates test design in the most capable agent.
- **KISS / POLA / AHA.** No router for a 4-view sidebar, no chart library for CSS bars, no
  premature abstractions across the Core services — duplication beats the wrong abstraction.
- **Command-Query Separation.** Commands mutate and return only success/error; queries
  return data and never mutate. Enforced in the IPC layer from Phase 1.
- **Idempotency-ready.** Client-generated entity ids, absolute updates, full-list reorders —
  the M2+ idempotency keys bolt onto these conventions without rework.
- **Lessons folder.** `docs/lessons/` — every mistake or failed experiment during the build
  becomes a markdown entry. We pay for a lesson once.
- **README for newcomers.** Each phase ends by updating `README.md` so any reader grasps the
  current state in five minutes.
- **Database context folder.** `docs/db-context/` — schema for-dummies, structural
  decisions, migration history. Created in Phase 1, kept current by every phase that touches
  the schema.

---

## The 6 Phases

### Phase 1 — Scaffold & Walking Skeleton
**Scope:** Spec §1 (full stack) + §2 (all migrations) + the shell of §6.
- New branch; move the legacy vanilla app (`index.html`, `js/`, `styles/`) to `legacy/` (it collides with the Vite root; its timer/sound behavior is the reference for Phase 4).
- Scaffold Tauri v2 + Vue 3 + Vite + TypeScript + Pinia (`npm create tauri-app@latest`, Vue+TS template; add Pinia and SQLx manually).
- All migrations as migration 0001 (9 tables incl. `settings`) + seed migration for the default "Standard" 20/5 PomodoroType.
- SQLx compile-time-checked-query workflow: committed `.env` with `DATABASE_URL`, dev-DB bootstrap script, `cargo sqlx prepare` with committed `.sqlx/`, `SQLX_OFFLINE=true` in CI.
- IPC convention as code: Rust `AppError` enum (serde-serializable) + `Result<T, AppError>` command signature + the TypeScript `invoke` wrapper every store will use.
- One vertical slice proving Vue → invoke → Rust → SQLx → back: `list_projects` returning the empty DB, rendered as the Backlog empty state.
- Test harnesses: one passing Rust test (in-memory SQLite pool + `sqlx::migrate!`), one passing vitest store test (`mockIPC` from `@tauri-apps/api/mocks`), GitHub Actions CI (macOS runner, cargo + node caches).

**Demonstrable:** app launches as a native window, 4-view sidebar navigates, DB created/migrated with the seeded pomodoro type visible via the slice command, both test suites green in CI.
**Depends on:** nothing. **Size:** small-medium (~2–3 days).
**Key risks:** SQLx offline workflow must be set up deliberately or every fresh clone breaks; don't hand-roll the Tauri config.

### Phase 2 — Backlog Domain (Structure + Pomodoro Types)
**Scope:** all Structure, Ranking, and Pomodoro-type commands (23 commands incl. `archive_*`, `complete_microtask`/`uncomplete_microtask`, the roll-up rule, 3 `reorder_*`). Queries: `list_projects`, `get_project_tree`, `get_microtask`. Backlog tree view (inline create, drag-and-drop reorder, quick estimation auto-computing pomodoro count) + the PomodoroType presets section of Settings. Stores: `useProjectStore`, `usePomodoroTypeStore`.

**Demonstrable:** full backlog management end-to-end — create project → goal → task → microtasks with estimates/types, drag-reorder, complete a microtask and watch the roll-up complete the task, archive/delete.
**Depends on:** Phase 1. **Size:** large (~5–7 days; mechanically repetitive CRUD).
**Key risks:** the roll-up rule must live in `microtask_service` inside one transaction, tested in Rust, not the UI. Reorders rewrite `sort_order` from the full ordered list in one transaction.

### Phase 3 — Day Planning
**Scope:** all Planning commands (`generate_day_plan`, `add_work_block`, `move_work_block`, `remove_work_block`, `reorder_work_blocks`, `commit_day_plan`, `clear_day_plan`). The deterministic planner (gap-filling, pomodoro expansion, break insertion, long-break rule, one-plan-per-date). Queries: `get_day_plan`, `get_today`. Day View timeline in draft mode (generate, drag-reorder, add block from backlog incl. manual meeting blocks, delete, commit) + `usePlanStore`. Runtime controls visible but disabled.

**Demonstrable:** pick a date, generate a draft plan from real backlog microtasks, see N pomodoro blocks + breaks laid out in the planning window, edit it, commit it.
**Depends on:** Phase 2 (planner input is microtasks + types). **Size:** medium-large (~4–5 days).
**Key risks:** keep the planner a pure function `fn plan(inputs, window, now) -> Vec<BlockSpec>` with persistence outside — table-driven Rust unit tests carry the densest coverage in M1.

### Phase 4 — Start Day Runtime (highest-risk phase)
**Scope:** all Day-running commands + focus-mode stubs. The tokio engine, sound & notification rules, incremental `PomodoroSession` writes, `FocusSession` on `end_day`. Query: `get_run_status`. Day View timer header + controls + active-block highlighting; `useRuntimeStore` subscribing to `runtime-tick`.

**Demonstrable:** the product moment — press Start Day on a committed plan, watch the countdown, get the 5-minute notification, hear the completion sound, auto-advance through the break, end day, see session rows in the DB.
**Depends on:** Phase 3 (needs a committed plan). **Size:** medium-large (~4–5 days).
**Key risks:**
- **Actor pattern, not a shared `Mutex`:** one tokio task owns `RuntimeState`; commands arrive via `mpsc`; the task selects over channel + 1s interval. A timer expiry racing a `pause_day` is a genuine hazard a shared mutex invites.
- Deterministic timer tests via `tokio::time::pause()`/`advance()` (`test-util` feature).
- Sounds from Rust via `rodio` (synthesized; webview audio is throttled when minimized and gated by autoplay policies); notifications via `tauri-plugin-notification` from Rust, permission requested on first launch.

### Phase 5 — Analytics + Import/Export
**Scope:** `get_stats` (define the aggregate shape in this phase's plan: per-day totals, completion rate, per-project breakdown) + Analytics view (history list + CSS-bar charts, no chart library). `export_data` / `import_data` — versioned single-file JSON dump; export from one consistent snapshot; import validates the version and runs in one transaction, parents inserted first; file pick via the Tauri dialog plugin.

**Demonstrable:** run a few focus sessions, see stats and history; export to JSON, wipe the dev DB, import, everything restored.
**Depends on:** Phase 4 (stats need session data). **Size:** small-medium (~2–3 days).

### Phase 6 — Settings, Hardening & Packaging
**Scope:** remaining Settings view (planning window default, audio volume, notification toggles — backed by the `settings` KV table; `get_settings`/`update_settings`). Empty/error states, dark-mode polish. App icon, bundle identifier, `tauri build` producing a .dmg. Manual QA checklist run across all phases.

**Demonstrable:** installable .app/.dmg with configurable settings.
**Depends on:** all prior. **Size:** small-medium (~2–3 days).

**Total:** roughly 20–26 focused days.

---

## Cross-cutting testing strategy

- **Rust:** unit tests per Core service with in-memory SQLite (`sqlite::memory:` + `sqlx::migrate!`). Densest coverage: planner (Phase 3) and roll-up (Phase 2). Runtime via tokio time control (Phase 4). CI runs with `SQLX_OFFLINE=true`.
- **Frontend:** vitest for Pinia stores using `mockIPC`; component tests only for the timeline's drag logic.
- **No E2E in M1:** the official `tauri-driver` does not support macOS (the dev platform). Third-party macOS WebDrivers for Tauri exist as of 2026 (open-source `tauri-webdriver` projects, CrabNebula's commercial service) but are young — not worth the setup cost for M1. Each phase plan ends with a manual QA checklist instead; revisit E2E in a later milestone.

---

## Planning conventions (apply to every phase plan)

1. **One plan per phase, all written upfront.** Written with the superpowers `writing-plans` skill, saved to `docs/superpowers/plans/YYYY-MM-DD-m1-phase-N-<name>.md`. Later plans inherit Phase 1's conventions (AppError, IPC wrapper, logging patterns); if execution of a phase invalidates part of a later plan, amend the plan and record the lesson in `docs/lessons/`.
2. **Difficulty tags.** Every task in a plan carries a tag: `[trivial]`, `[easy]`, `[medium]`, or `[hard]`.
3. **Small tasks.** Break work down as far as feasible — each step is one 2–5 minute action.
4. **TDD role split.** The most capable agent always designs the failing test for each task first. Implementation is then assigned by difficulty (cheaper/faster agents may take `[trivial]`/`[easy]` tasks). Every task is reviewed before its commit lands.
5. **Branch strategy.** All M1 work happens on feature branches off `main`, merged via PR. `main` keeps the legacy vanilla app until Phase 1's PR lands (which moves it to `legacy/`).
