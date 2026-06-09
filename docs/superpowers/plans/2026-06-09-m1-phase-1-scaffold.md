# M1 Phase 1 — Scaffold & Walking Skeleton Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Stand up the full Tauri v2 + Vue 3 + Rust + SQLite skeleton with one vertical slice (`list_projects`) proving the whole stack, plus the logging, error, testing, and CI conventions every later phase reuses.

**Architecture:** Tauri v2 desktop shell; Vue 3 + Vite + TS + Pinia frontend that only talks IPC; Rust backend owning SQLite via SQLx with compile-time-checked queries; `tracing` logging to console + a daily-rolling file in the exposed `logs/` folder.

**Tech Stack:** Tauri 2, Vue 3, Vite, TypeScript, Pinia, Vitest, Rust, SQLx 0.8 (sqlite), tokio, thiserror, tracing/tracing-subscriber/tracing-appender.

**Conventions (per `docs/specs/m1-roadmap.md`):** every task carries a difficulty tag. The failing test of each TDD task is designed by the most capable agent; implementation may be assigned by difficulty; every task is reviewed before its commit lands. Scaffold/tooling tasks (1–3) are not TDD-able — they end with manual verification instead.

**Philosophy (PHILOSOPHY.md):** CQS — commands mutate, queries read, never both. Logging per spec §7: a junior must follow the app from `logs/` alone. KISS — no router, no extra deps beyond the list above.

---

### Task 1: Move the legacy app aside `[trivial]`

**Files:**
- Move: `index.html`, `js/`, `styles/` → `legacy/`

- [ ] **Step 1: Move files with git so history follows**

```bash
mkdir legacy
git mv index.html js styles legacy/
```

- [ ] **Step 2: Verify the working tree**

Run: `git status --short`
Expected: three `R` (renamed) entries, nothing else.

- [ ] **Step 3: Commit**

```bash
git commit -m "chore: move legacy vanilla app to legacy/ to free the Vite root"
```

---

### Task 2: Scaffold Tauri v2 + Vue 3 + TypeScript `[easy]`

**Files:**
- Create (via scaffold): `package.json`, `index.html`, `vite.config.ts`, `tsconfig.json`, `src/` (Vue), `src-tauri/` (Rust: `Cargo.toml`, `tauri.conf.json`, `src/main.rs`, `src/lib.rs`, `capabilities/default.json`, `icons/`, `build.rs`)

- [ ] **Step 1: Scaffold into a temp dir (create-tauri-app refuses non-empty dirs), then move contents to the repo root**

```bash
npm create tauri-app@latest tmp-scaffold -- --template vue-ts --manager npm --yes
rsync -a tmp-scaffold/ ./ --exclude .git
rm -rf tmp-scaffold
npm install
```

- [ ] **Step 2: Set the app identity in `src-tauri/tauri.conf.json`**

Edit these keys (leave the rest as scaffolded):

```json
{
  "productName": "Focus Planner",
  "identifier": "com.sramzz.focusplanner",
  "app": {
    "windows": [{ "title": "Focus Planner", "width": 1200, "height": 800 }]
  }
}
```

- [ ] **Step 3: Add Pinia**

```bash
npm install pinia
```

In `src/main.ts`:

```ts
import { createApp } from "vue";
import { createPinia } from "pinia";
import App from "./App.vue";

createApp(App).use(createPinia()).mount("#app");
```

- [ ] **Step 4: Verify the app launches**

Run: `npm run tauri dev`
Expected: a native "Focus Planner" window opens showing the scaffold greeting page. Close it.

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "feat: scaffold Tauri v2 + Vue 3 + TypeScript + Pinia"
```

---

### Task 3: Rust dependencies + module skeleton `[easy]`

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Create: `src-tauri/src/{commands/mod.rs, core/mod.rs, db.rs, error.rs, logging.rs, models/mod.rs}`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Add dependencies to `src-tauri/Cargo.toml`**

```toml
[dependencies]
tauri = { version = "2", features = [] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
sqlx = { version = "0.8", features = ["runtime-tokio", "sqlite", "migrate"] }
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
thiserror = "2"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
tracing-appender = "0.2"
```

- [ ] **Step 2: Create empty module files and wire them in `src-tauri/src/lib.rs`**

```rust
pub mod commands;
pub mod core;
pub mod db;
pub mod error;
pub mod logging;
pub mod models;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

`commands/mod.rs`, `core/mod.rs`, `models/mod.rs` start empty; `db.rs`, `error.rs`, `logging.rs` are filled by Tasks 4–6 (create them empty now so the build passes).

- [ ] **Step 3: Verify it compiles**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`
Expected: success, no warnings about missing modules.

- [ ] **Step 4: Commit**

```bash
git add src-tauri
git commit -m "chore: add Rust deps and Domain Core module skeleton"
```

---

### Task 4: Logging infrastructure (spec §7) `[medium]`

**Files:**
- Create: `src-tauri/src/logging.rs`
- Modify: `src-tauri/src/lib.rs`, `.gitignore`
- Create: `logs/.gitkeep`

- [ ] **Step 1: Write `src-tauri/src/logging.rs`**

In dev the exposed Logs folder is `logs/` at the repo root (resolved from `CARGO_MANIFEST_DIR` so it works regardless of CWD); packaged builds use the OS app-log dir. The returned guard must stay alive for the process lifetime or the file layer silently drops lines.

```rust
use tauri::Manager;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

pub struct LogGuard(pub WorkerGuard);

fn logs_dir(app: &tauri::AppHandle) -> std::path::PathBuf {
    if cfg!(debug_assertions) {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../logs")
    } else {
        app.path().app_log_dir().expect("no app log dir")
    }
}

/// Console layer (pretty, for the dev terminal) + daily-rolling plain-text file
/// layer in the exposed Logs folder. Level via RUST_LOG; defaults: debug (dev),
/// info (release).
pub fn init(app: &tauri::AppHandle) -> LogGuard {
    let dir = logs_dir(app);
    std::fs::create_dir_all(&dir).ok();
    let file_appender = tracing_appender::rolling::daily(&dir, "focus-planner.log");
    let (file_writer, guard) = tracing_appender::non_blocking(file_appender);

    let default_level = if cfg!(debug_assertions) { "debug" } else { "info" };
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(default_level));

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().with_writer(std::io::stdout))
        .with(fmt::layer().with_writer(file_writer).with_ansi(false))
        .init();

    tracing::info!(logs_dir = %dir.display(), "logging initialized");
    LogGuard(guard)
}
```

- [ ] **Step 2: Initialize in `lib.rs` setup and keep the guard alive in managed state**

```rust
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let guard = logging::init(app.handle());
            app.manage(guard);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 3: Gitignore log files, keep the folder**

```bash
mkdir -p logs && touch logs/.gitkeep
printf "logs/*\n!logs/.gitkeep\n" >> .gitignore
```

- [ ] **Step 4: Verify**

Run: `npm run tauri dev`, close the window, then `cat logs/focus-planner.log.$(date +%Y-%m-%d)`
Expected: a line containing `logging initialized` with the `logs_dir` field.

- [ ] **Step 5: Commit**

```bash
git add src-tauri logs/.gitkeep .gitignore
git commit -m "feat: tracing logging to console + daily-rolling file in exposed logs/ folder"
```

---

### Task 5: Migrations + SQLx offline workflow `[medium]`

**Files:**
- Create: `src-tauri/migrations/0001_initial_schema.sql`, `src-tauri/migrations/0002_seed_default_pomodoro_type.sql`
- Create: `src-tauri/.env`, `scripts/setup-db.sh`
- Modify: `.gitignore`

- [ ] **Step 1: Write `src-tauri/migrations/0001_initial_schema.sql`**

The 9 tables exactly as in spec §2 (`docs/specs/m1-focus-planner-design.md`) — copy verbatim from the spec: `projects`, `goals`, `tasks`, `microtasks`, `pomodoro_types`, `plans` (with `date TEXT NOT NULL UNIQUE`), `work_blocks`, `focus_sessions`, `pomodoro_sessions`, plus:

```sql
CREATE TABLE settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
```

- [ ] **Step 2: Write `src-tauri/migrations/0002_seed_default_pomodoro_type.sql`**

```sql
INSERT INTO pomodoro_types (id, name, work_minutes, rest_minutes, long_break_minutes, long_break_every, is_default, created_at, updated_at)
VALUES (
    'a0000000-0000-4000-8000-000000000001',
    'Standard',
    20, 5, NULL, NULL, 1,
    strftime('%Y-%m-%dT%H:%M:%SZ', 'now'),
    strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
);
```

- [ ] **Step 3: Set up the offline-checking workflow**

`src-tauri/.env` (committed — it contains no secrets, only the dev-DB path the `sqlx` macros read at compile time):

```
DATABASE_URL=sqlite://.dev/dev.sqlite
```

`scripts/setup-db.sh` (run once per clone; requires `cargo install sqlx-cli --no-default-features --features sqlite`):

```bash
#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/../src-tauri"
mkdir -p .dev
cargo sqlx database create
cargo sqlx migrate run
echo "dev DB ready at src-tauri/.dev/dev.sqlite"
```

```bash
chmod +x scripts/setup-db.sh
printf "src-tauri/.dev/\n" >> .gitignore
```

- [ ] **Step 4: Run it and prepare the offline cache**

```bash
./scripts/setup-db.sh
cd src-tauri && cargo sqlx prepare && cd ..
```

Expected: `.sqlx/` directory appears under `src-tauri/` (commit it — CI compiles with `SQLX_OFFLINE=true` and no database).

- [ ] **Step 5: Verify the seed**

Run: `sqlite3 src-tauri/.dev/dev.sqlite "SELECT name, work_minutes, rest_minutes, is_default FROM pomodoro_types;"`
Expected: `Standard|20|5|1`

- [ ] **Step 6: Commit**

```bash
git add src-tauri/migrations src-tauri/.env src-tauri/.sqlx scripts .gitignore
git commit -m "feat: full M1 schema migrations, Standard 20/5 seed, SQLx offline workflow"
```

---

### Task 6: DB module + first Rust test (TDD starts here) `[medium]`

**Files:**
- Create: `src-tauri/src/db.rs`
- Modify: `src-tauri/src/lib.rs`
- Test: `src-tauri/tests/migrations.rs`

- [ ] **Step 1: Write the failing test** *(test designed by the strongest agent)*

`src-tauri/tests/migrations.rs` — `#[sqlx::test]` gives each test a fresh in-memory SQLite pool with `src-tauri/migrations` applied:

```rust
use sqlx::SqlitePool;

#[sqlx::test]
async fn migrations_apply_and_seed_the_default_pomodoro_type(pool: SqlitePool) {
    let row = sqlx::query!(
        r#"SELECT name, work_minutes, rest_minutes, is_default FROM pomodoro_types"#
    )
    .fetch_one(&pool)
    .await
    .expect("seed row must exist");

    assert_eq!(row.name, "Standard");
    assert_eq!(row.work_minutes, 20);
    assert_eq!(row.rest_minutes, 5);
    assert_eq!(row.is_default, 1);
}

#[sqlx::test]
async fn one_plan_per_date_is_enforced(pool: SqlitePool) {
    let insert = |id: &'static str| {
        sqlx::query("INSERT INTO plans (id, date, status, created_at, updated_at) VALUES (?, '2026-06-09', 'draft', '2026-06-09T08:00:00Z', '2026-06-09T08:00:00Z')")
            .bind(id)
            .execute(&pool)
    };
    insert("p1").await.expect("first plan inserts");
    let err = insert("p2").await.expect_err("second plan for same date must violate UNIQUE");
    assert!(err.to_string().contains("UNIQUE"));
}
```

- [ ] **Step 2: Run to verify current state**

Run: `cargo test --manifest-path src-tauri/Cargo.toml`
Expected: PASS already (the migrations exist from Task 5) — these tests pin the schema contract; if they fail, the migrations are wrong, fix them.

- [ ] **Step 3: Write `src-tauri/src/db.rs` (production pool against the app-data DB)**

```rust
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use tauri::Manager;

/// Tauri managed state wrapper for the single app-wide pool.
pub struct Db(pub SqlitePool);

pub async fn init(app: &tauri::AppHandle) -> Result<SqlitePool, sqlx::Error> {
    let dir = app.path().app_data_dir().expect("no app data dir");
    std::fs::create_dir_all(&dir).ok();
    let path = dir.join("focus-planner.sqlite");
    tracing::info!(db_path = %path.display(), "opening database");

    let options = SqliteConnectOptions::new()
        .filename(&path)
        .create_if_missing(true)
        .foreign_keys(true);
    let pool = SqlitePoolOptions::new().connect_with(options).await?;

    sqlx::migrate!("./migrations").run(&pool).await?;
    tracing::info!("migrations applied");
    Ok(pool)
}
```

- [ ] **Step 4: Wire into `lib.rs` setup (after logging init)**

```rust
        .setup(|app| {
            let guard = logging::init(app.handle());
            app.manage(guard);
            let handle = app.handle().clone();
            let pool = tauri::async_runtime::block_on(db::init(&handle))
                .expect("database initialization failed");
            app.manage(db::Db(pool));
            Ok(())
        })
```

- [ ] **Step 5: Run everything**

Run: `cargo test --manifest-path src-tauri/Cargo.toml && npm run tauri dev`
Expected: tests pass; the dev window opens; the log file shows `opening database` then `migrations applied`.

- [ ] **Step 6: Commit**

```bash
git add src-tauri
git commit -m "feat: db pool init with migrations on startup, schema contract tests"
```

---

### Task 7: AppError — the IPC error convention `[easy]`

**Files:**
- Create: `src-tauri/src/error.rs`

Every command in every phase returns `Result<T, AppError>`. The webview receives `{ code, message }`.

- [ ] **Step 1: Write `src-tauri/src/error.rs`**

```rust
use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("database error: {0}")]
    Db(#[from] sqlx::Error),
    #[error("{entity} not found: {id}")]
    NotFound { entity: &'static str, id: String },
    #[error("validation failed: {0}")]
    Validation(String),
}

impl Serialize for AppError {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        #[derive(Serialize)]
        struct Wire<'a> {
            code: &'a str,
            message: String,
        }
        let code = match self {
            AppError::Db(_) => "db",
            AppError::NotFound { .. } => "not_found",
            AppError::Validation(_) => "validation",
        };
        Wire { code, message: self.to_string() }.serialize(serializer)
    }
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`
Expected: success.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/error.rs
git commit -m "feat: AppError enum — the uniform IPC error convention"
```

---

### Task 8: Vertical slice backend — `list_projects` + `log_frontend` `[medium]`

**Files:**
- Create: `src-tauri/src/models/project.rs`, `src-tauri/src/commands/project.rs`, `src-tauri/src/commands/frontend_log.rs`
- Modify: `src-tauri/src/models/mod.rs`, `src-tauri/src/commands/mod.rs`, `src-tauri/src/lib.rs`
- Test: `src-tauri/tests/project_commands.rs`

Phase 1's `list_projects` returns plain project rows (empty DB → `[]`); the goal/task/microtask roll-up stats are added in Phase 2.

- [ ] **Step 1: Write the failing test** *(test designed by the strongest agent)*

`src-tauri/tests/project_commands.rs`:

```rust
use focus_planner_lib::core::project_service;
use sqlx::SqlitePool;

#[sqlx::test]
async fn list_projects_on_empty_db_returns_empty_vec(pool: SqlitePool) {
    let projects = project_service::list_projects(&pool, false).await.unwrap();
    assert!(projects.is_empty());
}

#[sqlx::test]
async fn list_projects_excludes_archived_unless_asked(pool: SqlitePool) {
    sqlx::query("INSERT INTO projects (id, name, status, is_archived, created_at, updated_at) VALUES ('p1', 'Active', 'open', 0, '2026-06-09T08:00:00Z', '2026-06-09T08:00:00Z'), ('p2', 'Archived', 'open', 1, '2026-06-09T08:00:00Z', '2026-06-09T08:00:00Z')")
        .execute(&pool).await.unwrap();

    let visible = project_service::list_projects(&pool, false).await.unwrap();
    assert_eq!(visible.len(), 1);
    assert_eq!(visible[0].name, "Active");

    let all = project_service::list_projects(&pool, true).await.unwrap();
    assert_eq!(all.len(), 2);
}
```

Note: `focus_planner_lib` is the lib name from the scaffold's `Cargo.toml` (`[lib] name = ...`); check it and adjust the `use` line if the scaffold generated a different name.

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml`
Expected: FAIL — `core::project_service` doesn't exist.

- [ ] **Step 3: Implement model + service + commands**

`src-tauri/src/models/project.rs` (+ `pub mod project;` in `models/mod.rs`):

```rust
use serde::Serialize;

#[derive(Debug, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub status: String,
    pub is_archived: bool,
    pub completed_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}
```

`src-tauri/src/core/project_service.rs` (+ `pub mod project_service;` in `core/mod.rs`) — queries live in the Core, commands stay thin:

```rust
use crate::error::AppError;
use crate::models::project::Project;
use sqlx::SqlitePool;

pub async fn list_projects(pool: &SqlitePool, include_archived: bool) -> Result<Vec<Project>, AppError> {
    let projects = sqlx::query_as!(
        Project,
        r#"SELECT id, name, description, status,
                  is_archived as "is_archived: bool",
                  completed_at, created_at, updated_at
           FROM projects
           WHERE is_archived = 0 OR ?1 = 1
           ORDER BY created_at"#,
        include_archived
    )
    .fetch_all(pool)
    .await?;
    Ok(projects)
}
```

`src-tauri/src/commands/project.rs` (+ `pub mod project;` in `commands/mod.rs`) — the instrumentation pattern every later command copies:

```rust
use crate::core::project_service;
use crate::db::Db;
use crate::error::AppError;
use crate::models::project::Project;

#[tauri::command]
#[tracing::instrument(skip(db))]
pub async fn list_projects(
    db: tauri::State<'_, Db>,
    include_archived: bool,
) -> Result<Vec<Project>, AppError> {
    let result = project_service::list_projects(&db.0, include_archived).await;
    match &result {
        Ok(p) => tracing::info!(count = p.len(), "ok"),
        Err(e) => tracing::error!(error = %e, "failed"),
    }
    result
}
```

`src-tauri/src/commands/frontend_log.rs` (+ `pub mod frontend_log;`) — forwards webview errors into the same log file:

```rust
#[tauri::command]
pub fn log_frontend(level: String, message: String, context: Option<String>) {
    let ctx = context.as_deref().unwrap_or("");
    match level.as_str() {
        "error" => tracing::error!(target: "frontend", context = ctx, "{message}"),
        "warn" => tracing::warn!(target: "frontend", context = ctx, "{message}"),
        _ => tracing::info!(target: "frontend", context = ctx, "{message}"),
    }
}
```

Register both in `lib.rs`:

```rust
        .invoke_handler(tauri::generate_handler![
            commands::project::list_projects,
            commands::frontend_log::log_frontend,
        ])
```

- [ ] **Step 4: Refresh the offline cache and run the tests**

```bash
cd src-tauri && cargo sqlx prepare && cd ..
cargo test --manifest-path src-tauri/Cargo.toml
```

Expected: PASS (4 tests total).

- [ ] **Step 5: Commit**

```bash
git add src-tauri
git commit -m "feat: list_projects vertical slice backend + frontend log forwarding"
```

---

### Task 9: TS IPC wrapper + domain types `[easy]`

**Files:**
- Create: `src/ipc/client.ts`, `src/ipc/types.ts`

- [ ] **Step 1: Write `src/ipc/types.ts`**

```ts
export interface Project {
  id: string;
  name: string;
  description: string | null;
  status: "open" | "completed";
  isArchived: boolean;
  completedAt: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface IpcError {
  code: "db" | "not_found" | "validation";
  message: string;
}
```

- [ ] **Step 2: Write `src/ipc/client.ts` — the one wrapper all stores use**

```ts
import { invoke } from "@tauri-apps/api/core";

export async function ipc<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(command, args);
  } catch (e) {
    const err = e as { code?: string; message?: string };
    console.error(`[ipc] ${command} failed:`, err);
    // Forward to the Rust log file so the failure is visible in logs/ (spec §7).
    void invoke("log_frontend", {
      level: "error",
      message: `${command} failed: ${err.message ?? String(e)}`,
      context: args ? JSON.stringify(args) : null,
    }).catch(() => {});
    throw e;
  }
}
```

- [ ] **Step 3: Verify it typechecks**

Run: `npx vue-tsc --noEmit`
Expected: no errors.

- [ ] **Step 4: Commit**

```bash
git add src/ipc
git commit -m "feat: typed IPC client wrapper with error forwarding to the log file"
```

---

### Task 10: `useProjectStore` + Vitest harness `[medium]`

**Files:**
- Create: `src/stores/projectStore.ts`, `vitest.config.ts`
- Test: `src/stores/projectStore.test.ts`
- Modify: `package.json`

- [ ] **Step 1: Install test deps and add the script**

```bash
npm install -D vitest jsdom
```

In `package.json` scripts: `"test": "vitest"`.

`vitest.config.ts`:

```ts
import { defineConfig } from "vitest/config";

export default defineConfig({
  test: { environment: "jsdom" },
});
```

- [ ] **Step 2: Write the failing test** *(test designed by the strongest agent)*

`src/stores/projectStore.test.ts` — `mockIPC` from `@tauri-apps/api/mocks` fakes the Tauri bridge:

```ts
import { beforeEach, describe, expect, it } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { mockIPC } from "@tauri-apps/api/mocks";
import { useProjectStore } from "./projectStore";

describe("useProjectStore", () => {
  beforeEach(() => setActivePinia(createPinia()));

  it("loadProjects fills state from the list_projects command", async () => {
    mockIPC((cmd, args) => {
      if (cmd === "list_projects") {
        expect((args as Record<string, unknown>).includeArchived).toBe(false);
        return [
          {
            id: "p1", name: "My project", description: null, status: "open",
            isArchived: false, completedAt: null,
            createdAt: "2026-06-09T08:00:00Z", updatedAt: "2026-06-09T08:00:00Z",
          },
        ];
      }
    });

    const store = useProjectStore();
    await store.loadProjects();
    expect(store.projects).toHaveLength(1);
    expect(store.projects[0].name).toBe("My project");
    expect(store.error).toBeNull();
  });

  it("loadProjects records the error message on failure", async () => {
    mockIPC((cmd) => {
      if (cmd === "list_projects") throw { code: "db", message: "boom" };
    });

    const store = useProjectStore();
    await store.loadProjects();
    expect(store.projects).toHaveLength(0);
    expect(store.error).toBe("boom");
  });
});
```

- [ ] **Step 3: Run to verify it fails**

Run: `npm test -- --run`
Expected: FAIL — `./projectStore` doesn't exist.

- [ ] **Step 4: Write `src/stores/projectStore.ts`**

```ts
import { defineStore } from "pinia";
import { ipc } from "../ipc/client";
import type { IpcError, Project } from "../ipc/types";

export const useProjectStore = defineStore("project", {
  state: () => ({
    projects: [] as Project[],
    loading: false,
    error: null as string | null,
  }),
  actions: {
    async loadProjects(includeArchived = false) {
      this.loading = true;
      this.error = null;
      try {
        this.projects = await ipc<Project[]>("list_projects", { includeArchived });
      } catch (e) {
        this.error = (e as IpcError).message ?? String(e);
      } finally {
        this.loading = false;
      }
    },
  },
});
```

- [ ] **Step 5: Run to verify it passes**

Run: `npm test -- --run`
Expected: 2 passing.

- [ ] **Step 6: Commit**

```bash
git add src/stores vitest.config.ts package.json package-lock.json
git commit -m "feat: useProjectStore with vitest + mockIPC harness"
```

---

### Task 11: App shell — sidebar + 4 views + Backlog empty state `[medium]`

**Files:**
- Create: `src/views/DayView.vue`, `src/views/BacklogView.vue`, `src/views/SettingsView.vue`, `src/views/AnalyticsView.vue`
- Modify: `src/App.vue`, `src/assets/` styles (replace scaffold CSS)

No router (KISS — a 4-view desktop sidebar needs no deep links): `App.vue` holds a `currentView` ref and a component map.

- [ ] **Step 1: Write `src/App.vue`**

```vue
<script setup lang="ts">
import { ref } from "vue";
import DayView from "./views/DayView.vue";
import BacklogView from "./views/BacklogView.vue";
import SettingsView from "./views/SettingsView.vue";
import AnalyticsView from "./views/AnalyticsView.vue";

const views = {
  day: { label: "Day", component: DayView },
  backlog: { label: "Backlog", component: BacklogView },
  analytics: { label: "Analytics", component: AnalyticsView },
  settings: { label: "Settings", component: SettingsView },
} as const;
type ViewKey = keyof typeof views;
const currentView = ref<ViewKey>("day");
</script>

<template>
  <div class="shell">
    <nav class="sidebar">
      <button
        v-for="(view, key) in views"
        :key="key"
        :class="{ active: currentView === key }"
        @click="currentView = key"
      >
        {{ view.label }}
      </button>
    </nav>
    <main class="content">
      <component :is="views[currentView].component" />
    </main>
  </div>
</template>

<style>
:root {
  color-scheme: dark;
  font-family: "Inter", system-ui, sans-serif;
}
body { margin: 0; background: #111418; color: #e6e9ef; }
.shell { display: flex; height: 100vh; }
.sidebar {
  width: 200px; padding: 16px 8px; display: flex; flex-direction: column; gap: 4px;
  background: #0b0e11; border-right: 1px solid #20242b;
}
.sidebar button {
  background: none; border: none; color: #9aa3b2; text-align: left;
  padding: 10px 12px; border-radius: 8px; font-size: 14px; cursor: pointer;
}
.sidebar button:hover { background: #181d24; color: #e6e9ef; }
.sidebar button.active { background: #1f2630; color: #fff; }
.content { flex: 1; padding: 24px; overflow-y: auto; }
</style>
```

- [ ] **Step 2: Write the three placeholder views**

`src/views/DayView.vue` (Settings/Analytics identical with their own titles):

```vue
<template>
  <section>
    <h1>Day</h1>
    <p class="placeholder">Day planning arrives in Phase 3; Start Day in Phase 4.</p>
  </section>
</template>
```

- [ ] **Step 3: Write `src/views/BacklogView.vue` — wired to the store (the slice's visible end)**

```vue
<script setup lang="ts">
import { onMounted } from "vue";
import { useProjectStore } from "../stores/projectStore";

const store = useProjectStore();
onMounted(() => store.loadProjects());
</script>

<template>
  <section>
    <h1>Backlog</h1>
    <p v-if="store.loading">Loading…</p>
    <p v-else-if="store.error" class="error">{{ store.error }}</p>
    <p v-else-if="store.projects.length === 0" class="placeholder">
      No projects yet. Project creation arrives in Phase 2.
    </p>
    <ul v-else>
      <li v-for="p in store.projects" :key="p.id">{{ p.name }}</li>
    </ul>
  </section>
</template>
```

- [ ] **Step 4: Remove scaffold leftovers**

Delete the scaffold's greeting component and its CSS imports so `npx vue-tsc --noEmit` stays clean.

- [ ] **Step 5: Verify the slice end-to-end**

Run: `npm run tauri dev`
Expected: window opens on Day; clicking Backlog shows "No projects yet…"; the log file shows `list_projects` with `include_archived=false` and `count=0 ok`.

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "feat: app shell with sidebar, 4 views, Backlog empty state via vertical slice"
```

---

### Task 12: CI workflow `[easy]`

**Files:**
- Create: `.github/workflows/ci.yml`

- [ ] **Step 1: Write the workflow**

macOS runner — no Linux system-dependency dance, and it matches the dev platform. `SQLX_OFFLINE=true` makes the macros compile from the committed `.sqlx/` cache.

```yaml
name: CI
on:
  push:
    branches: [main]
  pull_request:

jobs:
  test:
    runs-on: macos-latest
    env:
      SQLX_OFFLINE: "true"
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          node-version: 20
          cache: npm
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
        with:
          workspaces: src-tauri
      - run: npm ci
      - run: npx vue-tsc --noEmit
      - run: npm test -- --run
      - run: cargo test --manifest-path src-tauri/Cargo.toml
```

- [ ] **Step 2: Commit and push; confirm the run is green on GitHub**

```bash
git add .github
git commit -m "ci: typecheck, vitest, and cargo test on macOS with SQLX_OFFLINE"
git push
```

---

### Task 13: Docs — README, db-context, lessons folder `[easy]`

**Files:**
- Modify: `README.md`
- Create: `docs/db-context/README.md`, `docs/db-context/schema-for-dummies.md`, `docs/db-context/migration-history.md`, `docs/lessons/README.md`

- [ ] **Step 1: Update `README.md`**

Rewrite so a newcomer grasps the project in five minutes: what Focus Planner is (one paragraph), current state (Phase 1 skeleton; legacy vanilla app in `legacy/`), how to run (`./scripts/setup-db.sh`, `npm install`, `npm run tauri dev`), how to test (`npm test`, `cargo test --manifest-path src-tauri/Cargo.toml`), where the logs are (`logs/`), and pointers to `PHILOSOPHY.md`, `docs/specs/`, `docs/superpowers/plans/`.

- [ ] **Step 2: Create the database context folder (PHILOSOPHY.md)**

`docs/db-context/schema-for-dummies.md` — plain-language walk of the 10 tables: "a Project contains Goals, a Goal contains Tasks, a Task contains Microtasks — the unit you actually schedule. A Plan is one day made of WorkBlocks…" with one sentence per table and the two history tables explained as "what you actually did, kept forever for stats."

`docs/db-context/migration-history.md` — a table: migration file · date · what it did · why. Two rows so far (0001 initial schema, 0002 seed). Every schema change from any phase appends here.

`docs/db-context/README.md` — explains the folder's purpose and links the two files.

- [ ] **Step 3: Create `docs/lessons/README.md`**

"Every mistake and failed experiment becomes an entry here — we pay for a lesson once. One markdown file per category (e.g. `tauri.md`, `sqlx.md`, `frontend.md`), created the first time a category gets a lesson. Entry format: date, what happened, root cause, the rule going forward."

- [ ] **Step 4: Commit**

```bash
git add README.md docs/db-context docs/lessons
git commit -m "docs: README refresh, db-context for-dummies + migration history, lessons folder"
```

---

### Task 14: Phase acceptance — manual QA checklist `[trivial]`

- [ ] `./scripts/setup-db.sh && npm install && npm run tauri dev` works on a fresh clone
- [ ] Native window "Focus Planner" opens; all 4 sidebar views navigate
- [ ] Backlog shows the empty state (proving Vue → IPC → Rust → SQLx → back)
- [ ] `logs/focus-planner.log.<today>` shows: logging initialized → db opened → migrations applied → `list_projects … count=0 ok` — readable as a narrative by someone with zero context
- [ ] `sqlite3` against the app-data DB shows the seeded Standard 20/5 type
- [ ] `cargo test` (4 tests) and `npm test -- --run` (2 tests) pass locally
- [ ] CI is green on the PR
- [ ] README five-minute test: a newcomer can run the app from README alone
