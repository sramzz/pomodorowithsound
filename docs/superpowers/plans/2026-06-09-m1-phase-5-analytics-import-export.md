# M1 Phase 5 — Analytics + Import/Export Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship the `get_stats` aggregation query with its Analytics view (date-range presets, totals, CSS-bar chart, per-project table), plus `export_data`/`import_data` — a versioned single-file JSON backup with full-restore import — wired to Export/Import buttons in Settings via the Tauri dialog plugin.

**Architecture:** All aggregation happens in SQL inside a new `stats_service` (no Rust loops over rows); the report crosses IPC as one `StatsReport` consumed by a new `useStatsStore` + `AnalyticsView`. Backup lives in a new `backup_service`: export reads all 10 tables in **one transaction** (consistent snapshot) and writes one JSON file; import validates the `version` field, then — **destructively, by design** — wipes every table and re-inserts the file's rows parents-before-children, all in one transaction. The file path always comes from the UI via `tauri-plugin-dialog`; Rust commands receive a `path` argument and return `Result<(), AppError>` (CQS: they write, they return no data).

**Tech Stack:** Tauri 2, `tauri-plugin-dialog` + `@tauri-apps/plugin-dialog` (the only new dependencies), Vue 3, Pinia, Vitest (`mockIPC`), Rust, SQLx 0.8 (sqlite, compile-time-checked queries for stats), serde/serde_json, tracing.

**Conventions (per `docs/specs/m1-roadmap.md`):** every task carries a difficulty tag (`[trivial]`/`[easy]`/`[medium]`/`[hard]`). The failing test of each TDD task is designed by the most capable agent; implementation may be assigned by difficulty (cheaper agents take `[trivial]`/`[easy]`); every task is reviewed before its commit lands. Phase 1 conventions are inherited verbatim: `AppError` + `Result<T, AppError>` on every command, the `ipc<T>()` wrapper for every store call, `#[tracing::instrument]` on command handlers, services in `core/`, `#[sqlx::test]` for Rust, `mockIPC` for stores, `#[serde(rename_all = "camelCase")]` on IPC models.

**Philosophy (PHILOSOPHY.md):**
- **CQS:** `get_stats` is a query (returns data, mutates nothing). `export_data(path)` and `import_data(path)` are commands (mutate the filesystem/DB, return `Result<(), AppError>`, never data).
- **POLA — say the destructive thing out loud:** M1 import is a **full restore**: it deletes *every existing row* (including the seeded Standard pomodoro type) before inserting the file's rows. The UI confirm dialog must say "replaces ALL current data". This is repeated at every layer of this plan on purpose.
- **Idempotency:** importing the same file twice yields the identical end state (wipe + insert).
- **KISS:** no chart library — per-day bars are plain `<div>`s with percentage heights. No new date library — date math is `Date` + ISO slicing.
- **Logging (spec §7):** `get_stats` logs INFO with the range + row counts; export logs INFO with path + per-table row counts; import logs INFO with version, "wiped N existing rows", and per-table inserted counts — and ERROR with context on every validation failure. QA gate: an export+import session must be fully reconstructable from `logs/` alone.

**⚠️ NO SCHEMA CHANGES IN THIS PHASE.** Phase 5 adds zero migrations. `docs/db-context/migration-history.md` must NOT gain a row. If any task seems to need a migration, STOP — the plan is wrong; amend it and record a lesson in `docs/lessons/`.

**File map:**
- `src-tauri/src/models/stats.rs` — `StatsReport`, `DayStats`, `StatsTotals`, `ProjectStats` (IPC camelCase)
- `src-tauri/src/models/export.rs` — `ExportFile` + one row struct per table (file format, snake_case, mirrors spec §2 columns verbatim)
- `src-tauri/src/core/stats_service.rs` — SQL aggregation
- `src-tauri/src/core/backup_service.rs` — export/import
- `src-tauri/src/commands/stats.rs`, `src-tauri/src/commands/backup.rs` — thin instrumented IPC handlers
- `src-tauri/tests/stats.rs`, `src-tauri/tests/backup.rs` — `#[sqlx::test]` suites
- `src/ipc/types.ts` (extend), `src/stores/statsStore.ts` (+test), `src/views/AnalyticsView.vue` (replace placeholder), `src/components/DataBackupSection.vue`, `src/views/SettingsView.vue` (one-line mount)

---

### Task 1: `get_stats` core service — SQL aggregation `[hard]`

This task **defines the aggregate shape** the roadmap deferred to this plan. `StatsReport`:
- `perDay`: one entry per day in range that has any activity — `{date, totalWorkSeconds, totalBreakSeconds, pomodorosCompleted, blocksSkipped}`. Work/break/skipped come from `focus_sessions` (grouped by the date prefix of `start_time`); `pomodorosCompleted` counts `pomodoro_sessions` rows with `was_completed = 1` (grouped by the date prefix of `started_at`).
- `totals`: the same four fields summed over the range.
- `completionRate`: `SUM(blocks_completed) / (SUM(blocks_completed) + SUM(blocks_skipped))` over `focus_sessions` in range; `0.0` when there are no blocks.
- `perProject`: `{projectId, projectName, pomodorosCompleted, workSeconds}` — joins `pomodoro_sessions → microtasks → tasks → goals → projects`, completed sessions only, `workSeconds = SUM(work_minutes) * 60`, ordered by work seconds descending. Sessions whose `microtask_id` is NULL (unattributable) are simply absent from this breakdown — documented behavior.

All three aggregations are single SQL statements. No Rust loops over rows.

**Files:**
- Create: `src-tauri/src/models/stats.rs`, `src-tauri/src/core/stats_service.rs`
- Modify: `src-tauri/src/models/mod.rs`, `src-tauri/src/core/mod.rs`
- Test: `src-tauri/tests/stats.rs`

- [ ] **Step 1: Write the failing test** *(test designed by the strongest agent)*

`src-tauri/tests/stats.rs` — every expected number below is hand-computed from the fixture and annotated:

```rust
use focus_planner_lib::core::stats_service;
use focus_planner_lib::error::AppError;
use sqlx::SqlitePool;

/// Fixture (all expected numbers in the assertions derive from this):
///
/// Hierarchy: Website(pr1) → g1 → t1 → m1   |   Thesis(pr2) → g2 → t2 → m2
///
/// focus_sessions:
///   fs0 2026-05-25: work 9999, break 999, completed 5, skipped 5  (OUTSIDE range — must be ignored)
///   fs1 2026-06-01: work 4800, break 1200, completed 4, skipped 1
///   fs2 2026-06-02: work 2400, break  600, completed 2, skipped 0
///
/// pomodoro_sessions (work_minutes / started_at / was_completed / microtask):
///   s1 20 2026-06-01 1 m1 | s2 20 2026-06-01 1 m1 | s3 20 2026-06-01 1 m1
///   s4 20 2026-06-01 1 m2 | s5 20 2026-06-01 0 m1 (abandoned — never counted)
///   s6 25 2026-06-02 1 m2 | s7 25 2026-06-02 1 m2
///   s8 20 2026-05-25 1 m1 (OUTSIDE range — must be ignored)
async fn seed_stats_fixture(pool: &SqlitePool) {
    const T: &str = "2026-06-01T08:00:00Z"; // created_at/updated_at filler

    sqlx::query(
        "INSERT INTO projects (id, name, status, is_archived, created_at, updated_at) VALUES
         ('pr1', 'Website', 'open', 0, ?1, ?1),
         ('pr2', 'Thesis',  'open', 0, ?1, ?1)",
    )
    .bind(T).execute(pool).await.unwrap();

    sqlx::query(
        "INSERT INTO goals (id, project_id, title, status, is_archived, created_at, updated_at) VALUES
         ('g1', 'pr1', 'Launch', 'open', 0, ?1, ?1),
         ('g2', 'pr2', 'Write',  'open', 0, ?1, ?1)",
    )
    .bind(T).execute(pool).await.unwrap();

    sqlx::query(
        "INSERT INTO tasks (id, goal_id, title, status, is_archived, created_at, updated_at) VALUES
         ('t1', 'g1', 'Build landing page', 'open', 0, ?1, ?1),
         ('t2', 'g2', 'Chapter 1',          'open', 0, ?1, ?1)",
    )
    .bind(T).execute(pool).await.unwrap();

    sqlx::query(
        "INSERT INTO microtasks (id, task_id, title, estimated_minutes, pomodoro_count, status, is_archived, created_at, updated_at) VALUES
         ('m1', 't1', 'Hero section', 60, 3, 'open', 0, ?1, ?1),
         ('m2', 't2', 'Outline',      60, 3, 'open', 0, ?1, ?1)",
    )
    .bind(T).execute(pool).await.unwrap();

    sqlx::query(
        "INSERT INTO plans (id, date, status, created_at, updated_at) VALUES
         ('pl0', '2026-05-25', 'committed', ?1, ?1),
         ('pl1', '2026-06-01', 'committed', ?1, ?1),
         ('pl2', '2026-06-02', 'committed', ?1, ?1)",
    )
    .bind(T).execute(pool).await.unwrap();

    sqlx::query(
        "INSERT INTO focus_sessions (id, plan_id, start_time, end_time, total_work_seconds, total_break_seconds, blocks_completed, blocks_skipped, created_at) VALUES
         ('fs0', 'pl0', '2026-05-25T09:00:00Z', '2026-05-25T11:00:00Z', 9999, 999, 5, 5, ?1),
         ('fs1', 'pl1', '2026-06-01T09:00:00Z', '2026-06-01T11:00:00Z', 4800, 1200, 4, 1, ?1),
         ('fs2', 'pl2', '2026-06-02T09:00:00Z', '2026-06-02T10:00:00Z', 2400, 600, 2, 0, ?1)",
    )
    .bind(T).execute(pool).await.unwrap();

    sqlx::query(
        "INSERT INTO pomodoro_sessions (id, focus_session_id, microtask_id, work_minutes, started_at, completed_at, was_completed, created_at) VALUES
         ('s1', 'fs1', 'm1', 20, '2026-06-01T09:00:00Z', '2026-06-01T09:20:00Z', 1, ?1),
         ('s2', 'fs1', 'm1', 20, '2026-06-01T09:25:00Z', '2026-06-01T09:45:00Z', 1, ?1),
         ('s3', 'fs1', 'm1', 20, '2026-06-01T10:00:00Z', '2026-06-01T10:20:00Z', 1, ?1),
         ('s4', 'fs1', 'm2', 20, '2026-06-01T10:30:00Z', '2026-06-01T10:50:00Z', 1, ?1),
         ('s5', 'fs1', 'm1', 20, '2026-06-01T11:00:00Z', '2026-06-01T11:05:00Z', 0, ?1),
         ('s6', 'fs2', 'm2', 25, '2026-06-02T09:00:00Z', '2026-06-02T09:25:00Z', 1, ?1),
         ('s7', 'fs2', 'm2', 25, '2026-06-02T09:30:00Z', '2026-06-02T09:55:00Z', 1, ?1),
         ('s8', 'fs0', 'm1', 20, '2026-05-25T09:00:00Z', '2026-05-25T09:20:00Z', 1, ?1)",
    )
    .bind(T).execute(pool).await.unwrap();
}

#[sqlx::test]
async fn get_stats_aggregates_per_day_totals_rate_and_per_project(pool: SqlitePool) {
    seed_stats_fixture(&pool).await;

    let report = stats_service::get_stats(&pool, "2026-06-01", "2026-06-02")
        .await
        .unwrap();

    // per-day, ordered by date ascending
    assert_eq!(report.per_day.len(), 2);
    let d1 = &report.per_day[0];
    assert_eq!(d1.date, "2026-06-01");
    assert_eq!(d1.total_work_seconds, 4800);   // fs1
    assert_eq!(d1.total_break_seconds, 1200);  // fs1
    assert_eq!(d1.pomodoros_completed, 4);     // s1..s4 (s5 abandoned)
    assert_eq!(d1.blocks_skipped, 1);          // fs1
    let d2 = &report.per_day[1];
    assert_eq!(d2.date, "2026-06-02");
    assert_eq!(d2.total_work_seconds, 2400);   // fs2
    assert_eq!(d2.total_break_seconds, 600);   // fs2
    assert_eq!(d2.pomodoros_completed, 2);     // s6, s7
    assert_eq!(d2.blocks_skipped, 0);          // fs2

    // totals across the range
    assert_eq!(report.totals.total_work_seconds, 7200);  // 4800 + 2400
    assert_eq!(report.totals.total_break_seconds, 1800); // 1200 + 600
    assert_eq!(report.totals.pomodoros_completed, 6);    // 4 + 2
    assert_eq!(report.totals.blocks_skipped, 1);

    // completion rate: completed 4+2=6, skipped 1 → 6/7
    assert!((report.completion_rate - 6.0 / 7.0).abs() < 1e-9);

    // per-project, ordered by work seconds DESC:
    //   Thesis:  s4(20) + s6(25) + s7(25) = 70 min = 4200 s, 3 pomodoros
    //   Website: s1+s2+s3 = 60 min        = 3600 s, 3 pomodoros
    assert_eq!(report.per_project.len(), 2);
    assert_eq!(report.per_project[0].project_id, "pr2");
    assert_eq!(report.per_project[0].project_name, "Thesis");
    assert_eq!(report.per_project[0].pomodoros_completed, 3);
    assert_eq!(report.per_project[0].work_seconds, 4200);
    assert_eq!(report.per_project[1].project_id, "pr1");
    assert_eq!(report.per_project[1].project_name, "Website");
    assert_eq!(report.per_project[1].pomodoros_completed, 3);
    assert_eq!(report.per_project[1].work_seconds, 3600);
}

#[sqlx::test]
async fn get_stats_on_a_range_with_no_data_returns_zeroes(pool: SqlitePool) {
    seed_stats_fixture(&pool).await;

    let report = stats_service::get_stats(&pool, "2026-03-01", "2026-03-31")
        .await
        .unwrap();

    assert!(report.per_day.is_empty());
    assert_eq!(report.totals.total_work_seconds, 0);
    assert_eq!(report.totals.total_break_seconds, 0);
    assert_eq!(report.totals.pomodoros_completed, 0);
    assert_eq!(report.totals.blocks_skipped, 0);
    assert_eq!(report.completion_rate, 0.0); // no blocks → 0.0, never NaN
    assert!(report.per_project.is_empty());
}

#[sqlx::test]
async fn get_stats_rejects_bad_ranges(pool: SqlitePool) {
    let inverted = stats_service::get_stats(&pool, "2026-06-09", "2026-06-01").await;
    assert!(matches!(inverted, Err(AppError::Validation(_))));

    let bad_format = stats_service::get_stats(&pool, "06/01/2026", "2026-06-09").await;
    assert!(matches!(bad_format, Err(AppError::Validation(_))));
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test stats`
Expected: FAIL to compile with `error[E0432]`/`E0433` — `core::stats_service` does not exist.

- [ ] **Step 3: Write `src-tauri/src/models/stats.rs`** (+ `pub mod stats;` in `src-tauri/src/models/mod.rs`)

```rust
use serde::Serialize;

/// The Phase 5 aggregate shape (roadmap deferred its definition to this plan).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatsReport {
    pub per_day: Vec<DayStats>,
    pub totals: StatsTotals,
    /// blocks_completed / (blocks_completed + blocks_skipped) over focus_sessions
    /// in range; 0.0 when there are no blocks at all.
    pub completion_rate: f64,
    pub per_project: Vec<ProjectStats>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DayStats {
    /// YYYY-MM-DD
    pub date: String,
    pub total_work_seconds: i64,
    pub total_break_seconds: i64,
    pub pomodoros_completed: i64,
    pub blocks_skipped: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatsTotals {
    pub total_work_seconds: i64,
    pub total_break_seconds: i64,
    pub pomodoros_completed: i64,
    pub blocks_skipped: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectStats {
    pub project_id: String,
    pub project_name: String,
    pub pomodoros_completed: i64,
    pub work_seconds: i64,
}
```

- [ ] **Step 4: Write `src-tauri/src/core/stats_service.rs`** (+ `pub mod stats_service;` in `src-tauri/src/core/mod.rs`)

```rust
use crate::error::AppError;
use crate::models::stats::{DayStats, ProjectStats, StatsReport, StatsTotals};
use sqlx::SqlitePool;

fn is_iso_date(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 10
        && b[4] == b'-'
        && b[7] == b'-'
        && b.iter()
            .enumerate()
            .all(|(i, c)| matches!(i, 4 | 7) || c.is_ascii_digit())
}

/// Read-only aggregation over focus_sessions and pomodoro_sessions.
/// All aggregation happens in SQL — no Rust loops over rows.
pub async fn get_stats(
    pool: &SqlitePool,
    start_date: &str,
    end_date: &str,
) -> Result<StatsReport, AppError> {
    if !is_iso_date(start_date) || !is_iso_date(end_date) {
        return Err(AppError::Validation(format!(
            "dates must be YYYY-MM-DD, got start_date={start_date} end_date={end_date}"
        )));
    }
    if start_date > end_date {
        return Err(AppError::Validation(format!(
            "start_date {start_date} is after end_date {end_date}"
        )));
    }

    // Per-day: focus_sessions carry work/break/skipped; pomodoro_sessions carry the
    // completed-pomodoro count. UNION of both day sets so a day with only one kind
    // of row still appears.
    let per_day = sqlx::query_as!(
        DayStats,
        r#"
        WITH fs AS (
            SELECT substr(start_time, 1, 10) AS day,
                   SUM(total_work_seconds)   AS work_s,
                   SUM(total_break_seconds)  AS break_s,
                   SUM(blocks_skipped)       AS skipped
            FROM focus_sessions
            WHERE substr(start_time, 1, 10) BETWEEN ?1 AND ?2
            GROUP BY day
        ),
        pomos AS (
            SELECT substr(started_at, 1, 10) AS day,
                   COUNT(*)                  AS poms
            FROM pomodoro_sessions
            WHERE was_completed = 1
              AND substr(started_at, 1, 10) BETWEEN ?1 AND ?2
            GROUP BY day
        ),
        days AS (
            SELECT day FROM fs UNION SELECT day FROM pomos
        )
        SELECT days.day                AS "date!: String",
               COALESCE(fs.work_s, 0)  AS "total_work_seconds!: i64",
               COALESCE(fs.break_s, 0) AS "total_break_seconds!: i64",
               COALESCE(pomos.poms, 0) AS "pomodoros_completed!: i64",
               COALESCE(fs.skipped, 0) AS "blocks_skipped!: i64"
        FROM days
        LEFT JOIN fs    ON fs.day = days.day
        LEFT JOIN pomos ON pomos.day = days.day
        ORDER BY days.day
        "#,
        start_date,
        end_date
    )
    .fetch_all(pool)
    .await?;

    let t = sqlx::query!(
        r#"
        SELECT
            COALESCE((SELECT SUM(total_work_seconds)  FROM focus_sessions
                      WHERE substr(start_time, 1, 10) BETWEEN ?1 AND ?2), 0) AS "total_work_seconds!: i64",
            COALESCE((SELECT SUM(total_break_seconds) FROM focus_sessions
                      WHERE substr(start_time, 1, 10) BETWEEN ?1 AND ?2), 0) AS "total_break_seconds!: i64",
            COALESCE((SELECT SUM(blocks_completed)    FROM focus_sessions
                      WHERE substr(start_time, 1, 10) BETWEEN ?1 AND ?2), 0) AS "blocks_completed!: i64",
            COALESCE((SELECT SUM(blocks_skipped)      FROM focus_sessions
                      WHERE substr(start_time, 1, 10) BETWEEN ?1 AND ?2), 0) AS "blocks_skipped!: i64",
            (SELECT COUNT(*) FROM pomodoro_sessions
             WHERE was_completed = 1
               AND substr(started_at, 1, 10) BETWEEN ?1 AND ?2)              AS "pomodoros_completed!: i64"
        "#,
        start_date,
        end_date
    )
    .fetch_one(pool)
    .await?;

    let denominator = t.blocks_completed + t.blocks_skipped;
    let completion_rate = if denominator == 0 {
        0.0
    } else {
        t.blocks_completed as f64 / denominator as f64
    };

    // Per-project: pomodoro_sessions → microtasks → tasks → goals → projects.
    // Sessions with microtask_id NULL drop out of the joins (unattributable) — by design.
    let per_project = sqlx::query_as!(
        ProjectStats,
        r#"
        SELECT p.id                      AS "project_id!: String",
               p.name                    AS "project_name!: String",
               COUNT(*)                  AS "pomodoros_completed!: i64",
               SUM(ps.work_minutes) * 60 AS "work_seconds!: i64"
        FROM pomodoro_sessions ps
        JOIN microtasks m ON m.id = ps.microtask_id
        JOIN tasks t      ON t.id = m.task_id
        JOIN goals g      ON g.id = t.goal_id
        JOIN projects p   ON p.id = g.project_id
        WHERE ps.was_completed = 1
          AND substr(ps.started_at, 1, 10) BETWEEN ?1 AND ?2
        GROUP BY p.id, p.name
        ORDER BY SUM(ps.work_minutes) * 60 DESC, p.name
        "#,
        start_date,
        end_date
    )
    .fetch_all(pool)
    .await?;

    Ok(StatsReport {
        totals: StatsTotals {
            total_work_seconds: t.total_work_seconds,
            total_break_seconds: t.total_break_seconds,
            pomodoros_completed: t.pomodoros_completed,
            blocks_skipped: t.blocks_skipped,
        },
        completion_rate,
        per_day,
        per_project,
    })
}
```

- [ ] **Step 5: Refresh the SQLx offline cache, then run the tests**

```bash
cd src-tauri && cargo sqlx prepare && cd ..
cargo test --manifest-path src-tauri/Cargo.toml --test stats
```

Expected: 3 passed. Commit the updated `.sqlx/` files with this task.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/models src-tauri/src/core src-tauri/tests/stats.rs src-tauri/.sqlx
git commit -m "feat: get_stats SQL aggregation — per-day, totals, completion rate, per-project"
```

---

### Task 2: `get_stats` IPC command `[easy]`

Thin handler over the service (already fully tested in Task 1); this task adds the spec §7 logging: INFO with range (via instrument fields) + row counts.

**Files:**
- Create: `src-tauri/src/commands/stats.rs`
- Modify: `src-tauri/src/commands/mod.rs`, `src-tauri/src/lib.rs`

- [ ] **Step 1: Write `src-tauri/src/commands/stats.rs`** (+ `pub mod stats;` in `src-tauri/src/commands/mod.rs`)

```rust
use crate::core::stats_service;
use crate::db::Db;
use crate::error::AppError;
use crate::models::stats::StatsReport;

#[tauri::command]
#[tracing::instrument(skip(db))] // start_date / end_date stay as fields → the range is in the log line
pub async fn get_stats(
    db: tauri::State<'_, Db>,
    start_date: String,
    end_date: String,
) -> Result<StatsReport, AppError> {
    let result = stats_service::get_stats(&db.0, &start_date, &end_date).await;
    match &result {
        Ok(r) => tracing::info!(
            days = r.per_day.len(),
            projects = r.per_project.len(),
            pomodoros = r.totals.pomodoros_completed,
            "ok"
        ),
        Err(e) => tracing::error!(error = %e, "failed"),
    }
    result
}
```

- [ ] **Step 2: Register it in `src-tauri/src/lib.rs`**

Append to the existing `tauri::generate_handler![...]` list:

```rust
            commands::stats::get_stats,
```

- [ ] **Step 3: Verify it compiles and nothing regressed**

Run: `cargo test --manifest-path src-tauri/Cargo.toml`
Expected: all existing tests + the 3 from Task 1 pass.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/commands src-tauri/src/lib.rs
git commit -m "feat: get_stats IPC command with range + row-count logging"
```

---

### Task 3: TS stats types + `useStatsStore` `[medium]`

**Files:**
- Modify: `src/ipc/types.ts`
- Create: `src/stores/statsStore.ts`
- Test: `src/stores/statsStore.test.ts`

- [ ] **Step 1: Add the stats types to `src/ipc/types.ts`** (mirror of Task 1's Rust models, camelCase per the serde convention)

```ts
export interface DayStats {
  date: string; // YYYY-MM-DD
  totalWorkSeconds: number;
  totalBreakSeconds: number;
  pomodorosCompleted: number;
  blocksSkipped: number;
}

export interface StatsTotals {
  totalWorkSeconds: number;
  totalBreakSeconds: number;
  pomodorosCompleted: number;
  blocksSkipped: number;
}

export interface ProjectStats {
  projectId: string;
  projectName: string;
  pomodorosCompleted: number;
  workSeconds: number;
}

export interface StatsReport {
  perDay: DayStats[];
  totals: StatsTotals;
  /** completed / (completed + skipped), 0.0 when no blocks */
  completionRate: number;
  perProject: ProjectStats[];
}
```

- [ ] **Step 2: Write the failing test** *(test designed by the strongest agent)*

`src/stores/statsStore.test.ts`:

```ts
import { beforeEach, describe, expect, it } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { mockIPC } from "@tauri-apps/api/mocks";
import { useStatsStore } from "./statsStore";

const sampleReport = {
  perDay: [
    {
      date: "2026-06-01",
      totalWorkSeconds: 4800,
      totalBreakSeconds: 1200,
      pomodorosCompleted: 4,
      blocksSkipped: 1,
    },
  ],
  totals: {
    totalWorkSeconds: 4800,
    totalBreakSeconds: 1200,
    pomodorosCompleted: 4,
    blocksSkipped: 1,
  },
  completionRate: 0.8,
  perProject: [
    { projectId: "pr1", projectName: "Website", pomodorosCompleted: 4, workSeconds: 4800 },
  ],
};

describe("useStatsStore", () => {
  beforeEach(() => setActivePinia(createPinia()));

  it("setRange passes the range to get_stats and fills the report", async () => {
    let seen: Record<string, unknown> | undefined;
    mockIPC((cmd, args) => {
      if (cmd === "get_stats") {
        seen = args as Record<string, unknown>;
        return sampleReport;
      }
    });

    const store = useStatsStore();
    await store.setRange("2026-06-01", "2026-06-07");

    expect(seen).toEqual({ startDate: "2026-06-01", endDate: "2026-06-07" });
    expect(store.report?.totals.pomodorosCompleted).toBe(4);
    expect(store.report?.perProject[0].projectName).toBe("Website");
    expect(store.error).toBeNull();
  });

  it("setPreset(7) sets a 7-day window ending today and reloads", async () => {
    mockIPC((cmd) => (cmd === "get_stats" ? sampleReport : undefined));

    const store = useStatsStore();
    await store.setPreset(7);

    const spanMs =
      new Date(store.endDate).getTime() - new Date(store.startDate).getTime();
    expect(spanMs).toBe(6 * 86_400_000); // 7 days inclusive = 6 day-gaps
    expect(store.report).not.toBeNull();
  });

  it("records the error message on failure", async () => {
    mockIPC((cmd) => {
      if (cmd === "get_stats") throw { code: "db", message: "boom" };
    });

    const store = useStatsStore();
    await store.loadStats();

    expect(store.report).toBeNull();
    expect(store.error).toBe("boom");
  });
});
```

- [ ] **Step 3: Run to verify it fails**

Run: `npm test -- --run src/stores/statsStore.test.ts`
Expected: FAIL — `./statsStore` does not exist.

- [ ] **Step 4: Write `src/stores/statsStore.ts`**

```ts
import { defineStore } from "pinia";
import { ipc } from "../ipc/client";
import type { IpcError, StatsReport } from "../ipc/types";

function isoDate(d: Date): string {
  return d.toISOString().slice(0, 10);
}

export const useStatsStore = defineStore("stats", {
  state: () => {
    const today = new Date();
    const weekAgo = new Date(today.getTime() - 6 * 86_400_000);
    return {
      startDate: isoDate(weekAgo), // default preset: last 7 days
      endDate: isoDate(today),
      report: null as StatsReport | null,
      loading: false,
      error: null as string | null,
    };
  },
  actions: {
    async loadStats() {
      this.loading = true;
      this.error = null;
      try {
        this.report = await ipc<StatsReport>("get_stats", {
          startDate: this.startDate,
          endDate: this.endDate,
        });
      } catch (e) {
        this.error = (e as IpcError).message ?? String(e);
      } finally {
        this.loading = false;
      }
    },
    async setPreset(days: 7 | 30) {
      const today = new Date();
      this.startDate = isoDate(new Date(today.getTime() - (days - 1) * 86_400_000));
      this.endDate = isoDate(today);
      await this.loadStats();
    },
    async setRange(startDate: string, endDate: string) {
      this.startDate = startDate;
      this.endDate = endDate;
      await this.loadStats();
    },
  },
});
```

- [ ] **Step 5: Run to verify it passes**

Run: `npm test -- --run src/stores/statsStore.test.ts`
Expected: 3 passing. Also run `npx vue-tsc --noEmit` — no errors.

- [ ] **Step 6: Commit**

```bash
git add src/ipc/types.ts src/stores/statsStore.ts src/stores/statsStore.test.ts
git commit -m "feat: StatsReport types + useStatsStore with presets and range loading"
```

---

### Task 4: Analytics view — presets, totals, CSS-bar chart, project table `[medium]`

Replaces the Phase 1 placeholder. No chart library (KISS): the per-day chart is a flex row of `<div>` bars whose heights are percentages of the busiest day. View code is verified manually (roadmap: no E2E in M1).

**Files:**
- Modify: `src/views/AnalyticsView.vue` (replace the placeholder content entirely)

- [ ] **Step 1: Write `src/views/AnalyticsView.vue`**

```vue
<script setup lang="ts">
import { computed, onMounted } from "vue";
import { useStatsStore } from "../stores/statsStore";

const store = useStatsStore();
onMounted(() => store.loadStats());

function formatDuration(seconds: number): string {
  const h = Math.floor(seconds / 3600);
  const m = Math.round((seconds % 3600) / 60);
  return h > 0 ? `${h}h ${m}m` : `${m}m`;
}

const maxWorkSeconds = computed(() =>
  Math.max(1, ...(store.report?.perDay.map((d) => d.totalWorkSeconds) ?? []))
);

const hasData = computed(() => (store.report?.perDay.length ?? 0) > 0);

function onStartChange(event: Event) {
  store.setRange((event.target as HTMLInputElement).value, store.endDate);
}
function onEndChange(event: Event) {
  store.setRange(store.startDate, (event.target as HTMLInputElement).value);
}
</script>

<template>
  <section>
    <h1>Analytics</h1>

    <div class="range-bar">
      <button @click="store.setPreset(7)">Last 7 days</button>
      <button @click="store.setPreset(30)">Last 30 days</button>
      <input type="date" :value="store.startDate" @change="onStartChange" />
      <span>→</span>
      <input type="date" :value="store.endDate" @change="onEndChange" />
    </div>

    <p v-if="store.loading">Loading…</p>
    <p v-else-if="store.error" class="error">{{ store.error }}</p>
    <p v-else-if="!hasData" class="placeholder">
      No focus sessions in this range yet. Run a day (Day view → Start Day) and your
      stats will appear here.
    </p>

    <template v-else-if="store.report">
      <div class="totals-row">
        <div class="stat-card">
          <span class="stat-value">{{ formatDuration(store.report.totals.totalWorkSeconds) }}</span>
          <span class="stat-label">Focus time</span>
        </div>
        <div class="stat-card">
          <span class="stat-value">{{ formatDuration(store.report.totals.totalBreakSeconds) }}</span>
          <span class="stat-label">Break time</span>
        </div>
        <div class="stat-card">
          <span class="stat-value">{{ store.report.totals.pomodorosCompleted }}</span>
          <span class="stat-label">Pomodoros</span>
        </div>
        <div class="stat-card">
          <span class="stat-value">{{ Math.round(store.report.completionRate * 100) }}%</span>
          <span class="stat-label">Completion rate</span>
        </div>
      </div>

      <h2>Focus time per day</h2>
      <div class="bar-chart">
        <div
          v-for="day in store.report.perDay"
          :key="day.date"
          class="bar-col"
          :title="`${day.date}: ${formatDuration(day.totalWorkSeconds)}, ${day.pomodorosCompleted} pomodoros`"
        >
          <div
            class="bar"
            :style="{ height: `${(day.totalWorkSeconds / maxWorkSeconds) * 100}%` }"
          ></div>
          <span class="bar-label">{{ day.date.slice(5) }}</span>
        </div>
      </div>

      <h2>Per project</h2>
      <table class="project-table">
        <thead>
          <tr><th>Project</th><th>Pomodoros</th><th>Focus time</th></tr>
        </thead>
        <tbody>
          <tr v-for="p in store.report.perProject" :key="p.projectId">
            <td>{{ p.projectName }}</td>
            <td>{{ p.pomodorosCompleted }}</td>
            <td>{{ formatDuration(p.workSeconds) }}</td>
          </tr>
          <tr v-if="store.report.perProject.length === 0">
            <td colspan="3" class="placeholder">
              No completed pomodoros are linked to a project in this range.
            </td>
          </tr>
        </tbody>
      </table>
    </template>
  </section>
</template>

<style scoped>
.range-bar { display: flex; gap: 8px; align-items: center; margin-bottom: 20px; }
.range-bar button {
  background: #1f2630; color: #e6e9ef; border: 1px solid #20242b;
  border-radius: 8px; padding: 6px 12px; cursor: pointer;
}
.range-bar button:hover { background: #2a3340; }
.range-bar input[type="date"] {
  background: #181d24; color: #e6e9ef; border: 1px solid #20242b;
  border-radius: 8px; padding: 6px 8px;
}
.totals-row { display: flex; gap: 12px; margin-bottom: 24px; }
.stat-card {
  background: #181d24; border: 1px solid #20242b; border-radius: 12px;
  padding: 14px 18px; display: flex; flex-direction: column; gap: 4px; min-width: 110px;
}
.stat-value { font-size: 22px; font-weight: 600; }
.stat-label { font-size: 12px; color: #9aa3b2; }
.bar-chart {
  display: flex; align-items: flex-end; gap: 6px; height: 160px; padding: 8px;
  background: #181d24; border: 1px solid #20242b; border-radius: 12px; margin-bottom: 24px;
}
.bar-col {
  flex: 1; display: flex; flex-direction: column; align-items: center;
  justify-content: flex-end; height: 100%; gap: 4px;
}
.bar { width: 100%; max-width: 36px; background: #4c8dff; border-radius: 4px 4px 0 0; min-height: 2px; }
.bar-label { font-size: 10px; color: #9aa3b2; }
.project-table { width: 100%; border-collapse: collapse; }
.project-table th, .project-table td {
  text-align: left; padding: 8px 10px; border-bottom: 1px solid #20242b;
}
.project-table th { color: #9aa3b2; font-size: 12px; }
.placeholder { color: #9aa3b2; }
.error { color: #ff6b6b; }
</style>
```

- [ ] **Step 2: Typecheck**

Run: `npx vue-tsc --noEmit`
Expected: no errors.

- [ ] **Step 3: Verify manually**

Run: `npm run tauri dev`, open Analytics.
- With Phase 4 session data in the dev DB: bars, totals, and the project table render; presets reload the range; the log file shows `get_stats` with `start_date`/`end_date` fields and `days=… projects=… pomodoros=… ok`.
- Without data: the empty state renders ("No focus sessions in this range yet…").

If the running app's DB has no sessions yet, seed one directly (adjust nothing else):

```bash
sqlite3 "$HOME/Library/Application Support/com.sramzz.focusplanner/focus-planner.sqlite" \
  "INSERT INTO plans (id, date, status, created_at, updated_at) VALUES ('qa-pl1', date('now'), 'committed', strftime('%Y-%m-%dT%H:%M:%SZ','now'), strftime('%Y-%m-%dT%H:%M:%SZ','now'));
   INSERT INTO focus_sessions (id, plan_id, start_time, end_time, total_work_seconds, total_break_seconds, blocks_completed, blocks_skipped, created_at) VALUES ('qa-fs1', 'qa-pl1', strftime('%Y-%m-%dT%H:%M:%SZ','now'), strftime('%Y-%m-%dT%H:%M:%SZ','now'), 2400, 600, 2, 0, strftime('%Y-%m-%dT%H:%M:%SZ','now'));"
```

- [ ] **Step 4: Commit**

```bash
git add src/views/AnalyticsView.vue
git commit -m "feat: Analytics view — range presets, totals row, CSS-bar chart, per-project table"
```

---

### Task 5: Add `tauri-plugin-dialog` (plugin + capability) `[easy]`

Native save/open/ask dialogs for export/import. Not TDD-able (configuration); verified by compile + a later manual step.

**Files:**
- Modify: `src-tauri/Cargo.toml`, `src-tauri/src/lib.rs`, `src-tauri/capabilities/default.json`, `package.json`

- [ ] **Step 1: Install both halves of the plugin**

```bash
cargo add tauri-plugin-dialog --manifest-path src-tauri/Cargo.toml
npm install @tauri-apps/plugin-dialog
```

- [ ] **Step 2: Register the plugin in `src-tauri/src/lib.rs`**

On the existing builder chain (before `.setup(...)`):

```rust
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
```

- [ ] **Step 3: Grant the capability**

In `src-tauri/capabilities/default.json`, append to the `permissions` array:

```json
    "dialog:default"
```

- [ ] **Step 4: Verify it compiles and launches**

Run: `cargo check --manifest-path src-tauri/Cargo.toml && npm run tauri dev`
Expected: clean check; the app opens normally. Close it.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/src/lib.rs src-tauri/capabilities/default.json package.json package-lock.json
git commit -m "feat: add tauri-plugin-dialog with dialog:default capability"
```

---

### Task 6: Export file format — `ExportFile` + row structs + `AppError::Io` `[easy]`

The export file deliberately mirrors spec §2 **verbatim**: snake_case field names identical to the SQLite columns, `INTEGER` columns kept as `i64` (no bool conversion — this is a dump format, not an IPC model; the IPC models stay camelCase). Mechanical but long; behavior is pinned by Tasks 7–9's tests.

**Files:**
- Create: `src-tauri/src/models/export.rs`
- Modify: `src-tauri/src/models/mod.rs`, `src-tauri/src/error.rs`, `src/ipc/types.ts`

- [ ] **Step 1: Write `src-tauri/src/models/export.rs`** (+ `pub mod export;` in `src-tauri/src/models/mod.rs`)

```rust
use serde::{Deserialize, Serialize};

/// Bump only when the file shape changes; import refuses any other value.
pub const EXPORT_VERSION: u32 = 1;

/// The single-file backup: a versioned, consistent snapshot of all 10 tables.
/// Field names mirror the SQLite schema (spec §2) verbatim — snake_case, raw integers.
#[derive(Debug, Serialize, Deserialize)]
pub struct ExportFile {
    pub version: u32,
    pub exported_at: String, // ISO 8601 UTC
    pub projects: Vec<ProjectRow>,
    pub goals: Vec<GoalRow>,
    pub tasks: Vec<TaskRow>,
    pub microtasks: Vec<MicrotaskRow>,
    pub pomodoro_types: Vec<PomodoroTypeRow>,
    pub plans: Vec<PlanRow>,
    pub work_blocks: Vec<WorkBlockRow>,
    pub focus_sessions: Vec<FocusSessionRow>,
    pub pomodoro_sessions: Vec<PomodoroSessionRow>,
    pub settings: Vec<SettingRow>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct ProjectRow {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub status: String,
    pub is_archived: i64,
    pub completed_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct GoalRow {
    pub id: String,
    pub project_id: String,
    pub title: String,
    pub description: Option<String>,
    pub deadline: Option<String>,
    pub priority: i64,
    pub sort_order: i64,
    pub status: String,
    pub is_archived: i64,
    pub completed_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct TaskRow {
    pub id: String,
    pub goal_id: String,
    pub title: String,
    pub description: Option<String>,
    pub deadline: Option<String>,
    pub priority: i64,
    pub sort_order: i64,
    pub status: String,
    pub is_archived: i64,
    pub completed_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct MicrotaskRow {
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
    pub is_archived: i64,
    pub completed_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct PomodoroTypeRow {
    pub id: String,
    pub name: String,
    pub work_minutes: i64,
    pub rest_minutes: i64,
    pub long_break_minutes: Option<i64>,
    pub long_break_every: Option<i64>,
    pub is_default: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct PlanRow {
    pub id: String,
    pub date: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct WorkBlockRow {
    pub id: String,
    pub plan_id: String,
    pub block_type: String,
    pub microtask_id: Option<String>,
    pub calendar_event_id: Option<String>,
    pub start_time: String,
    pub end_time: String,
    pub pomodoro_index: Option<i64>,
    pub sort_order: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct FocusSessionRow {
    pub id: String,
    pub plan_id: String,
    pub start_time: String,
    pub end_time: String,
    pub total_work_seconds: i64,
    pub total_break_seconds: i64,
    pub blocks_completed: i64,
    pub blocks_skipped: i64,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct PomodoroSessionRow {
    pub id: String,
    pub focus_session_id: Option<String>,
    pub microtask_id: Option<String>,
    pub pomodoro_type_id: Option<String>,
    pub work_minutes: i64,
    pub started_at: String,
    pub completed_at: String,
    pub was_completed: i64,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct SettingRow {
    pub key: String,
    pub value: String,
    pub updated_at: String,
}
```

- [ ] **Step 2: Add the `Io` variant to `src-tauri/src/error.rs`** (additive extension of the Phase 1 convention — existing variants and codes are untouched)

In the `AppError` enum, add:

```rust
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
```

In the `Serialize` impl's `match`, add the arm:

```rust
            AppError::Io(_) => "io",
```

- [ ] **Step 3: Extend the TS error union in `src/ipc/types.ts`**

```ts
export interface IpcError {
  code: "db" | "not_found" | "validation" | "io";
  message: string;
}
```

- [ ] **Step 4: Verify it compiles**

Run: `cargo check --manifest-path src-tauri/Cargo.toml && npx vue-tsc --noEmit`
Expected: both clean.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/models src-tauri/src/error.rs src/ipc/types.ts
git commit -m "feat: versioned ExportFile format mirroring the schema + AppError::Io"
```

---

### Task 7: `export_data` service — one-transaction snapshot to JSON `[medium]`

**Files:**
- Create: `src-tauri/src/core/backup_service.rs`
- Modify: `src-tauri/src/core/mod.rs`
- Test: `src-tauri/tests/backup.rs`

- [ ] **Step 1: Write the failing test** *(test designed by the strongest agent)*

`src-tauri/tests/backup.rs`:

```rust
use focus_planner_lib::core::backup_service;
use focus_planner_lib::models::export::ExportFile;
use sqlx::SqlitePool;

fn tmp_path(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "focus-planner-test-{}-{name}.json",
        std::process::id()
    ))
}

#[sqlx::test]
async fn export_writes_a_versioned_snapshot_of_all_tables(pool: SqlitePool) {
    const T: &str = "2026-06-09T08:00:00Z";
    sqlx::query(
        "INSERT INTO projects (id, name, status, is_archived, created_at, updated_at) VALUES
         ('p1', 'Alpha', 'open', 0, ?1, ?1),
         ('p2', 'Beta',  'open', 0, ?1, ?1)",
    )
    .bind(T).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO settings (key, value, updated_at) VALUES ('audio_volume', '0.8', ?1)")
        .bind(T).execute(&pool).await.unwrap();

    let path = tmp_path("export");
    backup_service::export_data(&pool, path.to_str().unwrap())
        .await
        .unwrap();

    let raw = std::fs::read_to_string(&path).unwrap();
    std::fs::remove_file(&path).ok();
    let file: ExportFile = serde_json::from_str(&raw).unwrap();

    assert_eq!(file.version, 1);
    assert_eq!(file.exported_at.len(), 20); // YYYY-MM-DDTHH:MM:SSZ
    assert!(file.exported_at.ends_with('Z'));
    assert_eq!(file.projects.len(), 2);
    assert_eq!(file.projects[0].name, "Alpha"); // ordered by id
    assert_eq!(file.pomodoro_types.len(), 1); // the migration-seeded Standard type
    assert_eq!(file.pomodoro_types[0].name, "Standard");
    assert_eq!(file.settings.len(), 1);
    assert_eq!(file.settings[0].value, "0.8");
    assert!(file.goals.is_empty());
    assert!(file.tasks.is_empty());
    assert!(file.microtasks.is_empty());
    assert!(file.plans.is_empty());
    assert!(file.work_blocks.is_empty());
    assert!(file.focus_sessions.is_empty());
    assert!(file.pomodoro_sessions.is_empty());
}
```

Add `serde_json` to `[dev-dependencies]` in `src-tauri/Cargo.toml` if it is not already a dependency (it is a normal dependency since Phase 1, so the test can use it directly — verify before adding).

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test backup`
Expected: FAIL to compile with `error[E0432]` — `core::backup_service` does not exist.

- [ ] **Step 3: Write the export half of `src-tauri/src/core/backup_service.rs`** (+ `pub mod backup_service;` in `src-tauri/src/core/mod.rs`)

Runtime (`query_as::<_, Row>`) queries instead of the `query_as!` macros — a deliberate, documented exception for bulk dump/restore: 10 mechanical full-table reads whose shape is pinned by the row structs and the round-trip test (Task 9), with no `.sqlx/` cache churn. `exported_at` comes from SQLite's clock (`strftime`), matching how migrations stamp timestamps — no new time dependency.

```rust
use crate::error::AppError;
use crate::models::export::{
    ExportFile, FocusSessionRow, GoalRow, MicrotaskRow, PlanRow, PomodoroSessionRow,
    PomodoroTypeRow, ProjectRow, SettingRow, TaskRow, EXPORT_VERSION,
};
use sqlx::SqlitePool;

/// Writes a versioned JSON dump of all 10 tables to `path`.
/// All reads happen inside ONE transaction → one consistent snapshot.
/// CQS: this is a command — it writes a file and returns no data.
pub async fn export_data(pool: &SqlitePool, path: &str) -> Result<(), AppError> {
    let mut tx = pool.begin().await?;

    let exported_at: String =
        sqlx::query_scalar("SELECT strftime('%Y-%m-%dT%H:%M:%SZ', 'now')")
            .fetch_one(&mut *tx)
            .await?;

    let projects = sqlx::query_as::<_, ProjectRow>(
        "SELECT id, name, description, status, is_archived, completed_at, created_at, updated_at
         FROM projects ORDER BY id",
    )
    .fetch_all(&mut *tx)
    .await?;

    let goals = sqlx::query_as::<_, GoalRow>(
        "SELECT id, project_id, title, description, deadline, priority, sort_order, status, is_archived, completed_at, created_at, updated_at
         FROM goals ORDER BY id",
    )
    .fetch_all(&mut *tx)
    .await?;

    let tasks = sqlx::query_as::<_, TaskRow>(
        "SELECT id, goal_id, title, description, deadline, priority, sort_order, status, is_archived, completed_at, created_at, updated_at
         FROM tasks ORDER BY id",
    )
    .fetch_all(&mut *tx)
    .await?;

    let microtasks = sqlx::query_as::<_, MicrotaskRow>(
        "SELECT id, task_id, title, estimated_minutes, pomodoro_count, pomodoro_type_id, deadline, priority, sort_order, status, is_archived, completed_at, created_at, updated_at
         FROM microtasks ORDER BY id",
    )
    .fetch_all(&mut *tx)
    .await?;

    let pomodoro_types = sqlx::query_as::<_, PomodoroTypeRow>(
        "SELECT id, name, work_minutes, rest_minutes, long_break_minutes, long_break_every, is_default, created_at, updated_at
         FROM pomodoro_types ORDER BY id",
    )
    .fetch_all(&mut *tx)
    .await?;

    let plans = sqlx::query_as::<_, PlanRow>(
        "SELECT id, date, status, created_at, updated_at FROM plans ORDER BY id",
    )
    .fetch_all(&mut *tx)
    .await?;

    let work_blocks = sqlx::query_as::<_, WorkBlockRow>(
        "SELECT id, plan_id, block_type, microtask_id, calendar_event_id, start_time, end_time, pomodoro_index, sort_order, created_at, updated_at
         FROM work_blocks ORDER BY id",
    )
    .fetch_all(&mut *tx)
    .await?;

    let focus_sessions = sqlx::query_as::<_, FocusSessionRow>(
        "SELECT id, plan_id, start_time, end_time, total_work_seconds, total_break_seconds, blocks_completed, blocks_skipped, created_at
         FROM focus_sessions ORDER BY id",
    )
    .fetch_all(&mut *tx)
    .await?;

    let pomodoro_sessions = sqlx::query_as::<_, PomodoroSessionRow>(
        "SELECT id, focus_session_id, microtask_id, pomodoro_type_id, work_minutes, started_at, completed_at, was_completed, created_at
         FROM pomodoro_sessions ORDER BY id",
    )
    .fetch_all(&mut *tx)
    .await?;

    let settings = sqlx::query_as::<_, SettingRow>(
        "SELECT key, value, updated_at FROM settings ORDER BY key",
    )
    .fetch_all(&mut *tx)
    .await?;

    tx.commit().await?;

    let file = ExportFile {
        version: EXPORT_VERSION,
        exported_at,
        projects,
        goals,
        tasks,
        microtasks,
        pomodoro_types,
        plans,
        work_blocks,
        focus_sessions,
        pomodoro_sessions,
        settings,
    };

    tracing::info!(
        path,
        projects = file.projects.len(),
        goals = file.goals.len(),
        tasks = file.tasks.len(),
        microtasks = file.microtasks.len(),
        pomodoro_types = file.pomodoro_types.len(),
        plans = file.plans.len(),
        work_blocks = file.work_blocks.len(),
        focus_sessions = file.focus_sessions.len(),
        pomodoro_sessions = file.pomodoro_sessions.len(),
        settings = file.settings.len(),
        "export: consistent snapshot taken, writing file"
    );

    let json = serde_json::to_string_pretty(&file)
        .map_err(|e| AppError::Validation(format!("could not serialize export: {e}")))?;
    std::fs::write(path, json)?; // io errors → AppError::Io via #[from]
    tracing::info!(path, "export: file written");
    Ok(())
}
```

Add the missing import at the top alongside the others: `WorkBlockRow` is in the `use crate::models::export::{...}` list — keep that list exactly as written above plus `WorkBlockRow`:

```rust
use crate::models::export::{
    ExportFile, FocusSessionRow, GoalRow, MicrotaskRow, PlanRow, PomodoroSessionRow,
    PomodoroTypeRow, ProjectRow, SettingRow, TaskRow, WorkBlockRow, EXPORT_VERSION,
};
```

- [ ] **Step 4: Run to verify it passes**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test backup`
Expected: 1 passed.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/core src-tauri/tests/backup.rs
git commit -m "feat: export_data — one-transaction snapshot of all tables to versioned JSON"
```

---

### Task 8: `import_data` service — validate version, wipe, restore in one transaction `[hard]`

**⚠️ DESTRUCTIVE BY DESIGN.** M1 import = **full restore**: every existing row in every table is deleted (children before parents), then the file's rows are inserted (parents before children: pomodoro_types, projects, goals, tasks, microtasks, plans, work_blocks, focus_sessions, pomodoro_sessions, settings). Everything happens in ONE transaction — a failure anywhere rolls back to the pre-import state. Re-importing the same file is idempotent.

**Files:**
- Modify: `src-tauri/src/core/backup_service.rs`
- Test: `src-tauri/tests/backup.rs`

- [ ] **Step 1: Write the failing tests** *(tests designed by the strongest agent)*

Append to `src-tauri/tests/backup.rs` (extends the existing `use` lines — full set shown):

```rust
use focus_planner_lib::error::AppError;
use focus_planner_lib::models::export::{ProjectRow, EXPORT_VERSION};

fn empty_export(version: u32) -> ExportFile {
    ExportFile {
        version,
        exported_at: "2026-06-09T08:00:00Z".to_string(),
        projects: vec![],
        goals: vec![],
        tasks: vec![],
        microtasks: vec![],
        pomodoro_types: vec![],
        plans: vec![],
        work_blocks: vec![],
        focus_sessions: vec![],
        pomodoro_sessions: vec![],
        settings: vec![],
    }
}

#[sqlx::test]
async fn import_rejects_an_unsupported_version_and_leaves_the_db_untouched(pool: SqlitePool) {
    sqlx::query(
        "INSERT INTO projects (id, name, status, is_archived, created_at, updated_at)
         VALUES ('keep', 'Keep me', 'open', 0, '2026-06-09T08:00:00Z', '2026-06-09T08:00:00Z')",
    )
    .execute(&pool).await.unwrap();

    let path = tmp_path("bad-version");
    std::fs::write(&path, serde_json::to_string(&empty_export(99)).unwrap()).unwrap();

    let err = backup_service::import_data(&pool, path.to_str().unwrap())
        .await
        .expect_err("version 99 must be rejected");
    std::fs::remove_file(&path).ok();

    assert!(matches!(err, AppError::Validation(_)));
    assert!(err.to_string().contains("version"));

    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM projects")
        .fetch_one(&pool).await.unwrap();
    assert_eq!(count, 1, "a rejected import must not touch the database");
}

#[sqlx::test]
async fn import_rejects_files_that_are_not_an_export(pool: SqlitePool) {
    let path = tmp_path("garbage");
    std::fs::write(&path, "this is not json").unwrap();

    let err = backup_service::import_data(&pool, path.to_str().unwrap())
        .await
        .expect_err("garbage must be rejected");
    std::fs::remove_file(&path).ok();

    assert!(matches!(err, AppError::Validation(_)));
}

#[sqlx::test]
async fn import_wipes_all_existing_rows_and_restores_the_file(pool: SqlitePool) {
    // Existing data that must disappear — including the migration-seeded Standard type.
    sqlx::query(
        "INSERT INTO projects (id, name, status, is_archived, created_at, updated_at)
         VALUES ('old', 'Old project', 'open', 0, '2026-06-09T08:00:00Z', '2026-06-09T08:00:00Z')",
    )
    .execute(&pool).await.unwrap();

    let mut file = empty_export(EXPORT_VERSION);
    file.projects.push(ProjectRow {
        id: "new".to_string(),
        name: "Imported project".to_string(),
        description: None,
        status: "open".to_string(),
        is_archived: 0,
        completed_at: None,
        created_at: "2026-06-09T08:00:00Z".to_string(),
        updated_at: "2026-06-09T08:00:00Z".to_string(),
    });

    let path = tmp_path("restore");
    std::fs::write(&path, serde_json::to_string(&file).unwrap()).unwrap();
    backup_service::import_data(&pool, path.to_str().unwrap())
        .await
        .unwrap();
    std::fs::remove_file(&path).ok();

    let names: Vec<String> = sqlx::query_scalar("SELECT name FROM projects")
        .fetch_all(&pool).await.unwrap();
    assert_eq!(names, vec!["Imported project".to_string()]);

    // FULL restore: the seeded Standard pomodoro type was wiped too,
    // because the imported file did not contain it.
    let types: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM pomodoro_types")
        .fetch_one(&pool).await.unwrap();
    assert_eq!(types, 0);
}
```

- [ ] **Step 2: Run to verify they fail**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test backup`
Expected: FAIL to compile — `backup_service::import_data` does not exist.

- [ ] **Step 3: Append the import half to `src-tauri/src/core/backup_service.rs`**

```rust
/// Wipe order: children before parents, so foreign keys never dangle mid-transaction.
const WIPE_ORDER: [&str; 10] = [
    "settings",
    "pomodoro_sessions",
    "focus_sessions",
    "work_blocks",
    "plans",
    "microtasks",
    "tasks",
    "goals",
    "projects",
    "pomodoro_types",
];

/// Restores the database from an export file. DESTRUCTIVE: deletes every existing
/// row first (M1 import = full restore). Validates the version, then runs wipe +
/// insert inside ONE transaction — any failure rolls back completely.
/// Idempotent: importing the same file twice produces the same end state.
/// CQS: this is a command — it mutates and returns no data.
pub async fn import_data(pool: &SqlitePool, path: &str) -> Result<(), AppError> {
    let raw = std::fs::read_to_string(path)?; // io errors → AppError::Io

    let file: ExportFile = serde_json::from_str(&raw).map_err(|e| {
        tracing::error!(path, error = %e, "import rejected: file is not a valid export");
        AppError::Validation(format!("not a valid Focus Planner export file: {e}"))
    })?;

    if file.version != EXPORT_VERSION {
        tracing::error!(
            path,
            found_version = file.version,
            expected_version = EXPORT_VERSION,
            "import rejected: unsupported export version"
        );
        return Err(AppError::Validation(format!(
            "unsupported export version {} (this app reads version {EXPORT_VERSION})",
            file.version
        )));
    }

    let mut tx = pool.begin().await?;

    let mut wiped: u64 = 0;
    for table in WIPE_ORDER {
        wiped += sqlx::query(&format!("DELETE FROM {table}"))
            .execute(&mut *tx)
            .await?
            .rows_affected();
    }
    tracing::info!(wiped, "import: wiped {wiped} existing rows (full restore)");

    // Insert parents before children.
    for r in &file.pomodoro_types {
        sqlx::query(
            "INSERT INTO pomodoro_types (id, name, work_minutes, rest_minutes, long_break_minutes, long_break_every, is_default, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        )
        .bind(&r.id).bind(&r.name).bind(r.work_minutes).bind(r.rest_minutes)
        .bind(r.long_break_minutes).bind(r.long_break_every).bind(r.is_default)
        .bind(&r.created_at).bind(&r.updated_at)
        .execute(&mut *tx).await?;
    }
    for r in &file.projects {
        sqlx::query(
            "INSERT INTO projects (id, name, description, status, is_archived, completed_at, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        )
        .bind(&r.id).bind(&r.name).bind(&r.description).bind(&r.status)
        .bind(r.is_archived).bind(&r.completed_at).bind(&r.created_at).bind(&r.updated_at)
        .execute(&mut *tx).await?;
    }
    for r in &file.goals {
        sqlx::query(
            "INSERT INTO goals (id, project_id, title, description, deadline, priority, sort_order, status, is_archived, completed_at, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
        )
        .bind(&r.id).bind(&r.project_id).bind(&r.title).bind(&r.description)
        .bind(&r.deadline).bind(r.priority).bind(r.sort_order).bind(&r.status)
        .bind(r.is_archived).bind(&r.completed_at).bind(&r.created_at).bind(&r.updated_at)
        .execute(&mut *tx).await?;
    }
    for r in &file.tasks {
        sqlx::query(
            "INSERT INTO tasks (id, goal_id, title, description, deadline, priority, sort_order, status, is_archived, completed_at, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
        )
        .bind(&r.id).bind(&r.goal_id).bind(&r.title).bind(&r.description)
        .bind(&r.deadline).bind(r.priority).bind(r.sort_order).bind(&r.status)
        .bind(r.is_archived).bind(&r.completed_at).bind(&r.created_at).bind(&r.updated_at)
        .execute(&mut *tx).await?;
    }
    for r in &file.microtasks {
        sqlx::query(
            "INSERT INTO microtasks (id, task_id, title, estimated_minutes, pomodoro_count, pomodoro_type_id, deadline, priority, sort_order, status, is_archived, completed_at, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
        )
        .bind(&r.id).bind(&r.task_id).bind(&r.title).bind(r.estimated_minutes)
        .bind(r.pomodoro_count).bind(&r.pomodoro_type_id).bind(&r.deadline)
        .bind(r.priority).bind(r.sort_order).bind(&r.status).bind(r.is_archived)
        .bind(&r.completed_at).bind(&r.created_at).bind(&r.updated_at)
        .execute(&mut *tx).await?;
    }
    for r in &file.plans {
        sqlx::query(
            "INSERT INTO plans (id, date, status, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
        )
        .bind(&r.id).bind(&r.date).bind(&r.status).bind(&r.created_at).bind(&r.updated_at)
        .execute(&mut *tx).await?;
    }
    for r in &file.work_blocks {
        sqlx::query(
            "INSERT INTO work_blocks (id, plan_id, block_type, microtask_id, calendar_event_id, start_time, end_time, pomodoro_index, sort_order, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        )
        .bind(&r.id).bind(&r.plan_id).bind(&r.block_type).bind(&r.microtask_id)
        .bind(&r.calendar_event_id).bind(&r.start_time).bind(&r.end_time)
        .bind(r.pomodoro_index).bind(r.sort_order).bind(&r.created_at).bind(&r.updated_at)
        .execute(&mut *tx).await?;
    }
    for r in &file.focus_sessions {
        sqlx::query(
            "INSERT INTO focus_sessions (id, plan_id, start_time, end_time, total_work_seconds, total_break_seconds, blocks_completed, blocks_skipped, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        )
        .bind(&r.id).bind(&r.plan_id).bind(&r.start_time).bind(&r.end_time)
        .bind(r.total_work_seconds).bind(r.total_break_seconds)
        .bind(r.blocks_completed).bind(r.blocks_skipped).bind(&r.created_at)
        .execute(&mut *tx).await?;
    }
    for r in &file.pomodoro_sessions {
        sqlx::query(
            "INSERT INTO pomodoro_sessions (id, focus_session_id, microtask_id, pomodoro_type_id, work_minutes, started_at, completed_at, was_completed, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        )
        .bind(&r.id).bind(&r.focus_session_id).bind(&r.microtask_id)
        .bind(&r.pomodoro_type_id).bind(r.work_minutes).bind(&r.started_at)
        .bind(&r.completed_at).bind(r.was_completed).bind(&r.created_at)
        .execute(&mut *tx).await?;
    }
    for r in &file.settings {
        sqlx::query("INSERT INTO settings (key, value, updated_at) VALUES (?1, ?2, ?3)")
            .bind(&r.key).bind(&r.value).bind(&r.updated_at)
            .execute(&mut *tx).await?;
    }

    tx.commit().await?;

    tracing::info!(
        path,
        version = file.version,
        wiped,
        projects = file.projects.len(),
        goals = file.goals.len(),
        tasks = file.tasks.len(),
        microtasks = file.microtasks.len(),
        pomodoro_types = file.pomodoro_types.len(),
        plans = file.plans.len(),
        work_blocks = file.work_blocks.len(),
        focus_sessions = file.focus_sessions.len(),
        pomodoro_sessions = file.pomodoro_sessions.len(),
        settings = file.settings.len(),
        "import complete: full restore"
    );
    Ok(())
}
```

- [ ] **Step 4: Run to verify they pass**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test backup`
Expected: 4 passed (1 export + 3 import).

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/core/backup_service.rs src-tauri/tests/backup.rs
git commit -m "feat: import_data — version-validated, one-transaction full restore (wipes existing rows)"
```

---

### Task 9: Round-trip integrity test — seed → export → wipe → import → verify `[medium]`

The integration pin for the whole backup feature. If Tasks 7–8 are correct this passes on first run — it stays in the suite to catch any future drift between the export queries, the row structs, and the import inserts.

**Files:**
- Test: `src-tauri/tests/backup.rs` (append)

- [ ] **Step 1: Write the test** *(test designed by the strongest agent)*

Append to `src-tauri/tests/backup.rs`:

```rust
/// The id seeded by migration 0002.
const STANDARD_TYPE_ID: &str = "a0000000-0000-4000-8000-000000000001";

/// One row in every one of the 10 tables, FK-linked end to end.
async fn seed_full_tree(pool: &SqlitePool) {
    const T: &str = "2026-06-01T08:00:00Z";
    sqlx::query(
        "INSERT INTO projects (id, name, description, status, is_archived, created_at, updated_at)
         VALUES ('pr1', 'Website', 'marketing site', 'open', 0, ?1, ?1)",
    )
    .bind(T).execute(pool).await.unwrap();
    sqlx::query(
        "INSERT INTO goals (id, project_id, title, status, is_archived, created_at, updated_at)
         VALUES ('g1', 'pr1', 'Launch', 'open', 0, ?1, ?1)",
    )
    .bind(T).execute(pool).await.unwrap();
    sqlx::query(
        "INSERT INTO tasks (id, goal_id, title, status, is_archived, created_at, updated_at)
         VALUES ('t1', 'g1', 'Build landing page', 'open', 0, ?1, ?1)",
    )
    .bind(T).execute(pool).await.unwrap();
    sqlx::query(
        "INSERT INTO microtasks (id, task_id, title, estimated_minutes, pomodoro_count, pomodoro_type_id, status, is_archived, created_at, updated_at)
         VALUES ('m1', 't1', 'Hero section', 40, 2, ?2, 'open', 0, ?1, ?1)",
    )
    .bind(T).bind(STANDARD_TYPE_ID).execute(pool).await.unwrap();
    sqlx::query(
        "INSERT INTO plans (id, date, status, created_at, updated_at)
         VALUES ('pl1', '2026-06-01', 'committed', ?1, ?1)",
    )
    .bind(T).execute(pool).await.unwrap();
    sqlx::query(
        "INSERT INTO work_blocks (id, plan_id, block_type, microtask_id, start_time, end_time, pomodoro_index, sort_order, created_at, updated_at)
         VALUES ('wb1', 'pl1', 'task', 'm1', '2026-06-01T09:00:00Z', '2026-06-01T09:20:00Z', 1, 0, ?1, ?1)",
    )
    .bind(T).execute(pool).await.unwrap();
    sqlx::query(
        "INSERT INTO focus_sessions (id, plan_id, start_time, end_time, total_work_seconds, total_break_seconds, blocks_completed, blocks_skipped, created_at)
         VALUES ('fs1', 'pl1', '2026-06-01T09:00:00Z', '2026-06-01T11:00:00Z', 4800, 1200, 4, 1, ?1)",
    )
    .bind(T).execute(pool).await.unwrap();
    sqlx::query(
        "INSERT INTO pomodoro_sessions (id, focus_session_id, microtask_id, pomodoro_type_id, work_minutes, started_at, completed_at, was_completed, created_at)
         VALUES ('s1', 'fs1', 'm1', ?2, 20, '2026-06-01T09:00:00Z', '2026-06-01T09:20:00Z', 1, ?1)",
    )
    .bind(T).bind(STANDARD_TYPE_ID).execute(pool).await.unwrap();
    sqlx::query("INSERT INTO settings (key, value, updated_at) VALUES ('audio_volume', '0.8', ?1)")
        .bind(T).execute(pool).await.unwrap();
    // pomodoro_types row: the migration-seeded Standard type (referenced above) — 10/10 tables populated.
}

#[sqlx::test]
async fn round_trip_export_wipe_import_restores_everything(pool: SqlitePool) {
    seed_full_tree(&pool).await;

    let path = tmp_path("round-trip");
    backup_service::export_data(&pool, path.to_str().unwrap())
        .await
        .unwrap();

    // Wipe by hand, children first (foreign keys are ON in sqlx's SQLite pools).
    for table in [
        "settings", "pomodoro_sessions", "focus_sessions", "work_blocks", "plans",
        "microtasks", "tasks", "goals", "projects", "pomodoro_types",
    ] {
        sqlx::query(&format!("DELETE FROM {table}"))
            .execute(&pool).await.unwrap();
    }
    // Junk the import must clear before restoring.
    sqlx::query(
        "INSERT INTO projects (id, name, status, is_archived, created_at, updated_at)
         VALUES ('junk', 'Junk', 'open', 0, '2026-06-09T08:00:00Z', '2026-06-09T08:00:00Z')",
    )
    .execute(&pool).await.unwrap();

    backup_service::import_data(&pool, path.to_str().unwrap())
        .await
        .unwrap();
    std::fs::remove_file(&path).ok();

    // Row counts: exactly what was seeded, in every table.
    for (table, expected) in [
        ("projects", 1i64), ("goals", 1), ("tasks", 1), ("microtasks", 1),
        ("pomodoro_types", 1), ("plans", 1), ("work_blocks", 1),
        ("focus_sessions", 1), ("pomodoro_sessions", 1), ("settings", 1),
    ] {
        let count: i64 = sqlx::query_scalar(&format!("SELECT COUNT(*) FROM {table}"))
            .fetch_one(&pool).await.unwrap();
        assert_eq!(count, expected, "row count mismatch in {table}");
    }

    // Spot-check values survived byte-for-byte.
    let title: String = sqlx::query_scalar("SELECT title FROM microtasks WHERE id = 'm1'")
        .fetch_one(&pool).await.unwrap();
    assert_eq!(title, "Hero section");
    let est: i64 = sqlx::query_scalar("SELECT estimated_minutes FROM microtasks WHERE id = 'm1'")
        .fetch_one(&pool).await.unwrap();
    assert_eq!(est, 40);
    let work: i64 = sqlx::query_scalar("SELECT total_work_seconds FROM focus_sessions WHERE id = 'fs1'")
        .fetch_one(&pool).await.unwrap();
    assert_eq!(work, 4800);
    let type_name: String = sqlx::query_scalar("SELECT name FROM pomodoro_types")
        .fetch_one(&pool).await.unwrap();
    assert_eq!(type_name, "Standard");
    let vol: String = sqlx::query_scalar("SELECT value FROM settings WHERE key = 'audio_volume'")
        .fetch_one(&pool).await.unwrap();
    assert_eq!(vol, "0.8");
    let junk: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM projects WHERE id = 'junk'")
        .fetch_one(&pool).await.unwrap();
    assert_eq!(junk, 0, "import must have cleared the junk row");
}
```

- [ ] **Step 2: Run it**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test backup`
Expected: 5 passed. (If this fails, the export queries, row structs, and import inserts have drifted — fix the service, not the test.)

- [ ] **Step 3: Commit**

```bash
git add src-tauri/tests/backup.rs
git commit -m "test: round-trip integrity — export, wipe, import restores all 10 tables"
```

---

### Task 10: `export_data` / `import_data` IPC commands `[easy]`

Thin handlers (services fully tested in Tasks 7–9). CQS: both take the dialog-chosen `path` from the UI and return `Result<(), AppError>` — no data.

**Files:**
- Create: `src-tauri/src/commands/backup.rs`
- Modify: `src-tauri/src/commands/mod.rs`, `src-tauri/src/lib.rs`

- [ ] **Step 1: Write `src-tauri/src/commands/backup.rs`** (+ `pub mod backup;` in `src-tauri/src/commands/mod.rs`)

```rust
use crate::core::backup_service;
use crate::db::Db;
use crate::error::AppError;

#[tauri::command]
#[tracing::instrument(skip(db))] // path stays as a field → the file path is in the log line
pub async fn export_data(db: tauri::State<'_, Db>, path: String) -> Result<(), AppError> {
    let result = backup_service::export_data(&db.0, &path).await;
    match &result {
        Ok(()) => tracing::info!("ok"),
        Err(e) => tracing::error!(error = %e, "failed"),
    }
    result
}

#[tauri::command]
#[tracing::instrument(skip(db))]
pub async fn import_data(db: tauri::State<'_, Db>, path: String) -> Result<(), AppError> {
    let result = backup_service::import_data(&db.0, &path).await;
    match &result {
        Ok(()) => tracing::info!("ok"),
        Err(e) => tracing::error!(error = %e, "failed"),
    }
    result
}
```

- [ ] **Step 2: Register both in `src-tauri/src/lib.rs`**

Append to the existing `tauri::generate_handler![...]` list:

```rust
            commands::backup::export_data,
            commands::backup::import_data,
```

- [ ] **Step 3: Verify**

Run: `cargo test --manifest-path src-tauri/Cargo.toml`
Expected: full suite passes (including the 8 Phase 5 Rust tests).

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/commands src-tauri/src/lib.rs
git commit -m "feat: export_data / import_data IPC commands"
```

---

### Task 11: Settings view — Export/Import buttons with confirm dialog `[medium]`

The dialog plugin cannot run under vitest (it needs the native shell), so this component is deliberately thin — all logic lives in the tested Rust services — and is verified manually (roadmap: no E2E in M1). **POLA requirement:** the import confirm dialog must literally say it **replaces all current data**.

**Files:**
- Create: `src/components/DataBackupSection.vue`
- Modify: `src/views/SettingsView.vue`

- [ ] **Step 1: Write `src/components/DataBackupSection.vue`**

```vue
<script setup lang="ts">
import { ref } from "vue";
import { ask, message, open, save } from "@tauri-apps/plugin-dialog";
import { ipc } from "../ipc/client";

const busy = ref(false);
const status = ref<string | null>(null);

async function onExport() {
  const path = await save({
    title: "Export Focus Planner data",
    defaultPath: `focus-planner-export-${new Date().toISOString().slice(0, 10)}.json`,
    filters: [{ name: "JSON", extensions: ["json"] }],
  });
  if (!path) return; // user cancelled — do nothing
  busy.value = true;
  status.value = null;
  try {
    await ipc<void>("export_data", { path });
    status.value = `Exported to ${path}`;
  } catch (e) {
    status.value = `Export failed: ${(e as { message?: string }).message ?? String(e)}`;
  } finally {
    busy.value = false;
  }
}

async function onImport() {
  const path = await open({
    title: "Import Focus Planner data",
    multiple: false,
    filters: [{ name: "JSON", extensions: ["json"] }],
  });
  if (typeof path !== "string") return; // user cancelled — do nothing

  // POLA: import is destructive — say so, explicitly, before doing anything.
  const confirmed = await ask(
    "Importing REPLACES ALL current data with the file's contents. " +
      "Every project, plan, and session currently in the app will be deleted. " +
      "This cannot be undone. Continue?",
    { title: "Replace all data?", kind: "warning" }
  );
  if (!confirmed) return;

  busy.value = true;
  status.value = null;
  try {
    await ipc<void>("import_data", { path });
    await message("Import complete. The app will now reload.", { title: "Import" });
    window.location.reload(); // KISS: a webview reload re-initializes every store from the restored DB
  } catch (e) {
    status.value = `Import failed: ${(e as { message?: string }).message ?? String(e)}`;
    busy.value = false;
  }
}
</script>

<template>
  <div class="backup-section">
    <h2>Data</h2>
    <p class="hint">
      Export writes your entire database to one JSON file. Import restores from such a
      file and <strong>replaces all current data</strong>.
    </p>
    <div class="backup-buttons">
      <button :disabled="busy" @click="onExport">Export Data…</button>
      <button :disabled="busy" class="danger" @click="onImport">Import Data…</button>
    </div>
    <p v-if="status" class="status">{{ status }}</p>
  </div>
</template>

<style scoped>
.backup-section { margin-top: 32px; }
.hint { color: #9aa3b2; font-size: 13px; max-width: 480px; }
.backup-buttons { display: flex; gap: 8px; margin-top: 8px; }
.backup-buttons button {
  background: #1f2630; color: #e6e9ef; border: 1px solid #20242b;
  border-radius: 8px; padding: 8px 14px; cursor: pointer;
}
.backup-buttons button:hover:enabled { background: #2a3340; }
.backup-buttons button:disabled { opacity: 0.5; cursor: default; }
.backup-buttons button.danger { border-color: #5a2a2a; color: #ff9d9d; }
.status { font-size: 13px; color: #9aa3b2; margin-top: 8px; }
</style>
```

- [ ] **Step 2: Mount it in `src/views/SettingsView.vue`**

Add to the `<script setup lang="ts">` block:

```ts
import DataBackupSection from "../components/DataBackupSection.vue";
```

And append inside the view's root `<section>` element, after the existing content:

```vue
    <DataBackupSection />
```

- [ ] **Step 3: Typecheck and run the vitest suite**

Run: `npx vue-tsc --noEmit && npm test -- --run`
Expected: both clean (no test touches the dialog plugin).

- [ ] **Step 4: Verify manually**

Run: `npm run tauri dev` → Settings:
- Export Data… → native save dialog → pick a path → file appears, opens as pretty-printed JSON with `"version": 1` and the 10 arrays.
- Import Data… → native open dialog → pick the file → a native warning dialog says it **replaces ALL current data** → Yes → "Import complete" → app reloads with the restored state.
- Cancel the save dialog, cancel the open dialog, and answer No to the warning: nothing happens in all three cases.
- The log file shows the full story: `export_data` path → snapshot per-table counts → file written; `import_data` path → wiped N existing rows → per-table counts → `import complete: full restore`.

- [ ] **Step 5: Commit**

```bash
git add src/components/DataBackupSection.vue src/views/SettingsView.vue
git commit -m "feat: Settings export/import buttons with destructive-import confirm dialog"
```

---

### Task 12: Docs — README, export-format-for-dummies, migration history check `[easy]`

**Files:**
- Modify: `README.md`
- Create: `docs/db-context/export-format-for-dummies.md`
- Verify-only (NO change): `docs/db-context/migration-history.md`

- [ ] **Step 1: Update `README.md`**

Update the current-state section to Phase 5: add two feature bullets — "**Analytics:** pick a date range (last 7/30 days or custom) and see per-day focus bars, totals, completion rate, and a per-project breakdown" and "**Backup:** Settings → Export Data writes your whole database to one JSON file; Import Data restores from it (replacing all current data)". Keep the five-minute-newcomer bar: run/test instructions and doc pointers stay accurate.

- [ ] **Step 2: Write `docs/db-context/export-format-for-dummies.md`** (per PHILOSOPHY's for-dummies docs)

```markdown
# Export File Format — For Dummies

One JSON file = your entire Focus Planner database. That's the whole idea.

## What's in the file

```json
{
  "version": 1,
  "exported_at": "2026-06-09T17:30:00Z",
  "projects": [ ... ],
  "goals": [ ... ],
  "tasks": [ ... ],
  "microtasks": [ ... ],
  "pomodoro_types": [ ... ],
  "plans": [ ... ],
  "work_blocks": [ ... ],
  "focus_sessions": [ ... ],
  "pomodoro_sessions": [ ... ],
  "settings": [ ... ]
}
```

- `version` — the file-format version, currently `1`. Import refuses any other
  number, so an old app can never half-understand a newer file.
- `exported_at` — when the snapshot was taken (UTC). Informational only.
- Ten arrays, one per database table. Every object inside is a table row,
  verbatim: same column names, same values (see `schema-for-dummies.md` for what
  each table means). Booleans appear as `0`/`1` because that's how SQLite stores
  them.

## How export works

All ten tables are read inside **one database transaction**, so the file is a
consistent snapshot — no half-written day can leak in while you export.

## How import works — READ THIS

Import is a **full restore**: it deletes *everything* currently in the app, then
inserts the file's rows. It is not a merge. The UI warns you before doing it.
It all happens in one transaction — if anything fails, the database rolls back
to exactly what it was before. Rows are inserted parents-first
(pomodoro_types → projects → goals → tasks → microtasks → plans → work_blocks →
focus_sessions → pomodoro_sessions → settings) so no row ever points at a parent
that doesn't exist yet.

## Where the code lives

- Format: `src-tauri/src/models/export.rs` (`ExportFile`, `EXPORT_VERSION`)
- Logic: `src-tauri/src/core/backup_service.rs`
- Proof it round-trips: `src-tauri/tests/backup.rs`
```

- [ ] **Step 3: Verify `docs/db-context/migration-history.md` is UNCHANGED**

Run: `git diff --stat main -- docs/db-context/migration-history.md src-tauri/migrations`
Expected: **empty output**. Phase 5 makes no schema change, so migration-history must NOT gain a row. **If this shows anything, STOP** — a schema change crept in; revert it or, if it is genuinely needed, amend this plan, append the migration-history row, and record a lesson in `docs/lessons/`.

- [ ] **Step 4: Commit**

```bash
git add README.md docs/db-context/export-format-for-dummies.md
git commit -m "docs: README Phase 5 state + export format for-dummies"
```

---

### Task 13: Phase acceptance — manual QA checklist `[trivial]`

Run the app (`npm run tauri dev`) and walk the demonstrable end-to-end:

- [ ] Run one or two short Start Day sessions (Phase 4 flow) → Analytics shows today's bar, totals row, completion rate, and the project in the table
- [ ] "Last 7 days" / "Last 30 days" presets and custom date inputs all reload the stats; an empty range shows the empty state, never an error or NaN
- [ ] Settings → Export Data… → choose a path → the JSON file exists with `"version": 1`, `exported_at`, and 10 table arrays
- [ ] Quit the app, wipe the dev DB: `rm "$HOME/Library/Application Support/com.sramzz.focusplanner/focus-planner.sqlite"`, relaunch → app is empty (fresh migrations)
- [ ] Settings → Import Data… → pick the exported file → the confirm dialog explicitly says it **replaces all current data** → accept → backlog tree, plans, and Analytics history are all restored
- [ ] Cancel paths do nothing: save-dialog cancel, open-dialog cancel, answering "No" to the confirm
- [ ] Tamper test: edit the exported file's `version` to `99`, import it → clear error shown, existing data untouched
- [ ] **Logs narrative (spec §7 gate):** open `logs/focus-planner.log.<today>` — the export+import session is fully reconstructable from the log alone: `get_stats` with range + counts, `export_data` path → per-table counts → file written, `import_data` path → version → "wiped N existing rows" → per-table counts → "import complete: full restore". A junior with zero context can tell what happened.
- [ ] `cargo test --manifest-path src-tauri/Cargo.toml` (8 new Phase 5 tests among them) and `npm test -- --run` (3 new) pass locally; CI is green on the PR
- [ ] `git diff --stat main -- src-tauri/migrations docs/db-context/migration-history.md` is empty — **no schema change happened in this phase**

---

## Plan self-review (performed at writing time)

1. **Scope coverage:** roadmap Phase 5 row + spec §3 Import/Export, §5 `get_stats`, §6 Analytics view, §7 logging — `get_stats` shape defined (Task 1), SQL-side aggregation (Task 1), Analytics view with presets/totals/CSS bars/per-project/empty state (Tasks 3–4), `useStatsStore` (Task 3), versioned one-snapshot export + one-transaction destructive import with parents-first order (Tasks 6–8, 10), dialog plugin + capability (Task 5), Settings buttons + confirm wording (Task 11), round-trip test (Task 9), docs + QA (Tasks 12–13). No gaps found.
2. **Placeholder scan:** no TBD/TODO/"etc."/"similar to" remain; every test, struct, query, command, component, and doc file is written out in full with exact paths and commands.
3. **Type consistency:** `StatsReport`/`DayStats`/`StatsTotals`/`ProjectStats` field names match between Rust (snake_case + `rename_all = "camelCase"`), the SQL aliases, and `src/ipc/types.ts`; `ExportFile` row structs match spec §2 column names verbatim; service/command names (`stats_service::get_stats`, `backup_service::export_data`/`import_data`) are used identically across tasks; conventions (`AppError`, `Db`, `ipc<T>()`, `#[tracing::instrument(skip(db))]`, `focus_planner_lib`) match the Phase 1 plan.



