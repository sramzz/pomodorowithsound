# M1 Phase 2 — Backlog Domain Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Full backlog management end-to-end — every Structure, Ranking, and Pomodoro-type command from spec §3 (25 mutations), the roll-up rule in one transaction, the three backlog queries from spec §5, and the Backlog tree UI (inline create, drag-and-drop reorder, quick estimation) plus the PomodoroType presets section of Settings.

**Architecture:** Mutations live in Core services (`src-tauri/src/core/*_service.rs`) taking `&SqlitePool`, exposed by thin `#[tauri::command]` handlers that only instrument and delegate. CQS throughout: every mutation returns `Result<(), AppError>`, the UI refreshes by re-querying. The roll-up rule lives in `microtask_service` inside one transaction. The frontend extends `useProjectStore`, adds `usePomodoroTypeStore`, and renders a nested draggable tree.

**Tech Stack:** Everything Phase 1 established (Tauri 2, Vue 3, TS, Pinia, Vitest + mockIPC, Rust, SQLx 0.8 + `#[sqlx::test]`, tracing) plus two new deps: `chrono` (Rust, timestamps) and `vuedraggable@next` (Vue 3 drag-and-drop), each justified in its task.

**Conventions (per `docs/specs/m1-roadmap.md`):** every task carries a difficulty tag (`[trivial]` `[easy]` `[medium]` `[hard]`). The failing test of each TDD task is designed by the most capable agent; implementation may be assigned by difficulty; every task is reviewed before its commit lands. This plan builds on exactly what Phase 1 created: the `AppError` enum (`db` / `not_found` / `validation` wire codes), the `ipc<T>()` wrapper, the `#[tracing::instrument(skip(db))]` command pattern, services taking `&SqlitePool`, `sqlx::query_as!` macros with `cargo sqlx prepare` refreshes, `#[sqlx::test]` tests, mockIPC vitest tests, and serde camelCase models. The Rust lib is imported in tests as `focus_planner_lib` (verify against `[lib] name` in `src-tauri/Cargo.toml` and adjust `use` lines if Phase 1 ended up with a different name).

**Philosophy (PHILOSOPHY.md):** Logging per spec §7 — every command logs INFO entry/exit through the instrument pattern, validation failures WARN, the roll-up chain logs at INFO as one narrative line. KISS/AHA — services are deliberately repetitive CRUD; duplication beats a premature "generic entity service". POLA — absolute-value updates, no PATCH semantics. Idempotency-ready — client-generated UUIDs, full-ordered-list reorders.

**Loud flags (read before executing):**
1. **NO schema change.** Every table Phase 2 touches exists since migration 0001. If during execution you believe a migration is needed, STOP, flag it in the PR and in `docs/lessons/`, and amend this plan — do not invent a migration.
2. **Spec gap — `list_pomodoro_types`.** Spec §5 lists no read for pomodoro types, but the Settings presets section and the quick-estimation input both need them. This plan adds `list_pomodoro_types()` as a read-only query (CQS-clean). Backfill spec §5 with it in the docs task (Task 28).
3. **Update semantics decision.** Spec §3 writes `update_project(id, name?, description?)`. This plan implements updates as **absolute-value full-field writes**: the command sets every editable field to exactly what it receives; the UI sends the complete edit-form state. No PATCH/partial semantics — `Option` means "the new value is NULL", never "leave unchanged" (POLA; and it is what the idempotency convention in the roadmap expects). `status` is never a parameter — it flows only through complete/uncomplete and the roll-up rule.
4. **Roll-up stops at the goal** (spec §3): microtask → task → goal. Projects are never auto-completed.
5. **Archived children don't block completion and don't take part in reorders** — "last *open* microtask" counts only `status='open' AND is_archived=0`; reorder validation compares against non-archived children only (the tree the user drags shows only non-archived rows).

**Prereqs:** Phase 1 merged (`docs/superpowers/plans/2026-06-09-m1-phase-1-scaffold.md`); dev DB bootstrapped (`./scripts/setup-db.sh`); `cargo sqlx prepare` workflow working.

---

### Task 1: Branch `[trivial]`

- [ ] **Step 1: Create the feature branch off main**

```bash
git checkout main && git pull
git checkout -b feat/m1-phase-2-backlog-domain
```

- [ ] **Step 2: Verify**

Run: `git branch --show-current`
Expected: `feat/m1-phase-2-backlog-domain`

---

### Task 2: Timestamp helper — `chrono` + `core/time.rs` `[easy]`

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Create: `src-tauri/src/core/time.rs`
- Modify: `src-tauri/src/core/mod.rs`
- Test: `src-tauri/tests/time_format.rs`

Every mutation computes `updated_at`/`created_at`/`completed_at` in Rust as ISO 8601 UTC (spec §2). **Why `chrono` over `time`:** it is the de-facto standard crate, SQLx integrates with it via its `chrono` feature should we ever move off TEXT columns, and we only need UTC formatting today — no API-surface argument favors `time` here.

- [ ] **Step 1: Add the dependency to `src-tauri/Cargo.toml`** (under `[dependencies]`)

```toml
chrono = { version = "0.4", default-features = false, features = ["clock"] }
```

- [ ] **Step 2: Write the failing test** *(test designed by the strongest agent)*

`src-tauri/tests/time_format.rs`:

```rust
use focus_planner_lib::core::time::now_iso8601;

#[test]
fn now_iso8601_is_utc_second_precision_iso8601() {
    let ts = now_iso8601();
    // exact shape: YYYY-MM-DDTHH:MM:SSZ (20 chars), per spec §2
    assert_eq!(ts.len(), 20, "got {ts}");
    let bytes = ts.as_bytes();
    assert_eq!(bytes[4], b'-');
    assert_eq!(bytes[7], b'-');
    assert_eq!(bytes[10], b'T');
    assert_eq!(bytes[13], b':');
    assert_eq!(bytes[16], b':');
    assert_eq!(bytes[19], b'Z');
    assert!(ts[0..4].chars().all(|c| c.is_ascii_digit()));
}
```

- [ ] **Step 3: Run to verify it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test time_format`
Expected: FAIL — `core::time` module does not exist.

- [ ] **Step 4: Write `src-tauri/src/core/time.rs` and register the module**

```rust
/// ISO 8601 UTC "now" with second precision — the single timestamp source
/// for every mutation (spec §2: `YYYY-MM-DDTHH:MM:SSZ`).
pub fn now_iso8601() -> String {
    chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
}
```

In `src-tauri/src/core/mod.rs` add:

```rust
pub mod time;
```

- [ ] **Step 5: Run to verify it passes**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test time_format`
Expected: PASS (1 test).

- [ ] **Step 6: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/src/core
git commit -m "feat: chrono-based ISO 8601 UTC timestamp helper"
```

---

### Task 3: Project mutations — service `[medium]`

**Files:**
- Modify: `src-tauri/src/core/project_service.rs`
- Test: `src-tauri/tests/project_mutations.rs`

`create_project(id, name, description?)`, `update_project(id, name, description)` (absolute values), `archive_project(id)`, `delete_project(id)` (FK cascade removes the subtree — `foreign_keys(true)` is on in the app pool and is SQLx's default in `#[sqlx::test]` pools).

- [ ] **Step 1: Write the failing test** *(test designed by the strongest agent)*

`src-tauri/tests/project_mutations.rs`:

```rust
use focus_planner_lib::core::project_service;
use focus_planner_lib::error::AppError;
use sqlx::SqlitePool;

#[sqlx::test]
async fn create_project_inserts_open_unarchived_row_with_timestamps(pool: SqlitePool) {
    project_service::create_project(&pool, "p1", "Write the book", Some("a novel"))
        .await
        .unwrap();

    let row = sqlx::query!(
        r#"SELECT name, description, status, is_archived, completed_at, created_at, updated_at
           FROM projects WHERE id = 'p1'"#
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(row.name, "Write the book");
    assert_eq!(row.description.as_deref(), Some("a novel"));
    assert_eq!(row.status, "open");
    assert_eq!(row.is_archived, 0);
    assert!(row.completed_at.is_none());
    assert_eq!(row.created_at, row.updated_at);
    assert!(row.created_at.ends_with('Z'));
}

#[sqlx::test]
async fn create_project_rejects_blank_name(pool: SqlitePool) {
    let err = project_service::create_project(&pool, "p1", "   ", None)
        .await
        .unwrap_err();
    assert!(matches!(err, AppError::Validation(_)));
}

#[sqlx::test]
async fn update_project_sets_absolute_values_and_bumps_updated_at(pool: SqlitePool) {
    project_service::create_project(&pool, "p1", "Old", Some("old desc"))
        .await
        .unwrap();
    // description = None is an absolute write: it clears the column
    project_service::update_project(&pool, "p1", "New", None)
        .await
        .unwrap();

    let row = sqlx::query!("SELECT name, description FROM projects WHERE id = 'p1'")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(row.name, "New");
    assert!(row.description.is_none());
}

#[sqlx::test]
async fn update_project_unknown_id_is_not_found(pool: SqlitePool) {
    let err = project_service::update_project(&pool, "ghost", "X", None)
        .await
        .unwrap_err();
    assert!(matches!(err, AppError::NotFound { .. }));
}

#[sqlx::test]
async fn archive_project_sets_flag(pool: SqlitePool) {
    project_service::create_project(&pool, "p1", "P", None).await.unwrap();
    project_service::archive_project(&pool, "p1").await.unwrap();
    let row = sqlx::query!("SELECT is_archived FROM projects WHERE id = 'p1'")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(row.is_archived, 1);
}

#[sqlx::test]
async fn delete_project_cascades_to_goals(pool: SqlitePool) {
    project_service::create_project(&pool, "p1", "P", None).await.unwrap();
    sqlx::query(
        "INSERT INTO goals (id, project_id, title, priority, sort_order, status, is_archived, created_at, updated_at)
         VALUES ('g1', 'p1', 'G', 0, 0, 'open', 0, '2026-06-09T08:00:00Z', '2026-06-09T08:00:00Z')",
    )
    .execute(&pool)
    .await
    .unwrap();

    project_service::delete_project(&pool, "p1").await.unwrap();

    let goals = sqlx::query!(r#"SELECT COUNT(*) as "cnt: i64" FROM goals"#)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(goals.cnt, 0, "goal must be cascade-deleted");
}

#[sqlx::test]
async fn delete_project_unknown_id_is_not_found(pool: SqlitePool) {
    let err = project_service::delete_project(&pool, "ghost").await.unwrap_err();
    assert!(matches!(err, AppError::NotFound { .. }));
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test project_mutations`
Expected: FAIL — `create_project` etc. do not exist in `project_service`.

- [ ] **Step 3: Append the mutations to `src-tauri/src/core/project_service.rs`** (keep Phase 1's `list_projects` as-is for now; Task 16 extends it)

```rust
use crate::core::time::now_iso8601;

pub async fn create_project(
    pool: &SqlitePool,
    id: &str,
    name: &str,
    description: Option<&str>,
) -> Result<(), AppError> {
    let name = name.trim();
    if name.is_empty() {
        tracing::warn!(id, "validation: project name must not be empty");
        return Err(AppError::Validation("project name must not be empty".into()));
    }
    let now = now_iso8601();
    sqlx::query!(
        "INSERT INTO projects (id, name, description, status, is_archived, created_at, updated_at)
         VALUES (?, ?, ?, 'open', 0, ?, ?)",
        id, name, description, now, now
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn update_project(
    pool: &SqlitePool,
    id: &str,
    name: &str,
    description: Option<&str>,
) -> Result<(), AppError> {
    let name = name.trim();
    if name.is_empty() {
        tracing::warn!(id, "validation: project name must not be empty");
        return Err(AppError::Validation("project name must not be empty".into()));
    }
    let now = now_iso8601();
    let result = sqlx::query!(
        "UPDATE projects SET name = ?, description = ?, updated_at = ? WHERE id = ?",
        name, description, now, id
    )
    .execute(pool)
    .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound { entity: "project", id: id.to_string() });
    }
    Ok(())
}

pub async fn archive_project(pool: &SqlitePool, id: &str) -> Result<(), AppError> {
    let now = now_iso8601();
    let result = sqlx::query!(
        "UPDATE projects SET is_archived = 1, updated_at = ? WHERE id = ?",
        now, id
    )
    .execute(pool)
    .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound { entity: "project", id: id.to_string() });
    }
    Ok(())
}

pub async fn delete_project(pool: &SqlitePool, id: &str) -> Result<(), AppError> {
    let result = sqlx::query!("DELETE FROM projects WHERE id = ?", id)
        .execute(pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound { entity: "project", id: id.to_string() });
    }
    Ok(())
}
```

- [ ] **Step 4: Refresh the offline cache and run the tests**

```bash
cd src-tauri && cargo sqlx prepare && cd ..
cargo test --manifest-path src-tauri/Cargo.toml --test project_mutations
```

Expected: PASS (7 tests).

- [ ] **Step 5: Commit**

```bash
git add src-tauri
git commit -m "feat: project create/update/archive/delete in project_service"
```

---

### Task 4: Project mutation commands + the `log_outcome` helper `[easy]`

**Files:**
- Modify: `src-tauri/src/commands/mod.rs`, `src-tauri/src/commands/project.rs`, `src-tauri/src/lib.rs`

Mutation commands are not unit-tested separately — they only instrument and delegate; the service tests carry the behavior. The one new piece of logic is the shared outcome logger that enforces spec §7 levels (INFO ok / WARN validation / ERROR else) uniformly across all 25 commands — a justified abstraction, not a hasty one: it is the cross-cutting logging convention itself.

- [ ] **Step 1: Add `log_outcome` to `src-tauri/src/commands/mod.rs`**

```rust
pub mod frontend_log;
pub mod project;

use crate::error::AppError;

/// Uniform outcome logging for every IPC command (spec §7):
/// INFO `ok` · WARN validation rejections · ERROR everything else.
/// Runs inside the command's `#[tracing::instrument]` span, so the
/// command name and key params are already on the line.
pub(crate) fn log_outcome<T>(result: &Result<T, AppError>) {
    match result {
        Ok(_) => tracing::info!("ok"),
        Err(AppError::Validation(msg)) => tracing::warn!(reason = %msg, "rejected"),
        Err(e) => tracing::error!(error = %e, "failed"),
    }
}
```

- [ ] **Step 2: Append the four commands to `src-tauri/src/commands/project.rs`**

```rust
use crate::commands::log_outcome;

#[tauri::command]
#[tracing::instrument(skip(db, description))]
pub async fn create_project(
    db: tauri::State<'_, Db>,
    id: String,
    name: String,
    description: Option<String>,
) -> Result<(), AppError> {
    let result = project_service::create_project(&db.0, &id, &name, description.as_deref()).await;
    log_outcome(&result);
    result
}

#[tauri::command]
#[tracing::instrument(skip(db, description))]
pub async fn update_project(
    db: tauri::State<'_, Db>,
    id: String,
    name: String,
    description: Option<String>,
) -> Result<(), AppError> {
    let result = project_service::update_project(&db.0, &id, &name, description.as_deref()).await;
    log_outcome(&result);
    result
}

#[tauri::command]
#[tracing::instrument(skip(db))]
pub async fn archive_project(db: tauri::State<'_, Db>, id: String) -> Result<(), AppError> {
    let result = project_service::archive_project(&db.0, &id).await;
    log_outcome(&result);
    result
}

#[tauri::command]
#[tracing::instrument(skip(db))]
pub async fn delete_project(db: tauri::State<'_, Db>, id: String) -> Result<(), AppError> {
    let result = project_service::delete_project(&db.0, &id).await;
    log_outcome(&result);
    result
}
```

- [ ] **Step 3: Register in `src-tauri/src/lib.rs`** — add these lines inside the existing `tauri::generate_handler![...]` list (keep existing entries):

```rust
            commands::project::create_project,
            commands::project::update_project,
            commands::project::archive_project,
            commands::project::delete_project,
```

- [ ] **Step 4: Verify it compiles and all tests still pass**

Run: `cargo test --manifest-path src-tauri/Cargo.toml`
Expected: PASS, no new failures.

- [ ] **Step 5: Commit**

```bash
git add src-tauri
git commit -m "feat: project mutation IPC commands with uniform outcome logging"
```

---

### Task 5: Goal model + goal mutation service `[medium]`

**Files:**
- Create: `src-tauri/src/models/goal.rs`, `src-tauri/src/core/goal_service.rs`
- Modify: `src-tauri/src/models/mod.rs`, `src-tauri/src/core/mod.rs`
- Test: `src-tauri/tests/goal_service.rs`

`create_goal` appends at the end of the project's order (`sort_order = MAX+1`). Parent existence is pre-checked so the caller gets `not_found` instead of an opaque FK error (POLA).

- [ ] **Step 1: Write the failing test** *(test designed by the strongest agent)*

`src-tauri/tests/goal_service.rs`:

```rust
use focus_planner_lib::core::{goal_service, project_service};
use focus_planner_lib::error::AppError;
use sqlx::SqlitePool;

async fn seed_project(pool: &SqlitePool) {
    project_service::create_project(pool, "p1", "P", None).await.unwrap();
}

#[sqlx::test]
async fn create_goal_appends_sort_order(pool: SqlitePool) {
    seed_project(&pool).await;
    goal_service::create_goal(&pool, "g1", "p1", "First", None, None, 0).await.unwrap();
    goal_service::create_goal(&pool, "g2", "p1", "Second", Some("desc"), Some("2026-07-01T00:00:00Z"), 2)
        .await
        .unwrap();

    let rows = sqlx::query!(
        "SELECT id, sort_order, priority, deadline FROM goals ORDER BY sort_order"
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!((rows[0].id.as_str(), rows[0].sort_order), ("g1", 0));
    assert_eq!((rows[1].id.as_str(), rows[1].sort_order), ("g2", 1));
    assert_eq!(rows[1].priority, 2);
    assert_eq!(rows[1].deadline.as_deref(), Some("2026-07-01T00:00:00Z"));
}

#[sqlx::test]
async fn create_goal_unknown_project_is_not_found(pool: SqlitePool) {
    let err = goal_service::create_goal(&pool, "g1", "ghost", "G", None, None, 0)
        .await
        .unwrap_err();
    assert!(matches!(err, AppError::NotFound { entity: "project", .. }));
}

#[sqlx::test]
async fn create_goal_rejects_blank_title(pool: SqlitePool) {
    seed_project(&pool).await;
    let err = goal_service::create_goal(&pool, "g1", "p1", "  ", None, None, 0)
        .await
        .unwrap_err();
    assert!(matches!(err, AppError::Validation(_)));
}

#[sqlx::test]
async fn update_goal_sets_absolute_values(pool: SqlitePool) {
    seed_project(&pool).await;
    goal_service::create_goal(&pool, "g1", "p1", "Old", Some("d"), Some("2026-07-01T00:00:00Z"), 1)
        .await
        .unwrap();
    goal_service::update_goal(&pool, "g1", "New", None, None, 5).await.unwrap();

    let row = sqlx::query!("SELECT title, description, deadline, priority FROM goals WHERE id = 'g1'")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(row.title, "New");
    assert!(row.description.is_none());
    assert!(row.deadline.is_none());
    assert_eq!(row.priority, 5);
}

#[sqlx::test]
async fn update_goal_unknown_id_is_not_found(pool: SqlitePool) {
    let err = goal_service::update_goal(&pool, "ghost", "X", None, None, 0)
        .await
        .unwrap_err();
    assert!(matches!(err, AppError::NotFound { .. }));
}

#[sqlx::test]
async fn archive_and_delete_goal(pool: SqlitePool) {
    seed_project(&pool).await;
    goal_service::create_goal(&pool, "g1", "p1", "G", None, None, 0).await.unwrap();

    goal_service::archive_goal(&pool, "g1").await.unwrap();
    let row = sqlx::query!("SELECT is_archived FROM goals WHERE id = 'g1'")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(row.is_archived, 1);

    goal_service::delete_goal(&pool, "g1").await.unwrap();
    let cnt = sqlx::query!(r#"SELECT COUNT(*) as "cnt: i64" FROM goals"#)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(cnt.cnt, 0);

    let err = goal_service::delete_goal(&pool, "g1").await.unwrap_err();
    assert!(matches!(err, AppError::NotFound { .. }));
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test goal_service`
Expected: FAIL — `core::goal_service` does not exist.

- [ ] **Step 3: Write `src-tauri/src/models/goal.rs`** (+ `pub mod goal;` in `models/mod.rs`)

```rust
use serde::Serialize;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Goal {
    pub id: String,
    pub project_id: String,
    pub title: String,
    pub description: Option<String>,
    pub deadline: Option<String>,
    pub priority: i64,
    pub sort_order: i64,
    pub status: String,
    pub is_archived: bool,
    pub completed_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}
```

- [ ] **Step 4: Write `src-tauri/src/core/goal_service.rs`** (+ `pub mod goal_service;` in `core/mod.rs`)

```rust
use crate::core::time::now_iso8601;
use crate::error::AppError;
use sqlx::SqlitePool;

pub async fn create_goal(
    pool: &SqlitePool,
    id: &str,
    project_id: &str,
    title: &str,
    description: Option<&str>,
    deadline: Option<&str>,
    priority: i64,
) -> Result<(), AppError> {
    let title = title.trim();
    if title.is_empty() {
        tracing::warn!(id, "validation: goal title must not be empty");
        return Err(AppError::Validation("goal title must not be empty".into()));
    }
    let parent = sqlx::query!("SELECT id FROM projects WHERE id = ?", project_id)
        .fetch_optional(pool)
        .await?;
    if parent.is_none() {
        return Err(AppError::NotFound { entity: "project", id: project_id.to_string() });
    }
    let now = now_iso8601();
    sqlx::query!(
        "INSERT INTO goals (id, project_id, title, description, deadline, priority, sort_order,
                            status, is_archived, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?,
                 (SELECT COALESCE(MAX(sort_order) + 1, 0) FROM goals WHERE project_id = ?),
                 'open', 0, ?, ?)",
        id, project_id, title, description, deadline, priority, project_id, now, now
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn update_goal(
    pool: &SqlitePool,
    id: &str,
    title: &str,
    description: Option<&str>,
    deadline: Option<&str>,
    priority: i64,
) -> Result<(), AppError> {
    let title = title.trim();
    if title.is_empty() {
        tracing::warn!(id, "validation: goal title must not be empty");
        return Err(AppError::Validation("goal title must not be empty".into()));
    }
    let now = now_iso8601();
    let result = sqlx::query!(
        "UPDATE goals SET title = ?, description = ?, deadline = ?, priority = ?, updated_at = ?
         WHERE id = ?",
        title, description, deadline, priority, now, id
    )
    .execute(pool)
    .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound { entity: "goal", id: id.to_string() });
    }
    Ok(())
}

pub async fn archive_goal(pool: &SqlitePool, id: &str) -> Result<(), AppError> {
    let now = now_iso8601();
    let result = sqlx::query!(
        "UPDATE goals SET is_archived = 1, updated_at = ? WHERE id = ?",
        now, id
    )
    .execute(pool)
    .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound { entity: "goal", id: id.to_string() });
    }
    Ok(())
}

pub async fn delete_goal(pool: &SqlitePool, id: &str) -> Result<(), AppError> {
    let result = sqlx::query!("DELETE FROM goals WHERE id = ?", id)
        .execute(pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound { entity: "goal", id: id.to_string() });
    }
    Ok(())
}
```

- [ ] **Step 5: Refresh the offline cache and run the tests**

```bash
cd src-tauri && cargo sqlx prepare && cd ..
cargo test --manifest-path src-tauri/Cargo.toml --test goal_service
```

Expected: PASS (6 tests).

- [ ] **Step 6: Commit**

```bash
git add src-tauri
git commit -m "feat: goal model and create/update/archive/delete in goal_service"
```

---

### Task 6: Goal mutation commands `[easy]`

**Files:**
- Create: `src-tauri/src/commands/goal.rs`
- Modify: `src-tauri/src/commands/mod.rs`, `src-tauri/src/lib.rs`

- [ ] **Step 1: Write `src-tauri/src/commands/goal.rs`** (+ `pub mod goal;` in `commands/mod.rs`)

```rust
use crate::commands::log_outcome;
use crate::core::goal_service;
use crate::db::Db;
use crate::error::AppError;

#[tauri::command]
#[tracing::instrument(skip(db, description))]
pub async fn create_goal(
    db: tauri::State<'_, Db>,
    id: String,
    project_id: String,
    title: String,
    description: Option<String>,
    deadline: Option<String>,
    priority: Option<i64>,
) -> Result<(), AppError> {
    let result = goal_service::create_goal(
        &db.0,
        &id,
        &project_id,
        &title,
        description.as_deref(),
        deadline.as_deref(),
        priority.unwrap_or(0),
    )
    .await;
    log_outcome(&result);
    result
}

#[tauri::command]
#[tracing::instrument(skip(db, description))]
pub async fn update_goal(
    db: tauri::State<'_, Db>,
    id: String,
    title: String,
    description: Option<String>,
    deadline: Option<String>,
    priority: i64,
) -> Result<(), AppError> {
    let result = goal_service::update_goal(
        &db.0,
        &id,
        &title,
        description.as_deref(),
        deadline.as_deref(),
        priority,
    )
    .await;
    log_outcome(&result);
    result
}

#[tauri::command]
#[tracing::instrument(skip(db))]
pub async fn archive_goal(db: tauri::State<'_, Db>, id: String) -> Result<(), AppError> {
    let result = goal_service::archive_goal(&db.0, &id).await;
    log_outcome(&result);
    result
}

#[tauri::command]
#[tracing::instrument(skip(db))]
pub async fn delete_goal(db: tauri::State<'_, Db>, id: String) -> Result<(), AppError> {
    let result = goal_service::delete_goal(&db.0, &id).await;
    log_outcome(&result);
    result
}
```

- [ ] **Step 2: Register in `src-tauri/src/lib.rs`** — add inside `tauri::generate_handler![...]`:

```rust
            commands::goal::create_goal,
            commands::goal::update_goal,
            commands::goal::archive_goal,
            commands::goal::delete_goal,
```

- [ ] **Step 3: Verify**

Run: `cargo test --manifest-path src-tauri/Cargo.toml`
Expected: PASS, no new failures.

- [ ] **Step 4: Commit**

```bash
git add src-tauri
git commit -m "feat: goal mutation IPC commands"
```

---

### Task 7: Task model + task mutation service `[easy]`

**Files:**
- Create: `src-tauri/src/models/task.rs`, `src-tauri/src/core/task_service.rs`
- Modify: `src-tauri/src/models/mod.rs`, `src-tauri/src/core/mod.rs`
- Test: `src-tauri/tests/task_service.rs`

Deliberate near-duplicate of the goal service with `goal_id` as parent (AHA: no generic "tree entity" abstraction).

- [ ] **Step 1: Write the failing test** *(test designed by the strongest agent)*

`src-tauri/tests/task_service.rs`:

```rust
use focus_planner_lib::core::{goal_service, project_service, task_service};
use focus_planner_lib::error::AppError;
use sqlx::SqlitePool;

async fn seed_goal(pool: &SqlitePool) {
    project_service::create_project(pool, "p1", "P", None).await.unwrap();
    goal_service::create_goal(pool, "g1", "p1", "G", None, None, 0).await.unwrap();
}

#[sqlx::test]
async fn create_task_appends_sort_order_within_goal(pool: SqlitePool) {
    seed_goal(&pool).await;
    task_service::create_task(&pool, "t1", "g1", "First", None, None, 0).await.unwrap();
    task_service::create_task(&pool, "t2", "g1", "Second", None, None, 0).await.unwrap();

    let rows = sqlx::query!("SELECT id, sort_order FROM tasks ORDER BY sort_order")
        .fetch_all(&pool)
        .await
        .unwrap();
    assert_eq!((rows[0].id.as_str(), rows[0].sort_order), ("t1", 0));
    assert_eq!((rows[1].id.as_str(), rows[1].sort_order), ("t2", 1));
}

#[sqlx::test]
async fn create_task_unknown_goal_is_not_found(pool: SqlitePool) {
    let err = task_service::create_task(&pool, "t1", "ghost", "T", None, None, 0)
        .await
        .unwrap_err();
    assert!(matches!(err, AppError::NotFound { entity: "goal", .. }));
}

#[sqlx::test]
async fn create_task_rejects_blank_title(pool: SqlitePool) {
    seed_goal(&pool).await;
    let err = task_service::create_task(&pool, "t1", "g1", " ", None, None, 0)
        .await
        .unwrap_err();
    assert!(matches!(err, AppError::Validation(_)));
}

#[sqlx::test]
async fn update_archive_delete_task(pool: SqlitePool) {
    seed_goal(&pool).await;
    task_service::create_task(&pool, "t1", "g1", "Old", Some("d"), None, 1).await.unwrap();

    task_service::update_task(&pool, "t1", "New", None, Some("2026-08-01T00:00:00Z"), 3)
        .await
        .unwrap();
    let row = sqlx::query!("SELECT title, description, deadline, priority FROM tasks WHERE id = 't1'")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(row.title, "New");
    assert!(row.description.is_none());
    assert_eq!(row.deadline.as_deref(), Some("2026-08-01T00:00:00Z"));
    assert_eq!(row.priority, 3);

    task_service::archive_task(&pool, "t1").await.unwrap();
    let row = sqlx::query!("SELECT is_archived FROM tasks WHERE id = 't1'")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(row.is_archived, 1);

    task_service::delete_task(&pool, "t1").await.unwrap();
    let err = task_service::delete_task(&pool, "t1").await.unwrap_err();
    assert!(matches!(err, AppError::NotFound { .. }));
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test task_service`
Expected: FAIL — `core::task_service` does not exist.

- [ ] **Step 3: Write `src-tauri/src/models/task.rs`** (+ `pub mod task;` in `models/mod.rs`)

```rust
use serde::Serialize;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    pub id: String,
    pub goal_id: String,
    pub title: String,
    pub description: Option<String>,
    pub deadline: Option<String>,
    pub priority: i64,
    pub sort_order: i64,
    pub status: String,
    pub is_archived: bool,
    pub completed_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}
```

- [ ] **Step 4: Write `src-tauri/src/core/task_service.rs`** (+ `pub mod task_service;` in `core/mod.rs`)

```rust
use crate::core::time::now_iso8601;
use crate::error::AppError;
use sqlx::SqlitePool;

pub async fn create_task(
    pool: &SqlitePool,
    id: &str,
    goal_id: &str,
    title: &str,
    description: Option<&str>,
    deadline: Option<&str>,
    priority: i64,
) -> Result<(), AppError> {
    let title = title.trim();
    if title.is_empty() {
        tracing::warn!(id, "validation: task title must not be empty");
        return Err(AppError::Validation("task title must not be empty".into()));
    }
    let parent = sqlx::query!("SELECT id FROM goals WHERE id = ?", goal_id)
        .fetch_optional(pool)
        .await?;
    if parent.is_none() {
        return Err(AppError::NotFound { entity: "goal", id: goal_id.to_string() });
    }
    let now = now_iso8601();
    sqlx::query!(
        "INSERT INTO tasks (id, goal_id, title, description, deadline, priority, sort_order,
                            status, is_archived, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?,
                 (SELECT COALESCE(MAX(sort_order) + 1, 0) FROM tasks WHERE goal_id = ?),
                 'open', 0, ?, ?)",
        id, goal_id, title, description, deadline, priority, goal_id, now, now
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn update_task(
    pool: &SqlitePool,
    id: &str,
    title: &str,
    description: Option<&str>,
    deadline: Option<&str>,
    priority: i64,
) -> Result<(), AppError> {
    let title = title.trim();
    if title.is_empty() {
        tracing::warn!(id, "validation: task title must not be empty");
        return Err(AppError::Validation("task title must not be empty".into()));
    }
    let now = now_iso8601();
    let result = sqlx::query!(
        "UPDATE tasks SET title = ?, description = ?, deadline = ?, priority = ?, updated_at = ?
         WHERE id = ?",
        title, description, deadline, priority, now, id
    )
    .execute(pool)
    .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound { entity: "task", id: id.to_string() });
    }
    Ok(())
}

pub async fn archive_task(pool: &SqlitePool, id: &str) -> Result<(), AppError> {
    let now = now_iso8601();
    let result = sqlx::query!(
        "UPDATE tasks SET is_archived = 1, updated_at = ? WHERE id = ?",
        now, id
    )
    .execute(pool)
    .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound { entity: "task", id: id.to_string() });
    }
    Ok(())
}

pub async fn delete_task(pool: &SqlitePool, id: &str) -> Result<(), AppError> {
    let result = sqlx::query!("DELETE FROM tasks WHERE id = ?", id)
        .execute(pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound { entity: "task", id: id.to_string() });
    }
    Ok(())
}
```

- [ ] **Step 5: Refresh the offline cache and run the tests**

```bash
cd src-tauri && cargo sqlx prepare && cd ..
cargo test --manifest-path src-tauri/Cargo.toml --test task_service
```

Expected: PASS (4 tests).

- [ ] **Step 6: Commit**

```bash
git add src-tauri
git commit -m "feat: task model and create/update/archive/delete in task_service"
```

---

### Task 8: Task mutation commands `[easy]`

**Files:**
- Create: `src-tauri/src/commands/task.rs`
- Modify: `src-tauri/src/commands/mod.rs`, `src-tauri/src/lib.rs`

- [ ] **Step 1: Write `src-tauri/src/commands/task.rs`** (+ `pub mod task;` in `commands/mod.rs`)

```rust
use crate::commands::log_outcome;
use crate::core::task_service;
use crate::db::Db;
use crate::error::AppError;

#[tauri::command]
#[tracing::instrument(skip(db, description))]
pub async fn create_task(
    db: tauri::State<'_, Db>,
    id: String,
    goal_id: String,
    title: String,
    description: Option<String>,
    deadline: Option<String>,
    priority: Option<i64>,
) -> Result<(), AppError> {
    let result = task_service::create_task(
        &db.0,
        &id,
        &goal_id,
        &title,
        description.as_deref(),
        deadline.as_deref(),
        priority.unwrap_or(0),
    )
    .await;
    log_outcome(&result);
    result
}

#[tauri::command]
#[tracing::instrument(skip(db, description))]
pub async fn update_task(
    db: tauri::State<'_, Db>,
    id: String,
    title: String,
    description: Option<String>,
    deadline: Option<String>,
    priority: i64,
) -> Result<(), AppError> {
    let result = task_service::update_task(
        &db.0,
        &id,
        &title,
        description.as_deref(),
        deadline.as_deref(),
        priority,
    )
    .await;
    log_outcome(&result);
    result
}

#[tauri::command]
#[tracing::instrument(skip(db))]
pub async fn archive_task(db: tauri::State<'_, Db>, id: String) -> Result<(), AppError> {
    let result = task_service::archive_task(&db.0, &id).await;
    log_outcome(&result);
    result
}

#[tauri::command]
#[tracing::instrument(skip(db))]
pub async fn delete_task(db: tauri::State<'_, Db>, id: String) -> Result<(), AppError> {
    let result = task_service::delete_task(&db.0, &id).await;
    log_outcome(&result);
    result
}
```

- [ ] **Step 2: Register in `src-tauri/src/lib.rs`** — add inside `tauri::generate_handler![...]`:

```rust
            commands::task::create_task,
            commands::task::update_task,
            commands::task::archive_task,
            commands::task::delete_task,
```

- [ ] **Step 3: Verify**

Run: `cargo test --manifest-path src-tauri/Cargo.toml`
Expected: PASS, no new failures.

- [ ] **Step 4: Commit**

```bash
git add src-tauri
git commit -m "feat: task mutation IPC commands"
```

---

### Task 9: Microtask model + service (create/update/archive/delete) `[medium]`

**Files:**
- Create: `src-tauri/src/models/microtask.rs`, `src-tauri/src/core/microtask_service.rs`
- Modify: `src-tauri/src/models/mod.rs`, `src-tauri/src/core/mod.rs`
- Test: `src-tauri/tests/microtask_service.rs`

The backend stores the `pomodoro_count` it is told — the auto-computation from `estimated_minutes` lives in the frontend quick-estimation input (Task 22). Validation here: title non-blank, `estimated_minutes >= 1`, `pomodoro_count >= 1`, parent task exists, referenced pomodoro type (if any) exists.

- [ ] **Step 1: Write the failing test** *(test designed by the strongest agent)*

`src-tauri/tests/microtask_service.rs`:

```rust
use focus_planner_lib::core::{goal_service, microtask_service, project_service, task_service};
use focus_planner_lib::error::AppError;
use sqlx::SqlitePool;

async fn seed_task(pool: &SqlitePool) {
    project_service::create_project(pool, "p1", "P", None).await.unwrap();
    goal_service::create_goal(pool, "g1", "p1", "G", None, None, 0).await.unwrap();
    task_service::create_task(pool, "t1", "g1", "T", None, None, 0).await.unwrap();
}

#[sqlx::test]
async fn create_microtask_appends_sort_order_and_stores_given_count(pool: SqlitePool) {
    seed_task(&pool).await;
    microtask_service::create_microtask(&pool, "m1", "t1", "Outline", 50, 3, None, None, 0)
        .await
        .unwrap();
    microtask_service::create_microtask(&pool, "m2", "t1", "Draft", 20, 1, None, None, 0)
        .await
        .unwrap();

    let rows = sqlx::query!(
        "SELECT id, estimated_minutes, pomodoro_count, sort_order, status FROM microtasks ORDER BY sort_order"
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].id.as_str(), "m1");
    assert_eq!(rows[0].estimated_minutes, 50);
    assert_eq!(rows[0].pomodoro_count, 3);
    assert_eq!(rows[0].sort_order, 0);
    assert_eq!(rows[0].status, "open");
    assert_eq!(rows[1].sort_order, 1);
}

#[sqlx::test]
async fn create_microtask_accepts_the_seeded_pomodoro_type(pool: SqlitePool) {
    seed_task(&pool).await;
    // 'a0000000-0000-4000-8000-000000000001' is the Standard type seeded by migration 0002
    microtask_service::create_microtask(
        &pool, "m1", "t1", "Outline", 40, 2,
        Some("a0000000-0000-4000-8000-000000000001"), None, 0,
    )
    .await
    .unwrap();

    let row = sqlx::query!("SELECT pomodoro_type_id FROM microtasks WHERE id = 'm1'")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(row.pomodoro_type_id.as_deref(), Some("a0000000-0000-4000-8000-000000000001"));
}

#[sqlx::test]
async fn create_microtask_unknown_type_is_not_found(pool: SqlitePool) {
    seed_task(&pool).await;
    let err = microtask_service::create_microtask(&pool, "m1", "t1", "X", 20, 1, Some("ghost"), None, 0)
        .await
        .unwrap_err();
    assert!(matches!(err, AppError::NotFound { entity: "pomodoro_type", .. }));
}

#[sqlx::test]
async fn create_microtask_rejects_nonpositive_estimate_and_count(pool: SqlitePool) {
    seed_task(&pool).await;
    let err = microtask_service::create_microtask(&pool, "m1", "t1", "X", 0, 1, None, None, 0)
        .await
        .unwrap_err();
    assert!(matches!(err, AppError::Validation(_)));

    let err = microtask_service::create_microtask(&pool, "m1", "t1", "X", 20, 0, None, None, 0)
        .await
        .unwrap_err();
    assert!(matches!(err, AppError::Validation(_)));
}

#[sqlx::test]
async fn update_archive_delete_microtask(pool: SqlitePool) {
    seed_task(&pool).await;
    microtask_service::create_microtask(&pool, "m1", "t1", "Old", 20, 1, None, None, 0)
        .await
        .unwrap();

    microtask_service::update_microtask(&pool, "m1", "New", 60, 3, None, Some("2026-08-01T00:00:00Z"), 2)
        .await
        .unwrap();
    let row = sqlx::query!(
        "SELECT title, estimated_minutes, pomodoro_count, pomodoro_type_id, deadline, priority
         FROM microtasks WHERE id = 'm1'"
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(row.title, "New");
    assert_eq!(row.estimated_minutes, 60);
    assert_eq!(row.pomodoro_count, 3);
    assert!(row.pomodoro_type_id.is_none());
    assert_eq!(row.deadline.as_deref(), Some("2026-08-01T00:00:00Z"));
    assert_eq!(row.priority, 2);

    microtask_service::archive_microtask(&pool, "m1").await.unwrap();
    let row = sqlx::query!("SELECT is_archived FROM microtasks WHERE id = 'm1'")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(row.is_archived, 1);

    microtask_service::delete_microtask(&pool, "m1").await.unwrap();
    let err = microtask_service::delete_microtask(&pool, "m1").await.unwrap_err();
    assert!(matches!(err, AppError::NotFound { .. }));
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test microtask_service`
Expected: FAIL — `core::microtask_service` does not exist.

- [ ] **Step 3: Write `src-tauri/src/models/microtask.rs`** (+ `pub mod microtask;` in `models/mod.rs`)

```rust
use serde::Serialize;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Microtask {
    pub id: String,
    pub task_id: String,
    pub title: String,
    pub estimated_minutes: i64,
    pub pomodoro_count: i64,
    pub pomodoro_type_id: Option<String>,
    pub deadline: Option<String>,
    pub priority: i64,
    pub sort_order: i64,
    pub status: String,
    pub is_archived: bool,
    pub completed_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}
```

- [ ] **Step 4: Write `src-tauri/src/core/microtask_service.rs`** (+ `pub mod microtask_service;` in `core/mod.rs`)

```rust
use crate::core::time::now_iso8601;
use crate::error::AppError;
use sqlx::SqlitePool;

fn validate_microtask_fields(
    title: &str,
    estimated_minutes: i64,
    pomodoro_count: i64,
) -> Result<(), AppError> {
    if title.trim().is_empty() {
        tracing::warn!("validation: microtask title must not be empty");
        return Err(AppError::Validation("microtask title must not be empty".into()));
    }
    if estimated_minutes < 1 {
        tracing::warn!(estimated_minutes, "validation: estimated_minutes must be >= 1");
        return Err(AppError::Validation("estimated_minutes must be >= 1".into()));
    }
    if pomodoro_count < 1 {
        tracing::warn!(pomodoro_count, "validation: pomodoro_count must be >= 1");
        return Err(AppError::Validation("pomodoro_count must be >= 1".into()));
    }
    Ok(())
}

async fn ensure_pomodoro_type_exists(
    pool: &SqlitePool,
    pomodoro_type_id: Option<&str>,
) -> Result<(), AppError> {
    if let Some(type_id) = pomodoro_type_id {
        let exists = sqlx::query!("SELECT id FROM pomodoro_types WHERE id = ?", type_id)
            .fetch_optional(pool)
            .await?;
        if exists.is_none() {
            return Err(AppError::NotFound { entity: "pomodoro_type", id: type_id.to_string() });
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub async fn create_microtask(
    pool: &SqlitePool,
    id: &str,
    task_id: &str,
    title: &str,
    estimated_minutes: i64,
    pomodoro_count: i64,
    pomodoro_type_id: Option<&str>,
    deadline: Option<&str>,
    priority: i64,
) -> Result<(), AppError> {
    validate_microtask_fields(title, estimated_minutes, pomodoro_count)?;
    let title = title.trim();
    let parent = sqlx::query!("SELECT id FROM tasks WHERE id = ?", task_id)
        .fetch_optional(pool)
        .await?;
    if parent.is_none() {
        return Err(AppError::NotFound { entity: "task", id: task_id.to_string() });
    }
    ensure_pomodoro_type_exists(pool, pomodoro_type_id).await?;
    let now = now_iso8601();
    sqlx::query!(
        "INSERT INTO microtasks (id, task_id, title, estimated_minutes, pomodoro_count,
                                 pomodoro_type_id, deadline, priority, sort_order,
                                 status, is_archived, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?,
                 (SELECT COALESCE(MAX(sort_order) + 1, 0) FROM microtasks WHERE task_id = ?),
                 'open', 0, ?, ?)",
        id, task_id, title, estimated_minutes, pomodoro_count,
        pomodoro_type_id, deadline, priority, task_id, now, now
    )
    .execute(pool)
    .await?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub async fn update_microtask(
    pool: &SqlitePool,
    id: &str,
    title: &str,
    estimated_minutes: i64,
    pomodoro_count: i64,
    pomodoro_type_id: Option<&str>,
    deadline: Option<&str>,
    priority: i64,
) -> Result<(), AppError> {
    validate_microtask_fields(title, estimated_minutes, pomodoro_count)?;
    let title = title.trim();
    ensure_pomodoro_type_exists(pool, pomodoro_type_id).await?;
    let now = now_iso8601();
    let result = sqlx::query!(
        "UPDATE microtasks SET title = ?, estimated_minutes = ?, pomodoro_count = ?,
                               pomodoro_type_id = ?, deadline = ?, priority = ?, updated_at = ?
         WHERE id = ?",
        title, estimated_minutes, pomodoro_count, pomodoro_type_id, deadline, priority, now, id
    )
    .execute(pool)
    .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound { entity: "microtask", id: id.to_string() });
    }
    Ok(())
}

pub async fn archive_microtask(pool: &SqlitePool, id: &str) -> Result<(), AppError> {
    let now = now_iso8601();
    let result = sqlx::query!(
        "UPDATE microtasks SET is_archived = 1, updated_at = ? WHERE id = ?",
        now, id
    )
    .execute(pool)
    .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound { entity: "microtask", id: id.to_string() });
    }
    Ok(())
}

pub async fn delete_microtask(pool: &SqlitePool, id: &str) -> Result<(), AppError> {
    let result = sqlx::query!("DELETE FROM microtasks WHERE id = ?", id)
        .execute(pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound { entity: "microtask", id: id.to_string() });
    }
    Ok(())
}
```

- [ ] **Step 5: Refresh the offline cache and run the tests**

```bash
cd src-tauri && cargo sqlx prepare && cd ..
cargo test --manifest-path src-tauri/Cargo.toml --test microtask_service
```

Expected: PASS (5 tests).

- [ ] **Step 6: Commit**

```bash
git add src-tauri
git commit -m "feat: microtask model and create/update/archive/delete in microtask_service"
```

---

### Task 10: The roll-up rule — `complete_microtask` / `uncomplete_microtask` `[hard]`

**Files:**
- Modify: `src-tauri/src/core/microtask_service.rs`
- Test: `src-tauri/tests/rollup.rs`

Spec §3: completing the last open microtask of a task completes the task — inside **one transaction** — which may in turn complete the goal when its last task completes. `uncomplete_microtask` reverses the roll-up. Stops at the goal; projects are never auto-completed. Archived siblings never block ("open" means `status='open' AND is_archived=0`). The chain logs at INFO as one line: `microtask X completed -> task Y completed -> goal Z completed` (spec §7).

- [ ] **Step 1: Write the failing test** *(test designed by the strongest agent)*

`src-tauri/tests/rollup.rs`:

```rust
use focus_planner_lib::core::{goal_service, microtask_service, project_service, task_service};
use focus_planner_lib::error::AppError;
use sqlx::SqlitePool;

/// p1 -> g1 -> t1 (m1, m2) + t2 (m3)
async fn seed_tree(pool: &SqlitePool) {
    project_service::create_project(pool, "p1", "P", None).await.unwrap();
    goal_service::create_goal(pool, "g1", "p1", "G", None, None, 0).await.unwrap();
    task_service::create_task(pool, "t1", "g1", "T1", None, None, 0).await.unwrap();
    task_service::create_task(pool, "t2", "g1", "T2", None, None, 0).await.unwrap();
    microtask_service::create_microtask(pool, "m1", "t1", "M1", 20, 1, None, None, 0).await.unwrap();
    microtask_service::create_microtask(pool, "m2", "t1", "M2", 20, 1, None, None, 0).await.unwrap();
    microtask_service::create_microtask(pool, "m3", "t2", "M3", 20, 1, None, None, 0).await.unwrap();
}

async fn status_of(pool: &SqlitePool, table: &str, id: &str) -> (String, Option<String>) {
    let sql = format!("SELECT status, completed_at FROM {table} WHERE id = ?");
    let row: (String, Option<String>) = sqlx::query_as(&sql).bind(id).fetch_one(pool).await.unwrap();
    row
}

#[sqlx::test]
async fn completing_a_non_last_microtask_does_not_touch_the_task(pool: SqlitePool) {
    seed_tree(&pool).await;
    microtask_service::complete_microtask(&pool, "m1").await.unwrap();

    let (m1_status, m1_completed) = status_of(&pool, "microtasks", "m1").await;
    assert_eq!(m1_status, "completed");
    assert!(m1_completed.is_some());
    assert_eq!(status_of(&pool, "tasks", "t1").await.0, "open");
    assert_eq!(status_of(&pool, "goals", "g1").await.0, "open");
}

#[sqlx::test]
async fn completing_the_last_microtask_completes_the_task_but_not_the_goal_with_open_siblings(
    pool: SqlitePool,
) {
    seed_tree(&pool).await;
    microtask_service::complete_microtask(&pool, "m1").await.unwrap();
    microtask_service::complete_microtask(&pool, "m2").await.unwrap();

    assert_eq!(status_of(&pool, "tasks", "t1").await.0, "completed");
    assert!(status_of(&pool, "tasks", "t1").await.1.is_some());
    assert_eq!(status_of(&pool, "goals", "g1").await.0, "open", "t2 is still open");
}

#[sqlx::test]
async fn completing_the_last_task_completes_the_goal_but_never_the_project(pool: SqlitePool) {
    seed_tree(&pool).await;
    microtask_service::complete_microtask(&pool, "m1").await.unwrap();
    microtask_service::complete_microtask(&pool, "m2").await.unwrap();
    microtask_service::complete_microtask(&pool, "m3").await.unwrap();

    assert_eq!(status_of(&pool, "tasks", "t2").await.0, "completed");
    assert_eq!(status_of(&pool, "goals", "g1").await.0, "completed");
    assert_eq!(status_of(&pool, "projects", "p1").await.0, "open", "roll-up stops at the goal");
}

#[sqlx::test]
async fn archived_open_microtasks_do_not_block_task_completion(pool: SqlitePool) {
    seed_tree(&pool).await;
    microtask_service::archive_microtask(&pool, "m2").await.unwrap();
    microtask_service::complete_microtask(&pool, "m1").await.unwrap();

    assert_eq!(status_of(&pool, "tasks", "t1").await.0, "completed");
}

#[sqlx::test]
async fn uncomplete_reverses_the_full_chain(pool: SqlitePool) {
    seed_tree(&pool).await;
    for m in ["m1", "m2", "m3"] {
        microtask_service::complete_microtask(&pool, m).await.unwrap();
    }
    assert_eq!(status_of(&pool, "goals", "g1").await.0, "completed");

    microtask_service::uncomplete_microtask(&pool, "m3").await.unwrap();

    let (m3_status, m3_completed) = status_of(&pool, "microtasks", "m3").await;
    assert_eq!(m3_status, "open");
    assert!(m3_completed.is_none());
    assert_eq!(status_of(&pool, "tasks", "t2").await.0, "open");
    assert!(status_of(&pool, "tasks", "t2").await.1.is_none());
    assert_eq!(status_of(&pool, "goals", "g1").await.0, "open");
    // t1 keeps its own completion — only the ancestors of m3 reopen
    assert_eq!(status_of(&pool, "tasks", "t1").await.0, "completed");
}

#[sqlx::test]
async fn complete_is_idempotent_and_unknown_id_is_not_found(pool: SqlitePool) {
    seed_tree(&pool).await;
    microtask_service::complete_microtask(&pool, "m1").await.unwrap();
    microtask_service::complete_microtask(&pool, "m1").await.unwrap(); // no-op, still Ok

    let err = microtask_service::complete_microtask(&pool, "ghost").await.unwrap_err();
    assert!(matches!(err, AppError::NotFound { entity: "microtask", .. }));

    microtask_service::uncomplete_microtask(&pool, "m2").await.unwrap(); // already open: no-op Ok
    let err = microtask_service::uncomplete_microtask(&pool, "ghost").await.unwrap_err();
    assert!(matches!(err, AppError::NotFound { entity: "microtask", .. }));
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test rollup`
Expected: FAIL — `complete_microtask` / `uncomplete_microtask` do not exist.

- [ ] **Step 3: Append both functions to `src-tauri/src/core/microtask_service.rs`**

```rust
/// Roll-up rule (spec §3), one transaction: completing the last open,
/// non-archived microtask of a task completes the task, which may complete
/// the goal when its last open task completes. Stops at the goal.
pub async fn complete_microtask(pool: &SqlitePool, id: &str) -> Result<(), AppError> {
    let mut tx = pool.begin().await?;

    let micro = sqlx::query!("SELECT task_id, status FROM microtasks WHERE id = ?", id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| AppError::NotFound { entity: "microtask", id: id.to_string() })?;
    if micro.status == "completed" {
        tracing::info!(microtask_id = id, "already completed - no-op");
        return Ok(());
    }

    let now = now_iso8601();
    sqlx::query!(
        "UPDATE microtasks SET status = 'completed', completed_at = ?, updated_at = ? WHERE id = ?",
        now, now, id
    )
    .execute(&mut *tx)
    .await?;
    let mut chain = format!("microtask {id} completed");

    let open_siblings = sqlx::query!(
        r#"SELECT COUNT(*) as "cnt: i64" FROM microtasks
           WHERE task_id = ? AND status = 'open' AND is_archived = 0"#,
        micro.task_id
    )
    .fetch_one(&mut *tx)
    .await?;

    if open_siblings.cnt == 0 {
        sqlx::query!(
            "UPDATE tasks SET status = 'completed', completed_at = ?, updated_at = ?
             WHERE id = ? AND status = 'open'",
            now, now, micro.task_id
        )
        .execute(&mut *tx)
        .await?;
        chain.push_str(&format!(" -> task {} completed", micro.task_id));

        let goal_id = sqlx::query!("SELECT goal_id FROM tasks WHERE id = ?", micro.task_id)
            .fetch_one(&mut *tx)
            .await?
            .goal_id;
        let open_tasks = sqlx::query!(
            r#"SELECT COUNT(*) as "cnt: i64" FROM tasks
               WHERE goal_id = ? AND status = 'open' AND is_archived = 0"#,
            goal_id
        )
        .fetch_one(&mut *tx)
        .await?;
        if open_tasks.cnt == 0 {
            sqlx::query!(
                "UPDATE goals SET status = 'completed', completed_at = ?, updated_at = ?
                 WHERE id = ? AND status = 'open'",
                now, now, goal_id
            )
            .execute(&mut *tx)
            .await?;
            chain.push_str(&format!(" -> goal {goal_id} completed"));
        }
    }

    tx.commit().await?;
    tracing::info!("{chain}");
    Ok(())
}

/// Reverses the roll-up, one transaction: a task with an open microtask
/// cannot stay completed, nor can its goal — both reopen if completed.
pub async fn uncomplete_microtask(pool: &SqlitePool, id: &str) -> Result<(), AppError> {
    let mut tx = pool.begin().await?;

    let micro = sqlx::query!("SELECT task_id, status FROM microtasks WHERE id = ?", id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| AppError::NotFound { entity: "microtask", id: id.to_string() })?;
    if micro.status == "open" {
        tracing::info!(microtask_id = id, "already open - no-op");
        return Ok(());
    }

    let now = now_iso8601();
    sqlx::query!(
        "UPDATE microtasks SET status = 'open', completed_at = NULL, updated_at = ? WHERE id = ?",
        now, id
    )
    .execute(&mut *tx)
    .await?;
    let mut chain = format!("microtask {id} reopened");

    let task_reopened = sqlx::query!(
        "UPDATE tasks SET status = 'open', completed_at = NULL, updated_at = ?
         WHERE id = ? AND status = 'completed'",
        now, micro.task_id
    )
    .execute(&mut *tx)
    .await?
    .rows_affected();
    if task_reopened > 0 {
        chain.push_str(&format!(" -> task {} reopened", micro.task_id));
    }

    let goal_id = sqlx::query!("SELECT goal_id FROM tasks WHERE id = ?", micro.task_id)
        .fetch_one(&mut *tx)
        .await?
        .goal_id;
    let goal_reopened = sqlx::query!(
        "UPDATE goals SET status = 'open', completed_at = NULL, updated_at = ?
         WHERE id = ? AND status = 'completed'",
        now, goal_id
    )
    .execute(&mut *tx)
    .await?
    .rows_affected();
    if goal_reopened > 0 {
        chain.push_str(&format!(" -> goal {goal_id} reopened"));
    }

    tx.commit().await?;
    tracing::info!("{chain}");
    Ok(())
}
```

- [ ] **Step 4: Refresh the offline cache and run the tests**

```bash
cd src-tauri && cargo sqlx prepare && cd ..
cargo test --manifest-path src-tauri/Cargo.toml --test rollup
```

Expected: PASS (6 tests).

- [ ] **Step 5: Run the full backend suite (the roll-up must not break earlier tests)**

Run: `cargo test --manifest-path src-tauri/Cargo.toml`
Expected: PASS, all tests.

- [ ] **Step 6: Commit**

```bash
git add src-tauri
git commit -m "feat: completion roll-up rule in one transaction with INFO chain logging"
```

---

### Task 11: Microtask commands (incl. complete/uncomplete) `[easy]`

**Files:**
- Create: `src-tauri/src/commands/microtask.rs`
- Modify: `src-tauri/src/commands/mod.rs`, `src-tauri/src/lib.rs`

- [ ] **Step 1: Write `src-tauri/src/commands/microtask.rs`** (+ `pub mod microtask;` in `commands/mod.rs`)

```rust
use crate::commands::log_outcome;
use crate::core::microtask_service;
use crate::db::Db;
use crate::error::AppError;

#[tauri::command]
#[tracing::instrument(skip(db))]
#[allow(clippy::too_many_arguments)]
pub async fn create_microtask(
    db: tauri::State<'_, Db>,
    id: String,
    task_id: String,
    title: String,
    estimated_minutes: i64,
    pomodoro_count: i64,
    pomodoro_type_id: Option<String>,
    deadline: Option<String>,
    priority: Option<i64>,
) -> Result<(), AppError> {
    let result = microtask_service::create_microtask(
        &db.0,
        &id,
        &task_id,
        &title,
        estimated_minutes,
        pomodoro_count,
        pomodoro_type_id.as_deref(),
        deadline.as_deref(),
        priority.unwrap_or(0),
    )
    .await;
    log_outcome(&result);
    result
}

#[tauri::command]
#[tracing::instrument(skip(db))]
#[allow(clippy::too_many_arguments)]
pub async fn update_microtask(
    db: tauri::State<'_, Db>,
    id: String,
    title: String,
    estimated_minutes: i64,
    pomodoro_count: i64,
    pomodoro_type_id: Option<String>,
    deadline: Option<String>,
    priority: i64,
) -> Result<(), AppError> {
    let result = microtask_service::update_microtask(
        &db.0,
        &id,
        &title,
        estimated_minutes,
        pomodoro_count,
        pomodoro_type_id.as_deref(),
        deadline.as_deref(),
        priority,
    )
    .await;
    log_outcome(&result);
    result
}

#[tauri::command]
#[tracing::instrument(skip(db))]
pub async fn complete_microtask(db: tauri::State<'_, Db>, id: String) -> Result<(), AppError> {
    let result = microtask_service::complete_microtask(&db.0, &id).await;
    log_outcome(&result);
    result
}

#[tauri::command]
#[tracing::instrument(skip(db))]
pub async fn uncomplete_microtask(db: tauri::State<'_, Db>, id: String) -> Result<(), AppError> {
    let result = microtask_service::uncomplete_microtask(&db.0, &id).await;
    log_outcome(&result);
    result
}

#[tauri::command]
#[tracing::instrument(skip(db))]
pub async fn archive_microtask(db: tauri::State<'_, Db>, id: String) -> Result<(), AppError> {
    let result = microtask_service::archive_microtask(&db.0, &id).await;
    log_outcome(&result);
    result
}

#[tauri::command]
#[tracing::instrument(skip(db))]
pub async fn delete_microtask(db: tauri::State<'_, Db>, id: String) -> Result<(), AppError> {
    let result = microtask_service::delete_microtask(&db.0, &id).await;
    log_outcome(&result);
    result
}
```

- [ ] **Step 2: Register in `src-tauri/src/lib.rs`** — add inside `tauri::generate_handler![...]`:

```rust
            commands::microtask::create_microtask,
            commands::microtask::update_microtask,
            commands::microtask::complete_microtask,
            commands::microtask::uncomplete_microtask,
            commands::microtask::archive_microtask,
            commands::microtask::delete_microtask,
```

- [ ] **Step 3: Verify**

Run: `cargo test --manifest-path src-tauri/Cargo.toml`
Expected: PASS, no new failures.

- [ ] **Step 4: Commit**

```bash
git add src-tauri
git commit -m "feat: microtask mutation IPC commands incl. complete/uncomplete"
```

---

### Task 12: Reorder services — `reorder_goals` / `reorder_tasks` / `reorder_microtasks` `[medium]`

**Files:**
- Modify: `src-tauri/src/core/goal_service.rs`, `src-tauri/src/core/task_service.rs`, `src-tauri/src/core/microtask_service.rs`
- Test: `src-tauri/tests/reorder.rs`

Convention (roadmap, spec §3): the frontend sends the **full ordered id list** of the parent's non-archived children; the service validates it is exactly that set, then rewrites `sort_order = index` for every row **in one transaction**. Partial lists are a validation error (WARN).

- [ ] **Step 1: Write the failing test** *(test designed by the strongest agent)*

`src-tauri/tests/reorder.rs`:

```rust
use focus_planner_lib::core::{goal_service, microtask_service, project_service, task_service};
use focus_planner_lib::error::AppError;
use sqlx::SqlitePool;

async fn seed(pool: &SqlitePool) {
    project_service::create_project(pool, "p1", "P", None).await.unwrap();
    for g in ["g1", "g2", "g3"] {
        goal_service::create_goal(pool, g, "p1", g, None, None, 0).await.unwrap();
    }
    task_service::create_task(pool, "t1", "g1", "t1", None, None, 0).await.unwrap();
    task_service::create_task(pool, "t2", "g1", "t2", None, None, 0).await.unwrap();
    microtask_service::create_microtask(pool, "m1", "t1", "m1", 20, 1, None, None, 0).await.unwrap();
    microtask_service::create_microtask(pool, "m2", "t1", "m2", 20, 1, None, None, 0).await.unwrap();
}

async fn order_of(pool: &SqlitePool, table: &str, parent_col: &str, parent: &str) -> Vec<String> {
    let sql = format!("SELECT id FROM {table} WHERE {parent_col} = ? ORDER BY sort_order");
    sqlx::query_scalar(&sql).bind(parent).fetch_all(pool).await.unwrap()
}

#[sqlx::test]
async fn reorder_goals_rewrites_sort_order_from_the_full_list(pool: SqlitePool) {
    seed(&pool).await;
    goal_service::reorder_goals(
        &pool,
        "p1",
        &["g3".to_string(), "g1".to_string(), "g2".to_string()],
    )
    .await
    .unwrap();
    assert_eq!(order_of(&pool, "goals", "project_id", "p1").await, ["g3", "g1", "g2"]);
}

#[sqlx::test]
async fn reorder_goals_rejects_partial_or_foreign_lists(pool: SqlitePool) {
    seed(&pool).await;
    let err = goal_service::reorder_goals(&pool, "p1", &["g1".to_string(), "g2".to_string()])
        .await
        .unwrap_err();
    assert!(matches!(err, AppError::Validation(_)));

    let err = goal_service::reorder_goals(
        &pool,
        "p1",
        &["g1".to_string(), "g2".to_string(), "ghost".to_string()],
    )
    .await
    .unwrap_err();
    assert!(matches!(err, AppError::Validation(_)));

    // a rejected reorder must leave the original order intact (transaction rolled back)
    assert_eq!(order_of(&pool, "goals", "project_id", "p1").await, ["g1", "g2", "g3"]);
}

#[sqlx::test]
async fn reorder_goals_ignores_archived_children(pool: SqlitePool) {
    seed(&pool).await;
    goal_service::archive_goal(&pool, "g2").await.unwrap();
    // the visible tree shows g1, g3 only — that full list must be accepted
    goal_service::reorder_goals(&pool, "p1", &["g3".to_string(), "g1".to_string()])
        .await
        .unwrap();
    let visible: Vec<String> = sqlx::query_scalar(
        "SELECT id FROM goals WHERE project_id = 'p1' AND is_archived = 0 ORDER BY sort_order",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(visible, ["g3", "g1"]);
}

#[sqlx::test]
async fn reorder_tasks_and_microtasks_work_the_same_way(pool: SqlitePool) {
    seed(&pool).await;
    task_service::reorder_tasks(&pool, "g1", &["t2".to_string(), "t1".to_string()])
        .await
        .unwrap();
    assert_eq!(order_of(&pool, "tasks", "goal_id", "g1").await, ["t2", "t1"]);

    microtask_service::reorder_microtasks(&pool, "t1", &["m2".to_string(), "m1".to_string()])
        .await
        .unwrap();
    assert_eq!(order_of(&pool, "microtasks", "task_id", "t1").await, ["m2", "m1"]);
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test reorder`
Expected: FAIL — the three `reorder_*` functions do not exist.

- [ ] **Step 3: Append `reorder_goals` to `src-tauri/src/core/goal_service.rs`**

```rust
pub async fn reorder_goals(
    pool: &SqlitePool,
    project_id: &str,
    ordered_ids: &[String],
) -> Result<(), AppError> {
    let mut tx = pool.begin().await?;
    let existing: Vec<String> = sqlx::query_scalar!(
        "SELECT id FROM goals WHERE project_id = ? AND is_archived = 0",
        project_id
    )
    .fetch_all(&mut *tx)
    .await?;
    if existing.len() != ordered_ids.len() || !ordered_ids.iter().all(|id| existing.contains(id)) {
        tracing::warn!(
            project_id,
            expected = existing.len(),
            got = ordered_ids.len(),
            "validation: reorder_goals needs the full ordered list of the project's non-archived goals"
        );
        return Err(AppError::Validation(
            "ordered_ids must contain exactly the project's non-archived goals".into(),
        ));
    }
    let now = now_iso8601();
    for (index, id) in ordered_ids.iter().enumerate() {
        let index = index as i64;
        sqlx::query!(
            "UPDATE goals SET sort_order = ?, updated_at = ? WHERE id = ?",
            index, now, id
        )
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(())
}
```

Also add to the top of `goal_service.rs` if not already present: nothing new — `now_iso8601` and `AppError` are already imported.

- [ ] **Step 4: Append `reorder_tasks` to `src-tauri/src/core/task_service.rs`**

```rust
pub async fn reorder_tasks(
    pool: &SqlitePool,
    goal_id: &str,
    ordered_ids: &[String],
) -> Result<(), AppError> {
    let mut tx = pool.begin().await?;
    let existing: Vec<String> = sqlx::query_scalar!(
        "SELECT id FROM tasks WHERE goal_id = ? AND is_archived = 0",
        goal_id
    )
    .fetch_all(&mut *tx)
    .await?;
    if existing.len() != ordered_ids.len() || !ordered_ids.iter().all(|id| existing.contains(id)) {
        tracing::warn!(
            goal_id,
            expected = existing.len(),
            got = ordered_ids.len(),
            "validation: reorder_tasks needs the full ordered list of the goal's non-archived tasks"
        );
        return Err(AppError::Validation(
            "ordered_ids must contain exactly the goal's non-archived tasks".into(),
        ));
    }
    let now = now_iso8601();
    for (index, id) in ordered_ids.iter().enumerate() {
        let index = index as i64;
        sqlx::query!(
            "UPDATE tasks SET sort_order = ?, updated_at = ? WHERE id = ?",
            index, now, id
        )
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(())
}
```

- [ ] **Step 5: Append `reorder_microtasks` to `src-tauri/src/core/microtask_service.rs`**

```rust
pub async fn reorder_microtasks(
    pool: &SqlitePool,
    task_id: &str,
    ordered_ids: &[String],
) -> Result<(), AppError> {
    let mut tx = pool.begin().await?;
    let existing: Vec<String> = sqlx::query_scalar!(
        "SELECT id FROM microtasks WHERE task_id = ? AND is_archived = 0",
        task_id
    )
    .fetch_all(&mut *tx)
    .await?;
    if existing.len() != ordered_ids.len() || !ordered_ids.iter().all(|id| existing.contains(id)) {
        tracing::warn!(
            task_id,
            expected = existing.len(),
            got = ordered_ids.len(),
            "validation: reorder_microtasks needs the full ordered list of the task's non-archived microtasks"
        );
        return Err(AppError::Validation(
            "ordered_ids must contain exactly the task's non-archived microtasks".into(),
        ));
    }
    let now = now_iso8601();
    for (index, id) in ordered_ids.iter().enumerate() {
        let index = index as i64;
        sqlx::query!(
            "UPDATE microtasks SET sort_order = ?, updated_at = ? WHERE id = ?",
            index, now, id
        )
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(())
}
```

- [ ] **Step 6: Refresh the offline cache and run the tests**

```bash
cd src-tauri && cargo sqlx prepare && cd ..
cargo test --manifest-path src-tauri/Cargo.toml --test reorder
```

Expected: PASS (4 tests).

- [ ] **Step 7: Commit**

```bash
git add src-tauri
git commit -m "feat: full-list reorder services rewriting sort_order in one transaction"
```

---

### Task 13: Reorder commands `[easy]`

**Files:**
- Modify: `src-tauri/src/commands/goal.rs`, `src-tauri/src/commands/task.rs`, `src-tauri/src/commands/microtask.rs`, `src-tauri/src/lib.rs`

- [ ] **Step 1: Append to `src-tauri/src/commands/goal.rs`**

```rust
#[tauri::command]
#[tracing::instrument(skip(db))]
pub async fn reorder_goals(
    db: tauri::State<'_, Db>,
    project_id: String,
    ordered_ids: Vec<String>,
) -> Result<(), AppError> {
    let result = goal_service::reorder_goals(&db.0, &project_id, &ordered_ids).await;
    log_outcome(&result);
    result
}
```

- [ ] **Step 2: Append to `src-tauri/src/commands/task.rs`**

```rust
#[tauri::command]
#[tracing::instrument(skip(db))]
pub async fn reorder_tasks(
    db: tauri::State<'_, Db>,
    goal_id: String,
    ordered_ids: Vec<String>,
) -> Result<(), AppError> {
    let result = task_service::reorder_tasks(&db.0, &goal_id, &ordered_ids).await;
    log_outcome(&result);
    result
}
```

- [ ] **Step 3: Append to `src-tauri/src/commands/microtask.rs`**

```rust
#[tauri::command]
#[tracing::instrument(skip(db))]
pub async fn reorder_microtasks(
    db: tauri::State<'_, Db>,
    task_id: String,
    ordered_ids: Vec<String>,
) -> Result<(), AppError> {
    let result = microtask_service::reorder_microtasks(&db.0, &task_id, &ordered_ids).await;
    log_outcome(&result);
    result
}
```

- [ ] **Step 4: Register in `src-tauri/src/lib.rs`** — add inside `tauri::generate_handler![...]`:

```rust
            commands::goal::reorder_goals,
            commands::task::reorder_tasks,
            commands::microtask::reorder_microtasks,
```

- [ ] **Step 5: Verify**

Run: `cargo test --manifest-path src-tauri/Cargo.toml`
Expected: PASS, no new failures.

- [ ] **Step 6: Commit**

```bash
git add src-tauri
git commit -m "feat: reorder IPC commands for goals, tasks, microtasks"
```

---

### Task 14: PomodoroType model + service `[medium]`

**Files:**
- Create: `src-tauri/src/models/pomodoro_type.rs`, `src-tauri/src/core/pomodoro_type_service.rs`
- Modify: `src-tauri/src/models/mod.rs`, `src-tauri/src/core/mod.rs`
- Test: `src-tauri/tests/pomodoro_type_service.rs`

Mutations: `create_pomodoro_type`, `update_pomodoro_type`, `delete_pomodoro_type`, `set_default_pomodoro_type` (one transaction: clear all defaults, set the new one). Plus the read `list_pomodoro_types` (the loudly-flagged spec-§5 backfill). Validation: name non-blank, `work_minutes >= 1`, `rest_minutes >= 1`, long-break fields are **both-or-neither** (the planner needs both to apply the long-break rule, spec §4) and each `>= 1` when present. Deleting the default is allowed — microtasks fall back via `ON DELETE SET NULL`, and the spec defines the no-default fallback (20 minutes).

- [ ] **Step 1: Write the failing test** *(test designed by the strongest agent)*

`src-tauri/tests/pomodoro_type_service.rs`:

```rust
use focus_planner_lib::core::pomodoro_type_service;
use focus_planner_lib::error::AppError;
use sqlx::SqlitePool;

const SEEDED_STANDARD: &str = "a0000000-0000-4000-8000-000000000001";

#[sqlx::test]
async fn create_and_list_pomodoro_types(pool: SqlitePool) {
    pomodoro_type_service::create_pomodoro_type(&pool, "pt1", "Deep Work", 50, 10, Some(30), Some(2))
        .await
        .unwrap();

    let types = pomodoro_type_service::list_pomodoro_types(&pool).await.unwrap();
    // the seed migration already provides "Standard"
    assert_eq!(types.len(), 2);
    let deep = types.iter().find(|t| t.id == "pt1").unwrap();
    assert_eq!(deep.name, "Deep Work");
    assert_eq!(deep.work_minutes, 50);
    assert_eq!(deep.rest_minutes, 10);
    assert_eq!(deep.long_break_minutes, Some(30));
    assert_eq!(deep.long_break_every, Some(2));
    assert!(!deep.is_default, "new types are never default");
}

#[sqlx::test]
async fn create_rejects_bad_values(pool: SqlitePool) {
    let err = pomodoro_type_service::create_pomodoro_type(&pool, "x", " ", 20, 5, None, None)
        .await
        .unwrap_err();
    assert!(matches!(err, AppError::Validation(_)));

    let err = pomodoro_type_service::create_pomodoro_type(&pool, "x", "T", 0, 5, None, None)
        .await
        .unwrap_err();
    assert!(matches!(err, AppError::Validation(_)));

    // long break fields must come as a pair
    let err = pomodoro_type_service::create_pomodoro_type(&pool, "x", "T", 20, 5, Some(15), None)
        .await
        .unwrap_err();
    assert!(matches!(err, AppError::Validation(_)));
}

#[sqlx::test]
async fn update_pomodoro_type_sets_absolute_values(pool: SqlitePool) {
    pomodoro_type_service::create_pomodoro_type(&pool, "pt1", "Old", 25, 5, Some(20), Some(4))
        .await
        .unwrap();
    pomodoro_type_service::update_pomodoro_type(&pool, "pt1", "New", 45, 15, None, None)
        .await
        .unwrap();

    let row = sqlx::query!(
        "SELECT name, work_minutes, rest_minutes, long_break_minutes, long_break_every
         FROM pomodoro_types WHERE id = 'pt1'"
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(row.name, "New");
    assert_eq!(row.work_minutes, 45);
    assert_eq!(row.rest_minutes, 15);
    assert!(row.long_break_minutes.is_none());
    assert!(row.long_break_every.is_none());

    let err = pomodoro_type_service::update_pomodoro_type(&pool, "ghost", "X", 20, 5, None, None)
        .await
        .unwrap_err();
    assert!(matches!(err, AppError::NotFound { .. }));
}

#[sqlx::test]
async fn set_default_is_exclusive(pool: SqlitePool) {
    pomodoro_type_service::create_pomodoro_type(&pool, "pt1", "Deep", 50, 10, None, None)
        .await
        .unwrap();
    pomodoro_type_service::set_default_pomodoro_type(&pool, "pt1").await.unwrap();

    let defaults: Vec<String> =
        sqlx::query_scalar("SELECT id FROM pomodoro_types WHERE is_default = 1")
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(defaults, ["pt1"], "exactly one default; the seeded Standard lost the flag");

    let err = pomodoro_type_service::set_default_pomodoro_type(&pool, "ghost")
        .await
        .unwrap_err();
    assert!(matches!(err, AppError::NotFound { .. }));
    // a failed set_default must not have cleared the existing default (transaction)
    let defaults: Vec<String> =
        sqlx::query_scalar("SELECT id FROM pomodoro_types WHERE is_default = 1")
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(defaults, ["pt1"]);
}

#[sqlx::test]
async fn delete_pomodoro_type_nulls_microtask_references(pool: SqlitePool) {
    use focus_planner_lib::core::{goal_service, microtask_service, project_service, task_service};
    project_service::create_project(&pool, "p1", "P", None).await.unwrap();
    goal_service::create_goal(&pool, "g1", "p1", "G", None, None, 0).await.unwrap();
    task_service::create_task(&pool, "t1", "g1", "T", None, None, 0).await.unwrap();
    microtask_service::create_microtask(&pool, "m1", "t1", "M", 20, 1, Some(SEEDED_STANDARD), None, 0)
        .await
        .unwrap();

    pomodoro_type_service::delete_pomodoro_type(&pool, SEEDED_STANDARD).await.unwrap();

    let row = sqlx::query!("SELECT pomodoro_type_id FROM microtasks WHERE id = 'm1'")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(row.pomodoro_type_id.is_none(), "ON DELETE SET NULL must apply");

    let err = pomodoro_type_service::delete_pomodoro_type(&pool, SEEDED_STANDARD)
        .await
        .unwrap_err();
    assert!(matches!(err, AppError::NotFound { .. }));
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test pomodoro_type_service`
Expected: FAIL — `core::pomodoro_type_service` does not exist.

- [ ] **Step 3: Write `src-tauri/src/models/pomodoro_type.rs`** (+ `pub mod pomodoro_type;` in `models/mod.rs`)

```rust
use serde::Serialize;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct PomodoroType {
    pub id: String,
    pub name: String,
    pub work_minutes: i64,
    pub rest_minutes: i64,
    pub long_break_minutes: Option<i64>,
    pub long_break_every: Option<i64>,
    pub is_default: bool,
    pub created_at: String,
    pub updated_at: String,
}
```

- [ ] **Step 4: Write `src-tauri/src/core/pomodoro_type_service.rs`** (+ `pub mod pomodoro_type_service;` in `core/mod.rs`)

```rust
use crate::core::time::now_iso8601;
use crate::error::AppError;
use crate::models::pomodoro_type::PomodoroType;
use sqlx::SqlitePool;

fn validate_type_fields(
    name: &str,
    work_minutes: i64,
    rest_minutes: i64,
    long_break_minutes: Option<i64>,
    long_break_every: Option<i64>,
) -> Result<(), AppError> {
    if name.trim().is_empty() {
        tracing::warn!("validation: pomodoro type name must not be empty");
        return Err(AppError::Validation("pomodoro type name must not be empty".into()));
    }
    if work_minutes < 1 || rest_minutes < 1 {
        tracing::warn!(work_minutes, rest_minutes, "validation: minutes must be >= 1");
        return Err(AppError::Validation("work_minutes and rest_minutes must be >= 1".into()));
    }
    match (long_break_minutes, long_break_every) {
        (None, None) => Ok(()),
        (Some(m), Some(e)) if m >= 1 && e >= 1 => Ok(()),
        _ => {
            tracing::warn!(
                ?long_break_minutes,
                ?long_break_every,
                "validation: long break fields must come as a pair, each >= 1"
            );
            Err(AppError::Validation(
                "long_break_minutes and long_break_every must both be set (each >= 1) or both be empty".into(),
            ))
        }
    }
}

pub async fn list_pomodoro_types(pool: &SqlitePool) -> Result<Vec<PomodoroType>, AppError> {
    let types = sqlx::query_as!(
        PomodoroType,
        r#"SELECT id, name, work_minutes, rest_minutes, long_break_minutes, long_break_every,
                  is_default as "is_default: bool", created_at, updated_at
           FROM pomodoro_types ORDER BY created_at"#
    )
    .fetch_all(pool)
    .await?;
    Ok(types)
}

pub async fn create_pomodoro_type(
    pool: &SqlitePool,
    id: &str,
    name: &str,
    work_minutes: i64,
    rest_minutes: i64,
    long_break_minutes: Option<i64>,
    long_break_every: Option<i64>,
) -> Result<(), AppError> {
    validate_type_fields(name, work_minutes, rest_minutes, long_break_minutes, long_break_every)?;
    let name = name.trim();
    let now = now_iso8601();
    sqlx::query!(
        "INSERT INTO pomodoro_types (id, name, work_minutes, rest_minutes, long_break_minutes,
                                     long_break_every, is_default, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, 0, ?, ?)",
        id, name, work_minutes, rest_minutes, long_break_minutes, long_break_every, now, now
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn update_pomodoro_type(
    pool: &SqlitePool,
    id: &str,
    name: &str,
    work_minutes: i64,
    rest_minutes: i64,
    long_break_minutes: Option<i64>,
    long_break_every: Option<i64>,
) -> Result<(), AppError> {
    validate_type_fields(name, work_minutes, rest_minutes, long_break_minutes, long_break_every)?;
    let name = name.trim();
    let now = now_iso8601();
    let result = sqlx::query!(
        "UPDATE pomodoro_types SET name = ?, work_minutes = ?, rest_minutes = ?,
                                   long_break_minutes = ?, long_break_every = ?, updated_at = ?
         WHERE id = ?",
        name, work_minutes, rest_minutes, long_break_minutes, long_break_every, now, id
    )
    .execute(pool)
    .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound { entity: "pomodoro_type", id: id.to_string() });
    }
    Ok(())
}

pub async fn delete_pomodoro_type(pool: &SqlitePool, id: &str) -> Result<(), AppError> {
    let result = sqlx::query!("DELETE FROM pomodoro_types WHERE id = ?", id)
        .execute(pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound { entity: "pomodoro_type", id: id.to_string() });
    }
    tracing::info!(id, "pomodoro type deleted; microtasks referencing it fell back to NULL (default type applies)");
    Ok(())
}

/// One transaction: clear every default flag, then set the new one.
/// Rolls back (keeping the old default) when the id is unknown.
pub async fn set_default_pomodoro_type(pool: &SqlitePool, id: &str) -> Result<(), AppError> {
    let mut tx = pool.begin().await?;
    let now = now_iso8601();
    sqlx::query!(
        "UPDATE pomodoro_types SET is_default = 0, updated_at = ? WHERE is_default = 1",
        now
    )
    .execute(&mut *tx)
    .await?;
    let result = sqlx::query!(
        "UPDATE pomodoro_types SET is_default = 1, updated_at = ? WHERE id = ?",
        now, id
    )
    .execute(&mut *tx)
    .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound { entity: "pomodoro_type", id: id.to_string() });
        // tx drops here -> rollback, the old default survives
    }
    tx.commit().await?;
    Ok(())
}
```

- [ ] **Step 5: Refresh the offline cache and run the tests**

```bash
cd src-tauri && cargo sqlx prepare && cd ..
cargo test --manifest-path src-tauri/Cargo.toml --test pomodoro_type_service
```

Expected: PASS (5 tests).

- [ ] **Step 6: Commit**

```bash
git add src-tauri
git commit -m "feat: pomodoro type model, CRUD + exclusive set_default in one transaction"
```

---

### Task 15: PomodoroType commands `[easy]`

**Files:**
- Create: `src-tauri/src/commands/pomodoro_type.rs`
- Modify: `src-tauri/src/commands/mod.rs`, `src-tauri/src/lib.rs`

- [ ] **Step 1: Write `src-tauri/src/commands/pomodoro_type.rs`** (+ `pub mod pomodoro_type;` in `commands/mod.rs`)

```rust
use crate::commands::log_outcome;
use crate::core::pomodoro_type_service;
use crate::db::Db;
use crate::error::AppError;
use crate::models::pomodoro_type::PomodoroType;

#[tauri::command]
#[tracing::instrument(skip(db))]
pub async fn list_pomodoro_types(
    db: tauri::State<'_, Db>,
) -> Result<Vec<PomodoroType>, AppError> {
    let result = pomodoro_type_service::list_pomodoro_types(&db.0).await;
    match &result {
        Ok(types) => tracing::info!(count = types.len(), "ok"),
        Err(e) => tracing::error!(error = %e, "failed"),
    }
    result
}

#[tauri::command]
#[tracing::instrument(skip(db))]
pub async fn create_pomodoro_type(
    db: tauri::State<'_, Db>,
    id: String,
    name: String,
    work_minutes: i64,
    rest_minutes: i64,
    long_break_minutes: Option<i64>,
    long_break_every: Option<i64>,
) -> Result<(), AppError> {
    let result = pomodoro_type_service::create_pomodoro_type(
        &db.0, &id, &name, work_minutes, rest_minutes, long_break_minutes, long_break_every,
    )
    .await;
    log_outcome(&result);
    result
}

#[tauri::command]
#[tracing::instrument(skip(db))]
pub async fn update_pomodoro_type(
    db: tauri::State<'_, Db>,
    id: String,
    name: String,
    work_minutes: i64,
    rest_minutes: i64,
    long_break_minutes: Option<i64>,
    long_break_every: Option<i64>,
) -> Result<(), AppError> {
    let result = pomodoro_type_service::update_pomodoro_type(
        &db.0, &id, &name, work_minutes, rest_minutes, long_break_minutes, long_break_every,
    )
    .await;
    log_outcome(&result);
    result
}

#[tauri::command]
#[tracing::instrument(skip(db))]
pub async fn delete_pomodoro_type(db: tauri::State<'_, Db>, id: String) -> Result<(), AppError> {
    let result = pomodoro_type_service::delete_pomodoro_type(&db.0, &id).await;
    log_outcome(&result);
    result
}

#[tauri::command]
#[tracing::instrument(skip(db))]
pub async fn set_default_pomodoro_type(
    db: tauri::State<'_, Db>,
    id: String,
) -> Result<(), AppError> {
    let result = pomodoro_type_service::set_default_pomodoro_type(&db.0, &id).await;
    log_outcome(&result);
    result
}
```

- [ ] **Step 2: Register in `src-tauri/src/lib.rs`** — add inside `tauri::generate_handler![...]`:

```rust
            commands::pomodoro_type::list_pomodoro_types,
            commands::pomodoro_type::create_pomodoro_type,
            commands::pomodoro_type::update_pomodoro_type,
            commands::pomodoro_type::delete_pomodoro_type,
            commands::pomodoro_type::set_default_pomodoro_type,
```

- [ ] **Step 3: Verify**

Run: `cargo test --manifest-path src-tauri/Cargo.toml`
Expected: PASS, no new failures.

- [ ] **Step 4: Commit**

```bash
git add src-tauri
git commit -m "feat: pomodoro type IPC commands + list_pomodoro_types query"
```

---

### Task 16: Query — `list_projects` roll-up stats `[medium]`

Spec §5: `list_projects` returns projects **with completion roll-up stats**. Phase 1 returned plain rows; this task upgrades the return type to `ProjectSummary` (all project columns + non-archived microtask totals). The wire shape changes — the TS type follows in Task 19.

**Files:**
- Modify: `src-tauri/src/models/project.rs`, `src-tauri/src/core/project_service.rs`
- Test: `src-tauri/tests/project_queries.rs`

- [ ] **Step 1: Write the failing test** *(test designed by the strongest agent)*

`src-tauri/tests/project_queries.rs`:

```rust
use focus_planner_lib::core::{goal_service, microtask_service, project_service, task_service};
use sqlx::SqlitePool;

async fn seed_tree(pool: &SqlitePool) {
    project_service::create_project(pool, "p1", "P", None).await.unwrap();
    goal_service::create_goal(pool, "g1", "p1", "G", None, None, 0).await.unwrap();
    task_service::create_task(pool, "t1", "g1", "T", None, None, 0).await.unwrap();
    microtask_service::create_microtask(pool, "m1", "t1", "M1", 20, 1, None, None, 0).await.unwrap();
    microtask_service::create_microtask(pool, "m2", "t1", "M2", 20, 1, None, None, 0).await.unwrap();
    microtask_service::create_microtask(pool, "m3", "t1", "M3", 20, 1, None, None, 0).await.unwrap();
}

#[sqlx::test]
async fn list_projects_counts_completed_and_total_microtasks(pool: SqlitePool) {
    seed_tree(&pool).await;
    microtask_service::complete_microtask(&pool, "m1").await.unwrap();

    let projects = project_service::list_projects(&pool, false).await.unwrap();
    assert_eq!(projects.len(), 1);
    assert_eq!(projects[0].total_microtasks, 3);
    assert_eq!(projects[0].completed_microtasks, 1);
}

#[sqlx::test]
async fn archived_microtasks_are_excluded_from_the_stats(pool: SqlitePool) {
    seed_tree(&pool).await;
    microtask_service::archive_microtask(&pool, "m3").await.unwrap();

    let projects = project_service::list_projects(&pool, false).await.unwrap();
    assert_eq!(projects[0].total_microtasks, 2);
}

#[sqlx::test]
async fn empty_project_has_zero_stats(pool: SqlitePool) {
    project_service::create_project(&pool, "p1", "Empty", None).await.unwrap();
    let projects = project_service::list_projects(&pool, false).await.unwrap();
    assert_eq!(projects[0].total_microtasks, 0);
    assert_eq!(projects[0].completed_microtasks, 0);
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test project_queries`
Expected: FAIL to compile — `ProjectSummary` fields don't exist on the return type.

- [ ] **Step 3: Add `ProjectSummary` to `src-tauri/src/models/project.rs`** (keep the existing `Project` struct — the tree query and other callers still use it)

```rust
/// `list_projects` row: project columns + completion roll-up stats (spec §5).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectSummary {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub status: String,
    pub is_archived: bool,
    pub completed_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub total_microtasks: i64,
    pub completed_microtasks: i64,
}
```

- [ ] **Step 4: Rewrite `list_projects` in `src-tauri/src/core/project_service.rs`**

```rust
pub async fn list_projects(
    pool: &SqlitePool,
    include_archived: bool,
) -> Result<Vec<ProjectSummary>, AppError> {
    let projects = sqlx::query_as!(
        ProjectSummary,
        r#"SELECT p.id, p.name, p.description, p.status,
                  p.is_archived as "is_archived: bool",
                  p.completed_at, p.created_at, p.updated_at,
                  (SELECT COUNT(*) FROM microtasks m
                     JOIN tasks t ON m.task_id = t.id
                     JOIN goals g ON t.goal_id = g.id
                    WHERE g.project_id = p.id AND m.is_archived = 0
                  ) as "total_microtasks!: i64",
                  (SELECT COUNT(*) FROM microtasks m
                     JOIN tasks t ON m.task_id = t.id
                     JOIN goals g ON t.goal_id = g.id
                    WHERE g.project_id = p.id AND m.is_archived = 0
                      AND m.status = 'completed'
                  ) as "completed_microtasks!: i64"
           FROM projects p
           WHERE p.is_archived = 0 OR ?1 = 1
           ORDER BY p.created_at"#,
        include_archived
    )
    .fetch_all(pool)
    .await?;
    Ok(projects)
}
```

Update the `use` line to import `ProjectSummary`, and fix the Phase 1 test in `src-tauri/tests/project_commands.rs` if it referenced fields by the old type (the assertions on `name` keep working — only the type name changes).

- [ ] **Step 5: Refresh the offline cache and run**

```bash
cd src-tauri && cargo sqlx prepare && cd ..
cargo test --manifest-path src-tauri/Cargo.toml
```

Expected: PASS — all suites, including the 3 new tests.

- [ ] **Step 6: Commit**

```bash
git add src-tauri
git commit -m "feat: list_projects returns completion roll-up stats per spec §5"
```

---

### Task 17: Query — `get_project_tree` `[medium]`

Returns the full nested tree (spec §5): Project → Goals → Tasks → Microtasks, each level ordered by `sort_order`, **non-archived rows only** (loud flag 5: the tree the user drags shows only non-archived rows). Assembled from 4 flat queries — no N+1, no recursive SQL.

**Files:**
- Create: `src-tauri/src/models/tree.rs`
- Modify: `src-tauri/src/models/mod.rs`, `src-tauri/src/core/project_service.rs`
- Test: `src-tauri/tests/project_queries.rs` (append)

- [ ] **Step 1: Write the failing test** *(test designed by the strongest agent)* — append to `src-tauri/tests/project_queries.rs`:

```rust
#[sqlx::test]
async fn get_project_tree_nests_all_levels_in_sort_order(pool: SqlitePool) {
    seed_tree(&pool).await;
    goal_service::create_goal(&pool, "g2", "p1", "G2", None, None, 0).await.unwrap();
    goal_service::reorder_goals(&pool, "p1", &["g2".to_string(), "g1".to_string()])
        .await
        .unwrap();

    let tree = project_service::get_project_tree(&pool, "p1").await.unwrap();
    assert_eq!(tree.id, "p1");
    assert_eq!(tree.goals.len(), 2);
    assert_eq!(tree.goals[0].id, "g2", "goals come back in sort order");
    assert_eq!(tree.goals[1].tasks.len(), 1);
    assert_eq!(tree.goals[1].tasks[0].microtasks.len(), 3);
    assert_eq!(tree.goals[1].tasks[0].microtasks[0].id, "m1");
}

#[sqlx::test]
async fn get_project_tree_excludes_archived_rows(pool: SqlitePool) {
    seed_tree(&pool).await;
    microtask_service::archive_microtask(&pool, "m2").await.unwrap();

    let tree = project_service::get_project_tree(&pool, "p1").await.unwrap();
    let micro_ids: Vec<&str> = tree.goals[0].tasks[0]
        .microtasks
        .iter()
        .map(|m| m.id.as_str())
        .collect();
    assert_eq!(micro_ids, ["m1", "m3"]);
}

#[sqlx::test]
async fn get_project_tree_unknown_project_is_not_found(pool: SqlitePool) {
    let err = project_service::get_project_tree(&pool, "ghost").await.unwrap_err();
    assert!(matches!(
        err,
        focus_planner_lib::error::AppError::NotFound { entity: "project", .. }
    ));
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test project_queries`
Expected: FAIL to compile — `get_project_tree` doesn't exist.

- [ ] **Step 3: Write `src-tauri/src/models/tree.rs`** (+ `pub mod tree;` in `models/mod.rs`)

Self-contained DTOs for the tree wire shape — deliberately separate from the flat row models (the tree never carries archived rows or audit columns):

```rust
use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectTree {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub status: String,
    pub goals: Vec<TreeGoal>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TreeGoal {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub deadline: Option<String>,
    pub priority: i64,
    pub status: String,
    pub tasks: Vec<TreeTask>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TreeTask {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub deadline: Option<String>,
    pub priority: i64,
    pub status: String,
    pub microtasks: Vec<TreeMicrotask>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TreeMicrotask {
    pub id: String,
    pub title: String,
    pub estimated_minutes: i64,
    pub pomodoro_count: i64,
    pub pomodoro_type_id: Option<String>,
    pub deadline: Option<String>,
    pub priority: i64,
    pub status: String,
}
```

- [ ] **Step 4: Add `get_project_tree` to `src-tauri/src/core/project_service.rs`**

```rust
use crate::models::tree::{ProjectTree, TreeGoal, TreeMicrotask, TreeTask};

pub async fn get_project_tree(pool: &SqlitePool, project_id: &str) -> Result<ProjectTree, AppError> {
    let project = sqlx::query!(
        r#"SELECT id as "id!: String", name, description, status
           FROM projects WHERE id = ?1"#,
        project_id
    )
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound { entity: "project", id: project_id.to_string() })?;

    let goal_rows = sqlx::query!(
        r#"SELECT id as "id!: String", title, description, deadline, priority, status
           FROM goals WHERE project_id = ?1 AND is_archived = 0 ORDER BY sort_order"#,
        project_id
    )
    .fetch_all(pool)
    .await?;

    let task_rows = sqlx::query!(
        r#"SELECT t.id as "id!: String", t.goal_id as "goal_id!: String",
                  t.title, t.description, t.deadline, t.priority, t.status
           FROM tasks t JOIN goals g ON t.goal_id = g.id
           WHERE g.project_id = ?1 AND t.is_archived = 0 AND g.is_archived = 0
           ORDER BY t.sort_order"#,
        project_id
    )
    .fetch_all(pool)
    .await?;

    let micro_rows = sqlx::query!(
        r#"SELECT m.id as "id!: String", m.task_id as "task_id!: String",
                  m.title, m.estimated_minutes, m.pomodoro_count,
                  m.pomodoro_type_id, m.deadline, m.priority, m.status
           FROM microtasks m
           JOIN tasks t ON m.task_id = t.id
           JOIN goals g ON t.goal_id = g.id
           WHERE g.project_id = ?1 AND m.is_archived = 0
             AND t.is_archived = 0 AND g.is_archived = 0
           ORDER BY m.sort_order"#,
        project_id
    )
    .fetch_all(pool)
    .await?;

    let mut tasks_by_goal: std::collections::HashMap<String, Vec<TreeTask>> =
        std::collections::HashMap::new();
    let mut micros_by_task: std::collections::HashMap<String, Vec<TreeMicrotask>> =
        std::collections::HashMap::new();

    for m in micro_rows {
        micros_by_task.entry(m.task_id).or_default().push(TreeMicrotask {
            id: m.id,
            title: m.title,
            estimated_minutes: m.estimated_minutes,
            pomodoro_count: m.pomodoro_count,
            pomodoro_type_id: m.pomodoro_type_id,
            deadline: m.deadline,
            priority: m.priority,
            status: m.status,
        });
    }
    for t in task_rows {
        let microtasks = micros_by_task.remove(&t.id).unwrap_or_default();
        tasks_by_goal.entry(t.goal_id).or_default().push(TreeTask {
            id: t.id,
            title: t.title,
            description: t.description,
            deadline: t.deadline,
            priority: t.priority,
            status: t.status,
            microtasks,
        });
    }
    let goals = goal_rows
        .into_iter()
        .map(|g| TreeGoal {
            tasks: tasks_by_goal.remove(&g.id).unwrap_or_default(),
            id: g.id,
            title: g.title,
            description: g.description,
            deadline: g.deadline,
            priority: g.priority,
            status: g.status,
        })
        .collect();

    Ok(ProjectTree {
        id: project.id,
        name: project.name,
        description: project.description,
        status: project.status,
        goals,
    })
}
```

- [ ] **Step 5: Refresh the offline cache and run**

```bash
cd src-tauri && cargo sqlx prepare && cd ..
cargo test --manifest-path src-tauri/Cargo.toml --test project_queries
```

Expected: PASS — 6 tests.

- [ ] **Step 6: Commit**

```bash
git add src-tauri
git commit -m "feat: get_project_tree nested query over non-archived rows"
```

---

### Task 18: Query — `get_microtask` + query IPC handlers `[easy]`

**Files:**
- Modify: `src-tauri/src/core/microtask_service.rs`, `src-tauri/src/commands/project.rs`, `src-tauri/src/commands/microtask.rs`, `src-tauri/src/lib.rs`
- Test: `src-tauri/tests/microtask_service.rs` (append)

- [ ] **Step 1: Write the failing test** *(test designed by the strongest agent)* — append to `src-tauri/tests/microtask_service.rs`:

```rust
#[sqlx::test]
async fn get_microtask_returns_the_row_or_not_found(pool: SqlitePool) {
    seed_task(&pool).await;
    microtask_service::create_microtask(&pool, "m1", "t1", "Read", 40, 2, None, None, 0)
        .await
        .unwrap();

    let m = microtask_service::get_microtask(&pool, "m1").await.unwrap();
    assert_eq!(m.title, "Read");
    assert_eq!(m.estimated_minutes, 40);
    assert_eq!(m.pomodoro_count, 2);

    let err = microtask_service::get_microtask(&pool, "ghost").await.unwrap_err();
    assert!(matches!(err, AppError::NotFound { entity: "microtask", .. }));
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test microtask_service`
Expected: FAIL to compile — `get_microtask` doesn't exist.

- [ ] **Step 3: Add `get_microtask` to `src-tauri/src/core/microtask_service.rs`** (returns the `Microtask` model from Task 9)

```rust
pub async fn get_microtask(pool: &SqlitePool, id: &str) -> Result<Microtask, AppError> {
    sqlx::query_as!(
        Microtask,
        r#"SELECT id, task_id, title, estimated_minutes, pomodoro_count,
                  pomodoro_type_id, deadline, priority, sort_order, status,
                  is_archived as "is_archived: bool",
                  completed_at, created_at, updated_at
           FROM microtasks WHERE id = ?1"#,
        id
    )
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound { entity: "microtask", id: id.to_string() })
}
```

- [ ] **Step 4: Add the query IPC handlers** — `get_project_tree` in `src-tauri/src/commands/project.rs`, `get_microtask` in `src-tauri/src/commands/microtask.rs` (queries return data; `log_outcome` still applies):

```rust
#[tauri::command]
#[tracing::instrument(skip(db))]
pub async fn get_project_tree(
    db: tauri::State<'_, Db>,
    project_id: String,
) -> Result<crate::models::tree::ProjectTree, AppError> {
    let result = project_service::get_project_tree(&db.0, &project_id).await;
    log_outcome(&result);
    result
}
```

```rust
#[tauri::command]
#[tracing::instrument(skip(db))]
pub async fn get_microtask(
    db: tauri::State<'_, Db>,
    id: String,
) -> Result<crate::models::microtask::Microtask, AppError> {
    let result = microtask_service::get_microtask(&db.0, &id).await;
    log_outcome(&result);
    result
}
```

Register both in `src-tauri/src/lib.rs` inside `tauri::generate_handler![...]`:

```rust
            commands::project::get_project_tree,
            commands::microtask::get_microtask,
```

- [ ] **Step 5: Refresh the cache, run everything**

```bash
cd src-tauri && cargo sqlx prepare && cd ..
cargo test --manifest-path src-tauri/Cargo.toml
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add src-tauri
git commit -m "feat: get_microtask query + query IPC handlers registered"
```

---

### Task 19: TS domain types `[easy]`

**Files:**
- Modify: `src/ipc/types.ts`

- [ ] **Step 1: Append the Phase 2 types** (camelCase mirrors of the Rust wire models)

```ts
export interface ProjectSummary {
  id: string;
  name: string;
  description: string | null;
  status: "open" | "completed";
  isArchived: boolean;
  completedAt: string | null;
  createdAt: string;
  updatedAt: string;
  totalMicrotasks: number;
  completedMicrotasks: number;
}

export interface TreeMicrotask {
  id: string;
  title: string;
  estimatedMinutes: number;
  pomodoroCount: number;
  pomodoroTypeId: string | null;
  deadline: string | null;
  priority: number;
  status: "open" | "completed";
}

export interface TreeTask {
  id: string;
  title: string;
  description: string | null;
  deadline: string | null;
  priority: number;
  status: "open" | "completed";
  microtasks: TreeMicrotask[];
}

export interface TreeGoal {
  id: string;
  title: string;
  description: string | null;
  deadline: string | null;
  priority: number;
  status: "open" | "completed";
  tasks: TreeTask[];
}

export interface ProjectTree {
  id: string;
  name: string;
  description: string | null;
  status: "open" | "completed";
  goals: TreeGoal[];
}

export interface PomodoroType {
  id: string;
  name: string;
  workMinutes: number;
  restMinutes: number;
  longBreakMinutes: number | null;
  longBreakEvery: number | null;
  isDefault: boolean;
}
```

`useProjectStore.projects` switches from `Project[]` to `ProjectSummary[]` in Task 21; the `Project` interface from Phase 1 stays for now (removed only if nothing references it — check with `vue-tsc`).

- [ ] **Step 2: Verify**

Run: `npx vue-tsc --noEmit`
Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add src/ipc/types.ts
git commit -m "feat: TS wire types for backlog tree, project stats, pomodoro types"
```

---

### Task 20: `usePomodoroTypeStore` `[easy]`

**Files:**
- Create: `src/stores/pomodoroTypeStore.ts`
- Test: `src/stores/pomodoroTypeStore.test.ts`

- [ ] **Step 1: Write the failing test** *(test designed by the strongest agent)*

`src/stores/pomodoroTypeStore.test.ts`:

```ts
import { beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { mockIPC } from "@tauri-apps/api/mocks";
import { usePomodoroTypeStore } from "./pomodoroTypeStore";

const standard = {
  id: "pt-std", name: "Standard", workMinutes: 20, restMinutes: 5,
  longBreakMinutes: null, longBreakEvery: null, isDefault: true,
};

describe("usePomodoroTypeStore", () => {
  beforeEach(() => setActivePinia(createPinia()));

  it("loadTypes fills state and exposes the default type", async () => {
    mockIPC((cmd) => {
      if (cmd === "list_pomodoro_types") return [standard];
    });
    const store = usePomodoroTypeStore();
    await store.loadTypes();
    expect(store.types).toHaveLength(1);
    expect(store.defaultType?.id).toBe("pt-std");
  });

  it("createType sends a generated id and re-queries (CQS)", async () => {
    const calls: string[] = [];
    mockIPC((cmd, args) => {
      calls.push(cmd);
      if (cmd === "create_pomodoro_type") {
        const a = args as Record<string, unknown>;
        expect(typeof a.id).toBe("string");
        expect((a.id as string).length).toBeGreaterThan(10);
        expect(a.name).toBe("Deep");
        expect(a.workMinutes).toBe(50);
        return null;
      }
      if (cmd === "list_pomodoro_types") return [standard];
    });
    const store = usePomodoroTypeStore();
    await store.createType({ name: "Deep", workMinutes: 50, restMinutes: 10, longBreakMinutes: null, longBreakEvery: null });
    expect(calls).toContain("create_pomodoro_type");
    expect(calls.filter((c) => c === "list_pomodoro_types")).toHaveLength(1);
  });

  it("records the error message when a mutation is rejected", async () => {
    mockIPC((cmd) => {
      if (cmd === "set_default_pomodoro_type") throw { code: "not_found", message: "pomodoro_type not found: ghost" };
      if (cmd === "list_pomodoro_types") return [standard];
    });
    const store = usePomodoroTypeStore();
    await store.setDefault("ghost");
    expect(store.error).toContain("not found");
  });
});
```

- [ ] **Step 2: Run to verify it fails**

Run: `npm test -- --run`
Expected: FAIL — `./pomodoroTypeStore` doesn't exist.

- [ ] **Step 3: Write `src/stores/pomodoroTypeStore.ts`**

```ts
import { defineStore } from "pinia";
import { ipc } from "../ipc/client";
import type { IpcError, PomodoroType } from "../ipc/types";

export interface PomodoroTypeDraft {
  name: string;
  workMinutes: number;
  restMinutes: number;
  longBreakMinutes: number | null;
  longBreakEvery: number | null;
}

export const usePomodoroTypeStore = defineStore("pomodoroType", {
  state: () => ({
    types: [] as PomodoroType[],
    loading: false,
    error: null as string | null,
  }),
  getters: {
    defaultType: (s) => s.types.find((t) => t.isDefault) ?? null,
  },
  actions: {
    async loadTypes() {
      this.loading = true;
      this.error = null;
      try {
        this.types = await ipc<PomodoroType[]>("list_pomodoro_types");
      } catch (e) {
        this.error = (e as IpcError).message ?? String(e);
      } finally {
        this.loading = false;
      }
    },
    // CQS: every mutation returns nothing; state refreshes by re-querying.
    async mutate(cmd: string, args: Record<string, unknown>) {
      this.error = null;
      try {
        await ipc<void>(cmd, args);
        await this.loadTypes();
      } catch (e) {
        this.error = (e as IpcError).message ?? String(e);
      }
    },
    async createType(draft: PomodoroTypeDraft) {
      await this.mutate("create_pomodoro_type", { id: crypto.randomUUID(), ...draft });
    },
    async updateType(id: string, draft: PomodoroTypeDraft) {
      await this.mutate("update_pomodoro_type", { id, ...draft });
    },
    async deleteType(id: string) {
      await this.mutate("delete_pomodoro_type", { id });
    },
    async setDefault(id: string) {
      await this.mutate("set_default_pomodoro_type", { id });
    },
  },
});
```

- [ ] **Step 4: Run to verify it passes**

Run: `npm test -- --run`
Expected: PASS — 3 new tests. (The error-path test passes because `mutate` catches, records, and `loadTypes` is never reached after a throw.)

- [ ] **Step 5: Commit**

```bash
git add src/stores
git commit -m "feat: usePomodoroTypeStore with CQS mutate-then-requery pattern"
```

---

### Task 21: Extend `useProjectStore` — tree + mutation wrappers `[medium]`

**Files:**
- Modify: `src/stores/projectStore.ts`
- Test: `src/stores/projectStore.test.ts` (extend)

- [ ] **Step 1: Write the failing tests** *(test designed by the strongest agent)* — append to `src/stores/projectStore.test.ts`:

```ts
const emptyTree = {
  id: "p1", name: "My project", description: null, status: "open", goals: [],
};

it("loadProjectTree fills activeProjectTree", async () => {
  mockIPC((cmd, args) => {
    if (cmd === "get_project_tree") {
      expect((args as Record<string, unknown>).projectId).toBe("p1");
      return emptyTree;
    }
  });
  const store = useProjectStore();
  await store.loadProjectTree("p1");
  expect(store.activeProjectTree?.id).toBe("p1");
});

it("createGoal generates an id, then refreshes the active tree (CQS)", async () => {
  const calls: string[] = [];
  mockIPC((cmd, args) => {
    calls.push(cmd);
    if (cmd === "create_goal") {
      const a = args as Record<string, unknown>;
      expect(typeof a.id).toBe("string");
      expect(a.projectId).toBe("p1");
      expect(a.title).toBe("Ship it");
      return null;
    }
    if (cmd === "get_project_tree") return emptyTree;
    if (cmd === "list_projects") return [];
  });
  const store = useProjectStore();
  await store.loadProjectTree("p1");
  await store.createGoal("p1", "Ship it");
  expect(calls).toEqual(
    expect.arrayContaining(["create_goal", "get_project_tree"]),
  );
});

it("completeMicrotask refreshes both the tree and the project stats", async () => {
  const calls: string[] = [];
  mockIPC((cmd) => {
    calls.push(cmd);
    if (cmd === "complete_microtask") return null;
    if (cmd === "get_project_tree") return emptyTree;
    if (cmd === "list_projects") return [];
  });
  const store = useProjectStore();
  await store.loadProjectTree("p1");
  await store.completeMicrotask("m1");
  expect(calls).toContain("complete_microtask");
  expect(calls.filter((c) => c === "get_project_tree").length).toBeGreaterThanOrEqual(2);
  expect(calls).toContain("list_projects");
});
```

- [ ] **Step 2: Run to verify it fails**

Run: `npm test -- --run`
Expected: FAIL — `loadProjectTree` / `createGoal` / `completeMicrotask` don't exist.

- [ ] **Step 3: Rewrite `src/stores/projectStore.ts`**

```ts
import { defineStore } from "pinia";
import { ipc } from "../ipc/client";
import type { IpcError, ProjectSummary, ProjectTree } from "../ipc/types";

export const useProjectStore = defineStore("project", {
  state: () => ({
    projects: [] as ProjectSummary[],
    activeProjectTree: null as ProjectTree | null,
    loading: false,
    error: null as string | null,
  }),
  actions: {
    async loadProjects(includeArchived = false) {
      this.loading = true;
      this.error = null;
      try {
        this.projects = await ipc<ProjectSummary[]>("list_projects", { includeArchived });
      } catch (e) {
        this.error = (e as IpcError).message ?? String(e);
      } finally {
        this.loading = false;
      }
    },
    async loadProjectTree(projectId: string) {
      this.error = null;
      try {
        this.activeProjectTree = await ipc<ProjectTree>("get_project_tree", { projectId });
      } catch (e) {
        this.error = (e as IpcError).message ?? String(e);
      }
    },
    // CQS: mutate, then re-query the tree (and the stats list — counts change).
    async mutate(cmd: string, args: Record<string, unknown>) {
      this.error = null;
      try {
        await ipc<void>(cmd, args);
        if (this.activeProjectTree) await this.loadProjectTree(this.activeProjectTree.id);
        await this.loadProjects();
      } catch (e) {
        this.error = (e as IpcError).message ?? String(e);
      }
    },

    // ---- projects ----
    async createProject(name: string, description: string | null = null) {
      await this.mutate("create_project", { id: crypto.randomUUID(), name, description });
    },
    async updateProject(id: string, name: string, description: string | null) {
      await this.mutate("update_project", { id, name, description });
    },
    async archiveProject(id: string) {
      await this.mutate("archive_project", { id });
    },
    async deleteProject(id: string) {
      if (this.activeProjectTree?.id === id) this.activeProjectTree = null;
      await this.mutate("delete_project", { id });
    },

    // ---- goals ----
    async createGoal(projectId: string, title: string) {
      await this.mutate("create_goal", {
        id: crypto.randomUUID(), projectId, title,
        description: null, deadline: null, priority: 0,
      });
    },
    async updateGoal(id: string, title: string, description: string | null, deadline: string | null, priority: number) {
      await this.mutate("update_goal", { id, title, description, deadline, priority });
    },
    async archiveGoal(id: string) {
      await this.mutate("archive_goal", { id });
    },
    async deleteGoal(id: string) {
      await this.mutate("delete_goal", { id });
    },
    async reorderGoals(projectId: string, orderedIds: string[]) {
      await this.mutate("reorder_goals", { projectId, orderedIds });
    },

    // ---- tasks ----
    async createTask(goalId: string, title: string) {
      await this.mutate("create_task", {
        id: crypto.randomUUID(), goalId, title,
        description: null, deadline: null, priority: 0,
      });
    },
    async updateTask(id: string, title: string, description: string | null, deadline: string | null, priority: number) {
      await this.mutate("update_task", { id, title, description, deadline, priority });
    },
    async archiveTask(id: string) {
      await this.mutate("archive_task", { id });
    },
    async deleteTask(id: string) {
      await this.mutate("delete_task", { id });
    },
    async reorderTasks(goalId: string, orderedIds: string[]) {
      await this.mutate("reorder_tasks", { goalId, orderedIds });
    },

    // ---- microtasks ----
    async createMicrotask(taskId: string, title: string, estimatedMinutes: number, pomodoroCount: number, pomodoroTypeId: string | null) {
      await this.mutate("create_microtask", {
        id: crypto.randomUUID(), taskId, title,
        estimatedMinutes, pomodoroCount, pomodoroTypeId,
        deadline: null, priority: 0,
      });
    },
    async updateMicrotask(id: string, title: string, estimatedMinutes: number, pomodoroCount: number, pomodoroTypeId: string | null, deadline: string | null, priority: number) {
      await this.mutate("update_microtask", { id, title, estimatedMinutes, pomodoroCount, pomodoroTypeId, deadline, priority });
    },
    async completeMicrotask(id: string) {
      await this.mutate("complete_microtask", { id });
    },
    async uncompleteMicrotask(id: string) {
      await this.mutate("uncomplete_microtask", { id });
    },
    async archiveMicrotask(id: string) {
      await this.mutate("archive_microtask", { id });
    },
    async deleteMicrotask(id: string) {
      await this.mutate("delete_microtask", { id });
    },
    async reorderMicrotasks(taskId: string, orderedIds: string[]) {
      await this.mutate("reorder_microtasks", { taskId, orderedIds });
    },
  },
});
```

- [ ] **Step 4: Run to verify it passes**

Run: `npm test -- --run`
Expected: PASS — all store tests (Phase 1's two still green: the state shape they assert is unchanged).

- [ ] **Step 5: Commit**

```bash
git add src/stores
git commit -m "feat: project store tree loading + CQS mutation wrappers for the whole backlog"
```

---

### Task 22: Quick-estimation helper `[easy]`

The pomodoro-count auto-computation (spec §6: "quick-estimation inputs … auto-computing pomodoro count"). Pure function, frontend-owned (Task 9 backend stores what it is told).

**Files:**
- Create: `src/lib/estimation.ts`
- Test: `src/lib/estimation.test.ts`

- [ ] **Step 1: Write the failing test** *(test designed by the strongest agent)*

`src/lib/estimation.test.ts`:

```ts
import { describe, expect, it } from "vitest";
import { computePomodoroCount } from "./estimation";

describe("computePomodoroCount", () => {
  it("divides the estimate by the work length, rounding up", () => {
    expect(computePomodoroCount(40, 20)).toBe(2);
    expect(computePomodoroCount(45, 20)).toBe(3);
    expect(computePomodoroCount(50, 50)).toBe(1);
  });
  it("never returns less than one pomodoro", () => {
    expect(computePomodoroCount(5, 20)).toBe(1);
    expect(computePomodoroCount(0, 20)).toBe(1);
  });
  it("falls back to the spec's 20-minute work length when none is known", () => {
    expect(computePomodoroCount(60, null)).toBe(3);
  });
});
```

- [ ] **Step 2: Run to verify it fails**

Run: `npm test -- --run`
Expected: FAIL — `./estimation` doesn't exist.

- [ ] **Step 3: Write `src/lib/estimation.ts`**

```ts
/** Spec §2: if no pomodoro type resolves, fall back to 20 minutes of work. */
const FALLBACK_WORK_MINUTES = 20;

export function computePomodoroCount(
  estimatedMinutes: number,
  workMinutes: number | null,
): number {
  const work = workMinutes && workMinutes > 0 ? workMinutes : FALLBACK_WORK_MINUTES;
  if (estimatedMinutes <= 0) return 1;
  return Math.max(1, Math.ceil(estimatedMinutes / work));
}
```

- [ ] **Step 4: Run to verify it passes**

Run: `npm test -- --run`
Expected: PASS — 3 new tests.

- [ ] **Step 5: Commit**

```bash
git add src/lib
git commit -m "feat: pomodoro-count quick-estimation helper with 20-minute fallback"
```

---

### Task 23: Tree leaf components — `MicrotaskRow` + `InlineCreate` `[medium]`

**Files:**
- Create: `src/components/backlog/MicrotaskRow.vue`, `src/components/backlog/InlineCreate.vue`
- Modify: `vitest.config.ts`, `package.json` (dev deps)

`InlineCreate` is one shared input row (Enter = create, Esc = clear) reused at all four levels — a justified abstraction: identical behavior, four call sites.

- [ ] **Step 1: Add the component-test toolchain**

```bash
npm install -D @vue/test-utils @vitejs/plugin-vue
```

`vitest.config.ts` gains the Vue plugin so tests can mount SFCs:

```ts
import vue from "@vitejs/plugin-vue";
import { defineConfig } from "vitest/config";

export default defineConfig({
  plugins: [vue()],
  test: { environment: "jsdom" },
});
```

- [ ] **Step 2: Write `src/components/backlog/InlineCreate.vue`**

```vue
<script setup lang="ts">
import { ref } from "vue";

defineProps<{ placeholder: string }>();
const emit = defineEmits<{ create: [title: string] }>();
const draft = ref("");

function submit() {
  const title = draft.value.trim();
  if (!title) return;
  emit("create", title);
  draft.value = "";
}
</script>

<template>
  <input
    v-model="draft"
    class="inline-create"
    :placeholder="placeholder"
    @keydown.enter="submit"
    @keydown.esc="draft = ''"
  />
</template>

<style scoped>
.inline-create {
  width: 100%;
  background: transparent;
  border: 1px dashed #2a313c;
  border-radius: 6px;
  color: #e6e9ef;
  padding: 6px 10px;
  font-size: 13px;
}
.inline-create:focus { border-style: solid; outline: none; border-color: #3d4756; }
</style>
```

- [ ] **Step 3: Write `src/components/backlog/MicrotaskRow.vue`**

```vue
<script setup lang="ts">
import type { TreeMicrotask } from "../../ipc/types";
import { useProjectStore } from "../../stores/projectStore";

const props = defineProps<{ microtask: TreeMicrotask }>();
const store = useProjectStore();

function toggle() {
  if (props.microtask.status === "completed") {
    store.uncompleteMicrotask(props.microtask.id);
  } else {
    store.completeMicrotask(props.microtask.id);
  }
}
</script>

<template>
  <div class="microtask" :class="{ done: microtask.status === 'completed' }">
    <span class="drag-handle">⋮⋮</span>
    <input
      type="checkbox"
      :checked="microtask.status === 'completed'"
      @change="toggle"
    />
    <span class="title">{{ microtask.title }}</span>
    <span class="meta">{{ microtask.estimatedMinutes }}m · {{ microtask.pomodoroCount }}🍅</span>
    <button class="ghost" title="Archive" @click="store.archiveMicrotask(microtask.id)">⌫</button>
  </div>
</template>

<style scoped>
.microtask { display: flex; align-items: center; gap: 8px; padding: 4px 0; }
.microtask.done .title { text-decoration: line-through; color: #6b7484; }
.meta { color: #9aa3b2; font-size: 12px; margin-left: auto; }
.drag-handle { cursor: grab; color: #4a5260; user-select: none; }
.ghost { background: none; border: none; color: #6b7484; cursor: pointer; }
</style>
```

- [ ] **Step 4: Verify**

Run: `npx vue-tsc --noEmit && npm test -- --run`
Expected: clean typecheck, all tests still pass.

- [ ] **Step 5: Commit**

```bash
git add src/components vitest.config.ts package.json package-lock.json
git commit -m "feat: backlog leaf components — microtask row + shared inline-create input"
```

---

### Task 24: Draggable tree — `TaskNode` + `GoalNode` `[hard]`

**Files:**
- Create: `src/components/backlog/TaskNode.vue`, `src/components/backlog/GoalNode.vue`
- Modify: `package.json`
- Test: `src/components/backlog/GoalNode.test.ts`

**Why `vuedraggable@next`:** the de-facto Vue 3 sortable wrapper (SortableJS underneath) — handles nested lists, drag handles, and emits a clean post-drop list. Hand-rolling HTML5 DnD for a 3-level tree is the wrong place to spend novelty (AHA).

- [ ] **Step 1: Install**

```bash
npm install vuedraggable@next
```

- [ ] **Step 2: Write the failing component test** *(test designed by the strongest agent)*

The dense logic worth testing is the reorder emission: after a drop, the component must send the **full ordered id list** to the store. We test the handler directly (simulating real drags in jsdom is testing SortableJS, not our code).

`src/components/backlog/GoalNode.test.ts`:

```ts
import { describe, expect, it, vi } from "vitest";
import { mount } from "@vue/test-utils";
import { createTestingPinia } from "@pinia/testing";
import GoalNode from "./GoalNode.vue";
import { useProjectStore } from "../../stores/projectStore";

const goal = {
  id: "g1", title: "Goal", description: null, deadline: null,
  priority: 0, status: "open" as const,
  tasks: [
    { id: "t1", title: "A", description: null, deadline: null, priority: 0, status: "open" as const, microtasks: [] },
    { id: "t2", title: "B", description: null, deadline: null, priority: 0, status: "open" as const, microtasks: [] },
  ],
};

it("sends the full ordered task id list after a drop", async () => {
  const wrapper = mount(GoalNode, {
    props: { goal },
    global: { plugins: [createTestingPinia({ createSpy: vi.fn })] },
  });
  const store = useProjectStore();

  // simulate vuedraggable's post-drop state: local list already reordered
  wrapper.vm.localTasks.reverse();
  await wrapper.vm.onTaskDrop();

  expect(store.reorderTasks).toHaveBeenCalledWith("g1", ["t2", "t1"]);
});
```

Add the testing-pinia dev dep it uses:

```bash
npm install -D @pinia/testing
```

- [ ] **Step 3: Run to verify it fails**

Run: `npm test -- --run`
Expected: FAIL — `GoalNode.vue` doesn't exist.

- [ ] **Step 4: Write `src/components/backlog/TaskNode.vue`**

```vue
<script setup lang="ts">
import { ref, watch } from "vue";
import draggable from "vuedraggable";
import type { TreeTask } from "../../ipc/types";
import { useProjectStore } from "../../stores/projectStore";
import { usePomodoroTypeStore } from "../../stores/pomodoroTypeStore";
import { computePomodoroCount } from "../../lib/estimation";
import MicrotaskRow from "./MicrotaskRow.vue";
import InlineCreate from "./InlineCreate.vue";

const props = defineProps<{ task: TreeTask }>();
const store = useProjectStore();
const typeStore = usePomodoroTypeStore();

const localMicrotasks = ref([...props.task.microtasks]);
watch(() => props.task.microtasks, (m) => (localMicrotasks.value = [...m]));

async function onMicrotaskDrop() {
  await store.reorderMicrotasks(
    props.task.id,
    localMicrotasks.value.map((m) => m.id),
  );
}

// Quick estimation: "Outline the doc 45" -> 45 estimated minutes, count auto-computed
// from the default pomodoro type's work length (spec §6).
async function createMicrotask(text: string) {
  const match = text.match(/^(.*?)\s+(\d+)$/);
  const title = match ? match[1] : text;
  const estimated = match ? Number(match[2]) : 20;
  const workMinutes = typeStore.defaultType?.workMinutes ?? null;
  await store.createMicrotask(
    props.task.id,
    title,
    estimated,
    computePomodoroCount(estimated, workMinutes),
    null,
  );
}

defineExpose({ localMicrotasks, onMicrotaskDrop });
</script>

<template>
  <details class="task" open>
    <summary>
      <span class="drag-handle">⋮⋮</span>
      <span :class="{ done: task.status === 'completed' }">{{ task.title }}</span>
      <button class="ghost" title="Archive" @click.prevent="store.archiveTask(task.id)">⌫</button>
    </summary>
    <draggable
      v-model="localMicrotasks"
      item-key="id"
      handle=".drag-handle"
      @end="onMicrotaskDrop"
    >
      <template #item="{ element }">
        <MicrotaskRow :microtask="element" />
      </template>
    </draggable>
    <InlineCreate
      placeholder="New microtask — append minutes to estimate, e.g. “Outline 45”"
      @create="createMicrotask"
    />
  </details>
</template>

<style scoped>
.task { margin-left: 16px; padding: 2px 0; }
summary { display: flex; align-items: center; gap: 8px; cursor: pointer; list-style: none; }
.done { text-decoration: line-through; color: #6b7484; }
.drag-handle { cursor: grab; color: #4a5260; user-select: none; }
.ghost { background: none; border: none; color: #6b7484; cursor: pointer; margin-left: auto; }
</style>
```

- [ ] **Step 5: Write `src/components/backlog/GoalNode.vue`**

```vue
<script setup lang="ts">
import { ref, watch } from "vue";
import draggable from "vuedraggable";
import type { TreeGoal } from "../../ipc/types";
import { useProjectStore } from "../../stores/projectStore";
import TaskNode from "./TaskNode.vue";
import InlineCreate from "./InlineCreate.vue";

const props = defineProps<{ goal: TreeGoal }>();
const store = useProjectStore();

const localTasks = ref([...props.goal.tasks]);
watch(() => props.goal.tasks, (t) => (localTasks.value = [...t]));

async function onTaskDrop() {
  await store.reorderTasks(props.goal.id, localTasks.value.map((t) => t.id));
}

defineExpose({ localTasks, onTaskDrop });
</script>

<template>
  <details class="goal" open>
    <summary>
      <span class="drag-handle">⋮⋮</span>
      <strong :class="{ done: goal.status === 'completed' }">{{ goal.title }}</strong>
      <button class="ghost" title="Archive" @click.prevent="store.archiveGoal(goal.id)">⌫</button>
    </summary>
    <draggable v-model="localTasks" item-key="id" handle=".drag-handle" @end="onTaskDrop">
      <template #item="{ element }">
        <TaskNode :task="element" />
      </template>
    </draggable>
    <InlineCreate placeholder="New task" @create="(t) => store.createTask(goal.id, t)" />
  </details>
</template>

<style scoped>
.goal { margin-left: 8px; padding: 4px 0; }
summary { display: flex; align-items: center; gap: 8px; cursor: pointer; list-style: none; }
.done { text-decoration: line-through; color: #6b7484; }
.drag-handle { cursor: grab; color: #4a5260; user-select: none; }
.ghost { background: none; border: none; color: #6b7484; cursor: pointer; margin-left: auto; }
</style>
```

- [ ] **Step 6: Run to verify it passes**

Run: `npm test -- --run`
Expected: PASS — the GoalNode reorder-emission test is green.

- [ ] **Step 7: Commit**

```bash
git add src/components package.json package-lock.json
git commit -m "feat: draggable goal/task tree nodes with full-list reorder emission"
```

---

### Task 25: `BacklogView` assembly `[medium]`

Replaces Phase 1's empty state with the real two-pane backlog: project list (left), active project's tree (right). Inline creation at every level; goal reordering at the top level.

**Files:**
- Modify: `src/views/BacklogView.vue`

- [ ] **Step 1: Rewrite `src/views/BacklogView.vue`**

```vue
<script setup lang="ts">
import { onMounted, ref, watch } from "vue";
import draggable from "vuedraggable";
import { useProjectStore } from "../stores/projectStore";
import { usePomodoroTypeStore } from "../stores/pomodoroTypeStore";
import GoalNode from "../components/backlog/GoalNode.vue";
import InlineCreate from "../components/backlog/InlineCreate.vue";

const store = useProjectStore();
const typeStore = usePomodoroTypeStore();

onMounted(() => {
  store.loadProjects();
  typeStore.loadTypes(); // quick estimation needs the default type's work length
});

const localGoals = ref(store.activeProjectTree?.goals ?? []);
watch(
  () => store.activeProjectTree?.goals,
  (g) => (localGoals.value = g ? [...g] : []),
);

async function onGoalDrop() {
  if (!store.activeProjectTree) return;
  await store.reorderGoals(
    store.activeProjectTree.id,
    localGoals.value.map((g) => g.id),
  );
}
</script>

<template>
  <section class="backlog">
    <aside class="projects">
      <h1>Backlog</h1>
      <p v-if="store.error" class="error">{{ store.error }}</p>
      <ul>
        <li
          v-for="p in store.projects"
          :key="p.id"
          :class="{ active: store.activeProjectTree?.id === p.id }"
          @click="store.loadProjectTree(p.id)"
        >
          <span>{{ p.name }}</span>
          <span class="stats">{{ p.completedMicrotasks }}/{{ p.totalMicrotasks }}</span>
        </li>
      </ul>
      <InlineCreate placeholder="New project" @create="(name) => store.createProject(name)" />
    </aside>

    <div class="tree">
      <p v-if="!store.activeProjectTree" class="placeholder">
        Select or create a project to manage its goals, tasks, and microtasks.
      </p>
      <template v-else>
        <h2>{{ store.activeProjectTree.name }}</h2>
        <draggable v-model="localGoals" item-key="id" handle=".drag-handle" @end="onGoalDrop">
          <template #item="{ element }">
            <GoalNode :goal="element" />
          </template>
        </draggable>
        <InlineCreate
          placeholder="New goal"
          @create="(t) => store.createGoal(store.activeProjectTree!.id, t)"
        />
      </template>
    </div>
  </section>
</template>

<style scoped>
.backlog { display: flex; gap: 24px; height: 100%; }
.projects { width: 240px; border-right: 1px solid #20242b; padding-right: 16px; }
.projects ul { list-style: none; padding: 0; margin: 12px 0; }
.projects li {
  display: flex; justify-content: space-between; padding: 8px 10px;
  border-radius: 8px; cursor: pointer; color: #c6cdd8;
}
.projects li:hover { background: #181d24; }
.projects li.active { background: #1f2630; color: #fff; }
.stats { color: #6b7484; font-size: 12px; }
.tree { flex: 1; }
.placeholder { color: #6b7484; }
.error { color: #e06c75; }
</style>
```

- [ ] **Step 2: Verify end-to-end in the app**

Run: `npm run tauri dev`
Expected: create a project → goal → task → microtask “Outline 45” (auto-computes 3 pomodoros with the Standard 20/5 type); drag rows to reorder; complete the only microtask of a task → the task and goal show completed (roll-up); the project list stats update. The log file shows each command INFO line and one roll-up narrative line.

- [ ] **Step 3: Run the full suites**

Run: `npm test -- --run && npx vue-tsc --noEmit`
Expected: all green.

- [ ] **Step 4: Commit**

```bash
git add src/views
git commit -m "feat: backlog view — project pane + draggable tree with inline creation"
```

---

### Task 26: Settings — PomodoroType presets section `[medium]`

**Files:**
- Create: `src/components/settings/PomodoroTypesSection.vue`
- Modify: `src/views/SettingsView.vue`

- [ ] **Step 1: Write `src/components/settings/PomodoroTypesSection.vue`**

```vue
<script setup lang="ts">
import { onMounted, reactive } from "vue";
import { usePomodoroTypeStore } from "../../stores/pomodoroTypeStore";

const store = usePomodoroTypeStore();
onMounted(() => store.loadTypes());

const draft = reactive({
  name: "",
  workMinutes: 25,
  restMinutes: 5,
  longBreakMinutes: null as number | null,
  longBreakEvery: null as number | null,
});

async function create() {
  if (!draft.name.trim()) return;
  await store.createType({ ...draft, name: draft.name.trim() });
  if (!store.error) draft.name = "";
}

function confirmDelete(id: string, name: string) {
  if (window.confirm(`Delete the "${name}" preset? Microtasks using it fall back to the default type.`)) {
    store.deleteType(id);
  }
}
</script>

<template>
  <section class="presets">
    <h2>Pomodoro types</h2>
    <p v-if="store.error" class="error">{{ store.error }}</p>

    <table>
      <thead>
        <tr><th>Default</th><th>Name</th><th>Work</th><th>Rest</th><th>Long break</th><th></th></tr>
      </thead>
      <tbody>
        <tr v-for="t in store.types" :key="t.id">
          <td>
            <input
              type="radio"
              name="default-type"
              :checked="t.isDefault"
              @change="store.setDefault(t.id)"
            />
          </td>
          <td>{{ t.name }}</td>
          <td>{{ t.workMinutes }}m</td>
          <td>{{ t.restMinutes }}m</td>
          <td>
            <template v-if="t.longBreakMinutes">{{ t.longBreakMinutes }}m every {{ t.longBreakEvery }}</template>
            <template v-else>—</template>
          </td>
          <td><button class="ghost" @click="confirmDelete(t.id, t.name)">Delete</button></td>
        </tr>
      </tbody>
    </table>

    <div class="create-row">
      <input v-model="draft.name" placeholder="Name (e.g. Deep)" />
      <label>Work <input v-model.number="draft.workMinutes" type="number" min="1" /></label>
      <label>Rest <input v-model.number="draft.restMinutes" type="number" min="1" /></label>
      <label>Long break <input v-model.number="draft.longBreakMinutes" type="number" min="1" placeholder="—" /></label>
      <label>every <input v-model.number="draft.longBreakEvery" type="number" min="1" placeholder="—" /></label>
      <button @click="create">Add preset</button>
    </div>
    <p class="hint">Long-break fields go together: set both or neither.</p>
  </section>
</template>

<style scoped>
.presets table { width: 100%; border-collapse: collapse; margin: 12px 0; }
.presets th, .presets td { text-align: left; padding: 6px 10px; border-bottom: 1px solid #20242b; }
.create-row { display: flex; gap: 10px; align-items: center; flex-wrap: wrap; }
.create-row input[type="number"] { width: 60px; }
.hint { color: #6b7484; font-size: 12px; }
.error { color: #e06c75; }
.ghost { background: none; border: none; color: #6b7484; cursor: pointer; }
</style>
```

- [ ] **Step 2: Mount it in `src/views/SettingsView.vue`**

```vue
<script setup lang="ts">
import PomodoroTypesSection from "../components/settings/PomodoroTypesSection.vue";
</script>

<template>
  <section>
    <h1>Settings</h1>
    <PomodoroTypesSection />
    <p class="placeholder">Audio, notification, and planning-window settings arrive in Phase 6.</p>
  </section>
</template>
```

- [ ] **Step 3: Verify in the app**

Run: `npm run tauri dev`
Expected: Settings lists Standard 20/5 as default; add "Deep 50/10, long 30 every 2"; flip the default radio (exclusive); delete with confirm. The log shows each mutation INFO line; the both-or-neither validation rejection appears as WARN when you set only one long-break field.

- [ ] **Step 4: Commit**

```bash
git add src
git commit -m "feat: pomodoro type presets management in Settings"
```

---

### Task 27: Wire frontend failures into `logs/` smoke check `[trivial]`

Phase 1's `ipc()` wrapper already forwards every failed command to `log_frontend`. One manual check keeps the promise honest now that real mutations exist.

- [ ] **Step 1: Force a validation failure in the running app**

Run: `npm run tauri dev` — in Settings, create a preset with only one long-break field filled.
Expected: the form shows the validation message AND `logs/focus-planner.log.<today>` contains the backend WARN (`rejected`) line followed by a `target=frontend` ERROR line for the same command — both sides of the failure visible in one file.

---

### Task 28: Docs — README, spec §5 backfill, db-context check `[easy]`

**Files:**
- Modify: `README.md`, `docs/specs/m1-focus-planner-design.md`
- Verify-only: `docs/db-context/migration-history.md`

- [ ] **Step 1: Backfill spec §5** (loud flag 2): add to the query list in `docs/specs/m1-focus-planner-design.md`:

```markdown
* `list_pomodoro_types()` -> Returns all pomodoro type presets. (Added in Phase 2 — needed by the Settings presets section and the quick-estimation input.)
```

- [ ] **Step 2: Update `README.md`** — current-state section now says: backlog management is live (projects → goals → tasks → microtasks with completion roll-up, drag reordering, pomodoro type presets); Day planning arrives in Phase 3.

- [ ] **Step 3: Confirm no schema change happened**

Run: `git diff main --stat -- src-tauri/migrations`
Expected: empty. (If not empty, STOP — loud flag 1 was violated; record a lesson in `docs/lessons/` and amend this plan.)

- [ ] **Step 4: Commit**

```bash
git add README.md docs/specs/m1-focus-planner-design.md
git commit -m "docs: phase 2 README refresh + spec §5 list_pomodoro_types backfill"
```

---

### Task 29: Phase acceptance — manual QA checklist `[trivial]`

- [ ] Fresh `npm run tauri dev`: Backlog shows the project pane; create project → goal → task → microtasks (with “Title 45” quick estimation auto-computing the count from the default type)
- [ ] Drag-reorder goals, tasks, and microtasks; restart the app — order persisted
- [ ] Complete every microtask of a task → task auto-completes; complete the goal's last task the same way → goal auto-completes; uncomplete one microtask → task and goal reopen
- [ ] Project pane stats (`completed/total`) update after each completion
- [ ] Archive a microtask → it disappears from the tree, stats shrink, reorders still work
- [ ] Settings: Standard 20/5 seeded as default; create/edit/delete presets; default radio is exclusive; long-break both-or-neither validation rejects half-filled pairs
- [ ] Delete a project with children → whole subtree gone (FK cascade)
- [ ] **Logs narrative (spec §7 gate):** `logs/focus-planner.log.<today>` shows every command INFO entry/exit, WARN validation rejections, and the roll-up chain as one readable line — "microtask X completed -> task Y completed -> goal Z completed". A junior with zero context can follow the session
- [ ] `cargo test --manifest-path src-tauri/Cargo.toml` and `npm test -- --run` pass locally; CI green on the PR

---

## Plan self-review (performed at writing time)

1. **Scope coverage:** all 25 spec §3 mutations (project 4, goal 4, task 4, microtask 6, reorders 3, pomodoro types 4) have service + command tasks (3–15); spec §5 backlog queries `list_projects` (stats, Task 16), `get_project_tree` (Task 17), `get_microtask` (Task 18) plus the flagged `list_pomodoro_types` backfill (Tasks 14/28); spec §6 Backlog tree with inline create, drag reorder, quick estimation (Tasks 22–25) and the Settings presets section (Task 26); stores per spec §6 (Tasks 20–21). The roll-up rule is Task 10 `[hard]` with its narrative log line.
2. **Placeholder scan:** no TBD/TODO/"similar to Task N" — every step carries complete code, exact paths, exact commands with expected output.
3. **Type consistency:** `ProjectSummary`/`ProjectTree`/`Tree*`/`PomodoroType` defined once in Rust (Tasks 16–17, 14) and mirrored once in TS (Task 19); store actions call commands by their exact registered names; conventions (`AppError` codes, `Db` state, `ipc<T>()`, `log_outcome`, `#[tracing::instrument(skip(db))]`, `focus_planner_lib` test imports, `cargo sqlx prepare` refresh steps) match the Phase 1 plan verbatim.
