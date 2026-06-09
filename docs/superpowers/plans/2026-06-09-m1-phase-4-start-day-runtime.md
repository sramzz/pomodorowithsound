# M1 Phase 4 — Start Day Runtime Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The product moment: press Start Day on a committed plan and the Rust tokio engine runs the day — 1-second ticks streamed to the webview, the 5-minute warning, the work-end sound + notification, auto-advance through breaks, incremental `PomodoroSession` writes, a `FocusSession` on End Day — with the whole run readable as a narrative in `logs/`.

**Architecture:** One tokio task (the **actor**) exclusively owns `RuntimeState` — no shared `Mutex`. IPC commands send `RuntimeCmd` messages through an `mpsc::Sender` held in Tauri managed state and await a `oneshot` reply carrying only `Result<(), AppError>` (CQS: state flows back via `runtime-tick` events and `get_run_status`). The actor `select!`s over the command channel and a 1-second `tokio::time::interval`. The engine lives in `core/runtime_service.rs` and is **Tauri-free**: it takes `Arc<dyn SoundPlayer>`, `Arc<dyn Notifier>`, and `Arc<dyn TickSink>` instead of an `AppHandle`, so deterministic tests drive it with `tokio::time::pause()`/`advance()` and recording fakes. Thin adapters in `src/platform.rs` (rodio, tauri-plugin-notification, `app.emit`) bind it to the real world.

**Tech Stack:** Everything from Phase 1 (Tauri 2, Vue 3, Pinia, Vitest, SQLx 0.8, tracing) plus: `rodio` 0.19 (synthesized SineWave tones, no assets), `tauri-plugin-notification` 2, `uuid` (v4, session row ids), `chrono` (ISO timestamps, block durations), tokio `test-util` (dev-dependency, paused-time tests).

**Conventions (per `docs/specs/m1-roadmap.md`):** every task carries a difficulty tag (`[trivial]`/`[easy]`/`[medium]`/`[hard]`). TDD role split: the failing test of each TDD task is **designed by the most capable agent**; implementation may be assigned by difficulty (cheaper agents take `[trivial]`/`[easy]`); **every task is reviewed before its commit lands**. The actor + state machine (Tasks 5–9) is the `[hard]` core — do not parallelize those five tasks; they build on each other in order.

**Philosophy (PHILOSOPHY.md):** CQS — all runtime commands return `Result<(), AppError>`, only `get_run_status` returns data. Logging per spec §7 — this phase is the showcase: every state transition at INFO with from/to/block id/reason, every sound and notification fired at INFO, every session row written at INFO, per-second ticks at TRACE only. KISS — no extra crates beyond the list above, no persistence of live runtime state (crash semantics below).

---

## Key design decisions (read before implementing)

1. **State machine** (spec §4 mermaid): `Idle → WorkRunning ↔ WorkPaused`; `WorkRunning → BreakRunning` (timer end or `complete_current_block`) `↔ BreakPaused`; `BreakRunning → WorkRunning` (timer end or `skip_break`); any non-Idle → `Idle` via `end_day`. The actor serializes all transitions, eliminating the timer-expiry-vs-user-command race.
2. **Biased select, interval first:** the actor's `select!` is `biased;` with the interval arm first — a due tick is always processed before a simultaneous user command, so the race resolves the same way every time (and tests stay deterministic). Interval keeps the default `MissedTickBehavior::Burst`: no tick is ever lost. Accepted M1 quirk: after OS sleep the countdown catches up in a burst (note for M2).
3. **Block types at runtime:** `break` blocks run in Break mode; `task` **and** `meeting` blocks run in Work mode (the spec machine has only Work/Break). Only `task` blocks ever write `pomodoro_sessions` rows or count toward `blocks_completed`.
4. **Durations** come from each block's planned `end_time - start_time`, counted down tick by tick — the runtime is a countdown sequencer, not a wall-clock scheduler. `total_work_seconds`/`total_break_seconds` accumulate from counted ticks (deterministic under paused time); row timestamps use real `Utc::now()`.
5. **`was_completed` semantics:** timer end and `complete_current_block` write `was_completed = 1`; `skip_to_next_block` on a work block writes an audit row with `was_completed = 0`. `end_day` mid-block discards the partial block (no row).
6. **Microtask completion:** a microtask completes when **no pending** (not done, not skipped) block still references it, triggered on block *completion* — delegated to Phase 2's `microtask_service::complete_microtask` so the task→goal roll-up applies for free. A skipped block does not block completion (the user chose to move on). "Finish microtask early" = `complete_current_block` also skips that microtask's remaining work blocks **and each one's trailing break**.
7. **FocusSession linking:** `pomodoro_sessions` are written during the run with `focus_session_id = NULL` (the FK parent doesn't exist yet); `end_day` inserts the `focus_sessions` row, then links this run's session ids to it. **Crash semantics (accepted for M1):** live state lost, plan stays `committed`, written PomodoroSessions survive as unlinked rows.
8. **Counts:** `blocks_completed` = task blocks completed; `blocks_skipped` = any block skipped (work skips, break skips, finish-early skips).
9. **Auto-end:** when the last pending block finishes, the engine ends the day automatically (same path as `end_day`, reason `all_blocks_finished`).
10. **Spec §2 deviation (flagged):** `RuntimeState.active_plan_id` is `Option<String>` (spec shows `String`) — Idle has no plan and `null` on the wire is honest (POLA). Field names otherwise exactly as spec §2; serde camelCase on the wire per Phase 1 convention.
11. **Phase 2/3 dependencies assumed:** `core::microtask_service::complete_microtask(&pool, id) -> Result<(), AppError>` (Phase 2) and committed plans with ordered `work_blocks` (Phase 3). If execution of those phases changed a name or signature, adapt the single call site and **amend this plan + record a lesson** per roadmap convention.

---

### Task 1: Branch + preflight `[trivial]`

- [ ] **Step 1: Branch off main**

```bash
git checkout main && git pull && git checkout -b feat/m1-phase-4-start-day-runtime
```

- [ ] **Step 2: Verify both suites are green before touching anything**

Run: `cargo test --manifest-path src-tauri/Cargo.toml && npm test -- --run`
Expected: all Phase 1–3 tests pass. If not, stop and fix on main first.

- [ ] **Step 3: Verify Phase 3 left a committable plan path**

Run: `sqlite3 src-tauri/.dev/dev.sqlite "SELECT count(*) FROM work_blocks;"`
Expected: a number (any). This confirms the schema + dev DB are intact; actual committed-plan fixtures are created inside the tests.

---

### Task 2: Dependencies + notification plugin registration `[easy]`

**Files:**
- Modify: `src-tauri/Cargo.toml`, `src-tauri/src/lib.rs`

- [ ] **Step 1: Add to `[dependencies]` in `src-tauri/Cargo.toml`**

```toml
rodio = "0.19"
uuid = { version = "1", features = ["v4"] }
chrono = "0.4"
tauri-plugin-notification = "2"
```

- [ ] **Step 2: Add the dev-dependency for paused-time tests**

```toml
[dev-dependencies]
tokio = { version = "1", features = ["test-util", "macros"] }
```

(Cargo unions features: the main `tokio` dependency stays as-is; tests gain `test-util`.)

- [ ] **Step 3: Register the notification plugin in `src-tauri/src/lib.rs`**

In `run()`, on the builder chain (before `.setup(...)`):

```rust
        .plugin(tauri_plugin_notification::init())
```

No webview capability is needed — notifications are triggered from Rust only (spec §1: routing through the webview would reintroduce OS throttling).

- [ ] **Step 4: Verify it compiles**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`
Expected: success.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/src/lib.rs
git commit -m "chore: add rodio, uuid, chrono, notification plugin, tokio test-util for Phase 4"
```

---

### Task 3: Extend AppError with `Internal` `[easy]`

The IPC layer needs an error for "the runtime actor is unreachable" — neither Db, NotFound, nor Validation fits. This extends Phase 1's enum; the wire shape (`{ code, message }`) is unchanged.

**Files:**
- Modify: `src-tauri/src/error.rs`, `src/ipc/types.ts`

- [ ] **Step 1: Add the variant to `AppError` in `src-tauri/src/error.rs`**

```rust
    #[error("internal error: {0}")]
    Internal(String),
```

And the matching arm in the `Serialize` impl's `code` match:

```rust
            AppError::Internal(_) => "internal",
```

- [ ] **Step 2: Add the code to the TS union in `src/ipc/types.ts`**

```ts
export interface IpcError {
  code: "db" | "not_found" | "validation" | "internal";
  message: string;
}
```

- [ ] **Step 3: Verify**

Run: `cargo check --manifest-path src-tauri/Cargo.toml && npx vue-tsc --noEmit`
Expected: both succeed.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/error.rs src/ipc/types.ts
git commit -m "feat: AppError::Internal for unreachable-runtime failures"
```

---

### Task 4: RuntimeState + RuntimeMode models `[easy]`

The wire contract for `runtime-tick` and `get_run_status`. The serde test and the type are written together (a pure data shape can't fail-compile-first); the test pins the camelCase wire format the frontend depends on.

**Files:**
- Create: `src-tauri/src/models/runtime.rs`
- Modify: `src-tauri/src/models/mod.rs`

- [ ] **Step 1: Write `src-tauri/src/models/runtime.rs`** *(test designed by the strongest agent)*

```rust
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum RuntimeMode {
    Work,
    Break,
    Idle,
}

/// In-memory only (spec §2) — never persisted; lost on crash by design.
/// `active_plan_id` is Option (deliberate refinement of spec §2): Idle has no plan.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeState {
    pub active_plan_id: Option<String>,
    pub current_block_id: Option<String>,
    pub timer_seconds_remaining: u32,
    pub is_running: bool,
    pub mode: RuntimeMode,
    pub start_time: Option<String>,
}

impl RuntimeState {
    pub fn idle() -> Self {
        Self {
            active_plan_id: None,
            current_block_id: None,
            timer_seconds_remaining: 0,
            is_running: false,
            mode: RuntimeMode::Idle,
            start_time: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_state_serializes_camel_case_for_the_wire() {
        let s = RuntimeState {
            active_plan_id: Some("p1".into()),
            current_block_id: Some("b1".into()),
            timer_seconds_remaining: 1200,
            is_running: true,
            mode: RuntimeMode::Work,
            start_time: Some("2026-06-09T09:00:00Z".into()),
        };
        let json = serde_json::to_value(&s).unwrap();
        assert_eq!(json["activePlanId"], "p1");
        assert_eq!(json["currentBlockId"], "b1");
        assert_eq!(json["timerSecondsRemaining"], 1200);
        assert_eq!(json["isRunning"], true);
        assert_eq!(json["mode"], "work");
        assert_eq!(json["startTime"], "2026-06-09T09:00:00Z");
    }

    #[test]
    fn idle_state_serializes_nulls_and_idle_mode() {
        let json = serde_json::to_value(RuntimeState::idle()).unwrap();
        assert_eq!(json["activePlanId"], serde_json::Value::Null);
        assert_eq!(json["mode"], "idle");
        assert_eq!(json["isRunning"], false);
    }
}
```

- [ ] **Step 2: Register the module**

In `src-tauri/src/models/mod.rs` add:

```rust
pub mod runtime;
```

- [ ] **Step 3: Run the tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml runtime_state`
Expected: PASS — 2 tests (`runtime_state_serializes_camel_case_for_the_wire`, plus the idle test matches on substring `runtime_state` only for the first; if filtering misses, run the full suite — both must pass).

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/models
git commit -m "feat: RuntimeState + RuntimeMode wire models with camelCase serde contract"
```

---

### Task 5: Engine part 1 — actor, ports, `start_day`, pause/resume, `get_status` `[hard]`

The Tauri-free core. This task establishes the file, the three ports (traits), the message enum, the actor loop (command-only for now — the interval arrives in Task 6), and the `Idle → WorkRunning` transition with full validation.

**Files:**
- Create: `src-tauri/src/core/runtime_service.rs`
- Modify: `src-tauri/src/core/mod.rs`
- Test: `src-tauri/tests/runtime_engine.rs`

- [ ] **Step 1: Write the failing tests** *(test designed by the strongest agent)*

`src-tauri/tests/runtime_engine.rs` — the harness used by every engine test in this phase. `max_connections(1)` is mandatory (each in-memory SQLite connection is its own database) and `idle_timeout(None)`/`max_lifetime(None)` are mandatory too: tests advance hours of *virtual* time, and the pool reaper runs on tokio time — default timeouts would close the only connection and wipe the DB mid-test.

```rust
use std::sync::{Arc, Mutex};
use std::time::Duration;

use focus_planner_lib::core::runtime_service::{
    Notifier, RuntimeCmd, RuntimeEngine, SoundPlayer, TickSink,
};
use focus_planner_lib::error::AppError;
use focus_planner_lib::models::runtime::{RuntimeMode, RuntimeState};
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::SqlitePool;
use tokio::sync::{mpsc, oneshot};

const T: &str = "2026-06-09T08:00:00Z";

// ---------- Recording fakes (the engine never touches Tauri in tests) ----------

#[derive(Default)]
struct Recorder {
    sounds: Mutex<Vec<String>>,
    notifications: Mutex<Vec<(String, String)>>,
    ticks: Mutex<Vec<RuntimeState>>,
}

impl SoundPlayer for Recorder {
    fn play_work_end(&self) {
        self.sounds.lock().unwrap().push("work_end".into());
    }
    fn play_break_end(&self) {
        self.sounds.lock().unwrap().push("break_end".into());
    }
}

impl Notifier for Recorder {
    fn notify(&self, title: &str, body: &str) {
        self.notifications.lock().unwrap().push((title.into(), body.into()));
    }
}

impl TickSink for Recorder {
    fn emit(&self, state: &RuntimeState) {
        self.ticks.lock().unwrap().push(state.clone());
    }
}

// ---------- Harness ----------

struct Harness {
    pool: SqlitePool,
    tx: mpsc::Sender<RuntimeCmd>,
    rec: Arc<Recorder>,
}

async fn harness() -> Harness {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .min_connections(1)
        .idle_timeout(None)
        .max_lifetime(None)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    let rec = Arc::new(Recorder::default());
    let engine = RuntimeEngine::new(pool.clone(), rec.clone(), rec.clone(), rec.clone());
    let (tx, rx) = mpsc::channel(32);
    tokio::spawn(engine.run_actor(rx));
    Harness { pool, tx, rec }
}

impl Harness {
    async fn send(
        &self,
        make: impl FnOnce(oneshot::Sender<Result<(), AppError>>) -> RuntimeCmd,
    ) -> Result<(), AppError> {
        let (otx, orx) = oneshot::channel();
        self.tx.send(make(otx)).await.unwrap();
        orx.await.unwrap()
    }

    async fn status(&self) -> RuntimeState {
        let (otx, orx) = oneshot::channel();
        self.tx.send(RuntimeCmd::GetStatus { reply: otx }).await.unwrap();
        orx.await.unwrap()
    }
}

// ---------- Fixture: a committed plan — micro1 has 2 pomodoros (Standard 20/5) ----------
// Blocks in sort order: b-work1 (20m) -> b-break1 (5m) -> b-work2 (20m) -> b-break2 (5m)

async fn seed_committed_plan(pool: &SqlitePool) {
    for sql in [
        "INSERT INTO projects (id, name, status, is_archived, created_at, updated_at) VALUES ('proj1','P','open',0,?1,?1)",
        "INSERT INTO goals (id, project_id, title, status, is_archived, created_at, updated_at) VALUES ('goal1','proj1','G','open',0,?1,?1)",
        "INSERT INTO tasks (id, goal_id, title, status, is_archived, created_at, updated_at) VALUES ('task1','goal1','T','open',0,?1,?1)",
        "INSERT INTO microtasks (id, task_id, title, estimated_minutes, pomodoro_count, pomodoro_type_id, status, is_archived, created_at, updated_at) VALUES ('micro1','task1','M',40,2,'a0000000-0000-4000-8000-000000000001','open',0,?1,?1)",
        "INSERT INTO plans (id, date, status, created_at, updated_at) VALUES ('plan1','2026-06-09','committed',?1,?1)",
        "INSERT INTO work_blocks (id, plan_id, block_type, microtask_id, start_time, end_time, pomodoro_index, sort_order, created_at, updated_at) VALUES ('b-work1','plan1','task','micro1','2026-06-09T09:00:00Z','2026-06-09T09:20:00Z',1,0,?1,?1)",
        "INSERT INTO work_blocks (id, plan_id, block_type, microtask_id, start_time, end_time, pomodoro_index, sort_order, created_at, updated_at) VALUES ('b-break1','plan1','break',NULL,'2026-06-09T09:20:00Z','2026-06-09T09:25:00Z',NULL,1,?1,?1)",
        "INSERT INTO work_blocks (id, plan_id, block_type, microtask_id, start_time, end_time, pomodoro_index, sort_order, created_at, updated_at) VALUES ('b-work2','plan1','task','micro1','2026-06-09T09:25:00Z','2026-06-09T09:45:00Z',2,2,?1,?1)",
        "INSERT INTO work_blocks (id, plan_id, block_type, microtask_id, start_time, end_time, pomodoro_index, sort_order, created_at, updated_at) VALUES ('b-break2','plan1','break',NULL,'2026-06-09T09:45:00Z','2026-06-09T09:50:00Z',NULL,3,?1,?1)",
    ] {
        sqlx::query(sql).bind(T).execute(pool).await.unwrap();
    }
}

// ---------- Task 5 tests ----------

#[tokio::test(start_paused = true)]
async fn start_day_on_a_draft_plan_is_a_validation_error() {
    let h = harness().await;
    sqlx::query("INSERT INTO plans (id, date, status, created_at, updated_at) VALUES ('plan-draft','2026-06-10','draft',?1,?1)")
        .bind(T)
        .execute(&h.pool)
        .await
        .unwrap();

    let err = h
        .send(|reply| RuntimeCmd::StartDay { plan_id: "plan-draft".into(), reply })
        .await
        .unwrap_err();

    assert!(matches!(err, AppError::Validation(_)), "got: {err:?}");
    assert_eq!(h.status().await.mode, RuntimeMode::Idle);
}

#[tokio::test(start_paused = true)]
async fn start_day_on_a_committed_plan_enters_work_running_and_emits_a_tick() {
    let h = harness().await;
    seed_committed_plan(&h.pool).await;

    h.send(|reply| RuntimeCmd::StartDay { plan_id: "plan1".into(), reply })
        .await
        .unwrap();

    let s = h.status().await;
    assert_eq!(s.mode, RuntimeMode::Work);
    assert!(s.is_running);
    assert_eq!(s.active_plan_id.as_deref(), Some("plan1"));
    assert_eq!(s.current_block_id.as_deref(), Some("b-work1"));
    assert_eq!(s.timer_seconds_remaining, 1200); // 09:00 -> 09:20 = 20 min
    assert!(s.start_time.is_some());

    // CQS: the webview learns about the new state through the sink, not the reply.
    let ticks = h.rec.ticks.lock().unwrap();
    assert!(ticks
        .iter()
        .any(|t| t.mode == RuntimeMode::Work && t.current_block_id.as_deref() == Some("b-work1")));
}

#[tokio::test(start_paused = true)]
async fn start_day_while_a_day_is_running_is_rejected() {
    let h = harness().await;
    seed_committed_plan(&h.pool).await;
    h.send(|reply| RuntimeCmd::StartDay { plan_id: "plan1".into(), reply }).await.unwrap();

    let err = h
        .send(|reply| RuntimeCmd::StartDay { plan_id: "plan1".into(), reply })
        .await
        .unwrap_err();
    assert!(matches!(err, AppError::Validation(_)));
}

#[tokio::test(start_paused = true)]
async fn pause_day_when_idle_is_a_validation_error() {
    let h = harness().await;
    let err = h.send(|reply| RuntimeCmd::PauseDay { reply }).await.unwrap_err();
    assert!(matches!(err, AppError::Validation(_)));
}

const _: () = (); // ---- later tasks append below this line ----
```

(`Duration` is imported now and used from Task 6 on; if the compiler warns about it being unused at this step, that is expected and disappears in Task 6.)

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test runtime_engine`
Expected: **FAIL to compile** — `core::runtime_service` does not exist.

- [ ] **Step 3: Write `src-tauri/src/core/runtime_service.rs`**

```rust
use std::sync::Arc;

use sqlx::SqlitePool;
use tokio::sync::{mpsc, oneshot};

use crate::error::AppError;
use crate::models::runtime::{RuntimeMode, RuntimeState};

// ---------- Ports — implemented by src/platform.rs adapters and by test fakes.
// ---------- This module never imports tauri: the engine is testable without it.

pub trait SoundPlayer: Send + Sync {
    fn play_work_end(&self);
    fn play_break_end(&self);
}

pub trait Notifier: Send + Sync {
    fn notify(&self, title: &str, body: &str);
}

pub trait TickSink: Send + Sync {
    fn emit(&self, state: &RuntimeState);
}

// ---------- Messages ----------

pub type Reply = oneshot::Sender<Result<(), AppError>>;

pub enum RuntimeCmd {
    StartDay { plan_id: String, reply: Reply },
    PauseDay { reply: Reply },
    ResumeDay { reply: Reply },
    GetStatus { reply: oneshot::Sender<RuntimeState> },
}

impl RuntimeCmd {
    fn name(&self) -> &'static str {
        match self {
            RuntimeCmd::StartDay { .. } => "start_day",
            RuntimeCmd::PauseDay { .. } => "pause_day",
            RuntimeCmd::ResumeDay { .. } => "resume_day",
            RuntimeCmd::GetStatus { .. } => "get_run_status",
        }
    }
}

/// Tauri managed state: the only way IPC commands reach the actor.
pub struct RuntimeHandle(pub mpsc::Sender<RuntimeCmd>);

// ---------- Run bookkeeping (in-memory only; lost on crash by design) ----------

#[derive(Clone)]
struct RunBlock {
    id: String,
    block_type: String, // task | break | meeting
    microtask_id: Option<String>,
    pomodoro_type_id: Option<String>,
    duration_secs: u32,
    done: bool,
    skipped: bool,
}

struct ActiveRun {
    plan_id: String,
    blocks: Vec<RunBlock>,
    idx: usize,
    started_at: String,       // run start, ISO 8601 UTC
    block_started_at: String, // current block start, ISO 8601 UTC
    work_secs: u32,
    break_secs: u32,
    completed: u32,      // task blocks completed
    skipped_blocks: u32, // any blocks skipped
    warned: bool,        // current block's remaining-time warning fired
    session_ids: Vec<String>, // pomodoro_sessions written during this run
}

pub struct RuntimeEngine {
    pool: SqlitePool,
    sound: Arc<dyn SoundPlayer>,
    notifier: Arc<dyn Notifier>,
    sink: Arc<dyn TickSink>,
    state: RuntimeState,
    run: Option<ActiveRun>,
}

pub fn now_iso() -> String {
    chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

fn mode_for(block_type: &str) -> RuntimeMode {
    // Meetings run in Work mode: the spec state machine has only Work/Break.
    if block_type == "break" {
        RuntimeMode::Break
    } else {
        RuntimeMode::Work
    }
}

fn block_duration_secs(start: &str, end: &str) -> u32 {
    match (
        chrono::DateTime::parse_from_rfc3339(start),
        chrono::DateTime::parse_from_rfc3339(end),
    ) {
        (Ok(s), Ok(e)) => (e - s).num_seconds().max(0) as u32,
        _ => {
            tracing::warn!(start, end, "unparseable block times; defaulting to 20 minutes");
            20 * 60
        }
    }
}

impl RuntimeEngine {
    pub fn new(
        pool: SqlitePool,
        sound: Arc<dyn SoundPlayer>,
        notifier: Arc<dyn Notifier>,
        sink: Arc<dyn TickSink>,
    ) -> Self {
        Self {
            pool,
            sound,
            notifier,
            sink,
            state: RuntimeState::idle(),
            run: None,
        }
    }

    /// The actor: sole owner of RuntimeState. Nothing else ever mutates it.
    pub async fn run_actor(mut self, mut rx: mpsc::Receiver<RuntimeCmd>) {
        tracing::info!("runtime actor started");
        while let Some(cmd) = rx.recv().await {
            self.handle_cmd(cmd).await;
        }
        tracing::info!("runtime command channel closed; actor stopping");
    }

    async fn handle_cmd(&mut self, cmd: RuntimeCmd) {
        tracing::debug!(cmd = cmd.name(), "runtime command received");
        match cmd {
            RuntimeCmd::StartDay { plan_id, reply } => {
                let result = self.start_day(&plan_id).await;
                let _ = reply.send(result);
            }
            RuntimeCmd::PauseDay { reply } => {
                let _ = reply.send(self.pause_day());
            }
            RuntimeCmd::ResumeDay { reply } => {
                let _ = reply.send(self.resume_day());
            }
            RuntimeCmd::GetStatus { reply } => {
                let _ = reply.send(self.state.clone());
            }
        }
        // Commands change state; the UI learns about it the same way it learns
        // about ticks (CQS: no data flows back through the command reply).
        self.emit_tick();
    }

    fn emit_tick(&self) {
        self.sink.emit(&self.state);
    }

    /// Human name of the current state-machine node, for the log narrative.
    fn phase(&self) -> &'static str {
        match (self.state.mode, self.state.is_running) {
            (RuntimeMode::Idle, _) => "Idle",
            (RuntimeMode::Work, true) => "WorkRunning",
            (RuntimeMode::Work, false) => "WorkPaused",
            (RuntimeMode::Break, true) => "BreakRunning",
            (RuntimeMode::Break, false) => "BreakPaused",
        }
    }

    async fn start_day(&mut self, plan_id: &str) -> Result<(), AppError> {
        if self.state.mode != RuntimeMode::Idle {
            return Err(AppError::Validation(
                "a day is already running — end it first".into(),
            ));
        }
        let plan = sqlx::query!(
            r#"SELECT status as "status!: String" FROM plans WHERE id = ?1"#,
            plan_id
        )
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::NotFound { entity: "plan", id: plan_id.to_string() })?;
        if plan.status != "committed" {
            tracing::warn!(plan_id, status = %plan.status, "start_day rejected: plan is not committed");
            return Err(AppError::Validation(format!(
                "plan {plan_id} is not committed (status: {})",
                plan.status
            )));
        }

        let rows = sqlx::query!(
            r#"SELECT wb.id as "id!: String",
                      wb.block_type as "block_type!: String",
                      wb.microtask_id as "microtask_id?: String",
                      wb.start_time as "start_time!: String",
                      wb.end_time as "end_time!: String",
                      m.pomodoro_type_id as "pomodoro_type_id?: String"
               FROM work_blocks wb
               LEFT JOIN microtasks m ON m.id = wb.microtask_id
               WHERE wb.plan_id = ?1
               ORDER BY wb.sort_order"#,
            plan_id
        )
        .fetch_all(&self.pool)
        .await?;
        if rows.is_empty() {
            return Err(AppError::Validation(format!(
                "plan {plan_id} has no work blocks"
            )));
        }

        let blocks: Vec<RunBlock> = rows
            .into_iter()
            .map(|r| RunBlock {
                duration_secs: block_duration_secs(&r.start_time, &r.end_time),
                id: r.id,
                block_type: r.block_type,
                microtask_id: r.microtask_id,
                pomodoro_type_id: r.pomodoro_type_id,
                done: false,
                skipped: false,
            })
            .collect();

        let now = now_iso();
        let first_block_id = blocks[0].id.clone();
        let first_duration = blocks[0].duration_secs;
        let first_mode = mode_for(&blocks[0].block_type);
        let block_count = blocks.len();

        self.state = RuntimeState {
            active_plan_id: Some(plan_id.to_string()),
            current_block_id: Some(first_block_id.clone()),
            timer_seconds_remaining: first_duration,
            is_running: true,
            mode: first_mode,
            start_time: Some(now.clone()),
        };
        self.run = Some(ActiveRun {
            plan_id: plan_id.to_string(),
            blocks,
            idx: 0,
            started_at: now.clone(),
            block_started_at: now,
            work_secs: 0,
            break_secs: 0,
            completed: 0,
            skipped_blocks: 0,
            warned: false,
            session_ids: Vec::new(),
        });
        tracing::info!(
            from = "Idle", to = self.phase(), plan_id, block_id = %first_block_id,
            blocks = block_count, reason = "start_day", "state transition"
        );
        Ok(())
    }

    fn pause_day(&mut self) -> Result<(), AppError> {
        if self.state.mode == RuntimeMode::Idle {
            return Err(AppError::Validation("no day is running".into()));
        }
        if !self.state.is_running {
            return Err(AppError::Validation("the day is already paused".into()));
        }
        let from = self.phase();
        self.state.is_running = false;
        tracing::info!(from, to = self.phase(), block_id = ?self.state.current_block_id, reason = "pause_day", "state transition");
        Ok(())
    }

    fn resume_day(&mut self) -> Result<(), AppError> {
        if self.state.mode == RuntimeMode::Idle {
            return Err(AppError::Validation("no day is running".into()));
        }
        if self.state.is_running {
            return Err(AppError::Validation("the day is not paused".into()));
        }
        let from = self.phase();
        self.state.is_running = true;
        tracing::info!(from, to = self.phase(), block_id = ?self.state.current_block_id, reason = "resume_day", "state transition");
        Ok(())
    }
}
```

Register the module — in `src-tauri/src/core/mod.rs` add:

```rust
pub mod runtime_service;
```

- [ ] **Step 4: Refresh the SQLx offline cache (new `query!` macros)**

```bash
cd src-tauri && cargo sqlx prepare && cd ..
```

Expected: `.sqlx/` gains new query files. Commit them with this task.

- [ ] **Step 5: Run to verify it passes**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test runtime_engine`
Expected: PASS — 4 tests. `dead_code` warnings on `ActiveRun`/`RunBlock` fields (`work_secs`, `session_ids`, …) and on the `sound` field are expected — Tasks 6–9 consume them; they must all be gone by the end of Task 9.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/core src-tauri/tests/runtime_engine.rs src-tauri/.sqlx
git commit -m "feat: runtime actor core — ports, RuntimeCmd, start_day validation, pause/resume"
```

---

### Task 6: Engine part 2 — tick, warnings, auto-advance, sessions, auto-end `[hard]`

The interval arm joins the actor; timer expiry drives sounds, notifications, `PomodoroSession` writes, microtask completion, and the auto-end of the day. After this task the engine runs a full day on its own.

**Files:**
- Modify: `src-tauri/src/core/runtime_service.rs`
- Test: `src-tauri/tests/runtime_engine.rs` (append)

- [ ] **Step 1: Write the failing tests** *(test designed by the strongest agent)* — append below the marker line in `src-tauri/tests/runtime_engine.rs`:

```rust
// ---------- Task 6 tests ----------

async fn count_notifications(rec: &Recorder, title: &str) -> usize {
    rec.notifications.lock().unwrap().iter().filter(|(t, _)| t == title).count()
}

#[tokio::test(start_paused = true)]
async fn a_full_day_auto_advances_with_sounds_sessions_and_rollup() {
    let h = harness().await;
    seed_committed_plan(&h.pool).await;
    h.send(|reply| RuntimeCmd::StartDay { plan_id: "plan1".into(), reply }).await.unwrap();

    // b-work1 (20m) -> b-break1 (5m) -> b-work2 (20m) -> b-break2 (5m) -> auto-end
    tokio::time::advance(Duration::from_secs(1200)).await;
    let s = h.status().await;
    assert_eq!(s.mode, RuntimeMode::Break);
    assert_eq!(s.current_block_id.as_deref(), Some("b-break1"));
    assert_eq!(s.timer_seconds_remaining, 300);
    assert_eq!(h.rec.sounds.lock().unwrap().as_slice(), ["work_end"]);

    tokio::time::advance(Duration::from_secs(300)).await;
    let s = h.status().await;
    assert_eq!(s.mode, RuntimeMode::Work);
    assert_eq!(s.current_block_id.as_deref(), Some("b-work2"));

    tokio::time::advance(Duration::from_secs(1200 + 300)).await;
    let s = h.status().await;
    assert_eq!(s.mode, RuntimeMode::Idle, "last block finished -> day auto-ended");

    // sounds + notifications (spec §4 texts)
    assert_eq!(h.rec.sounds.lock().unwrap().as_slice(), ["work_end", "break_end", "work_end", "break_end"]);
    assert_eq!(count_notifications(&h.rec, "5 minutes remaining").await, 2);
    assert_eq!(count_notifications(&h.rec, "Pomodoro completed!").await, 2);
    assert_eq!(count_notifications(&h.rec, "1 minute remaining").await, 2);
    assert_eq!(count_notifications(&h.rec, "Break ended!").await, 2);

    // incremental PomodoroSessions, linked to the FocusSession written by auto-end
    let sessions = sqlx::query!(
        r#"SELECT microtask_id, work_minutes, was_completed,
                  focus_session_id as "focus_session_id?: String"
           FROM pomodoro_sessions ORDER BY started_at"#
    )
    .fetch_all(&h.pool).await.unwrap();
    assert_eq!(sessions.len(), 2);
    for s in &sessions {
        assert_eq!(s.microtask_id.as_deref(), Some("micro1"));
        assert_eq!(s.work_minutes, 20);
        assert_eq!(s.was_completed, 1);
        assert!(s.focus_session_id.is_some(), "end_day links sessions to the focus session");
    }

    let fs = sqlx::query!(
        "SELECT total_work_seconds, total_break_seconds, blocks_completed, blocks_skipped FROM focus_sessions"
    )
    .fetch_one(&h.pool).await.unwrap();
    assert_eq!(fs.total_work_seconds, 2400);
    assert_eq!(fs.total_break_seconds, 600);
    assert_eq!(fs.blocks_completed, 2);
    assert_eq!(fs.blocks_skipped, 0);

    // last block of micro1 completed -> microtask completed -> Phase 2 roll-up
    let m = sqlx::query!("SELECT status FROM microtasks WHERE id = 'micro1'")
        .fetch_one(&h.pool).await.unwrap();
    assert_eq!(m.status, "completed");
    let t = sqlx::query!("SELECT status FROM tasks WHERE id = 'task1'")
        .fetch_one(&h.pool).await.unwrap();
    assert_eq!(t.status, "completed", "roll-up reached the task");
}

#[tokio::test(start_paused = true)]
async fn the_five_minute_warning_fires_exactly_once_per_block() {
    let h = harness().await;
    seed_committed_plan(&h.pool).await;
    h.send(|reply| RuntimeCmd::StartDay { plan_id: "plan1".into(), reply }).await.unwrap();

    tokio::time::advance(Duration::from_secs(900)).await; // remaining hits 300 here
    assert_eq!(count_notifications(&h.rec, "5 minutes remaining").await, 1);
    tokio::time::advance(Duration::from_secs(30)).await;
    assert_eq!(count_notifications(&h.rec, "5 minutes remaining").await, 1, "no repeat");
}

#[tokio::test(start_paused = true)]
async fn pause_freezes_the_countdown_and_the_accumulators() {
    let h = harness().await;
    seed_committed_plan(&h.pool).await;
    h.send(|reply| RuntimeCmd::StartDay { plan_id: "plan1".into(), reply }).await.unwrap();

    tokio::time::advance(Duration::from_secs(10)).await;
    assert_eq!(h.status().await.timer_seconds_remaining, 1190);

    h.send(|reply| RuntimeCmd::PauseDay { reply }).await.unwrap();
    tokio::time::advance(Duration::from_secs(500)).await;
    let s = h.status().await;
    assert_eq!(s.timer_seconds_remaining, 1190, "paused time does not tick down");
    assert!(!s.is_running);

    h.send(|reply| RuntimeCmd::ResumeDay { reply }).await.unwrap();
    tokio::time::advance(Duration::from_secs(1)).await;
    assert_eq!(h.status().await.timer_seconds_remaining, 1189);
}
```

- [ ] **Step 2: Run to verify they fail**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test runtime_engine`
Expected: the three new tests FAIL (the countdown never moves — there is no interval arm yet).

- [ ] **Step 3: Replace `run_actor` and add the tick machinery in `src-tauri/src/core/runtime_service.rs`**

Replace the existing `run_actor` with the biased select (design decision 2):

```rust
    /// The actor: sole owner of RuntimeState. Nothing else ever mutates it.
    pub async fn run_actor(mut self, mut rx: mpsc::Receiver<RuntimeCmd>) {
        tracing::info!("runtime actor started");
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(1));
        loop {
            tokio::select! {
                biased; // a due tick always wins over a simultaneous command (decision 2)
                _ = interval.tick() => self.on_tick().await,
                cmd = rx.recv() => match cmd {
                    Some(cmd) => self.handle_cmd(cmd).await,
                    None => break,
                },
            }
        }
        tracing::info!("runtime command channel closed; actor stopping");
    }
```

Append the tick path to the `impl RuntimeEngine` block:

```rust
    async fn on_tick(&mut self) {
        if self.state.mode == RuntimeMode::Idle || !self.state.is_running {
            return; // idle and paused days don't tick (the interval still fires; we ignore it)
        }
        {
            let run = self.run.as_mut().expect("running state implies an active run");
            match self.state.mode {
                RuntimeMode::Work => run.work_secs += 1,
                RuntimeMode::Break => run.break_secs += 1,
                RuntimeMode::Idle => unreachable!(),
            }
        }
        self.state.timer_seconds_remaining = self.state.timer_seconds_remaining.saturating_sub(1);
        tracing::trace!(
            remaining = self.state.timer_seconds_remaining,
            block_id = ?self.state.current_block_id,
            "tick"
        );

        self.maybe_warn();
        if self.state.timer_seconds_remaining == 0 {
            self.on_block_timer_end().await;
        }
        self.emit_tick();
    }

    /// Spec §4: 5-minute warning in work blocks, 1-minute warning in breaks.
    /// Fires at most once per block (`warned` resets on every block advance).
    /// Blocks shorter than the threshold never warn — by design, not a bug.
    fn maybe_warn(&mut self) {
        let Some(run) = self.run.as_mut() else { return };
        if run.warned {
            return;
        }
        let (threshold, title, body) = match self.state.mode {
            RuntimeMode::Work => (300, "5 minutes remaining", "Wrap up your current focus item."),
            RuntimeMode::Break => (60, "1 minute remaining", "Get ready to focus."),
            RuntimeMode::Idle => return,
        };
        if self.state.timer_seconds_remaining == threshold {
            run.warned = true;
            self.notifier.notify(title, body);
            tracing::info!(
                block_id = ?self.state.current_block_id,
                threshold_secs = threshold,
                "warning notification fired"
            );
        }
    }

    async fn on_block_timer_end(&mut self) {
        let block_id = self.state.current_block_id.clone();
        match self.state.mode {
            RuntimeMode::Work => {
                self.sound.play_work_end();
                self.notifier.notify("Pomodoro completed!", "Time for a break.");
                tracing::info!(block_id = ?block_id, "work block ended: sound + notification fired");
            }
            RuntimeMode::Break => {
                self.sound.play_break_end();
                self.notifier.notify("Break ended!", "Starting next task.");
                tracing::info!(block_id = ?block_id, "break block ended: sound + notification fired");
            }
            RuntimeMode::Idle => return,
        }
        self.finish_current_block(true, "timer_end").await;
    }

    /// Marks the current block done (completed=true) or skipped, persists the
    /// PomodoroSession for completed task blocks, completes the microtask when
    /// its last pending block is gone, and advances (or auto-ends the day).
    async fn finish_current_block(&mut self, completed: bool, reason: &'static str) {
        let run = self.run.as_mut().expect("finish requires an active run");
        let idx = run.idx;
        if completed {
            run.blocks[idx].done = true;
            if run.blocks[idx].block_type == "task" {
                run.completed += 1;
            }
        } else {
            run.blocks[idx].skipped = true;
            run.skipped_blocks += 1;
        }

        let block = run.blocks[idx].clone();
        let block_started_at = run.block_started_at.clone();
        if block.block_type == "task" {
            // Decision 5: completions write was_completed=1; user skips write an
            // audit row with was_completed=0. Both are incremental (crash-safe).
            self.write_pomodoro_session(&block, &block_started_at, completed).await;
            if completed {
                self.maybe_complete_microtask(&block).await;
            }
        }

        self.advance_or_end(reason).await;
    }

    async fn write_pomodoro_session(&mut self, block: &RunBlock, started_at: &str, completed: bool) {
        let id = uuid::Uuid::new_v4().to_string();
        let now = now_iso();
        let work_minutes = (block.duration_secs / 60) as i64;
        let was_completed = completed as i64;
        // Decision 7: focus_session_id stays NULL until end_day links it.
        let result = sqlx::query!(
            r#"INSERT INTO pomodoro_sessions
               (id, focus_session_id, microtask_id, pomodoro_type_id, work_minutes,
                started_at, completed_at, was_completed, created_at)
               VALUES (?1, NULL, ?2, ?3, ?4, ?5, ?6, ?7, ?6)"#,
            id,
            block.microtask_id,
            block.pomodoro_type_id,
            work_minutes,
            started_at,
            now,
            was_completed
        )
        .execute(&self.pool)
        .await;
        match result {
            Ok(_) => {
                self.run.as_mut().unwrap().session_ids.push(id.clone());
                tracing::info!(
                    session_id = %id, block_id = %block.id,
                    microtask_id = ?block.microtask_id, was_completed = completed,
                    "pomodoro session written"
                );
            }
            // Tick-path persistence failures can't reach a caller: log and keep running.
            Err(e) => tracing::error!(error = %e, block_id = %block.id, "pomodoro session write FAILED"),
        }
    }

    /// Decision 6: a microtask completes when no pending (not done, not skipped)
    /// block still references it — delegated to Phase 2's service for the roll-up.
    async fn maybe_complete_microtask(&mut self, block: &RunBlock) {
        let Some(micro_id) = &block.microtask_id else { return };
        let run = self.run.as_ref().unwrap();
        let still_pending = run.blocks.iter().any(|b| {
            b.microtask_id.as_deref() == Some(micro_id) && !b.done && !b.skipped
        });
        if still_pending {
            return;
        }
        let open = sqlx::query!("SELECT status FROM microtasks WHERE id = ?1", micro_id)
            .fetch_optional(&self.pool)
            .await
            .ok()
            .flatten()
            .map(|r| r.status == "open")
            .unwrap_or(false);
        if !open {
            return; // already completed (or gone) — nothing to do
        }
        match crate::core::microtask_service::complete_microtask(&self.pool, micro_id).await {
            Ok(()) => tracing::info!(microtask_id = %micro_id, "last block done -> microtask completed (roll-up applies)"),
            Err(e) => tracing::error!(error = %e, microtask_id = %micro_id, "microtask completion FAILED"),
        }
    }

    /// Move to the next pending block, or auto-end the day (decision 9).
    async fn advance_or_end(&mut self, reason: &'static str) {
        let run = self.run.as_mut().expect("advance requires an active run");
        let next = run
            .blocks
            .iter()
            .enumerate()
            .skip(run.idx + 1)
            .find(|(_, b)| !b.done && !b.skipped)
            .map(|(i, _)| i);

        match next {
            Some(i) => {
                run.idx = i;
                run.warned = false;
                run.block_started_at = now_iso();
                let from = self.phase();
                let block = &self.run.as_ref().unwrap().blocks[i];
                self.state.current_block_id = Some(block.id.clone());
                self.state.timer_seconds_remaining = block.duration_secs;
                self.state.mode = mode_for(&block.block_type);
                self.state.is_running = true;
                tracing::info!(
                    from, to = self.phase(),
                    block_id = %self.run.as_ref().unwrap().blocks[i].id,
                    reason, "state transition"
                );
            }
            None => self.end_day_internal("all_blocks_finished").await,
        }
    }

    /// Decision 7: insert the FocusSession, then link this run's session rows.
    /// Used by both the user command (Task 7) and the auto-end path.
    async fn end_day_internal(&mut self, reason: &'static str) {
        let run = self.run.take().expect("end_day requires an active run");
        let from = self.phase();
        let id = uuid::Uuid::new_v4().to_string();
        let now = now_iso();
        let (work, brk) = (run.work_secs as i64, run.break_secs as i64);
        let (completed, skipped) = (run.completed as i64, run.skipped_blocks as i64);
        let insert = sqlx::query!(
            r#"INSERT INTO focus_sessions
               (id, plan_id, start_time, end_time, total_work_seconds,
                total_break_seconds, blocks_completed, blocks_skipped, created_at)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?4)"#,
            id, run.plan_id, run.started_at, now, work, brk, completed, skipped
        )
        .execute(&self.pool)
        .await;
        match insert {
            Ok(_) => {
                for sid in &run.session_ids {
                    if let Err(e) = sqlx::query!(
                        "UPDATE pomodoro_sessions SET focus_session_id = ?1 WHERE id = ?2",
                        id, sid
                    )
                    .execute(&self.pool)
                    .await
                    {
                        tracing::error!(error = %e, session_id = %sid, "focus session linking FAILED");
                    }
                }
                tracing::info!(
                    focus_session_id = %id, plan_id = %run.plan_id,
                    total_work_seconds = work, total_break_seconds = brk,
                    blocks_completed = completed, blocks_skipped = skipped,
                    sessions_linked = run.session_ids.len(),
                    "focus session written"
                );
            }
            Err(e) => tracing::error!(error = %e, plan_id = %run.plan_id, "focus session write FAILED"),
        }

        self.state = RuntimeState::idle();
        tracing::info!(from, to = "Idle", reason, "state transition: day ended");
    }
```

- [ ] **Step 4: Refresh the offline cache and run**

```bash
cd src-tauri && cargo sqlx prepare && cd ..
cargo test --manifest-path src-tauri/Cargo.toml --test runtime_engine
```

Expected: PASS — 7 tests. (If the full-day test hangs, the `biased` keyword or `start_paused = true` is missing — those two lines are what make virtual time deterministic.)

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/core src-tauri/tests/runtime_engine.rs src-tauri/.sqlx
git commit -m "feat: runtime tick engine — warnings, auto-advance, incremental sessions, auto-end"
```

---

### Task 7: Engine part 3 — user commands: complete, skip, skip break, end day `[hard]`

**Files:**
- Modify: `src-tauri/src/core/runtime_service.rs`
- Test: `src-tauri/tests/runtime_engine.rs` (append)

- [ ] **Step 1: Write the failing tests** *(test designed by the strongest agent)* — append:

```rust
// ---------- Task 7 tests ----------

#[tokio::test(start_paused = true)]
async fn complete_current_block_finishes_the_microtask_early() {
    let h = harness().await;
    seed_committed_plan(&h.pool).await;
    h.send(|reply| RuntimeCmd::StartDay { plan_id: "plan1".into(), reply }).await.unwrap();
    tokio::time::advance(Duration::from_secs(10)).await;

    h.send(|reply| RuntimeCmd::CompleteCurrentBlock { reply }).await.unwrap();

    // session written for b-work1; decision 6: b-work2 + its trailing b-break2 skipped
    let s = h.status().await;
    assert_eq!(s.mode, RuntimeMode::Break);
    assert_eq!(s.current_block_id.as_deref(), Some("b-break1"));
    assert_eq!(h.rec.sounds.lock().unwrap().as_slice(), ["work_end"]);

    let m = sqlx::query!("SELECT status FROM microtasks WHERE id = 'micro1'")
        .fetch_one(&h.pool).await.unwrap();
    assert_eq!(m.status, "completed", "finish-early completes the microtask immediately");

    // the one remaining pending block is b-break1; when it ends the day auto-ends
    tokio::time::advance(Duration::from_secs(300)).await;
    assert_eq!(h.status().await.mode, RuntimeMode::Idle);

    let fs = sqlx::query!(
        "SELECT blocks_completed, blocks_skipped FROM focus_sessions"
    )
    .fetch_one(&h.pool).await.unwrap();
    assert_eq!(fs.blocks_completed, 1);
    assert_eq!(fs.blocks_skipped, 2, "b-work2 and b-break2 were finish-early skips");

    let sessions = sqlx::query!("SELECT was_completed FROM pomodoro_sessions")
        .fetch_all(&h.pool).await.unwrap();
    assert_eq!(sessions.len(), 1, "finish-early skips write no audit rows — the work is done");
    assert_eq!(sessions[0].was_completed, 1);
}

#[tokio::test(start_paused = true)]
async fn complete_current_block_during_a_break_is_rejected() {
    let h = harness().await;
    seed_committed_plan(&h.pool).await;
    h.send(|reply| RuntimeCmd::StartDay { plan_id: "plan1".into(), reply }).await.unwrap();
    tokio::time::advance(Duration::from_secs(1200)).await; // now in b-break1

    let err = h.send(|reply| RuntimeCmd::CompleteCurrentBlock { reply }).await.unwrap_err();
    assert!(matches!(err, AppError::Validation(_)));
}

#[tokio::test(start_paused = true)]
async fn skip_to_next_block_writes_an_audit_row() {
    let h = harness().await;
    seed_committed_plan(&h.pool).await;
    h.send(|reply| RuntimeCmd::StartDay { plan_id: "plan1".into(), reply }).await.unwrap();
    tokio::time::advance(Duration::from_secs(10)).await;

    h.send(|reply| RuntimeCmd::SkipToNextBlock { reply }).await.unwrap();

    let s = h.status().await;
    assert_eq!(s.current_block_id.as_deref(), Some("b-break1"));
    let row = sqlx::query!("SELECT was_completed FROM pomodoro_sessions")
        .fetch_one(&h.pool).await.unwrap();
    assert_eq!(row.was_completed, 0, "a skipped work block leaves an audit row (decision 5)");
    assert!(h.rec.sounds.lock().unwrap().is_empty(), "user skips play no sound");

    let m = sqlx::query!("SELECT status FROM microtasks WHERE id = 'micro1'")
        .fetch_one(&h.pool).await.unwrap();
    assert_eq!(m.status, "open", "a skip never completes the microtask");
}

#[tokio::test(start_paused = true)]
async fn skip_break_jumps_straight_to_the_next_work_block() {
    let h = harness().await;
    seed_committed_plan(&h.pool).await;
    h.send(|reply| RuntimeCmd::StartDay { plan_id: "plan1".into(), reply }).await.unwrap();
    tokio::time::advance(Duration::from_secs(1200)).await; // in b-break1

    h.send(|reply| RuntimeCmd::SkipBreak { reply }).await.unwrap();
    let s = h.status().await;
    assert_eq!(s.mode, RuntimeMode::Work);
    assert_eq!(s.current_block_id.as_deref(), Some("b-work2"));

    // skip_break outside a break is rejected
    let err = h.send(|reply| RuntimeCmd::SkipBreak { reply }).await.unwrap_err();
    assert!(matches!(err, AppError::Validation(_)));
}

#[tokio::test(start_paused = true)]
async fn end_day_mid_block_discards_the_partial_block_and_records_totals() {
    let h = harness().await;
    seed_committed_plan(&h.pool).await;
    h.send(|reply| RuntimeCmd::StartDay { plan_id: "plan1".into(), reply }).await.unwrap();
    tokio::time::advance(Duration::from_secs(700)).await;

    h.send(|reply| RuntimeCmd::EndDay { reply }).await.unwrap();
    assert_eq!(h.status().await.mode, RuntimeMode::Idle);

    let fs = sqlx::query!(
        "SELECT total_work_seconds, total_break_seconds, blocks_completed, blocks_skipped FROM focus_sessions"
    )
    .fetch_one(&h.pool).await.unwrap();
    assert_eq!(fs.total_work_seconds, 700);
    assert_eq!(fs.total_break_seconds, 0);
    assert_eq!(fs.blocks_completed, 0);
    assert_eq!(fs.blocks_skipped, 0, "unrun blocks are not 'skipped'");
    let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM pomodoro_sessions")
        .fetch_one(&h.pool).await.unwrap();
    assert_eq!(n, 0, "decision 5: a discarded partial block writes no row");

    // the engine is reusable: a new day can start immediately
    h.send(|reply| RuntimeCmd::StartDay { plan_id: "plan1".into(), reply }).await.unwrap();
    assert_eq!(h.status().await.mode, RuntimeMode::Work);
}
```

- [ ] **Step 2: Run to verify they fail**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test runtime_engine`
Expected: FAIL to compile — the new `RuntimeCmd` variants don't exist.

- [ ] **Step 3: Extend the enum, `name()`, and `handle_cmd` in `runtime_service.rs`**

```rust
pub enum RuntimeCmd {
    StartDay { plan_id: String, reply: Reply },
    PauseDay { reply: Reply },
    ResumeDay { reply: Reply },
    CompleteCurrentBlock { reply: Reply },
    SkipToNextBlock { reply: Reply },
    SkipBreak { reply: Reply },
    EndDay { reply: Reply },
    GetStatus { reply: oneshot::Sender<RuntimeState> },
}
```

`name()` gains: `"complete_current_block"`, `"skip_to_next_block"`, `"skip_break"`, `"end_day"`. `handle_cmd` gains the four arms:

```rust
            RuntimeCmd::CompleteCurrentBlock { reply } => {
                let result = self.complete_current_block().await;
                let _ = reply.send(result);
            }
            RuntimeCmd::SkipToNextBlock { reply } => {
                let result = self.skip_to_next_block().await;
                let _ = reply.send(result);
            }
            RuntimeCmd::SkipBreak { reply } => {
                let result = self.skip_break().await;
                let _ = reply.send(result);
            }
            RuntimeCmd::EndDay { reply } => {
                let result = self.end_day().await;
                let _ = reply.send(result);
            }
```

And the four methods on `impl RuntimeEngine`:

```rust
    /// Decision 6: completing early also skips the microtask's remaining work
    /// blocks and each one's trailing break, then advances like a timer end
    /// (same sound + notification — spec §4 puts both on the same edge).
    async fn complete_current_block(&mut self) -> Result<(), AppError> {
        if self.state.mode != RuntimeMode::Work {
            return Err(AppError::Validation("no work block is running".into()));
        }
        let run = self.run.as_mut().expect("work mode implies an active run");
        if run.blocks[run.idx].block_type != "task" {
            return Err(AppError::Validation("the current block is a meeting, not a task".into()));
        }
        let micro_id = run.blocks[run.idx].microtask_id.clone();
        let idx = run.idx;

        // finish-early skips: remaining work blocks of this microtask + their trailing breaks
        let mut skipped = 0u32;
        for i in (idx + 1)..run.blocks.len() {
            if run.blocks[i].microtask_id == micro_id
                && run.blocks[i].block_type == "task"
                && !run.blocks[i].done
                && !run.blocks[i].skipped
            {
                run.blocks[i].skipped = true;
                skipped += 1;
                if let Some(next) = run.blocks.get_mut(i + 1) {
                    if next.block_type == "break" && !next.done && !next.skipped {
                        next.skipped = true;
                        skipped += 1;
                    }
                }
            }
        }
        run.skipped_blocks += skipped;
        if skipped > 0 {
            tracing::info!(
                microtask_id = ?micro_id, blocks_skipped = skipped,
                "finish-early: remaining blocks of the microtask skipped"
            );
        }

        self.sound.play_work_end();
        self.notifier.notify("Pomodoro completed!", "Time for a break.");
        self.finish_current_block(true, "user_complete").await;
        Ok(())
    }

    async fn skip_to_next_block(&mut self) -> Result<(), AppError> {
        if self.state.mode == RuntimeMode::Idle {
            return Err(AppError::Validation("no day is running".into()));
        }
        self.finish_current_block(false, "user_skip").await;
        Ok(())
    }

    async fn skip_break(&mut self) -> Result<(), AppError> {
        if self.state.mode != RuntimeMode::Break {
            return Err(AppError::Validation("no break is running".into()));
        }
        self.finish_current_block(false, "user_skip_break").await;
        Ok(())
    }

    async fn end_day(&mut self) -> Result<(), AppError> {
        if self.state.mode == RuntimeMode::Idle {
            return Err(AppError::Validation("no day is running".into()));
        }
        // Decision 5: the partial current block is discarded — no session row.
        self.end_day_internal("end_day").await;
        Ok(())
    }
```

- [ ] **Step 4: Run to verify they pass**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test runtime_engine`
Expected: PASS — 12 tests, and the `dead_code` warnings from Task 5 are gone.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/core src-tauri/tests/runtime_engine.rs
git commit -m "feat: runtime user commands — complete with finish-early, skips, end day"
```

---

### Task 8: Platform adapters — rodio sounds, notifications, runtime-tick emitter `[medium]`

The thin Tauri-bound side of the ports. No unit tests (they wrap external effects); the QA checklist (Task 13) verifies them with ears, eyes, and the log file.

**Files:**
- Create: `src-tauri/src/platform.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Write `src-tauri/src/platform.rs`**

```rust
use std::time::Duration;

use tauri::Emitter;

use crate::core::runtime_service::{Notifier, SoundPlayer, TickSink};
use crate::models::runtime::RuntimeState;

/// Synthesized two-note chimes (spec §1: no bundled assets, no webview audio).
/// Tones nod to the legacy app's Tone.js notes: a rising pair for work-end,
/// a falling pair for break-end. Each play gets its own thread because the
/// OutputStream must outlive playback and the actor must never block.
pub struct RodioSoundPlayer;

fn play_notes(label: &'static str, notes: &'static [(f32, u64)]) {
    std::thread::spawn(move || {
        use rodio::source::{SineWave, Source};
        match rodio::OutputStream::try_default() {
            Ok((_stream, handle)) => match rodio::Sink::try_new(&handle) {
                Ok(sink) => {
                    for &(freq, ms) in notes {
                        sink.append(
                            SineWave::new(freq)
                                .take_duration(Duration::from_millis(ms))
                                .amplify(0.6), // fixed in M1; Phase 6 wires the volume setting
                        );
                    }
                    sink.sleep_until_end();
                    tracing::info!(sound = label, "sound played");
                }
                Err(e) => tracing::error!(error = %e, sound = label, "audio sink creation failed"),
            },
            Err(e) => tracing::error!(error = %e, sound = label, "audio output unavailable"),
        }
    });
}

impl SoundPlayer for RodioSoundPlayer {
    fn play_work_end(&self) {
        play_notes("work_end", &[(880.0, 180), (1318.5, 280)]); // A5 -> E6, rising
    }
    fn play_break_end(&self) {
        play_notes("break_end", &[(659.3, 180), (440.0, 280)]); // E5 -> A4, falling
    }
}

/// tauri-plugin-notification, called from Rust (spec §1) so it fires with the
/// window hidden or minimized.
pub struct TauriNotifier(pub tauri::AppHandle);

impl Notifier for TauriNotifier {
    fn notify(&self, title: &str, body: &str) {
        use tauri_plugin_notification::NotificationExt;
        match self.0.notification().builder().title(title).body(body).show() {
            Ok(()) => tracing::info!(title, body, "notification sent"),
            Err(e) => tracing::error!(error = %e, title, "notification failed"),
        }
    }
}

/// Streams every state change to the webview as the `runtime-tick` event.
pub struct EventTickSink(pub tauri::AppHandle);

impl TickSink for EventTickSink {
    fn emit(&self, state: &RuntimeState) {
        if let Err(e) = self.0.emit("runtime-tick", state) {
            tracing::error!(error = %e, "runtime-tick emit failed");
        }
    }
}
```

- [ ] **Step 2: Spawn the actor in `src-tauri/src/lib.rs`**

Add `pub mod platform;` to the module list. In `setup`, after the `Db` is managed:

```rust
            // Start Day runtime: one actor owns all live state (spec §4).
            let engine = core::runtime_service::RuntimeEngine::new(
                pool,
                std::sync::Arc::new(platform::RodioSoundPlayer),
                std::sync::Arc::new(platform::TauriNotifier(handle.clone())),
                std::sync::Arc::new(platform::EventTickSink(handle.clone())),
            );
            let (tx, rx) = tokio::sync::mpsc::channel(32);
            tauri::async_runtime::spawn(engine.run_actor(rx));
            app.manage(core::runtime_service::RuntimeHandle(tx));

            // Ask for notification permission on first launch (macOS prompts once).
            {
                use tauri_plugin_notification::NotificationExt;
                match app.notification().request_permission() {
                    Ok(state) => tracing::info!(?state, "notification permission"),
                    Err(e) => tracing::warn!(error = %e, "notification permission request failed"),
                }
            }
```

(`pool` is the variable already created for `db::Db`; clone it into `db::Db(pool.clone())` so both lines compile.)

- [ ] **Step 3: Verify it compiles and launches**

Run: `cargo check --manifest-path src-tauri/Cargo.toml && npm run tauri dev`
Expected: window opens; the log shows `runtime actor started` and the notification-permission line.

- [ ] **Step 4: Commit**

```bash
git add src-tauri
git commit -m "feat: rodio/notification/event adapters + runtime actor spawned at startup"
```

---

### Task 9: Runtime + focus-mode IPC commands `[easy]`

**Files:**
- Create: `src-tauri/src/commands/runtime.rs`, `src-tauri/src/commands/focus.rs`
- Modify: `src-tauri/src/commands/mod.rs`, `src-tauri/src/lib.rs`

- [ ] **Step 1: Write `src-tauri/src/commands/runtime.rs`** (+ `pub mod runtime;` and `pub mod focus;` in `commands/mod.rs`)

```rust
use tokio::sync::oneshot;

use crate::commands::log_outcome;
use crate::core::runtime_service::{RuntimeCmd, RuntimeHandle};
use crate::error::AppError;
use crate::models::runtime::RuntimeState;

/// All runtime commands go through the actor's mailbox. An unreachable actor
/// is an Internal error — it means the engine task died, which is a bug.
async fn send(
    rt: &RuntimeHandle,
    make: impl FnOnce(oneshot::Sender<Result<(), AppError>>) -> RuntimeCmd,
) -> Result<(), AppError> {
    let (tx, rx) = oneshot::channel();
    rt.0.send(make(tx))
        .await
        .map_err(|_| AppError::Internal("runtime engine unreachable".into()))?;
    rx.await
        .map_err(|_| AppError::Internal("runtime engine dropped the reply".into()))?
}

#[tauri::command]
#[tracing::instrument(skip(rt))]
pub async fn start_day(
    rt: tauri::State<'_, RuntimeHandle>,
    plan_id: String,
) -> Result<(), AppError> {
    let result = send(&rt, |reply| RuntimeCmd::StartDay { plan_id, reply }).await;
    log_outcome(&result);
    if result.is_ok() {
        crate::commands::focus::engage_focus_mode("start_day");
    }
    result
}

#[tauri::command]
#[tracing::instrument(skip(rt))]
pub async fn pause_day(rt: tauri::State<'_, RuntimeHandle>) -> Result<(), AppError> {
    let result = send(&rt, |reply| RuntimeCmd::PauseDay { reply }).await;
    log_outcome(&result);
    result
}

#[tauri::command]
#[tracing::instrument(skip(rt))]
pub async fn resume_day(rt: tauri::State<'_, RuntimeHandle>) -> Result<(), AppError> {
    let result = send(&rt, |reply| RuntimeCmd::ResumeDay { reply }).await;
    log_outcome(&result);
    result
}

#[tauri::command]
#[tracing::instrument(skip(rt))]
pub async fn complete_current_block(rt: tauri::State<'_, RuntimeHandle>) -> Result<(), AppError> {
    let result = send(&rt, |reply| RuntimeCmd::CompleteCurrentBlock { reply }).await;
    log_outcome(&result);
    result
}

#[tauri::command]
#[tracing::instrument(skip(rt))]
pub async fn skip_to_next_block(rt: tauri::State<'_, RuntimeHandle>) -> Result<(), AppError> {
    let result = send(&rt, |reply| RuntimeCmd::SkipToNextBlock { reply }).await;
    log_outcome(&result);
    result
}

#[tauri::command]
#[tracing::instrument(skip(rt))]
pub async fn skip_break(rt: tauri::State<'_, RuntimeHandle>) -> Result<(), AppError> {
    let result = send(&rt, |reply| RuntimeCmd::SkipBreak { reply }).await;
    log_outcome(&result);
    result
}

#[tauri::command]
#[tracing::instrument(skip(rt))]
pub async fn end_day(rt: tauri::State<'_, RuntimeHandle>) -> Result<(), AppError> {
    let result = send(&rt, |reply| RuntimeCmd::EndDay { reply }).await;
    log_outcome(&result);
    if result.is_ok() {
        crate::commands::focus::disengage_focus_mode("end_day");
    }
    result
}

#[tauri::command]
#[tracing::instrument(skip(rt))]
pub async fn get_run_status(rt: tauri::State<'_, RuntimeHandle>) -> Result<RuntimeState, AppError> {
    let (tx, rx) = oneshot::channel();
    rt.0.send(RuntimeCmd::GetStatus { reply: tx })
        .await
        .map_err(|_| AppError::Internal("runtime engine unreachable".into()))?;
    rx.await
        .map_err(|_| AppError::Internal("runtime engine dropped the reply".into()))
}
```

- [ ] **Step 2: Write `src-tauri/src/commands/focus.rs`** — the M1 seam stubs (spec §4)

```rust
/// M1: logging no-ops. M2+ replaces the bodies with OS-level blocking; the
/// command surface (the seam) stays identical.
pub(crate) fn engage_focus_mode(trigger: &str) {
    tracing::info!(trigger, "focus mode START (M1 stub — nothing is blocked yet)");
}

pub(crate) fn disengage_focus_mode(trigger: &str) {
    tracing::info!(trigger, "focus mode STOP (M1 stub)");
}

#[tauri::command]
#[tracing::instrument]
pub fn start_focus_mode(context: Option<String>) {
    engage_focus_mode(context.as_deref().unwrap_or("manual"));
}

#[tauri::command]
#[tracing::instrument]
pub fn stop_focus_mode() {
    disengage_focus_mode("manual");
}
```

- [ ] **Step 3: Register all ten in `src-tauri/src/lib.rs`** inside `tauri::generate_handler![...]`:

```rust
            commands::runtime::start_day,
            commands::runtime::pause_day,
            commands::runtime::resume_day,
            commands::runtime::complete_current_block,
            commands::runtime::skip_to_next_block,
            commands::runtime::skip_break,
            commands::runtime::end_day,
            commands::runtime::get_run_status,
            commands::focus::start_focus_mode,
            commands::focus::stop_focus_mode,
```

- [ ] **Step 4: Verify**

Run: `cargo test --manifest-path src-tauri/Cargo.toml`
Expected: PASS — everything compiles, engine tests untouched.

- [ ] **Step 5: Commit**

```bash
git add src-tauri
git commit -m "feat: runtime + focus-mode IPC commands over the actor mailbox"
```

---

### Task 10: `useRuntimeStore` + TS wire type `[medium]`

**Files:**
- Modify: `src/ipc/types.ts`
- Create: `src/stores/runtimeStore.ts`
- Test: `src/stores/runtimeStore.test.ts`

- [ ] **Step 1: Add the wire type to `src/ipc/types.ts`**

```ts
export interface RuntimeStateWire {
  activePlanId: string | null;
  currentBlockId: string | null;
  timerSecondsRemaining: number;
  isRunning: boolean;
  mode: "work" | "break" | "idle";
  startTime: string | null;
}
```

- [ ] **Step 2: Write the failing tests** *(test designed by the strongest agent)*

`src/stores/runtimeStore.test.ts` — `listen` is mocked at module level so `initListener` can be driven by hand:

```ts
import { beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { mockIPC } from "@tauri-apps/api/mocks";

const listeners: Array<(e: { payload: unknown }) => void> = [];
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(async (_event: string, cb: (e: { payload: unknown }) => void) => {
    listeners.push(cb);
    return () => {};
  }),
}));

import { useRuntimeStore } from "./runtimeStore";

const runningTick = {
  activePlanId: "plan1", currentBlockId: "b1", timerSecondsRemaining: 1199,
  isRunning: true, mode: "work" as const, startTime: "2026-06-09T09:00:00Z",
};

describe("useRuntimeStore", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    listeners.length = 0;
  });

  it("initListener subscribes to runtime-tick and seeds from get_run_status", async () => {
    mockIPC((cmd) => {
      if (cmd === "get_run_status") {
        return { ...runningTick, timerSecondsRemaining: 1200 };
      }
    });
    const store = useRuntimeStore();
    await store.initListener();
    expect(store.remainingSeconds).toBe(1200);

    listeners[0]({ payload: runningTick });
    expect(store.remainingSeconds).toBe(1199);
    expect(store.mode).toBe("work");
    expect(store.currentBlockId).toBe("b1");
  });

  it("startDay surfaces a Validation rejection as the store error", async () => {
    mockIPC((cmd) => {
      if (cmd === "start_day") throw { code: "validation", message: "plan p9 is not committed (status: draft)" };
      if (cmd === "get_run_status") return { ...runningTick, mode: "idle", isRunning: false };
    });
    const store = useRuntimeStore();
    await store.startDay("p9");
    expect(store.error).toContain("not committed");
    expect(store.mode).toBe("idle");
  });

  it("initListener is idempotent — one subscription no matter how many views mount", async () => {
    mockIPC((cmd) => {
      if (cmd === "get_run_status") return { ...runningTick, mode: "idle", isRunning: false };
    });
    const store = useRuntimeStore();
    await store.initListener();
    await store.initListener();
    expect(listeners).toHaveLength(1);
  });
});
```

- [ ] **Step 3: Run to verify they fail**

Run: `npm test -- --run`
Expected: FAIL — `./runtimeStore` doesn't exist.

- [ ] **Step 4: Write `src/stores/runtimeStore.ts`**

The store keeps `currentBlockId` (the wire truth); Day View resolves the full block from `usePlanStore` — duplicating block data here would let the two stores disagree (POLA).

```ts
import { defineStore } from "pinia";
import { listen } from "@tauri-apps/api/event";
import { ipc } from "../ipc/client";
import type { IpcError, RuntimeStateWire } from "../ipc/types";

export const useRuntimeStore = defineStore("runtime", {
  state: () => ({
    activePlanId: null as string | null,
    currentBlockId: null as string | null,
    remainingSeconds: 0,
    isRunning: false,
    mode: "idle" as RuntimeStateWire["mode"],
    startTime: null as string | null,
    error: null as string | null,
    listening: false,
  }),
  actions: {
    async initListener() {
      if (this.listening) return;
      this.listening = true;
      await listen<RuntimeStateWire>("runtime-tick", (e) => this.applyTick(e.payload));
      // seed with the current state so a remounted view doesn't show stale zeros
      this.applyTick(await ipc<RuntimeStateWire>("get_run_status"));
    },
    applyTick(s: RuntimeStateWire) {
      this.activePlanId = s.activePlanId;
      this.currentBlockId = s.currentBlockId;
      this.remainingSeconds = s.timerSecondsRemaining;
      this.isRunning = s.isRunning;
      this.mode = s.mode;
      this.startTime = s.startTime;
    },
    // CQS: commands return nothing; state arrives via runtime-tick events.
    async exec(cmd: string, args?: Record<string, unknown>) {
      this.error = null;
      try {
        await ipc<void>(cmd, args);
      } catch (e) {
        this.error = (e as IpcError).message ?? String(e);
      }
    },
    async startDay(planId: string) {
      await this.exec("start_day", { planId });
    },
    async pauseDay() {
      await this.exec("pause_day");
    },
    async resumeDay() {
      await this.exec("resume_day");
    },
    async completeBlock() {
      await this.exec("complete_current_block");
    },
    async skipBlock() {
      await this.exec("skip_to_next_block");
    },
    async skipBreak() {
      await this.exec("skip_break");
    },
    async endDay() {
      await this.exec("end_day");
    },
  },
});
```

- [ ] **Step 5: Run to verify they pass**

Run: `npm test -- --run && npx vue-tsc --noEmit`
Expected: PASS — 3 new tests, clean typecheck.

- [ ] **Step 6: Commit**

```bash
git add src/ipc/types.ts src/stores
git commit -m "feat: runtime store fed by runtime-tick events with idempotent listener"
```

---

### Task 11: Day View — timer header, controls, active-block highlight `[medium]`

**Files:**
- Create: `src/lib/time.ts`, `src/components/day/TimerHeader.vue`, `src/components/day/RuntimeControls.vue`
- Modify: `src/views/DayView.vue`
- Test: `src/lib/time.test.ts`

- [ ] **Step 1: Write the failing test for the one pure function** *(test designed by the strongest agent)*

`src/lib/time.test.ts`:

```ts
import { describe, expect, it } from "vitest";
import { formatSeconds } from "./time";

describe("formatSeconds", () => {
  it("formats mm:ss with zero padding", () => {
    expect(formatSeconds(0)).toBe("00:00");
    expect(formatSeconds(59)).toBe("00:59");
    expect(formatSeconds(60)).toBe("01:00");
    expect(formatSeconds(1199)).toBe("19:59");
  });
  it("rolls hours into minutes (a 90-minute deep block reads 90:00)", () => {
    expect(formatSeconds(5400)).toBe("90:00");
  });
});
```

- [ ] **Step 2: Run to verify it fails, then write `src/lib/time.ts` and re-run**

Run: `npm test -- --run` → FAIL (`./time` missing), then:

```ts
export function formatSeconds(total: number): string {
  const minutes = Math.floor(total / 60);
  const seconds = total % 60;
  return `${String(minutes).padStart(2, "0")}:${String(seconds).padStart(2, "0")}`;
}
```

Run: `npm test -- --run` → PASS.

- [ ] **Step 3: Write `src/components/day/TimerHeader.vue`**

```vue
<script setup lang="ts">
import { computed } from "vue";
import { useRuntimeStore } from "../../stores/runtimeStore";
import { formatSeconds } from "../../lib/time";

const rt = useRuntimeStore();
const label = computed(() => {
  if (rt.mode === "idle") return "Ready";
  const mode = rt.mode === "work" ? "Focus" : "Break";
  return rt.isRunning ? mode : `${mode} — paused`;
});
</script>

<template>
  <header class="timer" :class="[rt.mode, { paused: !rt.isRunning && rt.mode !== 'idle' }]">
    <span class="countdown">{{ formatSeconds(rt.remainingSeconds) }}</span>
    <span class="label">{{ label }}</span>
  </header>
</template>

<style scoped>
.timer { display: flex; flex-direction: column; align-items: center; padding: 24px 0 8px; }
.countdown { font-size: 72px; font-weight: 700; font-variant-numeric: tabular-nums; letter-spacing: 2px; }
.label { color: #9aa3b2; font-size: 14px; text-transform: uppercase; letter-spacing: 3px; }
.timer.work .countdown { color: #e6e9ef; }
.timer.break .countdown { color: #7dc4a5; }
.timer.idle .countdown { color: #4a5260; }
.timer.paused .countdown { opacity: 0.5; }
</style>
```

- [ ] **Step 4: Write `src/components/day/RuntimeControls.vue`**

```vue
<script setup lang="ts">
import { useRuntimeStore } from "../../stores/runtimeStore";

defineProps<{ planId: string | null; planCommitted: boolean }>();
const rt = useRuntimeStore();

function confirmEnd() {
  if (window.confirm("End the day? Remaining blocks won't run; completed pomodoros are saved.")) {
    rt.endDay();
  }
}
</script>

<template>
  <div class="controls">
    <p v-if="rt.error" class="error">{{ rt.error }}</p>
    <template v-if="rt.mode === 'idle'">
      <button
        class="primary"
        :disabled="!planId || !planCommitted"
        :title="planCommitted ? '' : 'Commit the plan first'"
        @click="planId && rt.startDay(planId)"
      >
        Start Day
      </button>
    </template>
    <template v-else>
      <button v-if="rt.isRunning" @click="rt.pauseDay()">Pause</button>
      <button v-else class="primary" @click="rt.resumeDay()">Resume</button>
      <button v-if="rt.mode === 'work'" @click="rt.completeBlock()">Complete</button>
      <button v-if="rt.mode === 'break'" @click="rt.skipBreak()">Skip break</button>
      <button @click="rt.skipBlock()">Skip</button>
      <button class="danger" @click="confirmEnd">End Day</button>
    </template>
  </div>
</template>

<style scoped>
.controls { display: flex; gap: 10px; justify-content: center; padding: 12px 0 24px; }
button { background: #1f2630; border: 1px solid #2a313c; color: #e6e9ef; border-radius: 8px; padding: 8px 18px; cursor: pointer; }
button.primary { background: #2d6cdf; border-color: #2d6cdf; }
button.danger { border-color: #5a2a31; color: #e08c95; }
button:disabled { opacity: 0.4; cursor: not-allowed; }
.error { color: #e06c75; width: 100%; text-align: center; }
</style>
```

- [ ] **Step 5: Wire into `src/views/DayView.vue`**

Phase 3 built this view (timeline + draft editing). Add the runtime pieces — exact insertion points depend on the file Phase 3 produced; the contract is:

```vue
<script setup lang="ts">
// add to the existing imports:
import { onMounted } from "vue";
import TimerHeader from "../components/day/TimerHeader.vue";
import RuntimeControls from "../components/day/RuntimeControls.vue";
import { useRuntimeStore } from "../stores/runtimeStore";

const rt = useRuntimeStore();
onMounted(() => rt.initListener());
</script>

<template>
  <!-- above the existing timeline: -->
  <TimerHeader />
  <RuntimeControls
    :plan-id="planStore.activePlan?.id ?? null"
    :plan-committed="planStore.activePlan?.status === 'committed'"
  />
  <!-- on the existing timeline block row, add the runtime classes: -->
  <!-- :class="{ active: block.id === rt.currentBlockId,
                 completed: rt.mode !== 'idle' && isBefore(block, rt.currentBlockId) }" -->
</template>
```

`isBefore(block, currentId)` = the block's position in `planStore.workBlocks` is lower than the current block's position — a 3-line helper in the view. If Phase 3's store/property names differ, adapt the call sites and **amend this plan + record a lesson** (roadmap convention 1). Style hooks: `.active { outline: 1px solid #2d6cdf; }`, `.completed { opacity: 0.55; }` with a ✓ marker.

- [ ] **Step 6: Verify in the app**

Run: `npm run tauri dev`
Expected: commit a plan (Phase 3 flow), press Start Day — countdown runs, the active block is highlighted, Pause/Resume/Skip/Complete/End Day all behave, and Settings → no crash with the window minimized (the log keeps ticking).

- [ ] **Step 7: Commit**

```bash
git add src
git commit -m "feat: day view runtime surface — timer header, controls, active-block highlight"
```

---

### Task 12: Docs — README + no-schema-change check `[easy]`

**Files:**
- Modify: `README.md`
- Verify-only: `src-tauri/migrations/`

- [ ] **Step 1: Update `README.md`** — current-state: Start Day is live (timer engine in Rust, sounds, notifications, session history recorded); Analytics arrives in Phase 5. Add a "Where are my logs?" pointer (`logs/` in dev) — Phase 4 is when users start asking.

- [ ] **Step 2: Confirm no schema change happened**

Run: `git diff main --stat -- src-tauri/migrations`
Expected: empty. (Otherwise STOP — record a lesson in `docs/lessons/` and amend this plan; `focus_sessions`/`pomodoro_sessions` exist since migration 0001.)

- [ ] **Step 3: Commit**

```bash
git add README.md
git commit -m "docs: phase 4 README refresh"
```

---

### Task 13: Phase acceptance — manual QA checklist `[trivial]`

Use a real committed plan with at least two short pomodoros (create a 2-minute test PomodoroType so the run fits in QA time).

- [ ] Start Day on a committed plan → countdown runs; Start Day is disabled on draft plans with a tooltip
- [ ] Work block ends → the **rising** two-note chime is audible, the "Pomodoro completed!" notification appears, the break starts by itself
- [ ] Break ends → falling chime, "Break ended!" notification, next work block starts by itself
- [ ] The 5-minute warning notification arrives mid-block (with a ≥6-minute type); the 1-minute break warning arrives
- [ ] Pause freezes the countdown; Resume continues from the same second
- [ ] Complete mid-block → break starts immediately; the microtask's remaining blocks are skipped; the Backlog shows the microtask (and task roll-up) completed
- [ ] Skip on a work block → no sound, immediate advance; `pomodoro_sessions` has a `was_completed = 0` row (check via `sqlite3`)
- [ ] **Minimize the window for 2+ minutes during a work block** → on restore the countdown is correct, and the log shows uninterrupted ticking (the reason this engine is in Rust)
- [ ] End Day mid-run → confirm dialog; afterwards `focus_sessions` has one row with sensible totals and the linked `pomodoro_sessions` rows point at it
- [ ] Quit the app mid-run, relaunch → runtime is Idle, the plan is still `committed`, previously written `pomodoro_sessions` rows survive (crash semantics)
- [ ] **Logs narrative (spec §7 gate):** open `logs/focus-planner.log.<today>` — the entire run reads as a story: `start_day` → `state transition` lines with from/to/reason → warning + sound + notification lines → `pomodoro session written` → `focus session written` → `day ended`. A junior with zero context can reconstruct the day
- [ ] `cargo test --manifest-path src-tauri/Cargo.toml` (12 runtime tests among them) and `npm test -- --run` pass; CI green

---

## Plan self-review (performed at writing time)

1. **Scope coverage:** all seven spec §3 Day-running commands (Tasks 5–7, 9) + focus-mode stubs (Task 9) + `get_run_status` (Tasks 5, 9); spec §4 state machine incl. actor pattern and biased select (Tasks 5–6), sound & notification rules with exact spec texts (Task 6), block-completion semantics with incremental writes (Task 6), crash semantics (decision 7, QA item); spec §6 timer header + controls + `useRuntimeStore` with `initListener` (Tasks 10–11). Sounds via rodio behind `SoundPlayer`, notifications via `tauri-plugin-notification` behind `Notifier`, ticks via `TickSink` — engine never imports Tauri.
2. **Placeholder scan:** every type referenced is defined in a task (`RuntimeCmd` extended in Task 7 with the full enum repeated; `Recorder`, `Harness`, `ActiveRun`, `RunBlock` in Tasks 5–6); the single intentionally-open point (Phase 3's DayView property names, Task 11 Step 5) is flagged with the amend-plan convention rather than hidden.
3. **Type consistency:** `RuntimeState` field names match spec §2 (with the flagged `Option<String>` deviation, decision 10); wire camelCase matches `RuntimeStateWire` in TS (Task 10 Step 1) and the serde test (Task 4); `AppError::Internal` (Task 3) matches the `"internal"` wire code Phase 6's plan also references; session columns match spec §2 `pomodoro_sessions`/`focus_sessions` exactly.

