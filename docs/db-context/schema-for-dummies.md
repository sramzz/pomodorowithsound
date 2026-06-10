# Schema For Dummies

A plain-language walk of the 10 tables. The full SQL lives in `src-tauri/migrations/0001_initial_schema.sql`.

---

## The hierarchy (your backlog)

**`projects`** — the top-level container; everything belongs to a project.

**`goals`** — a milestone or objective inside a project; a project has many goals.

**`tasks`** — a concrete piece of work inside a goal; a goal has many tasks.

**`microtasks`** — the unit you actually schedule. A task is broken into microtasks, each with an `estimated_minutes` and a `pomodoro_count`. The planner only works with microtasks — it never touches tasks or goals directly.

**`pomodoro_types`** — reusable presets that define work/rest durations (e.g., "Standard": 20 min work / 5 min rest). A microtask references one pomodoro type; if none is set, the default type applies.

---

## The day plan

**`plans`** — one row per calendar date (`YYYY-MM-DD`). Status is `draft` while you're adjusting it, `committed` when you're ready to run it. Regenerating for the same date replaces the existing draft.

**`work_blocks`** — the individual time slots inside a plan: a block is either a `task` block (linked to a microtask), a `break` block, or a `meeting` block. The planner creates these; you can manually add, remove, and reorder them while the plan is in draft.

---

## The history (kept forever for stats)

**`focus_sessions`** — one row per completed Start Day run. Records how long you worked, how long you rested, how many blocks you completed vs. skipped. Think of it as the summary of a full day's session.

**`pomodoro_sessions`** — one row per individual completed pomodoro. Written immediately when a work block ends (not batched at end of day). Links back to the focus session, the microtask, and the pomodoro type used. This is the grain-level record that powers all statistics.

---

## App configuration

**`settings`** — a simple key/value table. Keys include things like the planning window start/end time, audio volume, and notification toggles. The app reads this on startup and writes it via `update_setting(key, value)`.

---

## Sync-readiness note

All tables use client-generated UUIDs for `id` (not AUTOINCREMENT integers) and carry an `updated_at` column. This is intentional: the schema is designed to support future sync (see the PowerSync ADR in `docs/specs/m1-focus-planner-design.md` §1) without a migration.
