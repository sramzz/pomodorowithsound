# M1 Phase 3 — Day Planning Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** All Planning commands from spec §3 (`generate_day_plan`, `add_work_block`, `move_work_block`, `remove_work_block`, `reorder_work_blocks`, `commit_day_plan`, `clear_day_plan`), the deterministic planner of spec §4 as a pure function with persistence outside it, the `get_day_plan` / `get_today` queries (spec §5), and the Day View timeline in draft mode (spec §6) with `usePlanStore`. Runtime controls are visible but disabled (Phase 4).

**Architecture:** The planner is a pure function `fn plan(input, window, now) -> Vec<BlockSpec>` in `src-tauri/src/core/planner.rs` operating on plain structs — no DB, no clock, no I/O — so its table-driven unit tests need nothing but values. A thin `core/plan_service.rs` gathers inputs from SQLite (eligible microtasks with resolved pomodoro types, manually-added meetings, the planning window from `settings`), calls the planner, and persists its output inside one transaction. IPC command handlers in `commands/plan.rs` stay thin and instrumented, exactly like Phase 1's `list_projects` pattern. The frontend talks only through the `ipc<T>()` wrapper; `usePlanStore` re-queries `get_day_plan` after every mutation (CQS).

**Tech Stack:** Everything from Phase 1 (Tauri 2, Vue 3, Vite, TypeScript, Pinia, Vitest, Rust, SQLx 0.8 sqlite, tokio, thiserror, tracing) plus: `chrono` (time math; Phase 2 may already have added it — the dependency step is idempotent), `uuid` (Rust-side ids for planner-generated blocks), `vuedraggable@next` (drag-and-drop, the mechanism Phase 2 chose for the backlog tree — also add-if-missing), `@vue/test-utils` (the one component test the roadmap allows: the timeline's drag logic).

**Conventions (per `docs/specs/m1-roadmap.md`):** every task carries a difficulty tag (`[trivial]` `[easy]` `[medium]` `[hard]`). The failing test of each TDD task is designed by the most capable agent; implementation may be assigned by difficulty (cheaper agents take `[trivial]`/`[easy]`); every task is reviewed before its commit lands. Type names follow Phase 1: `AppError` (variants `Db`/`NotFound`/`Validation`), `Db(SqlitePool)` managed state, `ipc<T>()` TS wrapper, serde `camelCase` models, `sqlx::query!`/`query_as!` macros with `cargo sqlx prepare` refreshing the committed `.sqlx/` cache, `#[sqlx::test]` for service tests, `mockIPC` for store tests.

**Philosophy (PHILOSOPHY.md):** CQS — all seven mutations return `Result<(), AppError>`; the two queries return data and never mutate; the UI re-queries after mutating. Logging is non-negotiable (spec §7): every command INFO entry/exit via `#[tracing::instrument]`; the planner logs a DEBUG input summary (N microtasks, window, M meetings) and **every placement decision** ("placed work block 09:00–09:20 for microtask X in gap ending …", "pair didn't fit gap ending 12:00, moved to next gap"); commit and clear log INFO. The phase is not done until a junior can follow a generate-and-commit run in `logs/` alone (QA Task 16).

**Phase decisions (resolving spec ambiguities — reviewers check these first):**
1. **UTC end-to-end.** Spec §2 stores timestamps as ISO 8601 UTC and plan dates as `YYYY-MM-DD`. M1 treats all wall-clock times (planning window, block times, time inputs) as UTC, and the UI formats `HH:MM` by slicing the ISO string — what you plan at 09:00 displays as 09:00 (POLA). Local-timezone mapping is deferred to M2+; if that ever bites, record a lesson in `docs/lessons/`.
2. **Meetings survive regeneration.** `generate_day_plan` reads the existing plan's `block_type = 'meeting'` rows as planner input before the delete+recreate transaction, and the planner re-emits them at their fixed times. Workflow: generate (possibly empty) → add meetings → regenerate around them.
3. **Committed plans are sealed.** Committing or regenerating a committed plan is a `Validation` error (per scope). Add/move/remove/reorder/clear on a committed plan are also `Validation` errors — `commit_day_plan` "seals the plan" (spec §4) and Phase 4's runtime depends on committed plans not shifting underneath it.
4. **`clear_day_plan` deletes the plan row and its blocks** (explicit deletes, one transaction). The `UNIQUE(date)` slot frees up for a fresh generate.
5. **Long-break run counting:** the planner tracks consecutive work blocks of the same pomodoro type across the whole day in placement order; a work block of a *different type* resets the run; meetings and gap boundaries do not. When the run count is a multiple of `long_break_every`, that block's break uses `long_break_minutes`.
6. **Unplaceable pairs:** if a work+break pair fits no remaining gap, that pomodoro **and the microtask's later pomodoros** are left unplanned (you can't do pomodoro 3 before 2) — logged at DEBUG; the planner moves on to the next microtask.
7. **`strategy`** accepts only `"sequential"` (or absent, which means the same). Anything else is a `Validation` error — the parameter exists so M2+ strategies don't change the IPC contract.
8. **No UI for `move_work_block` in this phase.** The command, service, store action, and tests all exist (scope requires the command); the Day View edits times via regenerate/delete/re-add. Surfacing a time editor is a later UI nicety.
9. **NO schema change.** Spec §2's migration 0001 already contains `plans`, `work_blocks`, and `settings` — Phase 3 adds zero migrations. **If during execution you believe a schema change is needed, STOP: that contradicts the spec — amend this plan and record a lesson in `docs/lessons/` first.**

---

### Task 1: Branch + dependencies (idempotent) `[trivial]`

**Files:**
- Modify: `src-tauri/Cargo.toml`, `package.json` (only if the deps are missing)

- [ ] **Step 1: Create the phase branch**

```bash
git checkout main && git pull
git checkout -b feat/m1-phase-3-day-planning
```

- [ ] **Step 2: Add Rust deps only if not already present (Phase 2 may have added chrono)**

```bash
grep -q '^chrono' src-tauri/Cargo.toml || cargo add --manifest-path src-tauri/Cargo.toml chrono --features serde
grep -q '^uuid' src-tauri/Cargo.toml || cargo add --manifest-path src-tauri/Cargo.toml uuid --features v4
```

- [ ] **Step 3: Add frontend deps only if not already present (Phase 2 chose vuedraggable@next for the backlog tree)**

```bash
node -e "process.exit(require('./package.json').dependencies?.vuedraggable ? 0 : 1)" || npm install vuedraggable@next
node -e "process.exit(require('./package.json').devDependencies?.['@vue/test-utils'] ? 0 : 1)" || npm install -D @vue/test-utils
```

- [ ] **Step 4: Verify everything still builds**

Run: `cargo check --manifest-path src-tauri/Cargo.toml && npx vue-tsc --noEmit`
Expected: both succeed.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/Cargo.lock package.json package-lock.json
git commit -m "chore: add chrono, uuid, vuedraggable, vue-test-utils for day planning"
```

---

### Task 2: Planner domain types (plain structs, no DB) `[easy]`

**Files:**
- Create: `src-tauri/src/core/planner.rs`
- Modify: `src-tauri/src/core/mod.rs`

Pure data only — the `plan()` function itself arrives TDD-style in Task 3. These structs are the planner's whole world; resolving DB rows into them is `plan_service`'s job (Task 5), except the type-fallback chain (`microtask's type → default type → 20/5 fallback`), which lives *inside* the planner so the fallback rule is unit-testable without a DB.

- [ ] **Step 1: Write `src-tauri/src/core/planner.rs`**

```rust
use chrono::{DateTime, NaiveDate, NaiveTime, Utc};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlockKind {
    Task,
    Break,
    Meeting,
}

impl BlockKind {
    /// The `work_blocks.block_type` column value (spec §2).
    pub fn as_str(&self) -> &'static str {
        match self {
            BlockKind::Task => "task",
            BlockKind::Break => "break",
            BlockKind::Meeting => "meeting",
        }
    }
}

/// The planner's output unit. Persistence (ids, sort_order, timestamps)
/// happens outside the planner, in plan_service.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockSpec {
    pub kind: BlockKind,
    pub microtask_id: Option<String>,
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    /// 1-based pomodoro index within the microtask; None for breaks/meetings.
    pub pomodoro_index: Option<i64>,
}

/// Work/break parameters resolved from a pomodoro_types row.
/// `type_key` identifies the type for the long-break consecutive-run rule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PomodoroParams {
    pub type_key: String,
    pub work_minutes: i64,
    pub rest_minutes: i64,
    pub long_break_minutes: Option<i64>,
    pub long_break_every: Option<i64>,
}

/// Spec §2 seed note: if no default type is configured, fall back to 20
/// minutes of work (we mirror the Standard seed's 5-minute rest).
pub fn fallback_params() -> PomodoroParams {
    PomodoroParams {
        type_key: "fallback".into(),
        work_minutes: 20,
        rest_minutes: 5,
        long_break_minutes: None,
        long_break_every: None,
    }
}

#[derive(Debug, Clone)]
pub struct PlannerMicrotask {
    pub id: String,
    pub title: String,
    pub pomodoro_count: i64,
    /// The microtask's own type. None falls back to
    /// `PlannerInput::default_pomodoro`, then to `fallback_params()`.
    pub pomodoro: Option<PomodoroParams>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Meeting {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
}

/// Planning window, default 09:00–17:00, read from the settings table by
/// settings_service (Task 4).
#[derive(Debug, Clone, Copy)]
pub struct PlanningWindow {
    pub start: NaiveTime,
    pub end: NaiveTime,
}

#[derive(Debug, Clone)]
pub struct PlannerInput {
    pub date: NaiveDate,
    /// Pre-sorted by the caller: priority DESC, deadline ASC (NULLs last),
    /// created_at ASC. The planner preserves this order (spec §4: inputs
    /// arrive "sorted by priority and deadline").
    pub microtasks: Vec<PlannerMicrotask>,
    pub meetings: Vec<Meeting>,
    pub default_pomodoro: Option<PomodoroParams>,
}
```

- [ ] **Step 2: Register the module in `src-tauri/src/core/mod.rs`**

Append:

```rust
pub mod planner;
```

- [ ] **Step 3: Verify it compiles**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`
Expected: success.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/core
git commit -m "feat: planner domain types — pure structs, persistence-free"
```

---

### Task 3: The deterministic planner — pure function `[hard]`

**Files:**
- Modify: `src-tauri/src/core/planner.rs`
- Test: `src-tauri/tests/planner.rs`

The heart of the phase and the densest test coverage in M1 (roadmap). Table-driven scenarios, full expected outputs, zero DB.

- [ ] **Step 1: Write the failing table-driven tests** *(test designed by the strongest agent)*

`src-tauri/tests/planner.rs`:

```rust
use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use focus_planner_lib::core::planner::{
    plan, BlockKind, BlockSpec, Meeting, PlannerInput, PlannerMicrotask, PlanningWindow,
    PomodoroParams,
};

fn date() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 6, 15).unwrap()
}

fn at(hhmm: &str) -> DateTime<Utc> {
    date()
        .and_time(NaiveTime::parse_from_str(hhmm, "%H:%M").unwrap())
        .and_utc()
}

fn window(start: &str, end: &str) -> PlanningWindow {
    PlanningWindow {
        start: NaiveTime::parse_from_str(start, "%H:%M").unwrap(),
        end: NaiveTime::parse_from_str(end, "%H:%M").unwrap(),
    }
}

fn standard() -> PomodoroParams {
    PomodoroParams {
        type_key: "standard".into(),
        work_minutes: 20,
        rest_minutes: 5,
        long_break_minutes: None,
        long_break_every: None,
    }
}

fn deep(every: i64, long: i64) -> PomodoroParams {
    PomodoroParams {
        type_key: "deep".into(),
        work_minutes: 20,
        rest_minutes: 5,
        long_break_minutes: Some(long),
        long_break_every: Some(every),
    }
}

fn mt(id: &str, pomodoros: i64, params: Option<PomodoroParams>) -> PlannerMicrotask {
    PlannerMicrotask {
        id: id.into(),
        title: id.into(),
        pomodoro_count: pomodoros,
        pomodoro: params,
    }
}

fn meeting(start: &str, end: &str) -> Meeting {
    Meeting { start: at(start), end: at(end) }
}

fn input(
    microtasks: Vec<PlannerMicrotask>,
    meetings: Vec<Meeting>,
    default_pomodoro: Option<PomodoroParams>,
) -> PlannerInput {
    PlannerInput { date: date(), microtasks, meetings, default_pomodoro }
}

fn work(mt_id: &str, start: &str, end: &str, idx: i64) -> BlockSpec {
    BlockSpec {
        kind: BlockKind::Task,
        microtask_id: Some(mt_id.into()),
        start: at(start),
        end: at(end),
        pomodoro_index: Some(idx),
    }
}

fn brk(start: &str, end: &str) -> BlockSpec {
    BlockSpec {
        kind: BlockKind::Break,
        microtask_id: None,
        start: at(start),
        end: at(end),
        pomodoro_index: None,
    }
}

fn meet(start: &str, end: &str) -> BlockSpec {
    BlockSpec {
        kind: BlockKind::Meeting,
        microtask_id: None,
        start: at(start),
        end: at(end),
        pomodoro_index: None,
    }
}

struct Case {
    name: &'static str,
    input: PlannerInput,
    window: PlanningWindow,
    now: DateTime<Utc>,
    expected: Vec<BlockSpec>,
}

#[test]
fn planner_table_driven_scenarios() {
    let w = window("09:00", "17:00");
    let early = at("00:00"); // `now` long before the window: no clamping

    let cases = vec![
        Case {
            name: "empty backlog produces no blocks",
            input: input(vec![], vec![], Some(standard())),
            window: w,
            now: early,
            expected: vec![],
        },
        Case {
            name: "single microtask expands to N work+break pairs",
            input: input(vec![mt("mt1", 3, Some(standard()))], vec![], None),
            window: w,
            now: early,
            expected: vec![
                work("mt1", "09:00", "09:20", 1),
                brk("09:20", "09:25"),
                work("mt1", "09:25", "09:45", 2),
                brk("09:45", "09:50"),
                work("mt1", "09:50", "10:10", 3),
                brk("10:10", "10:15"),
            ],
        },
        Case {
            name: "meeting splits the morning gap; pairs fill around it",
            input: input(
                vec![mt("mt1", 2, Some(standard()))],
                vec![meeting("09:25", "10:00")],
                None,
            ),
            window: w,
            now: early,
            expected: vec![
                work("mt1", "09:00", "09:20", 1),
                brk("09:20", "09:25"),
                meet("09:25", "10:00"),
                work("mt1", "10:00", "10:20", 2),
                brk("10:20", "10:25"),
            ],
        },
        Case {
            name: "pair that cannot fit before a meeting moves to the next gap",
            input: input(
                vec![mt("mt1", 1, Some(standard()))],
                vec![meeting("09:20", "10:00")],
                None,
            ),
            window: w,
            now: early,
            expected: vec![
                meet("09:20", "10:00"),
                work("mt1", "10:00", "10:20", 1),
                brk("10:20", "10:25"),
            ],
        },
        Case {
            name: "long-break rule: every Nth consecutive block of a type",
            input: input(vec![mt("mt1", 4, Some(deep(4, 15)))], vec![], None),
            window: w,
            now: early,
            expected: vec![
                work("mt1", "09:00", "09:20", 1),
                brk("09:20", "09:25"),
                work("mt1", "09:25", "09:45", 2),
                brk("09:45", "09:50"),
                work("mt1", "09:50", "10:10", 3),
                brk("10:10", "10:15"),
                work("mt1", "10:15", "10:35", 4),
                brk("10:35", "10:50"), // 15-minute long break
            ],
        },
        Case {
            name: "consecutive run spans microtasks of the same type",
            input: input(
                vec![mt("a", 2, Some(deep(4, 15))), mt("b", 2, Some(deep(4, 15)))],
                vec![],
                None,
            ),
            window: w,
            now: early,
            expected: vec![
                work("a", "09:00", "09:20", 1),
                brk("09:20", "09:25"),
                work("a", "09:25", "09:45", 2),
                brk("09:45", "09:50"),
                work("b", "09:50", "10:10", 1),
                brk("10:10", "10:15"),
                work("b", "10:15", "10:35", 2),
                brk("10:35", "10:50"), // 4th consecutive 'deep' block → long break
            ],
        },
        Case {
            name: "no type and no default type falls back to 20/5",
            input: input(vec![mt("mt1", 1, None)], vec![], None),
            window: w,
            now: early,
            expected: vec![work("mt1", "09:00", "09:20", 1), brk("09:20", "09:25")],
        },
        Case {
            name: "no type uses the default type when one is configured",
            input: input(
                vec![mt("mt1", 1, None)],
                vec![],
                Some(PomodoroParams {
                    type_key: "default30".into(),
                    work_minutes: 30,
                    rest_minutes: 10,
                    long_break_minutes: None,
                    long_break_every: None,
                }),
            ),
            window: w,
            now: early,
            expected: vec![work("mt1", "09:00", "09:30", 1), brk("09:30", "09:40")],
        },
        Case {
            name: "now inside the window: nothing scheduled in the past",
            input: input(vec![mt("mt1", 1, Some(standard()))], vec![], None),
            window: w,
            now: at("13:00"),
            expected: vec![work("mt1", "13:00", "13:20", 1), brk("13:20", "13:25")],
        },
        Case {
            name: "pair that fits nowhere is left unplanned",
            input: input(vec![mt("mt1", 2, Some(standard()))], vec![], None),
            window: window("09:00", "09:10"),
            now: early,
            expected: vec![],
        },
        Case {
            name: "microtasks fill sequentially in the given (pre-sorted) order",
            input: input(
                vec![mt("a", 1, Some(standard())), mt("b", 1, Some(standard()))],
                vec![],
                None,
            ),
            window: w,
            now: early,
            expected: vec![
                work("a", "09:00", "09:20", 1),
                brk("09:20", "09:25"),
                work("b", "09:25", "09:45", 1),
                brk("09:45", "09:50"),
            ],
        },
    ];

    for case in cases {
        let got = plan(&case.input, &case.window, case.now);
        assert_eq!(got, case.expected, "case '{}'", case.name);
    }
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test planner`
Expected: FAIL — compile error `unresolved import focus_planner_lib::core::planner::plan` (the function doesn't exist yet).

- [ ] **Step 3: Implement `plan()` — append to `src-tauri/src/core/planner.rs`**

Algorithm (spec §4): meetings first at fixed times → gaps = window minus meetings, never before `now` → fill gaps sequentially with work+break pairs in microtask order, expanding N pomodoros into N pairs, long-break rule on consecutive same-type runs, pairs that don't fit a gap move to the next gap. Every placement decision logs at DEBUG (spec §7).

```rust
use chrono::Duration;

pub fn plan(input: &PlannerInput, window: &PlanningWindow, now: DateTime<Utc>) -> Vec<BlockSpec> {
    let day_start = input.date.and_time(window.start).and_utc();
    let day_end = input.date.and_time(window.end).and_utc();
    tracing::debug!(
        date = %input.date,
        microtasks = input.microtasks.len(),
        meetings = input.meetings.len(),
        window_start = %window.start.format("%H:%M"),
        window_end = %window.end.format("%H:%M"),
        "planner: input summary"
    );

    // 1. Meetings first, at their fixed times.
    let mut meetings = input.meetings.clone();
    meetings.sort_by_key(|m| (m.start, m.end));
    let mut blocks: Vec<BlockSpec> = meetings
        .iter()
        .map(|m| {
            tracing::debug!(
                "planner: placed meeting at its fixed time {}–{}",
                m.start.format("%H:%M"),
                m.end.format("%H:%M")
            );
            BlockSpec {
                kind: BlockKind::Meeting,
                microtask_id: None,
                start: m.start,
                end: m.end,
                pomodoro_index: None,
            }
        })
        .collect();

    // 2. Gaps = the window minus meetings, never before `now`.
    let fill_start = day_start.max(now);
    let mut gaps: Vec<(DateTime<Utc>, DateTime<Utc>)> = Vec::new();
    let mut cursor = fill_start;
    for m in &meetings {
        if m.start > cursor {
            let gap_end = m.start.min(day_end);
            if gap_end > cursor {
                gaps.push((cursor, gap_end));
            }
        }
        if m.end > cursor {
            cursor = m.end;
        }
    }
    if day_end > cursor {
        gaps.push((cursor, day_end));
    }

    // 3. Fill gaps sequentially with work+break pairs.
    let mut gi = 0usize; // current gap index
    let mut pos = gaps.first().map(|g| g.0); // free position inside gaps[gi]
    let mut last_type_key: Option<String> = None;
    let mut run = 0i64; // consecutive work blocks of last_type_key

    for microtask in &input.microtasks {
        let params = microtask
            .pomodoro
            .clone()
            .or_else(|| input.default_pomodoro.clone())
            .unwrap_or_else(|| {
                tracing::debug!(
                    microtask_id = %microtask.id,
                    "planner: no type on microtask and no default type — using the 20-minute fallback"
                );
                fallback_params()
            });

        for pomodoro_index in 1..=microtask.pomodoro_count {
            let next_run = if last_type_key.as_deref() == Some(params.type_key.as_str()) {
                run + 1
            } else {
                1
            };
            // Long-break rule: every Nth consecutive work block of this type.
            let break_minutes = match (params.long_break_every, params.long_break_minutes) {
                (Some(every), Some(long)) if every > 0 && next_run % every == 0 => long,
                _ => params.rest_minutes,
            };
            let pair = Duration::minutes(params.work_minutes + break_minutes);

            let mut placed = false;
            let mut i = gi;
            while i < gaps.len() {
                let start = if i == gi { pos.expect("pos is Some while gaps exist") } else { gaps[i].0 };
                if start + pair <= gaps[i].1 {
                    let work_end = start + Duration::minutes(params.work_minutes);
                    let break_end = work_end + Duration::minutes(break_minutes);
                    tracing::debug!(
                        microtask_id = %microtask.id,
                        pomodoro = pomodoro_index,
                        "planner: placed work block {}–{} for microtask {} in gap ending {} (break until {})",
                        start.format("%H:%M"),
                        work_end.format("%H:%M"),
                        microtask.title,
                        gaps[i].1.format("%H:%M"),
                        break_end.format("%H:%M")
                    );
                    blocks.push(BlockSpec {
                        kind: BlockKind::Task,
                        microtask_id: Some(microtask.id.clone()),
                        start,
                        end: work_end,
                        pomodoro_index: Some(pomodoro_index),
                    });
                    blocks.push(BlockSpec {
                        kind: BlockKind::Break,
                        microtask_id: None,
                        start: work_end,
                        end: break_end,
                        pomodoro_index: None,
                    });
                    gi = i;
                    pos = Some(break_end);
                    last_type_key = Some(params.type_key.clone());
                    run = next_run;
                    placed = true;
                    break;
                }
                tracing::debug!(
                    microtask_id = %microtask.id,
                    pomodoro = pomodoro_index,
                    "planner: pair didn't fit gap ending {}, moved to next gap",
                    gaps[i].1.format("%H:%M")
                );
                i += 1;
            }
            if !placed {
                tracing::debug!(
                    microtask_id = %microtask.id,
                    pomodoro = pomodoro_index,
                    total = microtask.pomodoro_count,
                    "planner: no remaining gap fits this pair — leaving this and later pomodoros of the microtask unplanned"
                );
                break; // can't do pomodoro N+1 before N
            }
        }
    }

    blocks.sort_by_key(|b| (b.start, b.end));
    let work_count = blocks.iter().filter(|b| b.kind == BlockKind::Task).count();
    let break_count = blocks.iter().filter(|b| b.kind == BlockKind::Break).count();
    tracing::debug!(
        total = blocks.len(),
        work = work_count,
        breaks = break_count,
        meetings = meetings.len(),
        "planner: done"
    );
    blocks
}
```

- [ ] **Step 4: Run to verify it passes**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test planner`
Expected: `test planner_table_driven_scenarios ... ok` — 1 test (11 table cases) passing.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/core/planner.rs src-tauri/tests/planner.rs
git commit -m "feat: deterministic pure planner with table-driven tests (gaps, long breaks, overflow, fallback)"
```

---

### Task 4: Planning window from settings (default 09:00–17:00) `[easy]`

**Files:**
- Create: `src-tauri/src/core/settings_service.rs`
- Modify: `src-tauri/src/core/mod.rs`
- Test: `src-tauri/tests/settings_service.rs`

A read-only helper (CQS: query). Keys: `planning_window_start`, `planning_window_end`, values `HH:MM`. Phase 6 adds the full `get_settings`/`update_settings` surface; this phase only needs the window.

- [ ] **Step 1: Write the failing test** *(test designed by the strongest agent)*

`src-tauri/tests/settings_service.rs`:

```rust
use focus_planner_lib::core::settings_service;
use focus_planner_lib::error::AppError;
use sqlx::SqlitePool;

#[sqlx::test]
async fn planning_window_defaults_to_nine_to_five(pool: SqlitePool) {
    let w = settings_service::get_planning_window(&pool).await.unwrap();
    assert_eq!(w.start.format("%H:%M").to_string(), "09:00");
    assert_eq!(w.end.format("%H:%M").to_string(), "17:00");
}

#[sqlx::test]
async fn planning_window_reads_overrides_from_settings(pool: SqlitePool) {
    sqlx::query(
        "INSERT INTO settings (key, value, updated_at) VALUES
         ('planning_window_start', '08:30', '2026-06-09T08:00:00Z'),
         ('planning_window_end', '16:00', '2026-06-09T08:00:00Z')",
    )
    .execute(&pool)
    .await
    .unwrap();

    let w = settings_service::get_planning_window(&pool).await.unwrap();
    assert_eq!(w.start.format("%H:%M").to_string(), "08:30");
    assert_eq!(w.end.format("%H:%M").to_string(), "16:00");
}

#[sqlx::test]
async fn malformed_window_value_is_a_validation_error(pool: SqlitePool) {
    sqlx::query(
        "INSERT INTO settings (key, value, updated_at) VALUES
         ('planning_window_start', 'nonsense', '2026-06-09T08:00:00Z')",
    )
    .execute(&pool)
    .await
    .unwrap();

    let err = settings_service::get_planning_window(&pool).await.unwrap_err();
    assert!(matches!(err, AppError::Validation(_)));
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test settings_service`
Expected: FAIL — `core::settings_service` doesn't exist.

- [ ] **Step 3: Write `src-tauri/src/core/settings_service.rs` (+ `pub mod settings_service;` in `core/mod.rs`)**

```rust
use chrono::NaiveTime;
use sqlx::SqlitePool;

use crate::core::planner::PlanningWindow;
use crate::error::AppError;

const DEFAULT_START: &str = "09:00";
const DEFAULT_END: &str = "17:00";

/// Query (CQS): reads the planning window from the settings table,
/// defaulting to 09:00–17:00 (spec §4).
pub async fn get_planning_window(pool: &SqlitePool) -> Result<PlanningWindow, AppError> {
    let rows = sqlx::query!(
        r#"SELECT key as "key!", value as "value!"
           FROM settings
           WHERE key IN ('planning_window_start', 'planning_window_end')"#
    )
    .fetch_all(pool)
    .await?;

    let mut start = DEFAULT_START.to_string();
    let mut end = DEFAULT_END.to_string();
    for row in rows {
        match row.key.as_str() {
            "planning_window_start" => start = row.value,
            "planning_window_end" => end = row.value,
            _ => {}
        }
    }

    let parse = |value: &str, key: &str| {
        NaiveTime::parse_from_str(value, "%H:%M").map_err(|_| {
            tracing::warn!(key, value, "invalid planning window setting");
            AppError::Validation(format!("setting {key} must be HH:MM, got '{value}'"))
        })
    };
    Ok(PlanningWindow {
        start: parse(&start, "planning_window_start")?,
        end: parse(&end, "planning_window_end")?,
    })
}
```

- [ ] **Step 4: Refresh the offline cache and run**

```bash
cd src-tauri && cargo sqlx prepare && cd ..
cargo test --manifest-path src-tauri/Cargo.toml --test settings_service
```

Expected: 3 tests passing.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/core src-tauri/tests/settings_service.rs src-tauri/.sqlx
git commit -m "feat: planning window read from settings with 09:00-17:00 default"
```

---

### Task 5: `generate_day_plan` service — gather, plan, persist `[medium]`

**Files:**
- Create: `src-tauri/src/core/plan_service.rs`
- Modify: `src-tauri/src/core/mod.rs`
- Test: `src-tauri/tests/day_plan_generate.rs`

The thin persistence shell around the pure planner: eligible microtasks (open, unarchived up the whole chain) with resolved types sorted by priority then deadline, existing meetings, the window — then delete+recreate the draft in one transaction. Committed plans cannot be regenerated.

- [ ] **Step 1: Write the failing tests** *(test designed by the strongest agent)*

`src-tauri/tests/day_plan_generate.rs` — assertions use raw SQL because `get_day_plan` arrives in Task 8. The plan date is in the future (2030) so `Utc::now()` never clamps the window:

```rust
use focus_planner_lib::core::plan_service;
use focus_planner_lib::error::AppError;
use sqlx::SqlitePool;

const TS: &str = "2026-06-09T08:00:00Z";
const DATE: &str = "2030-01-07";

async fn seed_backlog(pool: &SqlitePool) {
    sqlx::query("INSERT INTO projects (id, name, status, is_archived, created_at, updated_at) VALUES ('proj1', 'P', 'open', 0, ?1, ?1)")
        .bind(TS).execute(pool).await.unwrap();
    sqlx::query("INSERT INTO goals (id, project_id, title, status, is_archived, created_at, updated_at) VALUES ('goal1', 'proj1', 'G', 'open', 0, ?1, ?1)")
        .bind(TS).execute(pool).await.unwrap();
    sqlx::query("INSERT INTO tasks (id, goal_id, title, status, is_archived, created_at, updated_at) VALUES ('task1', 'goal1', 'T', 'open', 0, ?1, ?1)")
        .bind(TS).execute(pool).await.unwrap();
}

async fn seed_microtask(pool: &SqlitePool, id: &str, pomodoros: i64, priority: i64) {
    sqlx::query("INSERT INTO microtasks (id, task_id, title, estimated_minutes, pomodoro_count, priority, status, is_archived, created_at, updated_at) VALUES (?1, 'task1', ?1, 40, ?2, ?3, 'open', 0, ?4, ?4)")
        .bind(id).bind(pomodoros).bind(priority).bind(TS).execute(pool).await.unwrap();
}

#[derive(Debug, sqlx::FromRow)]
struct BlockRow {
    block_type: String,
    microtask_id: Option<String>,
    start_time: String,
    end_time: String,
    pomodoro_index: Option<i64>,
    sort_order: i64,
}

async fn blocks_for_date(pool: &SqlitePool, date: &str) -> Vec<BlockRow> {
    sqlx::query_as::<_, BlockRow>(
        "SELECT wb.block_type, wb.microtask_id, wb.start_time, wb.end_time, wb.pomodoro_index, wb.sort_order
         FROM work_blocks wb JOIN plans p ON p.id = wb.plan_id
         WHERE p.date = ?1 ORDER BY wb.sort_order",
    )
    .bind(date)
    .fetch_all(pool)
    .await
    .unwrap()
}

#[sqlx::test]
async fn generate_creates_a_draft_plan_with_work_and_break_blocks(pool: SqlitePool) {
    seed_backlog(&pool).await;
    seed_microtask(&pool, "mt1", 2, 0).await;

    plan_service::generate_day_plan(&pool, "plan1".into(), DATE.into(), None)
        .await
        .unwrap();

    let status: String = sqlx::query_scalar("SELECT status FROM plans WHERE date = ?1")
        .bind(DATE).fetch_one(&pool).await.unwrap();
    assert_eq!(status, "draft");

    // mt1 has no type; the seeded Standard 20/5 default applies → 2 pairs.
    let blocks = blocks_for_date(&pool, DATE).await;
    assert_eq!(blocks.len(), 4);
    assert_eq!(blocks[0].block_type, "task");
    assert_eq!(blocks[0].microtask_id.as_deref(), Some("mt1"));
    assert_eq!(blocks[0].start_time, "2030-01-07T09:00:00Z");
    assert_eq!(blocks[0].end_time, "2030-01-07T09:20:00Z");
    assert_eq!(blocks[0].pomodoro_index, Some(1));
    assert_eq!(blocks[1].block_type, "break");
    assert_eq!(blocks[1].end_time, "2030-01-07T09:25:00Z");
    assert_eq!(blocks[2].pomodoro_index, Some(2));
    assert_eq!(blocks[3].end_time, "2030-01-07T09:50:00Z");
    assert_eq!(
        blocks.iter().map(|b| b.sort_order).collect::<Vec<_>>(),
        vec![0, 1, 2, 3],
        "sort_order is chronological"
    );
}

#[sqlx::test]
async fn regenerate_replaces_the_draft_and_keeps_manual_meetings(pool: SqlitePool) {
    seed_backlog(&pool).await;
    seed_microtask(&pool, "mt1", 1, 0).await;
    plan_service::generate_day_plan(&pool, "plan1".into(), DATE.into(), None)
        .await
        .unwrap();
    // Manual meeting on the existing draft (raw insert; add_work_block arrives in Task 6).
    sqlx::query("INSERT INTO work_blocks (id, plan_id, block_type, start_time, end_time, sort_order, created_at, updated_at) VALUES ('meet1', 'plan1', 'meeting', '2030-01-07T09:20:00Z', '2030-01-07T10:00:00Z', 99, ?1, ?1)")
        .bind(TS).execute(&pool).await.unwrap();

    plan_service::generate_day_plan(&pool, "plan2".into(), DATE.into(), None)
        .await
        .unwrap();

    let plan_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM plans")
        .fetch_one(&pool).await.unwrap();
    assert_eq!(plan_count, 1, "one plan per date — old draft replaced");
    let plan_id: String = sqlx::query_scalar("SELECT id FROM plans WHERE date = ?1")
        .bind(DATE).fetch_one(&pool).await.unwrap();
    assert_eq!(plan_id, "plan2");

    let blocks = blocks_for_date(&pool, DATE).await;
    let meetings: Vec<_> = blocks.iter().filter(|b| b.block_type == "meeting").collect();
    assert_eq!(meetings.len(), 1, "manual meeting survives regeneration");
    assert_eq!(meetings[0].start_time, "2030-01-07T09:20:00Z");
    // The 25-minute pair no longer fits before 09:20 → first work block at 10:00.
    let first_work = blocks.iter().find(|b| b.block_type == "task").unwrap();
    assert_eq!(first_work.start_time, "2030-01-07T10:00:00Z");
}

#[sqlx::test]
async fn regenerating_a_committed_plan_is_a_validation_error(pool: SqlitePool) {
    seed_backlog(&pool).await;
    seed_microtask(&pool, "mt1", 1, 0).await;
    plan_service::generate_day_plan(&pool, "plan1".into(), DATE.into(), None)
        .await
        .unwrap();
    sqlx::query("UPDATE plans SET status = 'committed' WHERE id = 'plan1'")
        .execute(&pool).await.unwrap();

    let err = plan_service::generate_day_plan(&pool, "plan2".into(), DATE.into(), None)
        .await
        .unwrap_err();
    assert!(matches!(err, AppError::Validation(_)));
}

#[sqlx::test]
async fn microtasks_are_planned_by_priority_then_deadline(pool: SqlitePool) {
    seed_backlog(&pool).await;
    seed_microtask(&pool, "low", 1, 0).await;
    seed_microtask(&pool, "high", 1, 5).await;

    plan_service::generate_day_plan(&pool, "plan1".into(), DATE.into(), None)
        .await
        .unwrap();

    let blocks = blocks_for_date(&pool, DATE).await;
    let work: Vec<_> = blocks.iter().filter(|b| b.block_type == "task").collect();
    assert_eq!(work[0].microtask_id.as_deref(), Some("high"));
    assert_eq!(work[1].microtask_id.as_deref(), Some("low"));
}

#[sqlx::test]
async fn completed_and_archived_microtasks_are_excluded(pool: SqlitePool) {
    seed_backlog(&pool).await;
    seed_microtask(&pool, "open1", 1, 0).await;
    sqlx::query("INSERT INTO microtasks (id, task_id, title, estimated_minutes, pomodoro_count, status, is_archived, created_at, updated_at) VALUES ('done1', 'task1', 'done1', 20, 1, 'completed', 0, ?1, ?1), ('arch1', 'task1', 'arch1', 20, 1, 'open', 1, ?1, ?1)")
        .bind(TS).execute(&pool).await.unwrap();

    plan_service::generate_day_plan(&pool, "plan1".into(), DATE.into(), None)
        .await
        .unwrap();

    let blocks = blocks_for_date(&pool, DATE).await;
    let work: Vec<_> = blocks.iter().filter(|b| b.block_type == "task").collect();
    assert_eq!(work.len(), 1);
    assert_eq!(work[0].microtask_id.as_deref(), Some("open1"));
}

#[sqlx::test]
async fn bad_date_and_unknown_strategy_are_validation_errors(pool: SqlitePool) {
    let bad_date = plan_service::generate_day_plan(&pool, "p".into(), "07/01/2030".into(), None)
        .await
        .unwrap_err();
    assert!(matches!(bad_date, AppError::Validation(_)));

    let bad_strategy =
        plan_service::generate_day_plan(&pool, "p".into(), DATE.into(), Some("magic".into()))
            .await
            .unwrap_err();
    assert!(matches!(bad_strategy, AppError::Validation(_)));
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test day_plan_generate`
Expected: FAIL — `core::plan_service` doesn't exist.

- [ ] **Step 3: Write `src-tauri/src/core/plan_service.rs` (+ `pub mod plan_service;` in `core/mod.rs`)**

```rust
use chrono::{DateTime, NaiveDate, Utc};
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::core::planner::{plan, Meeting, PlannerInput, PlannerMicrotask, PomodoroParams};
use crate::core::settings_service;
use crate::error::AppError;

fn now_iso() -> String {
    Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

fn fmt_utc(dt: DateTime<Utc>) -> String {
    dt.format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

fn parse_utc(value: &str, field: &str) -> Result<DateTime<Utc>, AppError> {
    DateTime::parse_from_rfc3339(value)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|_| AppError::Validation(format!("{field} must be an ISO 8601 UTC timestamp, got '{value}'")))
}

/// Command (CQS): runs the deterministic planner and persists a draft plan,
/// replacing any existing draft for the date (one plan per date, spec §2).
pub async fn generate_day_plan(
    pool: &SqlitePool,
    plan_id: String,
    date: String,
    strategy: Option<String>,
) -> Result<(), AppError> {
    if let Some(s) = strategy.as_deref() {
        if s != "sequential" {
            tracing::warn!(strategy = s, "generate_day_plan rejected: unknown strategy");
            return Err(AppError::Validation(format!(
                "unknown strategy '{s}' — M1 supports only 'sequential'"
            )));
        }
    }
    let parsed_date = NaiveDate::parse_from_str(&date, "%Y-%m-%d")
        .map_err(|_| AppError::Validation(format!("date must be YYYY-MM-DD, got '{date}'")))?;

    let existing = sqlx::query!(
        r#"SELECT id as "id!", status as "status!" FROM plans WHERE date = ?1"#,
        date
    )
    .fetch_optional(pool)
    .await?;
    if let Some(p) = &existing {
        if p.status == "committed" {
            tracing::warn!(date = %date, "generate_day_plan rejected: plan already committed");
            return Err(AppError::Validation(format!(
                "plan for {date} is already committed and cannot be regenerated"
            )));
        }
    }

    // Eligible microtasks with their types, sorted by priority then deadline.
    let rows = sqlx::query!(
        r#"SELECT m.id as "id!", m.title as "title!", m.pomodoro_count as "pomodoro_count!",
                  pt.id as "type_id?", pt.work_minutes as "work_minutes?",
                  pt.rest_minutes as "rest_minutes?",
                  pt.long_break_minutes as "long_break_minutes?",
                  pt.long_break_every as "long_break_every?"
           FROM microtasks m
           JOIN tasks t ON t.id = m.task_id
           JOIN goals g ON g.id = t.goal_id
           JOIN projects p ON p.id = g.project_id
           LEFT JOIN pomodoro_types pt ON pt.id = m.pomodoro_type_id
           WHERE m.status = 'open' AND m.is_archived = 0
             AND t.is_archived = 0 AND g.is_archived = 0 AND p.is_archived = 0
           ORDER BY m.priority DESC, (m.deadline IS NULL), m.deadline, m.created_at"#
    )
    .fetch_all(pool)
    .await?;
    let microtasks: Vec<PlannerMicrotask> = rows
        .into_iter()
        .map(|r| PlannerMicrotask {
            id: r.id,
            title: r.title,
            pomodoro_count: r.pomodoro_count,
            pomodoro: match (r.type_id, r.work_minutes, r.rest_minutes) {
                (Some(type_key), Some(work_minutes), Some(rest_minutes)) => Some(PomodoroParams {
                    type_key,
                    work_minutes,
                    rest_minutes,
                    long_break_minutes: r.long_break_minutes,
                    long_break_every: r.long_break_every,
                }),
                _ => None,
            },
        })
        .collect();

    let default_pomodoro = sqlx::query!(
        r#"SELECT id as "id!", work_minutes as "work_minutes!", rest_minutes as "rest_minutes!",
                  long_break_minutes, long_break_every
           FROM pomodoro_types WHERE is_default = 1 LIMIT 1"#
    )
    .fetch_optional(pool)
    .await?
    .map(|r| PomodoroParams {
        type_key: r.id,
        work_minutes: r.work_minutes,
        rest_minutes: r.rest_minutes,
        long_break_minutes: r.long_break_minutes,
        long_break_every: r.long_break_every,
    });

    // Manually-added meetings on the existing draft survive regeneration.
    let mut meetings = Vec::new();
    if let Some(p) = &existing {
        let meeting_rows = sqlx::query!(
            r#"SELECT start_time as "start_time!", end_time as "end_time!"
               FROM work_blocks WHERE plan_id = ?1 AND block_type = 'meeting'
               ORDER BY start_time"#,
            p.id
        )
        .fetch_all(pool)
        .await?;
        for m in meeting_rows {
            meetings.push(Meeting {
                start: parse_utc(&m.start_time, "meeting start_time")?,
                end: parse_utc(&m.end_time, "meeting end_time")?,
            });
        }
    }

    let window = settings_service::get_planning_window(pool).await?;
    let input = PlannerInput { date: parsed_date, microtasks, meetings, default_pomodoro };
    let specs = plan(&input, &window, Utc::now());

    // Replace the existing draft and persist — one transaction. Explicit
    // child-row delete: no reliance on the FK cascade pragma.
    let mut tx = pool.begin().await?;
    if let Some(p) = &existing {
        sqlx::query!("DELETE FROM work_blocks WHERE plan_id = ?1", p.id)
            .execute(&mut *tx)
            .await?;
        sqlx::query!("DELETE FROM plans WHERE id = ?1", p.id)
            .execute(&mut *tx)
            .await?;
    }
    let ts = now_iso();
    sqlx::query!(
        "INSERT INTO plans (id, date, status, created_at, updated_at) VALUES (?1, ?2, 'draft', ?3, ?4)",
        plan_id,
        date,
        ts,
        ts
    )
    .execute(&mut *tx)
    .await?;
    for (i, spec) in specs.iter().enumerate() {
        let id = Uuid::new_v4().to_string();
        let block_type = spec.kind.as_str();
        let microtask_id = spec.microtask_id.clone();
        let start_time = fmt_utc(spec.start);
        let end_time = fmt_utc(spec.end);
        let pomodoro_index = spec.pomodoro_index;
        let sort_order = i as i64;
        sqlx::query!(
            "INSERT INTO work_blocks (id, plan_id, block_type, microtask_id, calendar_event_id, start_time, end_time, pomodoro_index, sort_order, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, NULL, ?5, ?6, ?7, ?8, ?9, ?10)",
            id,
            plan_id,
            block_type,
            microtask_id,
            start_time,
            end_time,
            pomodoro_index,
            sort_order,
            ts,
            ts
        )
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    tracing::info!(
        plan_id = %plan_id,
        date = %date,
        blocks = specs.len(),
        replaced_existing_draft = existing.is_some(),
        "day plan generated as draft"
    );
    Ok(())
}
```

- [ ] **Step 4: Refresh the offline cache and run**

```bash
cd src-tauri && cargo sqlx prepare && cd ..
cargo test --manifest-path src-tauri/Cargo.toml --test day_plan_generate
```

Expected: 6 tests passing.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/core src-tauri/tests/day_plan_generate.rs src-tauri/.sqlx
git commit -m "feat: generate_day_plan — gather inputs, run pure planner, persist draft transactionally"
```

---

### Task 6: `add_work_block`, `move_work_block`, `remove_work_block` `[medium]`

**Files:**
- Modify: `src-tauri/src/core/plan_service.rs`
- Test: `src-tauri/tests/day_plan_edit.rs`

- [ ] **Step 1: Write the failing tests** *(test designed by the strongest agent)*

`src-tauri/tests/day_plan_edit.rs`:

```rust
use focus_planner_lib::core::plan_service;
use focus_planner_lib::error::AppError;
use sqlx::SqlitePool;

const TS: &str = "2026-06-09T08:00:00Z";
const DATE: &str = "2030-01-07";

async fn seed_draft_plan(pool: &SqlitePool) {
    sqlx::query("INSERT INTO plans (id, date, status, created_at, updated_at) VALUES ('plan1', ?1, 'draft', ?2, ?2)")
        .bind(DATE).bind(TS).execute(pool).await.unwrap();
}

async fn seed_microtask(pool: &SqlitePool) {
    sqlx::query("INSERT INTO projects (id, name, status, is_archived, created_at, updated_at) VALUES ('proj1', 'P', 'open', 0, ?1, ?1)")
        .bind(TS).execute(pool).await.unwrap();
    sqlx::query("INSERT INTO goals (id, project_id, title, status, is_archived, created_at, updated_at) VALUES ('goal1', 'proj1', 'G', 'open', 0, ?1, ?1)")
        .bind(TS).execute(pool).await.unwrap();
    sqlx::query("INSERT INTO tasks (id, goal_id, title, status, is_archived, created_at, updated_at) VALUES ('task1', 'goal1', 'T', 'open', 0, ?1, ?1)")
        .bind(TS).execute(pool).await.unwrap();
    sqlx::query("INSERT INTO microtasks (id, task_id, title, estimated_minutes, pomodoro_count, status, is_archived, created_at, updated_at) VALUES ('mt1', 'task1', 'Write spec', 20, 1, 'open', 0, ?1, ?1)")
        .bind(TS).execute(pool).await.unwrap();
}

#[sqlx::test]
async fn add_requires_microtask_id_iff_block_type_is_task(pool: SqlitePool) {
    seed_draft_plan(&pool).await;

    let task_without = plan_service::add_work_block(
        &pool, "b1".into(), "plan1".into(), "task".into(), None,
        "2030-01-07T09:00:00Z".into(), "2030-01-07T09:20:00Z".into(),
    ).await.unwrap_err();
    assert!(matches!(task_without, AppError::Validation(_)));

    let meeting_with = plan_service::add_work_block(
        &pool, "b2".into(), "plan1".into(), "meeting".into(), Some("mt1".into()),
        "2030-01-07T09:00:00Z".into(), "2030-01-07T09:20:00Z".into(),
    ).await.unwrap_err();
    assert!(matches!(meeting_with, AppError::Validation(_)));

    let bad_type = plan_service::add_work_block(
        &pool, "b3".into(), "plan1".into(), "lunch".into(), None,
        "2030-01-07T09:00:00Z".into(), "2030-01-07T09:20:00Z".into(),
    ).await.unwrap_err();
    assert!(matches!(bad_type, AppError::Validation(_)));
}

#[sqlx::test]
async fn add_task_block_and_manual_meeting_block(pool: SqlitePool) {
    seed_draft_plan(&pool).await;
    seed_microtask(&pool).await;

    plan_service::add_work_block(
        &pool, "b1".into(), "plan1".into(), "task".into(), Some("mt1".into()),
        "2030-01-07T09:00:00Z".into(), "2030-01-07T09:20:00Z".into(),
    ).await.unwrap();
    plan_service::add_work_block(
        &pool, "b2".into(), "plan1".into(), "meeting".into(), None,
        "2030-01-07T13:00:00Z".into(), "2030-01-07T14:00:00Z".into(),
    ).await.unwrap();

    let rows: Vec<(String, Option<String>, i64)> = sqlx::query_as(
        "SELECT block_type, microtask_id, sort_order FROM work_blocks WHERE plan_id = 'plan1' ORDER BY sort_order",
    ).fetch_all(&pool).await.unwrap();
    assert_eq!(rows[0], ("task".into(), Some("mt1".into()), 0));
    assert_eq!(rows[1], ("meeting".into(), None, 1));
}

#[sqlx::test]
async fn add_rejects_start_not_before_end_and_missing_plan(pool: SqlitePool) {
    seed_draft_plan(&pool).await;

    let backwards = plan_service::add_work_block(
        &pool, "b1".into(), "plan1".into(), "meeting".into(), None,
        "2030-01-07T14:00:00Z".into(), "2030-01-07T13:00:00Z".into(),
    ).await.unwrap_err();
    assert!(matches!(backwards, AppError::Validation(_)));

    let no_plan = plan_service::add_work_block(
        &pool, "b2".into(), "ghost".into(), "meeting".into(), None,
        "2030-01-07T13:00:00Z".into(), "2030-01-07T14:00:00Z".into(),
    ).await.unwrap_err();
    assert!(matches!(no_plan, AppError::NotFound { .. }));
}

#[sqlx::test]
async fn move_without_new_end_preserves_duration(pool: SqlitePool) {
    seed_draft_plan(&pool).await;
    plan_service::add_work_block(
        &pool, "b1".into(), "plan1".into(), "meeting".into(), None,
        "2030-01-07T09:00:00Z".into(), "2030-01-07T10:00:00Z".into(),
    ).await.unwrap();

    plan_service::move_work_block(&pool, "b1".into(), "2030-01-07T11:00:00Z".into(), None)
        .await
        .unwrap();

    let (start, end): (String, String) =
        sqlx::query_as("SELECT start_time, end_time FROM work_blocks WHERE id = 'b1'")
            .fetch_one(&pool).await.unwrap();
    assert_eq!(start, "2030-01-07T11:00:00Z");
    assert_eq!(end, "2030-01-07T12:00:00Z");
}

#[sqlx::test]
async fn move_with_explicit_new_end_reschedules_both(pool: SqlitePool) {
    seed_draft_plan(&pool).await;
    plan_service::add_work_block(
        &pool, "b1".into(), "plan1".into(), "meeting".into(), None,
        "2030-01-07T09:00:00Z".into(), "2030-01-07T10:00:00Z".into(),
    ).await.unwrap();

    plan_service::move_work_block(
        &pool, "b1".into(),
        "2030-01-07T11:00:00Z".into(), Some("2030-01-07T11:30:00Z".into()),
    ).await.unwrap();

    let (start, end): (String, String) =
        sqlx::query_as("SELECT start_time, end_time FROM work_blocks WHERE id = 'b1'")
            .fetch_one(&pool).await.unwrap();
    assert_eq!(start, "2030-01-07T11:00:00Z");
    assert_eq!(end, "2030-01-07T11:30:00Z");
}

#[sqlx::test]
async fn remove_deletes_the_block_and_missing_id_is_not_found(pool: SqlitePool) {
    seed_draft_plan(&pool).await;
    plan_service::add_work_block(
        &pool, "b1".into(), "plan1".into(), "meeting".into(), None,
        "2030-01-07T09:00:00Z".into(), "2030-01-07T10:00:00Z".into(),
    ).await.unwrap();

    plan_service::remove_work_block(&pool, "b1".into()).await.unwrap();
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM work_blocks")
        .fetch_one(&pool).await.unwrap();
    assert_eq!(count, 0);

    let err = plan_service::remove_work_block(&pool, "b1".into()).await.unwrap_err();
    assert!(matches!(err, AppError::NotFound { .. }));
}

#[sqlx::test]
async fn editing_blocks_of_a_committed_plan_is_rejected(pool: SqlitePool) {
    seed_draft_plan(&pool).await;
    plan_service::add_work_block(
        &pool, "b1".into(), "plan1".into(), "meeting".into(), None,
        "2030-01-07T09:00:00Z".into(), "2030-01-07T10:00:00Z".into(),
    ).await.unwrap();
    sqlx::query("UPDATE plans SET status = 'committed' WHERE id = 'plan1'")
        .execute(&pool).await.unwrap();

    let add = plan_service::add_work_block(
        &pool, "b2".into(), "plan1".into(), "meeting".into(), None,
        "2030-01-07T13:00:00Z".into(), "2030-01-07T14:00:00Z".into(),
    ).await.unwrap_err();
    assert!(matches!(add, AppError::Validation(_)));

    let mv = plan_service::move_work_block(&pool, "b1".into(), "2030-01-07T11:00:00Z".into(), None)
        .await.unwrap_err();
    assert!(matches!(mv, AppError::Validation(_)));

    let rm = plan_service::remove_work_block(&pool, "b1".into()).await.unwrap_err();
    assert!(matches!(rm, AppError::Validation(_)));
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test day_plan_edit`
Expected: FAIL — `add_work_block` etc. don't exist.

- [ ] **Step 3: Append the three commands to `src-tauri/src/core/plan_service.rs`**

```rust
/// Command (CQS): adds a block to a draft plan. microtask_id is required
/// iff block_type = 'task'; 'meeting' is the manual meeting entry path in M1.
pub async fn add_work_block(
    pool: &SqlitePool,
    id: String,
    plan_id: String,
    block_type: String,
    microtask_id: Option<String>,
    start: String,
    end: String,
) -> Result<(), AppError> {
    if !matches!(block_type.as_str(), "task" | "break" | "meeting") {
        return Err(AppError::Validation(format!(
            "block_type must be task, break, or meeting — got '{block_type}'"
        )));
    }
    match (block_type.as_str(), &microtask_id) {
        ("task", None) => {
            return Err(AppError::Validation("microtask_id is required for task blocks".into()))
        }
        ("break", Some(_)) | ("meeting", Some(_)) => {
            return Err(AppError::Validation("microtask_id is only allowed on task blocks".into()))
        }
        _ => {}
    }
    let start_dt = parse_utc(&start, "start")?;
    let end_dt = parse_utc(&end, "end")?;
    if start_dt >= end_dt {
        return Err(AppError::Validation("start must be before end".into()));
    }

    let plan = sqlx::query!(r#"SELECT status as "status!" FROM plans WHERE id = ?1"#, plan_id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound { entity: "plan", id: plan_id.clone() })?;
    if plan.status != "draft" {
        tracing::warn!(plan_id = %plan_id, "add_work_block rejected: plan is committed");
        return Err(AppError::Validation("committed plans cannot be edited".into()));
    }
    if let Some(mt_id) = &microtask_id {
        sqlx::query!(r#"SELECT id as "id!" FROM microtasks WHERE id = ?1"#, mt_id)
            .fetch_optional(pool)
            .await?
            .ok_or_else(|| AppError::NotFound { entity: "microtask", id: mt_id.clone() })?;
    }

    let next_sort = sqlx::query!(
        r#"SELECT COALESCE(MAX(sort_order) + 1, 0) as "next!: i64" FROM work_blocks WHERE plan_id = ?1"#,
        plan_id
    )
    .fetch_one(pool)
    .await?
    .next;
    let ts = now_iso();
    let start_norm = fmt_utc(start_dt);
    let end_norm = fmt_utc(end_dt);
    sqlx::query!(
        "INSERT INTO work_blocks (id, plan_id, block_type, microtask_id, calendar_event_id, start_time, end_time, pomodoro_index, sort_order, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, NULL, ?5, ?6, NULL, ?7, ?8, ?9)",
        id,
        plan_id,
        block_type,
        microtask_id,
        start_norm,
        end_norm,
        next_sort,
        ts,
        ts
    )
    .execute(pool)
    .await?;
    Ok(())
}

/// Command (CQS): reschedules a block. When new_end is absent the block's
/// duration is preserved (a pure shift).
pub async fn move_work_block(
    pool: &SqlitePool,
    id: String,
    new_start: String,
    new_end: Option<String>,
) -> Result<(), AppError> {
    let row = sqlx::query!(
        r#"SELECT wb.start_time as "start_time!", wb.end_time as "end_time!", p.status as "status!"
           FROM work_blocks wb JOIN plans p ON p.id = wb.plan_id WHERE wb.id = ?1"#,
        id
    )
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound { entity: "work_block", id: id.clone() })?;
    if row.status != "draft" {
        tracing::warn!(block_id = %id, "move_work_block rejected: plan is committed");
        return Err(AppError::Validation("committed plans cannot be edited".into()));
    }

    let start_dt = parse_utc(&new_start, "new_start")?;
    let end_dt = match new_end {
        Some(e) => parse_utc(&e, "new_end")?,
        None => {
            let old_start = parse_utc(&row.start_time, "start_time")?;
            let old_end = parse_utc(&row.end_time, "end_time")?;
            start_dt + (old_end - old_start)
        }
    };
    if start_dt >= end_dt {
        return Err(AppError::Validation("new_start must be before new_end".into()));
    }
    let start_norm = fmt_utc(start_dt);
    let end_norm = fmt_utc(end_dt);
    let ts = now_iso();
    sqlx::query!(
        "UPDATE work_blocks SET start_time = ?1, end_time = ?2, updated_at = ?3 WHERE id = ?4",
        start_norm,
        end_norm,
        ts,
        id
    )
    .execute(pool)
    .await?;
    Ok(())
}

/// Command (CQS): deletes a block from a draft plan.
pub async fn remove_work_block(pool: &SqlitePool, id: String) -> Result<(), AppError> {
    let row = sqlx::query!(
        r#"SELECT p.status as "status!" FROM work_blocks wb JOIN plans p ON p.id = wb.plan_id WHERE wb.id = ?1"#,
        id
    )
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound { entity: "work_block", id: id.clone() })?;
    if row.status != "draft" {
        tracing::warn!(block_id = %id, "remove_work_block rejected: plan is committed");
        return Err(AppError::Validation("committed plans cannot be edited".into()));
    }
    sqlx::query!("DELETE FROM work_blocks WHERE id = ?1", id)
        .execute(pool)
        .await?;
    Ok(())
}
```

- [ ] **Step 4: Refresh the offline cache and run**

```bash
cd src-tauri && cargo sqlx prepare && cd ..
cargo test --manifest-path src-tauri/Cargo.toml --test day_plan_edit
```

Expected: 7 tests passing.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/core/plan_service.rs src-tauri/tests/day_plan_edit.rs src-tauri/.sqlx
git commit -m "feat: add/move/remove work block commands with draft-only and iff-task validations"
```

---

### Task 7: `reorder_work_blocks`, `commit_day_plan`, `clear_day_plan` `[medium]`

**Files:**
- Modify: `src-tauri/src/core/plan_service.rs`
- Test: `src-tauri/tests/day_plan_lifecycle.rs`

- [ ] **Step 1: Write the failing tests** *(test designed by the strongest agent)*

`src-tauri/tests/day_plan_lifecycle.rs`:

```rust
use focus_planner_lib::core::plan_service;
use focus_planner_lib::error::AppError;
use sqlx::SqlitePool;

const TS: &str = "2026-06-09T08:00:00Z";
const DATE: &str = "2030-01-07";

async fn seed_draft_plan_with_two_blocks(pool: &SqlitePool) {
    sqlx::query("INSERT INTO plans (id, date, status, created_at, updated_at) VALUES ('plan1', ?1, 'draft', ?2, ?2)")
        .bind(DATE).bind(TS).execute(pool).await.unwrap();
    sqlx::query("INSERT INTO work_blocks (id, plan_id, block_type, start_time, end_time, sort_order, created_at, updated_at) VALUES
        ('b1', 'plan1', 'meeting', '2030-01-07T09:00:00Z', '2030-01-07T10:00:00Z', 0, ?1, ?1),
        ('b2', 'plan1', 'meeting', '2030-01-07T13:00:00Z', '2030-01-07T14:00:00Z', 1, ?1, ?1)")
        .bind(TS).execute(pool).await.unwrap();
}

#[sqlx::test]
async fn reorder_rewrites_sort_order_from_the_full_list(pool: SqlitePool) {
    seed_draft_plan_with_two_blocks(&pool).await;

    plan_service::reorder_work_blocks(&pool, "plan1".into(), vec!["b2".into(), "b1".into()])
        .await
        .unwrap();

    let rows: Vec<(String, i64)> =
        sqlx::query_as("SELECT id, sort_order FROM work_blocks ORDER BY sort_order")
            .fetch_all(&pool).await.unwrap();
    assert_eq!(rows, vec![("b2".into(), 0), ("b1".into(), 1)]);
}

#[sqlx::test]
async fn reorder_rejects_a_list_that_does_not_match_the_plans_blocks(pool: SqlitePool) {
    seed_draft_plan_with_two_blocks(&pool).await;

    let missing = plan_service::reorder_work_blocks(&pool, "plan1".into(), vec!["b1".into()])
        .await.unwrap_err();
    assert!(matches!(missing, AppError::Validation(_)));

    let foreign = plan_service::reorder_work_blocks(
        &pool, "plan1".into(), vec!["b1".into(), "ghost".into()],
    ).await.unwrap_err();
    assert!(matches!(foreign, AppError::Validation(_)));
}

#[sqlx::test]
async fn commit_seals_the_plan_and_double_commit_is_rejected(pool: SqlitePool) {
    seed_draft_plan_with_two_blocks(&pool).await;

    plan_service::commit_day_plan(&pool, "plan1".into()).await.unwrap();
    let status: String = sqlx::query_scalar("SELECT status FROM plans WHERE id = 'plan1'")
        .fetch_one(&pool).await.unwrap();
    assert_eq!(status, "committed");

    let again = plan_service::commit_day_plan(&pool, "plan1".into()).await.unwrap_err();
    assert!(matches!(again, AppError::Validation(_)));

    let ghost = plan_service::commit_day_plan(&pool, "ghost".into()).await.unwrap_err();
    assert!(matches!(ghost, AppError::NotFound { .. }));
}

#[sqlx::test]
async fn reorder_on_a_committed_plan_is_rejected(pool: SqlitePool) {
    seed_draft_plan_with_two_blocks(&pool).await;
    plan_service::commit_day_plan(&pool, "plan1".into()).await.unwrap();

    let err = plan_service::reorder_work_blocks(&pool, "plan1".into(), vec!["b2".into(), "b1".into()])
        .await.unwrap_err();
    assert!(matches!(err, AppError::Validation(_)));
}

#[sqlx::test]
async fn clear_deletes_the_draft_plan_and_its_blocks(pool: SqlitePool) {
    seed_draft_plan_with_two_blocks(&pool).await;

    plan_service::clear_day_plan(&pool, "plan1".into()).await.unwrap();

    let plans: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM plans").fetch_one(&pool).await.unwrap();
    let blocks: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM work_blocks").fetch_one(&pool).await.unwrap();
    assert_eq!((plans, blocks), (0, 0));
}

#[sqlx::test]
async fn clear_on_a_committed_plan_is_rejected(pool: SqlitePool) {
    seed_draft_plan_with_two_blocks(&pool).await;
    plan_service::commit_day_plan(&pool, "plan1".into()).await.unwrap();

    let err = plan_service::clear_day_plan(&pool, "plan1".into()).await.unwrap_err();
    assert!(matches!(err, AppError::Validation(_)));
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test day_plan_lifecycle`
Expected: FAIL — the three functions don't exist.

- [ ] **Step 3: Append to `src-tauri/src/core/plan_service.rs`**

```rust
/// Command (CQS): full-list reorder — rewrites sort_order from ordered_ids
/// in one transaction (same convention as Phase 2's reorder_* commands).
pub async fn reorder_work_blocks(
    pool: &SqlitePool,
    plan_id: String,
    ordered_ids: Vec<String>,
) -> Result<(), AppError> {
    let plan = sqlx::query!(r#"SELECT status as "status!" FROM plans WHERE id = ?1"#, plan_id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound { entity: "plan", id: plan_id.clone() })?;
    if plan.status != "draft" {
        tracing::warn!(plan_id = %plan_id, "reorder_work_blocks rejected: plan is committed");
        return Err(AppError::Validation("committed plans cannot be edited".into()));
    }

    let existing: Vec<String> =
        sqlx::query!(r#"SELECT id as "id!" FROM work_blocks WHERE plan_id = ?1"#, plan_id)
            .fetch_all(pool)
            .await?
            .into_iter()
            .map(|r| r.id)
            .collect();
    let mut want = existing.clone();
    want.sort();
    let mut got = ordered_ids.clone();
    got.sort();
    if want != got {
        tracing::warn!(plan_id = %plan_id, "reorder_work_blocks rejected: id set mismatch");
        return Err(AppError::Validation(
            "ordered_ids must contain exactly the plan's work block ids".into(),
        ));
    }

    let ts = now_iso();
    let mut tx = pool.begin().await?;
    for (i, block_id) in ordered_ids.iter().enumerate() {
        let sort_order = i as i64;
        sqlx::query!(
            "UPDATE work_blocks SET sort_order = ?1, updated_at = ?2 WHERE id = ?3",
            sort_order,
            ts,
            block_id
        )
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(())
}

/// Command (CQS): seals the draft — Phase 4's runtime only runs committed plans.
pub async fn commit_day_plan(pool: &SqlitePool, plan_id: String) -> Result<(), AppError> {
    let plan = sqlx::query!(
        r#"SELECT status as "status!", date as "date!" FROM plans WHERE id = ?1"#,
        plan_id
    )
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound { entity: "plan", id: plan_id.clone() })?;
    if plan.status == "committed" {
        tracing::warn!(plan_id = %plan_id, "commit_day_plan rejected: already committed");
        return Err(AppError::Validation("plan is already committed".into()));
    }
    let ts = now_iso();
    sqlx::query!(
        "UPDATE plans SET status = 'committed', updated_at = ?1 WHERE id = ?2",
        ts,
        plan_id
    )
    .execute(pool)
    .await?;
    tracing::info!(plan_id = %plan_id, date = %plan.date, "day plan committed");
    Ok(())
}

/// Command (CQS): deletes the draft plan and all its blocks, freeing the
/// date's UNIQUE slot. Committed plans are sealed and cannot be cleared in M1.
pub async fn clear_day_plan(pool: &SqlitePool, plan_id: String) -> Result<(), AppError> {
    let plan = sqlx::query!(
        r#"SELECT status as "status!", date as "date!" FROM plans WHERE id = ?1"#,
        plan_id
    )
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound { entity: "plan", id: plan_id.clone() })?;
    if plan.status == "committed" {
        tracing::warn!(plan_id = %plan_id, "clear_day_plan rejected: plan is committed");
        return Err(AppError::Validation("committed plans cannot be cleared".into()));
    }
    let mut tx = pool.begin().await?;
    sqlx::query!("DELETE FROM work_blocks WHERE plan_id = ?1", plan_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query!("DELETE FROM plans WHERE id = ?1", plan_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    tracing::info!(plan_id = %plan_id, date = %plan.date, "day plan cleared (plan and blocks deleted)");
    Ok(())
}
```

- [ ] **Step 4: Refresh the offline cache and run**

```bash
cd src-tauri && cargo sqlx prepare && cd ..
cargo test --manifest-path src-tauri/Cargo.toml --test day_plan_lifecycle
```

Expected: 6 tests passing.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/core/plan_service.rs src-tauri/tests/day_plan_lifecycle.rs src-tauri/.sqlx
git commit -m "feat: reorder_work_blocks, commit_day_plan, clear_day_plan with sealing rules"
```

---

### Task 8: Plan models + `get_day_plan` / `get_today` queries `[medium]`

**Files:**
- Create: `src-tauri/src/models/plan.rs`
- Modify: `src-tauri/src/models/mod.rs`, `src-tauri/src/core/plan_service.rs`
- Test: `src-tauri/tests/day_plan_queries.rs`

Queries return data and never mutate (CQS). `get_day_plan(date)` returns the plan of any status (or `None`); `get_today(date?)` returns only a **committed** plan, defaulting to today.

- [ ] **Step 1: Write the failing tests** *(test designed by the strongest agent)*

`src-tauri/tests/day_plan_queries.rs`:

```rust
use focus_planner_lib::core::plan_service;
use sqlx::SqlitePool;

const TS: &str = "2026-06-09T08:00:00Z";
const DATE: &str = "2030-01-07";

async fn seed_plan_with_task_block(pool: &SqlitePool, status: &str) {
    sqlx::query("INSERT INTO projects (id, name, status, is_archived, created_at, updated_at) VALUES ('proj1', 'P', 'open', 0, ?1, ?1)")
        .bind(TS).execute(pool).await.unwrap();
    sqlx::query("INSERT INTO goals (id, project_id, title, status, is_archived, created_at, updated_at) VALUES ('goal1', 'proj1', 'G', 'open', 0, ?1, ?1)")
        .bind(TS).execute(pool).await.unwrap();
    sqlx::query("INSERT INTO tasks (id, goal_id, title, status, is_archived, created_at, updated_at) VALUES ('task1', 'goal1', 'T', 'open', 0, ?1, ?1)")
        .bind(TS).execute(pool).await.unwrap();
    sqlx::query("INSERT INTO microtasks (id, task_id, title, estimated_minutes, pomodoro_count, status, is_archived, created_at, updated_at) VALUES ('mt1', 'task1', 'Write spec', 20, 1, 'open', 0, ?1, ?1)")
        .bind(TS).execute(pool).await.unwrap();
    sqlx::query("INSERT INTO plans (id, date, status, created_at, updated_at) VALUES ('plan1', ?1, ?2, ?3, ?3)")
        .bind(DATE).bind(status).bind(TS).execute(pool).await.unwrap();
    sqlx::query("INSERT INTO work_blocks (id, plan_id, block_type, microtask_id, start_time, end_time, pomodoro_index, sort_order, created_at, updated_at) VALUES
        ('b1', 'plan1', 'task', 'mt1', '2030-01-07T09:00:00Z', '2030-01-07T09:20:00Z', 1, 0, ?1, ?1),
        ('b2', 'plan1', 'break', NULL, '2030-01-07T09:20:00Z', '2030-01-07T09:25:00Z', NULL, 1, ?1, ?1)")
        .bind(TS).execute(pool).await.unwrap();
}

#[sqlx::test]
async fn get_day_plan_returns_any_status_with_ordered_blocks_and_titles(pool: SqlitePool) {
    seed_plan_with_task_block(&pool, "draft").await;

    let day_plan = plan_service::get_day_plan(&pool, DATE.into())
        .await
        .unwrap()
        .expect("plan exists");
    assert_eq!(day_plan.plan.id, "plan1");
    assert_eq!(day_plan.plan.status, "draft");
    assert_eq!(day_plan.blocks.len(), 2);
    assert_eq!(day_plan.blocks[0].id, "b1");
    assert_eq!(day_plan.blocks[0].microtask_title.as_deref(), Some("Write spec"));
    assert_eq!(day_plan.blocks[0].pomodoro_index, Some(1));
    assert_eq!(day_plan.blocks[1].block_type, "break");
    assert_eq!(day_plan.blocks[1].microtask_title, None);
}

#[sqlx::test]
async fn get_day_plan_returns_none_when_no_plan_exists(pool: SqlitePool) {
    let result = plan_service::get_day_plan(&pool, DATE.into()).await.unwrap();
    assert!(result.is_none());
}

#[sqlx::test]
async fn get_today_returns_only_committed_plans(pool: SqlitePool) {
    seed_plan_with_task_block(&pool, "draft").await;

    let draft = plan_service::get_today(&pool, Some(DATE.into())).await.unwrap();
    assert!(draft.is_none(), "drafts are invisible to get_today");

    sqlx::query("UPDATE plans SET status = 'committed' WHERE id = 'plan1'")
        .execute(&pool).await.unwrap();

    let committed = plan_service::get_today(&pool, Some(DATE.into()))
        .await
        .unwrap()
        .expect("committed plan visible");
    assert_eq!(committed.plan.status, "committed");
    assert_eq!(committed.blocks.len(), 2);
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test day_plan_queries`
Expected: FAIL — `get_day_plan` / `get_today` don't exist.

- [ ] **Step 3: Write `src-tauri/src/models/plan.rs` (+ `pub mod plan;` in `models/mod.rs`)**

Column names exactly per spec §2; serde camelCase per Phase 1 convention:

```rust
use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Plan {
    pub id: String,
    pub date: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkBlockView {
    pub id: String,
    pub plan_id: String,
    pub block_type: String,
    pub microtask_id: Option<String>,
    /// Joined for display; None for breaks/meetings.
    pub microtask_title: Option<String>,
    pub calendar_event_id: Option<String>,
    pub start_time: String,
    pub end_time: String,
    pub pomodoro_index: Option<i64>,
    pub sort_order: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DayPlan {
    pub plan: Plan,
    pub blocks: Vec<WorkBlockView>,
}
```

- [ ] **Step 4: Append the queries to `src-tauri/src/core/plan_service.rs`**

```rust
use crate::models::plan::{DayPlan, Plan, WorkBlockView};

async fn fetch_blocks(pool: &SqlitePool, plan_id: &str) -> Result<Vec<WorkBlockView>, AppError> {
    Ok(sqlx::query_as!(
        WorkBlockView,
        r#"SELECT wb.id as "id!", wb.plan_id as "plan_id!", wb.block_type as "block_type!",
                  wb.microtask_id, m.title as "microtask_title?", wb.calendar_event_id,
                  wb.start_time as "start_time!", wb.end_time as "end_time!",
                  wb.pomodoro_index, wb.sort_order as "sort_order!",
                  wb.created_at as "created_at!", wb.updated_at as "updated_at!"
           FROM work_blocks wb
           LEFT JOIN microtasks m ON m.id = wb.microtask_id
           WHERE wb.plan_id = ?1
           ORDER BY wb.sort_order, wb.start_time"#,
        plan_id
    )
    .fetch_all(pool)
    .await?)
}

/// Query (CQS): the date's plan regardless of status, with ordered blocks.
pub async fn get_day_plan(pool: &SqlitePool, date: String) -> Result<Option<DayPlan>, AppError> {
    let plan = sqlx::query_as!(
        Plan,
        r#"SELECT id as "id!", date as "date!", status as "status!",
                  created_at as "created_at!", updated_at as "updated_at!"
           FROM plans WHERE date = ?1"#,
        date
    )
    .fetch_optional(pool)
    .await?;
    let Some(plan) = plan else { return Ok(None) };
    let blocks = fetch_blocks(pool, &plan.id).await?;
    Ok(Some(DayPlan { plan, blocks }))
}

/// Query (CQS): the committed plan for the date (defaults to today, UTC).
pub async fn get_today(pool: &SqlitePool, date: Option<String>) -> Result<Option<DayPlan>, AppError> {
    let date = date.unwrap_or_else(|| Utc::now().format("%Y-%m-%d").to_string());
    let plan = sqlx::query_as!(
        Plan,
        r#"SELECT id as "id!", date as "date!", status as "status!",
                  created_at as "created_at!", updated_at as "updated_at!"
           FROM plans WHERE date = ?1 AND status = 'committed'"#,
        date
    )
    .fetch_optional(pool)
    .await?;
    let Some(plan) = plan else { return Ok(None) };
    let blocks = fetch_blocks(pool, &plan.id).await?;
    Ok(Some(DayPlan { plan, blocks }))
}
```

- [ ] **Step 5: Refresh the offline cache and run the whole Rust suite**

```bash
cd src-tauri && cargo sqlx prepare && cd ..
cargo test --manifest-path src-tauri/Cargo.toml
```

Expected: all suites pass (planner, settings_service, day_plan_generate, day_plan_edit, day_plan_lifecycle, day_plan_queries, plus Phase 1/2 tests).

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/models src-tauri/src/core/plan_service.rs src-tauri/tests/day_plan_queries.rs src-tauri/.sqlx
git commit -m "feat: Plan/WorkBlock models, get_day_plan and get_today queries"
```

---

### Task 9: IPC command handlers + registration `[easy]`

**Files:**
- Create: `src-tauri/src/commands/plan.rs`
- Modify: `src-tauri/src/commands/mod.rs`, `src-tauri/src/lib.rs`

Thin handlers, all logic already tested at the service layer. Same `#[tracing::instrument]` pattern as Phase 1's `list_projects`: INFO `ok` / ERROR `failed` on exit, payloads skipped, key params as fields (spec §7).

- [ ] **Step 1: Write `src-tauri/src/commands/plan.rs` (+ `pub mod plan;` in `commands/mod.rs`)**

```rust
use crate::core::plan_service;
use crate::db::Db;
use crate::error::AppError;
use crate::models::plan::DayPlan;

fn log_exit(result: &Result<(), AppError>) {
    match result {
        Ok(_) => tracing::info!("ok"),
        Err(e) => tracing::error!(error = %e, "failed"),
    }
}

#[tauri::command]
#[tracing::instrument(skip(db))]
pub async fn generate_day_plan(
    db: tauri::State<'_, Db>,
    plan_id: String,
    date: String,
    strategy: Option<String>,
) -> Result<(), AppError> {
    let result = plan_service::generate_day_plan(&db.0, plan_id, date, strategy).await;
    log_exit(&result);
    result
}

#[tauri::command]
#[tracing::instrument(skip(db))]
pub async fn add_work_block(
    db: tauri::State<'_, Db>,
    id: String,
    plan_id: String,
    block_type: String,
    microtask_id: Option<String>,
    start: String,
    end: String,
) -> Result<(), AppError> {
    let result =
        plan_service::add_work_block(&db.0, id, plan_id, block_type, microtask_id, start, end)
            .await;
    log_exit(&result);
    result
}

#[tauri::command]
#[tracing::instrument(skip(db))]
pub async fn move_work_block(
    db: tauri::State<'_, Db>,
    id: String,
    new_start: String,
    new_end: Option<String>,
) -> Result<(), AppError> {
    let result = plan_service::move_work_block(&db.0, id, new_start, new_end).await;
    log_exit(&result);
    result
}

#[tauri::command]
#[tracing::instrument(skip(db))]
pub async fn remove_work_block(db: tauri::State<'_, Db>, id: String) -> Result<(), AppError> {
    let result = plan_service::remove_work_block(&db.0, id).await;
    log_exit(&result);
    result
}

#[tauri::command]
#[tracing::instrument(skip(db, ordered_ids), fields(count = ordered_ids.len()))]
pub async fn reorder_work_blocks(
    db: tauri::State<'_, Db>,
    plan_id: String,
    ordered_ids: Vec<String>,
) -> Result<(), AppError> {
    let result = plan_service::reorder_work_blocks(&db.0, plan_id, ordered_ids).await;
    log_exit(&result);
    result
}

#[tauri::command]
#[tracing::instrument(skip(db))]
pub async fn commit_day_plan(db: tauri::State<'_, Db>, plan_id: String) -> Result<(), AppError> {
    let result = plan_service::commit_day_plan(&db.0, plan_id).await;
    log_exit(&result);
    result
}

#[tauri::command]
#[tracing::instrument(skip(db))]
pub async fn clear_day_plan(db: tauri::State<'_, Db>, plan_id: String) -> Result<(), AppError> {
    let result = plan_service::clear_day_plan(&db.0, plan_id).await;
    log_exit(&result);
    result
}

#[tauri::command]
#[tracing::instrument(skip(db))]
pub async fn get_day_plan(
    db: tauri::State<'_, Db>,
    date: String,
) -> Result<Option<DayPlan>, AppError> {
    let result = plan_service::get_day_plan(&db.0, date).await;
    match &result {
        Ok(Some(p)) => tracing::info!(plan_id = %p.plan.id, blocks = p.blocks.len(), status = %p.plan.status, "ok"),
        Ok(None) => tracing::info!("ok (no plan for date)"),
        Err(e) => tracing::error!(error = %e, "failed"),
    }
    result
}

#[tauri::command]
#[tracing::instrument(skip(db))]
pub async fn get_today(
    db: tauri::State<'_, Db>,
    date: Option<String>,
) -> Result<Option<DayPlan>, AppError> {
    let result = plan_service::get_today(&db.0, date).await;
    match &result {
        Ok(Some(p)) => tracing::info!(plan_id = %p.plan.id, blocks = p.blocks.len(), "ok"),
        Ok(None) => tracing::info!("ok (no committed plan)"),
        Err(e) => tracing::error!(error = %e, "failed"),
    }
    result
}
```

- [ ] **Step 2: Register the commands in `src-tauri/src/lib.rs`**

Append these lines inside the existing `tauri::generate_handler![ ... ]` list (which already holds Phase 1's and Phase 2's commands — do not remove anything):

```rust
            commands::plan::generate_day_plan,
            commands::plan::add_work_block,
            commands::plan::move_work_block,
            commands::plan::remove_work_block,
            commands::plan::reorder_work_blocks,
            commands::plan::commit_day_plan,
            commands::plan::clear_day_plan,
            commands::plan::get_day_plan,
            commands::plan::get_today,
```

- [ ] **Step 3: Verify the whole backend builds and tests stay green**

Run: `cargo test --manifest-path src-tauri/Cargo.toml`
Expected: all tests pass; no warnings about unused handlers.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/commands src-tauri/src/lib.rs
git commit -m "feat: IPC handlers for all 7 planning commands + 2 plan queries, instrumented"
```

---

### Task 10: TypeScript domain types `[easy]`

**Files:**
- Modify: `src/ipc/types.ts`

- [ ] **Step 1: Append to `src/ipc/types.ts`**

```ts
export interface Plan {
  id: string;
  date: string; // YYYY-MM-DD
  status: "draft" | "committed";
  createdAt: string;
  updatedAt: string;
}

export interface WorkBlock {
  id: string;
  planId: string;
  blockType: "task" | "break" | "meeting";
  microtaskId: string | null;
  microtaskTitle: string | null;
  calendarEventId: string | null;
  startTime: string; // ISO 8601 UTC
  endTime: string; // ISO 8601 UTC
  pomodoroIndex: number | null;
  sortOrder: number;
  createdAt: string;
  updatedAt: string;
}

export interface DayPlan {
  plan: Plan;
  blocks: WorkBlock[];
}
```

- [ ] **Step 2: Verify it typechecks**

Run: `npx vue-tsc --noEmit`
Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add src/ipc/types.ts
git commit -m "feat: Plan, WorkBlock, DayPlan TypeScript types"
```

---

### Task 11: `usePlanStore` `[medium]`

**Files:**
- Create: `src/stores/planStore.ts`
- Test: `src/stores/planStore.test.ts`

CQS at the store level: every mutation action invokes its command, then re-queries `get_day_plan` to refresh state. `reorderBlocks` additionally reorders local state first for visual responsiveness (spec §6).

- [ ] **Step 1: Write the failing tests** *(test designed by the strongest agent)*

`src/stores/planStore.test.ts`:

```ts
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { usePlanStore } from "./planStore";
import type { DayPlan, WorkBlock } from "../ipc/types";

const DATE = "2030-01-07";

function block(
  id: string,
  blockType: WorkBlock["blockType"],
  start: string,
  end: string,
): WorkBlock {
  return {
    id,
    planId: "plan1",
    blockType,
    microtaskId: blockType === "task" ? "mt1" : null,
    microtaskTitle: blockType === "task" ? "Write spec" : null,
    calendarEventId: null,
    startTime: `${DATE}T${start}:00Z`,
    endTime: `${DATE}T${end}:00Z`,
    pomodoroIndex: blockType === "task" ? 1 : null,
    sortOrder: 0,
    createdAt: `${DATE}T08:00:00Z`,
    updatedAt: `${DATE}T08:00:00Z`,
  };
}

function dayPlan(status: "draft" | "committed", blocks: WorkBlock[]): DayPlan {
  return {
    plan: {
      id: "plan1",
      date: DATE,
      status,
      createdAt: `${DATE}T08:00:00Z`,
      updatedAt: `${DATE}T08:00:00Z`,
    },
    blocks,
  };
}

describe("usePlanStore", () => {
  beforeEach(() => setActivePinia(createPinia()));
  afterEach(() => clearMocks());

  it("loadPlan handles a missing plan (null) with an empty state", async () => {
    mockIPC((cmd) => {
      if (cmd === "get_day_plan") return null;
    });
    const store = usePlanStore();
    await store.loadPlan(DATE);
    expect(store.selectedDate).toBe(DATE);
    expect(store.activePlan).toBeNull();
    expect(store.workBlocks).toHaveLength(0);
    expect(store.error).toBeNull();
  });

  it("generatePlan invokes generate_day_plan then re-queries get_day_plan (CQS)", async () => {
    const calls: string[] = [];
    mockIPC((cmd, args) => {
      calls.push(cmd);
      if (cmd === "generate_day_plan") {
        const a = args as Record<string, unknown>;
        expect(a.date).toBe(DATE);
        expect(typeof a.planId).toBe("string");
        return null;
      }
      if (cmd === "get_day_plan") {
        return dayPlan("draft", [
          block("b1", "task", "09:00", "09:20"),
          block("b2", "break", "09:20", "09:25"),
        ]);
      }
    });

    const store = usePlanStore();
    store.selectedDate = DATE;
    await store.generatePlan();

    expect(calls).toEqual(["generate_day_plan", "get_day_plan"]);
    expect(store.activePlan?.status).toBe("draft");
    expect(store.isDraft).toBe(true);
    expect(store.workBlocks).toHaveLength(2);
  });

  it("reorderBlocks sends the full ordered id list and re-queries", async () => {
    const received: string[][] = [];
    const reordered = [block("b2", "break", "09:20", "09:25"), block("b1", "task", "09:00", "09:20")];
    mockIPC((cmd, args) => {
      if (cmd === "reorder_work_blocks") {
        received.push((args as { orderedIds: string[] }).orderedIds);
        return null;
      }
      if (cmd === "get_day_plan") return dayPlan("draft", reordered);
    });

    const store = usePlanStore();
    store.selectedDate = DATE;
    store.activePlan = dayPlan("draft", []).plan;
    store.workBlocks = [block("b1", "task", "09:00", "09:20"), block("b2", "break", "09:20", "09:25")];
    await store.reorderBlocks(["b2", "b1"]);

    expect(received).toEqual([["b2", "b1"]]);
    expect(store.workBlocks.map((b) => b.id)).toEqual(["b2", "b1"]);
  });

  it("addBlock passes blockType and microtaskId through to add_work_block", async () => {
    const payloads: Record<string, unknown>[] = [];
    mockIPC((cmd, args) => {
      if (cmd === "add_work_block") {
        payloads.push(args as Record<string, unknown>);
        return null;
      }
      if (cmd === "get_day_plan") return dayPlan("draft", []);
    });

    const store = usePlanStore();
    store.selectedDate = DATE;
    store.activePlan = dayPlan("draft", []).plan;
    await store.addBlock("meeting", `${DATE}T13:00:00Z`, `${DATE}T14:00:00Z`);
    await store.addBlock("task", `${DATE}T09:00:00Z`, `${DATE}T09:20:00Z`, "mt1");

    expect(payloads[0].blockType).toBe("meeting");
    expect(payloads[0].microtaskId).toBeNull();
    expect(payloads[1].blockType).toBe("task");
    expect(payloads[1].microtaskId).toBe("mt1");
    expect(payloads[1].planId).toBe("plan1");
  });

  it("commitPlan surfaces a backend Validation error and still re-queries", async () => {
    mockIPC((cmd) => {
      if (cmd === "commit_day_plan") {
        throw { code: "validation", message: "plan is already committed" };
      }
      if (cmd === "get_day_plan") return dayPlan("committed", []);
    });

    const store = usePlanStore();
    store.selectedDate = DATE;
    store.activePlan = dayPlan("committed", []).plan;
    await store.commitPlan();

    expect(store.error).toBe("plan is already committed");
    expect(store.activePlan?.status).toBe("committed");
  });
});
```

- [ ] **Step 2: Run to verify it fails**

Run: `npm test -- --run src/stores/planStore.test.ts`
Expected: FAIL — `./planStore` doesn't exist.

- [ ] **Step 3: Write `src/stores/planStore.ts`**

```ts
import { defineStore } from "pinia";
import { ipc } from "../ipc/client";
import type { DayPlan, IpcError, Plan, WorkBlock } from "../ipc/types";

function todayIso(): string {
  return new Date().toISOString().slice(0, 10);
}

export const usePlanStore = defineStore("plan", {
  state: () => ({
    selectedDate: todayIso(),
    activePlan: null as Plan | null,
    workBlocks: [] as WorkBlock[],
    loading: false,
    error: null as string | null,
  }),
  getters: {
    isDraft: (state) => state.activePlan?.status === "draft",
  },
  actions: {
    async loadPlan(date?: string) {
      if (date) this.selectedDate = date;
      this.loading = true;
      this.error = null;
      try {
        const dayPlan = await ipc<DayPlan | null>("get_day_plan", { date: this.selectedDate });
        this.activePlan = dayPlan?.plan ?? null;
        this.workBlocks = dayPlan?.blocks ?? [];
      } catch (e) {
        this.error = (e as IpcError).message ?? String(e);
      } finally {
        this.loading = false;
      }
    },

    // CQS: every mutation below sends a command (no data back) and then
    // re-queries get_day_plan via loadPlan().
    async generatePlan(strategy?: string) {
      await this.mutate("generate_day_plan", {
        planId: crypto.randomUUID(),
        date: this.selectedDate,
        strategy: strategy ?? null,
      });
    },
    async commitPlan() {
      if (!this.activePlan) return;
      await this.mutate("commit_day_plan", { planId: this.activePlan.id });
    },
    async clearPlan() {
      if (!this.activePlan) return;
      await this.mutate("clear_day_plan", { planId: this.activePlan.id });
    },
    async addBlock(
      blockType: WorkBlock["blockType"],
      start: string,
      end: string,
      microtaskId?: string,
    ) {
      if (!this.activePlan) return;
      await this.mutate("add_work_block", {
        id: crypto.randomUUID(),
        planId: this.activePlan.id,
        blockType,
        microtaskId: microtaskId ?? null,
        start,
        end,
      });
    },
    async moveBlock(id: string, newStart: string, newEnd?: string) {
      await this.mutate("move_work_block", { id, newStart, newEnd: newEnd ?? null });
    },
    async removeBlock(id: string) {
      await this.mutate("remove_work_block", { id });
    },
    async reorderBlocks(orderedIds: string[]) {
      if (!this.activePlan) return;
      // Optimistic local reorder for visual responsiveness (spec §6); the
      // re-query inside mutate() reconciles with the backend.
      const byId = new Map(this.workBlocks.map((b) => [b.id, b]));
      this.workBlocks = orderedIds
        .map((id) => byId.get(id))
        .filter((b): b is WorkBlock => b !== undefined);
      await this.mutate("reorder_work_blocks", {
        planId: this.activePlan.id,
        orderedIds,
      });
    },

    async mutate(command: string, args: Record<string, unknown>) {
      this.error = null;
      try {
        await ipc<void>(command, args);
      } catch (e) {
        this.error = (e as IpcError).message ?? String(e);
      }
      await this.loadPlan();
    },
  },
});
```

- [ ] **Step 4: Run to verify it passes**

Run: `npm test -- --run src/stores/planStore.test.ts`
Expected: 5 passing.

- [ ] **Step 5: Commit**

```bash
git add src/stores/planStore.ts src/stores/planStore.test.ts
git commit -m "feat: usePlanStore — mutations re-query after commanding (CQS), optimistic reorder"
```

---

### Task 12: `PlanTimeline` component + drag test `[medium]`

**Files:**
- Create: `src/components/PlanTimeline.vue`
- Test: `src/components/PlanTimeline.test.ts`

The one component test the roadmap budgets for: the timeline's drag logic. Times display by slicing the ISO string (Phase decision 1 — UTC end-to-end, no `Date` conversion).

- [ ] **Step 1: Write the failing test** *(test designed by the strongest agent)*

`src/components/PlanTimeline.test.ts`:

```ts
import { describe, expect, it } from "vitest";
import { mount } from "@vue/test-utils";
import PlanTimeline from "./PlanTimeline.vue";
import type { WorkBlock } from "../ipc/types";

const DATE = "2030-01-07";

function block(
  id: string,
  blockType: WorkBlock["blockType"],
  start: string,
  end: string,
): WorkBlock {
  return {
    id,
    planId: "plan1",
    blockType,
    microtaskId: blockType === "task" ? "mt1" : null,
    microtaskTitle: blockType === "task" ? "Write spec" : null,
    calendarEventId: null,
    startTime: `${DATE}T${start}:00Z`,
    endTime: `${DATE}T${end}:00Z`,
    pomodoroIndex: blockType === "task" ? 1 : null,
    sortOrder: 0,
    createdAt: `${DATE}T08:00:00Z`,
    updatedAt: `${DATE}T08:00:00Z`,
  };
}

const blocks = [
  block("b1", "task", "09:00", "09:20"),
  block("b2", "break", "09:20", "09:25"),
  block("b3", "meeting", "13:00", "14:00"),
];

describe("PlanTimeline", () => {
  it("renders blocks with HH:MM–HH:MM times and labels", () => {
    const wrapper = mount(PlanTimeline, { props: { blocks, editable: true } });
    expect(wrapper.text()).toContain("09:00–09:20");
    expect(wrapper.text()).toContain("Write spec");
    expect(wrapper.text()).toContain("Break");
    expect(wrapper.text()).toContain("Meeting");
  });

  it("emits reorder with the new id order when the draggable model updates", async () => {
    const wrapper = mount(PlanTimeline, { props: { blocks, editable: true } });
    const draggable = wrapper.findComponent({ name: "draggable" });
    draggable.vm.$emit("update:modelValue", [blocks[1], blocks[0], blocks[2]]);
    await wrapper.vm.$nextTick();
    expect(wrapper.emitted("reorder")).toEqual([[["b2", "b1", "b3"]]]);
  });

  it("emits remove with the block id when its delete button is clicked", async () => {
    const wrapper = mount(PlanTimeline, { props: { blocks, editable: true } });
    await wrapper.find("button.remove").trigger("click");
    expect(wrapper.emitted("remove")).toEqual([["b1"]]);
  });

  it("hides delete buttons when not editable", () => {
    const wrapper = mount(PlanTimeline, { props: { blocks, editable: false } });
    expect(wrapper.find("button.remove").exists()).toBe(false);
  });
});
```

- [ ] **Step 2: Run to verify it fails**

Run: `npm test -- --run src/components/PlanTimeline.test.ts`
Expected: FAIL — `./PlanTimeline.vue` doesn't exist.

- [ ] **Step 3: Write `src/components/PlanTimeline.vue`**

```vue
<script setup lang="ts">
import { computed } from "vue";
import draggable from "vuedraggable";
import type { WorkBlock } from "../ipc/types";

const props = defineProps<{ blocks: WorkBlock[]; editable: boolean }>();
const emit = defineEmits<{
  (e: "reorder", orderedIds: string[]): void;
  (e: "remove", id: string): void;
}>();

// v-model proxy: reads from props, drag-drop writes emit the new id order.
const localBlocks = computed({
  get: () => props.blocks,
  set: (value: WorkBlock[]) => emit("reorder", value.map((b) => b.id)),
});

// Phase decision: UTC end-to-end — display HH:MM by slicing the ISO string.
function hhmm(iso: string): string {
  return iso.slice(11, 16);
}

function label(block: WorkBlock): string {
  if (block.blockType === "meeting") return "Meeting";
  if (block.blockType === "break") return "Break";
  return block.microtaskTitle ?? "Task";
}
</script>

<template>
  <draggable
    v-model="localBlocks"
    item-key="id"
    :disabled="!editable"
    ghost-class="ghost"
    class="timeline"
  >
    <template #item="{ element }">
      <div class="block" :class="element.blockType">
        <span class="time">{{ hhmm(element.startTime) }}–{{ hhmm(element.endTime) }}</span>
        <span class="label">
          {{ label(element) }}
          <template v-if="element.pomodoroIndex"> · pomodoro {{ element.pomodoroIndex }}</template>
        </span>
        <button
          v-if="editable"
          class="remove"
          :aria-label="`Remove ${label(element)}`"
          @click="emit('remove', element.id)"
        >
          ×
        </button>
      </div>
    </template>
  </draggable>
</template>

<style scoped>
.timeline { display: flex; flex-direction: column; gap: 6px; margin: 16px 0; }
.block {
  display: flex; align-items: center; gap: 12px;
  padding: 10px 14px; border-radius: 8px; background: #181d24;
  border-left: 3px solid #3b82f6;
}
.block.break { border-left-color: #5fd068; opacity: 0.8; }
.block.meeting { border-left-color: #e6b450; }
.block .time { font-variant-numeric: tabular-nums; color: #9aa3b2; min-width: 100px; }
.block .label { flex: 1; }
.block .remove {
  background: none; border: none; color: #9aa3b2; cursor: pointer;
  font-size: 16px; padding: 2px 8px; border-radius: 4px;
}
.block .remove:hover { background: #2a1518; color: #ff6b6b; }
.ghost { opacity: 0.4; }
</style>
```

- [ ] **Step 4: Run to verify it passes**

Run: `npm test -- --run src/components/PlanTimeline.test.ts`
Expected: 4 passing.

- [ ] **Step 5: Commit**

```bash
git add src/components/PlanTimeline.vue src/components/PlanTimeline.test.ts
git commit -m "feat: PlanTimeline with vuedraggable reorder, delete buttons, drag-logic tests"
```

---

### Task 13: `AddBlockPanel` component (backlog picker + manual meeting) `[medium]`

**Files:**
- Create: `src/components/AddBlockPanel.vue`

Per the roadmap's testing strategy, component tests are reserved for the timeline's drag logic; this panel is covered by the store tests (payload shapes) and manual QA (Task 16). It reuses Phase 2's `useProjectStore` (`loadProjects`, `loadProjectTree`, `activeProjectTree`). **Amendment note:** if Phase 2's `ProjectTree` field names differ from `goals[].tasks[].microtasks[]` with `title`/`status`/`isArchived`, adjust only the `microtaskOptions` computed and amend this plan (roadmap convention 1).

- [ ] **Step 1: Write `src/components/AddBlockPanel.vue`**

```vue
<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { useProjectStore } from "../stores/projectStore";

const props = defineProps<{ date: string }>();
const emit = defineEmits<{
  (e: "add-task", microtaskId: string, start: string, end: string): void;
  (e: "add-meeting", start: string, end: string): void;
}>();

const projectStore = useProjectStore();
projectStore.loadProjects();

const selectedProjectId = ref("");
const selectedMicrotaskId = ref("");
const taskStart = ref("09:00");
const taskEnd = ref("09:20");
const meetingStart = ref("13:00");
const meetingEnd = ref("14:00");

watch(selectedProjectId, (id) => {
  selectedMicrotaskId.value = "";
  if (id) projectStore.loadProjectTree(id);
});

// Open, unarchived microtasks of the selected project, flattened for a <select>.
const microtaskOptions = computed(() => {
  const tree = projectStore.activeProjectTree;
  if (!tree || tree.id !== selectedProjectId.value) return [];
  return tree.goals.flatMap((goal) =>
    goal.tasks.flatMap((task) =>
      task.microtasks
        .filter((m) => m.status === "open" && !m.isArchived)
        .map((m) => ({ id: m.id, label: `${task.title} › ${m.title}` })),
    ),
  );
});

// Phase decision: UTC end-to-end — HH:MM inputs become <date>THH:MM:00Z.
function iso(hhmmValue: string): string {
  return `${props.date}T${hhmmValue}:00Z`;
}

function submitTask() {
  if (!selectedMicrotaskId.value) return;
  emit("add-task", selectedMicrotaskId.value, iso(taskStart.value), iso(taskEnd.value));
}

function submitMeeting() {
  emit("add-meeting", iso(meetingStart.value), iso(meetingEnd.value));
}
</script>

<template>
  <div class="add-panel">
    <fieldset>
      <legend>Add work block from backlog</legend>
      <select v-model="selectedProjectId" aria-label="Project">
        <option value="" disabled>Project…</option>
        <option v-for="p in projectStore.projects" :key="p.id" :value="p.id">
          {{ p.name }}
        </option>
      </select>
      <select
        v-model="selectedMicrotaskId"
        :disabled="microtaskOptions.length === 0"
        aria-label="Microtask"
      >
        <option value="" disabled>Microtask…</option>
        <option v-for="m in microtaskOptions" :key="m.id" :value="m.id">
          {{ m.label }}
        </option>
      </select>
      <input v-model="taskStart" type="time" aria-label="Work block start" />
      <input v-model="taskEnd" type="time" aria-label="Work block end" />
      <button :disabled="!selectedMicrotaskId" @click="submitTask">Add block</button>
    </fieldset>

    <fieldset>
      <legend>Add meeting</legend>
      <input v-model="meetingStart" type="time" aria-label="Meeting start" />
      <input v-model="meetingEnd" type="time" aria-label="Meeting end" />
      <button @click="submitMeeting">Add meeting</button>
    </fieldset>
  </div>
</template>

<style scoped>
.add-panel { display: flex; gap: 16px; flex-wrap: wrap; }
.add-panel fieldset {
  display: flex; gap: 8px; align-items: center;
  border: 1px solid #20242b; border-radius: 8px; padding: 12px;
}
.add-panel legend { color: #9aa3b2; font-size: 12px; padding: 0 6px; }
</style>
```

Note: meeting blocks have no title column in spec §2 (`calendar_event_id` arrives in M4) — meetings display as "Meeting 13:00–14:00" in the timeline. Intentional, not an omission.

- [ ] **Step 2: Verify it typechecks**

Run: `npx vue-tsc --noEmit`
Expected: no errors (if `ProjectTree` field names differ, fix `microtaskOptions` per the amendment note above).

- [ ] **Step 3: Commit**

```bash
git add src/components/AddBlockPanel.vue
git commit -m "feat: AddBlockPanel — backlog microtask picker and manual meeting form"
```

---

### Task 14: Day View — wire it all together `[medium]`

**Files:**
- Modify: `src/views/DayView.vue` (replace the Phase 1 placeholder)

Generate button with date picker, status badge, timeline with drag-reorder and delete (draft only), add panel (draft only), Commit/Clear buttons, and the runtime controls row — visible but disabled until Phase 4.

- [ ] **Step 1: Rewrite `src/views/DayView.vue`**

```vue
<script setup lang="ts">
import { onMounted } from "vue";
import { usePlanStore } from "../stores/planStore";
import PlanTimeline from "../components/PlanTimeline.vue";
import AddBlockPanel from "../components/AddBlockPanel.vue";

const store = usePlanStore();
onMounted(() => store.loadPlan());

function onDateChange(event: Event) {
  store.loadPlan((event.target as HTMLInputElement).value);
}
</script>

<template>
  <section class="day-view">
    <header class="day-header">
      <h1>Day</h1>
      <input
        type="date"
        :value="store.selectedDate"
        aria-label="Plan date"
        @change="onDateChange"
      />
      <button @click="store.generatePlan()">
        {{ store.activePlan ? "Regenerate" : "Generate plan" }}
      </button>
      <template v-if="store.activePlan">
        <span class="status" :class="store.activePlan.status">{{ store.activePlan.status }}</span>
        <button
          v-if="store.isDraft"
          :disabled="store.workBlocks.length === 0"
          @click="store.commitPlan()"
        >
          Commit
        </button>
        <button v-if="store.isDraft" class="danger" @click="store.clearPlan()">Clear</button>
      </template>
    </header>

    <!-- Runtime controls: visible but disabled until Phase 4's Start Day engine. -->
    <div class="runtime-controls">
      <button disabled title="Available in Phase 4 (Start Day runtime)">Start Day</button>
      <button disabled title="Available in Phase 4 (Start Day runtime)">Pause</button>
      <button disabled title="Available in Phase 4 (Start Day runtime)">Resume</button>
      <button disabled title="Available in Phase 4 (Start Day runtime)">Skip</button>
      <button disabled title="Available in Phase 4 (Start Day runtime)">End Day</button>
    </div>

    <p v-if="store.error" class="error">{{ store.error }}</p>
    <p v-if="store.loading">Loading…</p>

    <p v-else-if="!store.activePlan" class="placeholder">
      No plan for {{ store.selectedDate }} yet. Generate one — open backlog microtasks are
      scheduled into the planning window as pomodoro + break blocks. You can then add meetings
      and regenerate to plan around them.
    </p>

    <template v-else>
      <PlanTimeline
        :blocks="store.workBlocks"
        :editable="store.isDraft"
        @reorder="store.reorderBlocks($event)"
        @remove="store.removeBlock($event)"
      />
      <AddBlockPanel
        v-if="store.isDraft"
        :date="store.selectedDate"
        @add-task="(microtaskId, start, end) => store.addBlock('task', start, end, microtaskId)"
        @add-meeting="(start, end) => store.addBlock('meeting', start, end)"
      />
    </template>
  </section>
</template>

<style scoped>
.day-header { display: flex; align-items: center; gap: 12px; flex-wrap: wrap; }
.runtime-controls { display: flex; gap: 8px; margin: 16px 0; opacity: 0.5; }
.status { padding: 2px 10px; border-radius: 999px; font-size: 12px; text-transform: uppercase; }
.status.draft { background: #2b2415; color: #e6b450; }
.status.committed { background: #15291a; color: #5fd068; }
.error { color: #ff6b6b; }
button.danger { color: #ff6b6b; }
</style>
```

- [ ] **Step 2: Typecheck and run the full frontend suite**

Run: `npx vue-tsc --noEmit && npm test -- --run`
Expected: no type errors; all vitest suites pass.

- [ ] **Step 3: Verify end-to-end in the app**

Run: `npm run tauri dev`
Expected: Day view shows the date picker and disabled runtime controls. With backlog microtasks present (create some via the Phase 2 Backlog view if the dev DB is empty), pick a date → Generate → work/break blocks appear from 09:00; add a meeting → Regenerate → blocks flow around it; drag a block → order persists after switching views and back; delete a block; Commit → status flips to committed, editing controls disappear.

- [ ] **Step 4: Verify the log narrative (spec §7)**

Run: `cat logs/focus-planner.log.$(date +%Y-%m-%d)`
Expected, readable as a story: `generate_day_plan` entry with plan_id/date → `planner: input summary` (N microtasks, window, M meetings) → one `planner: placed work block …`/`planner: pair didn't fit gap ending …, moved to next gap` line per decision → `day plan generated as draft` → `get_day_plan … ok` → on commit: `day plan committed`.

- [ ] **Step 5: Commit**

```bash
git add src/views/DayView.vue
git commit -m "feat: Day View draft-mode timeline — generate, edit, commit; runtime controls disabled"
```

---

### Task 15: Docs — README refresh, no migration history change `[easy]`

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Update `README.md`**

Update the "current state" section: Phase 3 done — day planning works end-to-end (generate a draft plan from backlog microtasks into the 09:00–17:00 window, edit it, add meetings, commit it; Start Day arrives in Phase 4). Add a three-line "Plan your day" usage blurb under the existing run instructions (Backlog → create microtasks; Day → pick date → Generate → Commit). Keep the five-minute-newcomer bar.

- [ ] **Step 2: Confirm `docs/db-context/migration-history.md` needs NO entry**

Phase 3 adds **no migration** — the schema from migration 0001 already covers plans, work_blocks, and settings. Verify:

```bash
ls src-tauri/migrations
```

Expected: the same files Phase 2 left behind, nothing new. **If you added a migration during this phase, something went off-plan: stop, write the lesson in `docs/lessons/`, amend this plan, and only then append to migration-history.**

- [ ] **Step 3: Commit**

```bash
git add README.md
git commit -m "docs: README — Phase 3 day planning state and usage"
```

---

### Task 16: Phase acceptance — manual QA checklist `[trivial]`

Run on a fresh `./scripts/setup-db.sh && npm install && npm run tauri dev` build, with a backlog of at least: one microtask with 3 pomodoros, one with priority 5, one with a non-default pomodoro type that has `long_break_every` set, plus one completed and one archived microtask.

- [ ] Generate for a future date → draft plan appears; blocks start at 09:00; the 3-pomodoro microtask yields 3 work blocks each followed by a break; the priority-5 microtask is scheduled first; completed/archived microtasks are absent
- [ ] The long-break type's Nth work block is followed by the long break (longer than its normal rest)
- [ ] Add a manual meeting → Regenerate → meeting still there at its fixed time; work/break pairs flow around it; a pair that no longer fits before the meeting lands after it
- [ ] Regenerate replaces the draft (no duplicate blocks); `sqlite3` against the app-data DB shows exactly one row in `plans` for the date
- [ ] Drag-reorder two blocks → switch view and back → order persisted; restart the app → still persisted
- [ ] Add a work block from the backlog picker (project → microtask → times) → appears in the timeline with the microtask's title
- [ ] Delete a block → gone, stays gone after reload
- [ ] Commit → status badge flips to committed; drag/delete/add panel disappear; Clear button gone
- [ ] With the plan committed, Regenerate shows the Validation error in the UI ("already committed") — and the same error is WARN-logged
- [ ] Clear a fresh draft → plan gone, empty state returns, Generate works again
- [ ] Runtime controls (Start Day, Pause, Resume, Skip, End Day) are visible, disabled, with the Phase 4 tooltip
- [ ] **Logging bar (spec §7 / PHILOSOPHY.md):** generate and commit a plan, then read `logs/focus-planner.log.<today>` — a junior with zero context can follow every placement decision: input summary → each "placed work block …" / "pair didn't fit gap ending …, moved to next gap" → "day plan generated as draft" → "day plan committed"
- [ ] `cargo test --manifest-path src-tauri/Cargo.toml` and `npm test -- --run` green locally; CI green on the PR
- [ ] README five-minute test still holds for a newcomer

---

### Task 17: Self-review pass `[trivial]`

Before opening the PR, verify the plan's own promises landed:

- [ ] **Scope coverage** — every spec §3 Planning command has a service + handler + test: `generate_day_plan` (T5/T9), `add_work_block` (T6/T9), `move_work_block` (T6/T9), `remove_work_block` (T6/T9), `reorder_work_blocks` (T7/T9), `commit_day_plan` (T7/T9), `clear_day_plan` (T7/T9); queries `get_day_plan` + `get_today` (T8/T9); planner §4 (T2–T3); Day View + `usePlanStore` §6 (T11–T14)
- [ ] **Placeholder scan** — `grep -nE "TBD|TODO|FIXME|similar to Task" docs/superpowers/plans/2026-06-09-m1-phase-3-day-planning.md src-tauri/src/core/planner.rs src-tauri/src/core/plan_service.rs` returns nothing
- [ ] **Convention consistency with Phase 1** — errors are `AppError::{Db, NotFound, Validation}` only; all frontend IPC goes through `ipc<T>()`; commands take `tauri::State<'_, Db>`; models serialize camelCase; `.sqlx/` cache is committed and CI passes with `SQLX_OFFLINE=true`
- [ ] **Spec §2 column-name consistency** — code uses exactly `plans(id, date, status, created_at, updated_at)` and `work_blocks(id, plan_id, block_type, microtask_id, calendar_event_id, start_time, end_time, pomodoro_index, sort_order, created_at, updated_at)`; block_type values are `task|break|meeting`; plan status values are `draft|committed`
- [ ] **CQS** — all seven mutation handlers return `Result<(), AppError>`; the two query handlers return data; no store action reads data from a mutation response
- [ ] **No schema change shipped** (Phase decision 9) — `git diff main --stat -- src-tauri/migrations` is empty
