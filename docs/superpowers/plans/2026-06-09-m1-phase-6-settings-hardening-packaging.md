# M1 Phase 6 — Settings, Hardening & Packaging Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Finish M1: a settings system over the existing `settings` KV table (single Rust registry of known keys, defaults, and validation) wired into the Phase 3 planner and Phase 4 engine; the remaining Settings view sections (planning window, audio with test-sound, notifications); a hardening pass (shared empty/error/loading states, confirm dialogs, dark-mode polish, keyboard focus); a packaged, ad-hoc-signed `.app`/`.dmg` whose logs and DB land in the OS app dirs; and the full M1 manual QA walkthrough.

**Architecture:** No schema change — the `settings` table from migration 0001 is the store. `core/settings_service.rs` owns a `SETTINGS` registry (key, default, validator) as the single source of truth; `get_settings` overlays stored rows on defaults, `update_setting` validates and upserts. Consumers: the plan service reads the planning window via Phase 3's existing `settings_service::get_planning_window` (this phase only adds the registry validation around it); the runtime engine receives a typed `SettingsSnapshot` inside its `StartDay` message (settings are read once at start-of-day — POLA: mid-run changes apply on the next run; the test-sound command always reads the current value). New IPC: `get_settings` (query), `update_setting` (command), `play_test_sound` (command — an effect, returns no data). Frontend: `useSettingsStore` with optimistic update + rollback; shared `ErrorBanner`/`EmptyState`/`ConfirmDialog` components adopted by all 4 views.

**Tech Stack:** everything already in the tree — Tauri 2, Vue 3, Vite, TypeScript, Pinia, Vitest (`mockIPC`), Rust, SQLx 0.8 (sqlite, `#[sqlx::test]`), tokio, thiserror, tracing, rodio (Phase 4), tauri-plugin-notification (Phase 4). New assets only: bundled Inter font files, a placeholder app icon. No new dependencies.

**Conventions (per `docs/specs/m1-roadmap.md`):** every task carries a difficulty tag (`[trivial]`/`[easy]`/`[medium]`/`[hard]`). The failing test of each TDD task is designed by the most capable agent; implementation may be assigned by difficulty; every task is reviewed before its commit lands. UI-polish and packaging tasks are not unit-testable — they end with manual verification steps instead.

**Philosophy (PHILOSOPHY.md):** CQS — `update_setting` and `play_test_sound` return `Result<(), AppError>`; `get_settings` returns data and never mutates. Logging per spec §7 — every settings read/change at INFO (`setting planning_window_start changed 09:00 -> 08:30`), validation rejections at WARN, `play_test_sound` at INFO; the QA checklist requires a settings session to be reconstructable from the log file alone. KISS — no settings framework, no theme system, a `&[SettingDef]` slice and `fn(&str)` validators.

**Integration reality check:** Tasks 5 and 6 modify Phase 3 (`plan_service`/`generate_day_plan`) and Phase 4 (runtime engine, audio module) code that lands before this plan executes. The code below is written against the spec's stated shapes (pure planner fn taking a window; actor engine fed by `RuntimeCmd` over `mpsc`; rodio synth module). If landed names differ, adapt the call sites, **amend this plan, and record a `docs/lessons/` entry** per roadmap convention 1 — do not fork the conventions.

**Before starting:** branch off `main`:

```bash
git checkout main && git pull && git checkout -b feat/m1-phase-6-settings-hardening-packaging
```

---

### Task 1: `AppError::Internal` variant `[trivial]`

Phase 1's `AppError` has `Db`/`NotFound`/`Validation`. The audio path can fail in ways that are neither (no output device, panicked blocking task). Skip this task if Phase 4 already added an equivalent variant — reuse it everywhere this plan says `Internal`.

**Files:**
- Modify: `src-tauri/src/error.rs`

- [ ] **Step 1: Add the variant**

In the `AppError` enum:

```rust
    #[error("internal error: {0}")]
    Internal(String),
```

In the `Serialize` impl's `match`:

```rust
            AppError::Internal(_) => "internal",
```

- [ ] **Step 2: Mirror it in the TS type**

In `src/ipc/types.ts`, extend the union:

```ts
export interface IpcError {
  code: "db" | "not_found" | "validation" | "internal";
  message: string;
}
```

(If Phase 2–5 already widened this union, just append `"internal"`.)

- [ ] **Step 3: Verify**

Run: `cargo check --manifest-path src-tauri/Cargo.toml && npx vue-tsc --noEmit`
Expected: both clean.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/error.rs src/ipc/types.ts
git commit -m "feat: AppError::Internal variant for non-domain failures (audio, task panics)"
```

---

### Task 2: `settings_service` — registry, get, update, snapshot (TDD) `[medium]`

The single Rust source of truth for known keys, defaults, and validation. Seven keys:

| key | default | validation |
|---|---|---|
| `planning_window_start` | `09:00` | `HH:MM` 24h; window must not invert |
| `planning_window_end` | `17:00` | `HH:MM` 24h; window must not invert |
| `sound_volume` | `0.8` | float in 0.0–1.0 |
| `sound_enabled` | `true` | `true`/`false` |
| `notifications_enabled` | `true` | `true`/`false` |
| `notify_work_warning_minutes` | `5` | integer 1–120 |
| `notify_break_warning_minutes` | `1` | integer 1–120 |

Setting keys are **data** (map keys), not struct fields — they cross the IPC boundary as snake_case strings unchanged; serde camelCase renaming applies only to struct fields (POLA: the frontend reads `settings.planning_window_start`).

**Files:**
- Create: `src-tauri/src/core/settings_service.rs`
- Modify: `src-tauri/src/core/mod.rs`
- Test: `src-tauri/tests/settings_service.rs`

- [ ] **Step 1: Write the failing tests** *(test designed by the strongest agent)*

Append to `src-tauri/tests/settings_service.rs` — Phase 3 created this file with the three planning-window tests; keep them, and the imports below are already at the top of the file:

```rust
use focus_planner_lib::core::settings_service;
use focus_planner_lib::error::AppError;
use sqlx::SqlitePool;

#[sqlx::test]
async fn get_settings_returns_all_defaults_on_empty_table(pool: SqlitePool) {
    let map = settings_service::get_settings(&pool).await.unwrap();
    assert_eq!(map.len(), 7, "exactly the 7 known keys");
    assert_eq!(map["planning_window_start"], "09:00");
    assert_eq!(map["planning_window_end"], "17:00");
    assert_eq!(map["sound_volume"], "0.8");
    assert_eq!(map["sound_enabled"], "true");
    assert_eq!(map["notifications_enabled"], "true");
    assert_eq!(map["notify_work_warning_minutes"], "5");
    assert_eq!(map["notify_break_warning_minutes"], "1");
}

#[sqlx::test]
async fn get_settings_overlays_stored_values_on_defaults(pool: SqlitePool) {
    settings_service::update_setting(&pool, "sound_volume", "0.5").await.unwrap();
    let map = settings_service::get_settings(&pool).await.unwrap();
    assert_eq!(map["sound_volume"], "0.5", "stored value wins");
    assert_eq!(map["sound_enabled"], "true", "untouched keys keep defaults");
    assert_eq!(map.len(), 7);
}

#[sqlx::test]
async fn update_setting_rejects_unknown_keys(pool: SqlitePool) {
    let err = settings_service::update_setting(&pool, "theme", "dark").await.unwrap_err();
    assert!(matches!(err, AppError::Validation(_)), "unknown key must be a Validation error, got {err:?}");
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM settings").fetch_one(&pool).await.unwrap();
    assert_eq!(count, 0, "nothing written");
}

#[sqlx::test]
async fn update_setting_validates_values(pool: SqlitePool) {
    for (key, bad) in [
        ("sound_volume", "1.5"),
        ("sound_volume", "loud"),
        ("planning_window_start", "9am"),
        ("planning_window_start", "25:00"),
        ("sound_enabled", "yes"),
        ("notify_work_warning_minutes", "0"),
        ("notify_work_warning_minutes", "-1"),
        ("notify_break_warning_minutes", "121"),
    ] {
        let err = settings_service::update_setting(&pool, key, bad).await.unwrap_err();
        assert!(matches!(err, AppError::Validation(_)), "{key}={bad} must be rejected, got {err:?}");
    }
}

#[sqlx::test]
async fn update_setting_rejects_an_inverted_planning_window(pool: SqlitePool) {
    // end before the default 09:00 start
    let err = settings_service::update_setting(&pool, "planning_window_end", "08:00").await.unwrap_err();
    assert!(matches!(err, AppError::Validation(_)));
    // start after the default 17:00 end
    let err = settings_service::update_setting(&pool, "planning_window_start", "18:00").await.unwrap_err();
    assert!(matches!(err, AppError::Validation(_)));
    // a valid narrowing is fine
    settings_service::update_setting(&pool, "planning_window_start", "10:00").await.unwrap();
    settings_service::update_setting(&pool, "planning_window_end", "15:00").await.unwrap();
}

#[sqlx::test]
async fn update_setting_upserts(pool: SqlitePool) {
    settings_service::update_setting(&pool, "sound_volume", "0.3").await.unwrap();
    settings_service::update_setting(&pool, "sound_volume", "0.9").await.unwrap();
    let map = settings_service::get_settings(&pool).await.unwrap();
    assert_eq!(map["sound_volume"], "0.9");
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM settings WHERE key = 'sound_volume'")
        .fetch_one(&pool).await.unwrap();
    assert_eq!(count, 1, "one row per key, not one per write");
}

#[sqlx::test]
async fn snapshot_parses_typed_values(pool: SqlitePool) {
    settings_service::update_setting(&pool, "sound_volume", "0.25").await.unwrap();
    settings_service::update_setting(&pool, "sound_enabled", "false").await.unwrap();
    settings_service::update_setting(&pool, "notify_work_warning_minutes", "10").await.unwrap();
    let snap = settings_service::snapshot(&pool).await.unwrap();
    assert_eq!(snap.sound_volume, 0.25);
    assert!(!snap.sound_enabled);
    assert!(snap.notifications_enabled, "default still true");
    assert_eq!(snap.notify_work_warning_minutes, 10);
    assert_eq!(snap.notify_break_warning_minutes, 1);
    assert_eq!(snap.work_warning_seconds(), 600);
    assert_eq!(snap.break_warning_seconds(), 60);
}

#[sqlx::test]
async fn get_planning_window_respects_update_setting_writes(pool: SqlitePool) {
    // Phase 3's reader and this phase's writer must agree on keys and format.
    let w = settings_service::get_planning_window(&pool).await.unwrap();
    assert_eq!(w.start.format("%H:%M").to_string(), "09:00");
    settings_service::update_setting(&pool, "planning_window_start", "10:00").await.unwrap();
    let w = settings_service::get_planning_window(&pool).await.unwrap();
    assert_eq!(w.start.format("%H:%M").to_string(), "10:00");
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test settings_service`
Expected: FAIL to compile — `update_setting`, `get_settings`, and `snapshot` don't exist yet. (The module itself exists since Phase 3; its three planning-window tests must stay green throughout.)

- [ ] **Step 3: Extend the existing `src-tauri/src/core/settings_service.rs`** — Phase 3 created it with `get_planning_window`; keep that function and its `chrono`/`PlanningWindow` imports, and append:

```rust
use crate::error::AppError;
use sqlx::SqlitePool;
use std::collections::HashMap;

/// The single source of truth for known settings: key, default, validator.
/// Adding a setting in any later milestone means adding one row here —
/// `get_settings`, `update_setting`, and the Settings view follow automatically.
pub struct SettingDef {
    pub key: &'static str,
    pub default: &'static str,
    pub validate: fn(&str) -> Result<(), String>,
}

fn valid_time(v: &str) -> Result<(), String> {
    let ok = v.len() == 5
        && v.as_bytes()[2] == b':'
        && v[..2].parse::<u8>().is_ok_and(|h| h < 24)
        && v[3..].parse::<u8>().is_ok_and(|m| m < 60);
    if ok { Ok(()) } else { Err(format!("expected HH:MM (24h), got '{v}'")) }
}

fn valid_volume(v: &str) -> Result<(), String> {
    match v.parse::<f32>() {
        Ok(f) if (0.0..=1.0).contains(&f) => Ok(()),
        _ => Err(format!("expected a number between 0.0 and 1.0, got '{v}'")),
    }
}

fn valid_bool(v: &str) -> Result<(), String> {
    if v == "true" || v == "false" { Ok(()) } else { Err(format!("expected 'true' or 'false', got '{v}'")) }
}

fn valid_minutes(v: &str) -> Result<(), String> {
    match v.parse::<u32>() {
        Ok(m) if (1..=120).contains(&m) => Ok(()),
        _ => Err(format!("expected whole minutes 1-120, got '{v}'")),
    }
}

pub const SETTINGS: &[SettingDef] = &[
    SettingDef { key: "planning_window_start", default: "09:00", validate: valid_time },
    SettingDef { key: "planning_window_end", default: "17:00", validate: valid_time },
    SettingDef { key: "sound_volume", default: "0.8", validate: valid_volume },
    SettingDef { key: "sound_enabled", default: "true", validate: valid_bool },
    SettingDef { key: "notifications_enabled", default: "true", validate: valid_bool },
    SettingDef { key: "notify_work_warning_minutes", default: "5", validate: valid_minutes },
    SettingDef { key: "notify_break_warning_minutes", default: "1", validate: valid_minutes },
];

fn def_for(key: &str) -> Option<&'static SettingDef> {
    SETTINGS.iter().find(|d| d.key == key)
}

/// Query: full key-value map, defaults applied for keys with no stored row.
/// Unknown rows in the table (hand-edits, future versions) are ignored, not surfaced.
pub async fn get_settings(pool: &SqlitePool) -> Result<HashMap<String, String>, AppError> {
    let rows = sqlx::query!(r#"SELECT key as "key!", value as "value!" FROM settings"#)
        .fetch_all(pool)
        .await?;
    let mut map: HashMap<String, String> = SETTINGS
        .iter()
        .map(|d| (d.key.to_string(), d.default.to_string()))
        .collect();
    for row in rows {
        if map.contains_key(row.key.as_str()) {
            map.insert(row.key, row.value);
        }
    }
    Ok(map)
}

/// Query: one key's effective value (stored, else default). Unknown key = Validation.
pub async fn get_setting(pool: &SqlitePool, key: &str) -> Result<String, AppError> {
    let def = def_for(key)
        .ok_or_else(|| AppError::Validation(format!("unknown setting key: {key}")))?;
    let row = sqlx::query!(r#"SELECT value as "value!" FROM settings WHERE key = ?1"#, key)
        .fetch_optional(pool)
        .await?;
    Ok(row.map(|r| r.value).unwrap_or_else(|| def.default.to_string()))
}

/// Command (CQS): validates against the registry, upserts, returns no data.
pub async fn update_setting(pool: &SqlitePool, key: &str, value: &str) -> Result<(), AppError> {
    let def = def_for(key).ok_or_else(|| {
        tracing::warn!(key, "update_setting rejected: unknown setting key");
        AppError::Validation(format!("unknown setting key: {key}"))
    })?;
    if let Err(msg) = (def.validate)(value) {
        tracing::warn!(key, value, "update_setting rejected: {msg}");
        return Err(AppError::Validation(format!("invalid value for {key}: {msg}")));
    }
    // Cross-key rule: the planning window must not invert. HH:MM strings
    // compare lexicographically == chronologically, so plain `>=` is correct.
    if key == "planning_window_start" || key == "planning_window_end" {
        let other_key = if key == "planning_window_start" { "planning_window_end" } else { "planning_window_start" };
        let other = get_setting(pool, other_key).await?;
        let (start, end) = if key == "planning_window_start" { (value, other.as_str()) } else { (other.as_str(), value) };
        if start >= end {
            tracing::warn!(start, end, "update_setting rejected: planning window would invert");
            return Err(AppError::Validation(format!(
                "planning window must start before it ends ({start} >= {end})"
            )));
        }
    }
    let old = get_setting(pool, key).await?;
    sqlx::query!(
        r#"INSERT INTO settings (key, value, updated_at)
           VALUES (?1, ?2, strftime('%Y-%m-%dT%H:%M:%SZ','now'))
           ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at"#,
        key,
        value
    )
    .execute(pool)
    .await?;
    tracing::info!("setting {key} changed {old} -> {value}");
    Ok(())
}

/// Typed view of the audio/notification settings, read once by `start_day`
/// and handed to the runtime engine (spec §4). Values were validated on
/// write; a hand-edited DB falls back to the registry defaults.
#[derive(Debug, Clone, PartialEq)]
pub struct SettingsSnapshot {
    pub sound_volume: f32,
    pub sound_enabled: bool,
    pub notifications_enabled: bool,
    pub notify_work_warning_minutes: u32,
    pub notify_break_warning_minutes: u32,
}

impl SettingsSnapshot {
    pub fn work_warning_seconds(&self) -> u32 {
        self.notify_work_warning_minutes * 60
    }
    pub fn break_warning_seconds(&self) -> u32 {
        self.notify_break_warning_minutes * 60
    }
}

pub async fn snapshot(pool: &SqlitePool) -> Result<SettingsSnapshot, AppError> {
    let map = get_settings(pool).await?;
    let get = |key: &str| map.get(key).cloned().unwrap_or_default();
    Ok(SettingsSnapshot {
        sound_volume: get("sound_volume").parse().unwrap_or(0.8),
        sound_enabled: get("sound_enabled") == "true",
        notifications_enabled: get("notifications_enabled") == "true",
        notify_work_warning_minutes: get("notify_work_warning_minutes").parse().unwrap_or(5),
        notify_break_warning_minutes: get("notify_break_warning_minutes").parse().unwrap_or(1),
    })
}

```

(No new window helper: Phase 3's `get_planning_window` already returns the typed `PlanningWindow` the planner consumes — one reader, one writer, one registry.)

- [ ] **Step 4: Refresh the offline cache, run to verify it passes**

```bash
cd src-tauri && cargo sqlx prepare && cd ..
cargo test --manifest-path src-tauri/Cargo.toml --test settings_service
```

Expected: PASS — 8 new tests, 11 total in the file with Phase 3's three. Commit the updated `.sqlx/`.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/core src-tauri/tests/settings_service.rs src-tauri/.sqlx
git commit -m "feat: settings_service with SETTINGS registry, validated upserts, typed snapshot"
```

---

### Task 3: IPC commands `get_settings` + `update_setting` `[easy]`

CQS at the boundary: the query returns the map; the command returns `Result<(), AppError>`.

**Files:**
- Create: `src-tauri/src/commands/settings.rs`
- Modify: `src-tauri/src/commands/mod.rs`, `src-tauri/src/lib.rs`

- [ ] **Step 1: Write `src-tauri/src/commands/settings.rs`** (+ `pub mod settings;` in `commands/mod.rs`)

The Phase 1 instrumentation pattern, verbatim:

```rust
use crate::core::settings_service;
use crate::db::Db;
use crate::error::AppError;
use std::collections::HashMap;

#[tauri::command]
#[tracing::instrument(skip(db))]
pub async fn get_settings(db: tauri::State<'_, Db>) -> Result<HashMap<String, String>, AppError> {
    let result = settings_service::get_settings(&db.0).await;
    match &result {
        Ok(map) => tracing::info!(count = map.len(), "ok"),
        Err(e) => tracing::error!(error = %e, "failed"),
    }
    result
}

#[tauri::command]
#[tracing::instrument(skip(db))]
pub async fn update_setting(
    db: tauri::State<'_, Db>,
    key: String,
    value: String,
) -> Result<(), AppError> {
    let result = settings_service::update_setting(&db.0, &key, &value).await;
    match &result {
        Ok(()) => tracing::info!("ok"),
        Err(e) => tracing::error!(error = %e, "failed"),
    }
    result
}
```

(`update_setting` itself already logs the `changed old -> new` line at the service layer — the command adds only the uniform entry/exit envelope.)

- [ ] **Step 2: Register both in the `lib.rs` `invoke_handler` list**

```rust
            commands::settings::get_settings,
            commands::settings::update_setting,
```

- [ ] **Step 3: Verify**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`
Expected: clean.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/commands src-tauri/src/lib.rs
git commit -m "feat: get_settings query and update_setting command over the settings registry"
```

---

### Task 4: `play_test_sound` command + volume-aware audio `[medium]`

Phase 4 created the rodio synth module (per spec §1, SineWave sources). Two changes: the completion-sound function must accept a `volume: f32` (refactor it if Phase 4 hardcoded the level — update the engine call sites in the same commit), and a new `play_test_sound` command lets the Settings view audition the current volume. CQS: it is an effect — `Result<(), AppError>`, no data. POLA: the test button always plays, even when `sound_enabled` is `false` — the user explicitly asked to hear it.

**Files:**
- Modify: Phase 4's audio module (assumed `src-tauri/src/core/audio.rs` — adapt the path to what Phase 4 landed)
- Create: `src-tauri/src/commands/sound.rs`
- Modify: `src-tauri/src/commands/mod.rs`, `src-tauri/src/lib.rs`

- [ ] **Step 1: Ensure the audio module exposes a volume-taking completion sound**

Target signature and body (the two-note sequence from spec §4; match the rodio version Phase 4 pinned — this is the 0.21+ API, rodio 0.22):

```rust
use crate::error::AppError;
use rodio::source::{SineWave, Source};
use rodio::{OutputStreamBuilder, Sink};
use std::time::Duration;

/// Blocking — callers in async context wrap in spawn_blocking.
pub fn play_completion_sound(volume: f32) -> Result<(), AppError> {
    let stream = OutputStreamBuilder::open_default_stream()
        .map_err(|e| AppError::Internal(format!("no audio output device: {e}")))?;
    let sink = Sink::connect_new(stream.mixer());
    sink.set_volume(volume);
    sink.append(SineWave::new(880.0).take_duration(Duration::from_millis(150)).amplify(0.6));
    sink.append(SineWave::new(1174.66).take_duration(Duration::from_millis(250)).amplify(0.6));
    sink.sleep_until_end();
    Ok(())
}
```

If Phase 4's function already exists without the `volume` parameter, add it and pass the engine's snapshot volume at every call site (Task 6 supplies the snapshot).

- [ ] **Step 2: Write `src-tauri/src/commands/sound.rs`** (+ `pub mod sound;` in `commands/mod.rs`)

```rust
use crate::core::{audio, settings_service};
use crate::db::Db;
use crate::error::AppError;

#[tauri::command]
#[tracing::instrument(skip(db))]
pub async fn play_test_sound(db: tauri::State<'_, Db>) -> Result<(), AppError> {
    let snap = settings_service::snapshot(&db.0).await?;
    if !snap.sound_enabled {
        tracing::info!("sound_enabled=false but test sound explicitly requested — playing anyway");
    }
    let volume = snap.sound_volume;
    tauri::async_runtime::spawn_blocking(move || audio::play_completion_sound(volume))
        .await
        .map_err(|e| AppError::Internal(format!("audio task panicked: {e}")))??;
    tracing::info!(volume, "test sound played");
    Ok(())
}
```

- [ ] **Step 3: Register it in `lib.rs`**

```rust
            commands::sound::play_test_sound,
```

- [ ] **Step 4: Verify by ear and by log**

Run: `npm run tauri dev`, open the dev console, and execute:
`window.__TAURI__.core.invoke("play_test_sound")`
(If the global is not exposed, defer the by-ear check to Task 9's Settings view button.)
Expected: the two-note sound plays; `logs/focus-planner.log.$(date +%Y-%m-%d)` shows `play_test_sound` entry and `test sound played volume=0.8`.

- [ ] **Step 5: Commit**

```bash
git add src-tauri
git commit -m "feat: play_test_sound command; completion sound takes volume from settings"
```

---

### Task 5: Planner reads the window via `settings_service` `[medium]`

Spec §4: the planner is a pure `fn plan(inputs, window, now)` — the **caller** (Phase 3's plan service / `generate_day_plan` path) resolves the window via `settings_service::get_planning_window`, which Phase 3 already built. This task adds the integration pin: a test proving a window changed through `update_setting` actually moves generated blocks, so the registry, the reader, and the planner can never drift apart silently.

**Files:**
- Modify: `src-tauri/src/core/plan_service.rs` (adapt to Phase 3's landed path)
- Test: extend Phase 3's plan service test file (e.g. `src-tauri/tests/plan_service.rs`)

- [ ] **Step 1: Write the failing test** *(test designed by the strongest agent)*

Add to the plan service tests — adapt the generate call and block assertions to Phase 3's landed signatures/fixtures (it already has helpers that seed a microtask and generate a plan):

```rust
#[sqlx::test]
async fn generate_day_plan_uses_the_planning_window_from_settings(pool: SqlitePool) {
    settings_service::update_setting(&pool, "planning_window_start", "10:00").await.unwrap();
    settings_service::update_setting(&pool, "planning_window_end", "15:00").await.unwrap();
    seed_microtask(&pool, "m1", 2 /* pomodoros */).await; // Phase 3 fixture helper

    plan_service::generate_day_plan(&pool, "plan-1", "2026-06-15", None).await.unwrap();

    let blocks = fetch_blocks(&pool, "plan-1").await; // Phase 3 fixture helper
    let first_start = &blocks.first().unwrap().start_time;
    let last_end = &blocks.last().unwrap().end_time;
    assert!(first_start.contains("10:00"), "first block must start at the window start, got {first_start}");
    assert!(last_end <= &"2026-06-15T15:00:00Z".to_string(), "no block may end after the window, got {last_end}");
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test plan_service`
Expected: FAIL if Phase 3 hardcoded/ad-hoc-read the window. (If Phase 3 already reads both keys correctly, the test PASSES — keep it as the pin, and Step 3 becomes a pure refactor to route through `settings_service`; still do it.)

- [ ] **Step 3: Confirm the plan service routes through `get_planning_window`**

Phase 3's plan service already calls `settings_service::get_planning_window(pool)` before the pure planner — this phase changes nothing at that call site; the registry validation in `update_setting` now guarantees the stored values it reads are well-formed. Add the window log line next to the existing call if Phase 3 didn't already log it:

```rust
let window = settings_service::get_planning_window(pool).await?;
tracing::info!(start = %window.start.format("%H:%M"), end = %window.end.format("%H:%M"), "planning window from settings");
```

- [ ] **Step 4: Run to verify it passes**

```bash
cd src-tauri && cargo sqlx prepare && cd ..
cargo test --manifest-path src-tauri/Cargo.toml
```

Expected: full suite PASS, including all pre-existing planner tests (they relied on the 09:00–17:00 defaults, which the registry preserves).

- [ ] **Step 5: Commit**

```bash
git add src-tauri
git commit -m "refactor: planner resolves its window through settings_service (single source of truth)"
```

---

### Task 6: Engine receives a `SettingsSnapshot` at start_day `[hard]`

Spec §4: the engine reads volume/enabled flags **at start_day**. The snapshot rides inside the `StartDay` message so the actor task owns it with the rest of `RuntimeState` — no shared reads mid-run. The warning thresholds (previously the hardcoded 5-minute/1-minute rules) come from the snapshot too.

**Files:**
- Modify: Phase 4's engine module (assumed `src-tauri/src/core/runtime_service.rs`) and the `start_day` command (assumed `src-tauri/src/commands/runtime.rs`) — adapt paths/names to what Phase 4 landed, then amend this plan + record a lesson if they differ
- Test: extend Phase 4's engine test file (paused-tokio-time harness)

- [ ] **Step 1: Extend the `StartDay` message**

```rust
use crate::core::settings_service::SettingsSnapshot;

pub enum RuntimeCmd {
    StartDay { plan_id: String, settings: SettingsSnapshot }, // settings is the new field
    // ...existing variants unchanged
}
```

Store the snapshot in the engine task's state alongside `RuntimeState` when handling `StartDay`.

- [ ] **Step 2: Snapshot in the `start_day` command and log it**

```rust
#[tauri::command]
#[tracing::instrument(skip(db, engine))]
pub async fn start_day(
    db: tauri::State<'_, Db>,
    engine: tauri::State<'_, EngineHandle>, // Phase 4's mpsc sender wrapper
    plan_id: String,
) -> Result<(), AppError> {
    let settings = settings_service::snapshot(&db.0).await?;
    tracing::info!(
        volume = settings.sound_volume,
        sound = settings.sound_enabled,
        notifications = settings.notifications_enabled,
        work_warning_min = settings.notify_work_warning_minutes,
        break_warning_min = settings.notify_break_warning_minutes,
        "settings snapshot for runtime"
    );
    // ...Phase 4's existing plan loading/validation stays as-is...
    engine.send(RuntimeCmd::StartDay { plan_id, settings }).await
}
```

- [ ] **Step 3: Use the snapshot inside the engine loop**

Replace Phase 4's hardcoded rules at their existing sites:

```rust
// was: remaining == 5 * 60 in Work mode
if state.mode == RuntimeMode::Work
    && state.timer_seconds_remaining == settings.work_warning_seconds()
    && settings.notifications_enabled
{
    notify(&app, &format!(
        "{} minutes remaining. Wrap up your current focus item.",
        settings.notify_work_warning_minutes
    ));
}

// was: remaining == 60 in Break mode
if state.mode == RuntimeMode::Break
    && state.timer_seconds_remaining == settings.break_warning_seconds()
    && settings.notifications_enabled
{
    notify(&app, &format!(
        "{} minute(s) remaining. Get ready to focus.",
        settings.notify_break_warning_minutes
    ));
}

// every block-end sound site:
if settings.sound_enabled {
    if let Err(e) = audio::play_completion_sound(settings.sound_volume) {
        tracing::error!(error = %e, "completion sound failed");
    }
} else {
    tracing::info!("sound suppressed (sound_enabled=false)");
}

// every block-end notification site gains: if settings.notifications_enabled { ... }
// else tracing::info!("notification suppressed (notifications_enabled=false)");
```

(Suppressions are logged at INFO so a silent run is still a readable narrative per spec §7.)

- [ ] **Step 4: Extend the engine tests** *(test designed by the strongest agent)*

In Phase 4's paused-time harness (`tokio::time::pause()`/`advance()`, fake effects sink — adapt to its actual names), add:

```rust
#[tokio::test(start_paused = true)]
async fn work_warning_fires_at_the_configured_minutes() {
    // 10-minute warning instead of the default 5
    let settings = SettingsSnapshot {
        sound_volume: 0.8,
        sound_enabled: true,
        notifications_enabled: true,
        notify_work_warning_minutes: 10,
        notify_break_warning_minutes: 1,
    };
    let mut harness = EngineHarness::with_work_block_minutes(20, settings); // Phase 4 helper
    harness.start().await;
    harness.advance_minutes(10).await; // 10 remaining of 20
    assert_eq!(harness.notifications(), vec!["10 minutes remaining. Wrap up your current focus item."]);
}

#[tokio::test(start_paused = true)]
async fn disabled_sound_and_notifications_suppress_effects_but_not_transitions() {
    let settings = SettingsSnapshot {
        sound_volume: 0.8,
        sound_enabled: false,
        notifications_enabled: false,
        notify_work_warning_minutes: 5,
        notify_break_warning_minutes: 1,
    };
    let mut harness = EngineHarness::with_work_block_minutes(20, settings);
    harness.start().await;
    harness.advance_minutes(20).await; // work block completes
    assert!(harness.sounds_played().is_empty());
    assert!(harness.notifications().is_empty());
    assert_eq!(harness.mode(), RuntimeMode::Break, "auto-advance still happens");
}
```

- [ ] **Step 5: Run everything**

Run: `cargo test --manifest-path src-tauri/Cargo.toml`
Expected: full suite PASS (Phase 4's existing tests now construct a default snapshot — add a `SettingsSnapshot` literal with the registry defaults to its harness constructor, or a `Default` impl matching the registry defaults, whichever Phase 4's harness makes cleaner).

- [ ] **Step 6: Manual verification of the live wiring**

`npm run tauri dev` → Settings (after Task 9) or `sqlite3` the dev DB: set `notify_work_warning_minutes` to `1`, plan and commit a tiny day, Start Day with a short pomodoro type. Expected: the "1 minutes remaining" notification arrives 1 minute before block end; the log shows `settings snapshot for runtime` then the narrative.

- [ ] **Step 7: Commit**

```bash
git add src-tauri
git commit -m "feat: runtime engine consumes SettingsSnapshot (volume, toggles, warning thresholds) at start_day"
```

---

### Task 7: `useSettingsStore` — optimistic update with rollback (TDD) `[medium]`

**Files:**
- Create: `src/stores/settingsStore.ts`
- Test: `src/stores/settingsStore.test.ts`
- Modify: `src/ipc/types.ts`

- [ ] **Step 1: Add the type**

In `src/ipc/types.ts`:

```ts
/** Setting keys are data (a KV map), not struct fields — they stay snake_case. */
export type SettingsMap = Record<string, string>;
```

- [ ] **Step 2: Write the failing tests** *(test designed by the strongest agent)*

`src/stores/settingsStore.test.ts`:

```ts
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { useSettingsStore } from "./settingsStore";

const FULL_MAP = {
  planning_window_start: "09:00",
  planning_window_end: "17:00",
  sound_volume: "0.8",
  sound_enabled: "true",
  notifications_enabled: "true",
  notify_work_warning_minutes: "5",
  notify_break_warning_minutes: "1",
};

describe("useSettingsStore", () => {
  beforeEach(() => setActivePinia(createPinia()));
  afterEach(() => clearMocks());

  it("loadSettings fills the map from get_settings", async () => {
    mockIPC((cmd) => {
      if (cmd === "get_settings") return { ...FULL_MAP };
    });
    const store = useSettingsStore();
    await store.loadSettings();
    expect(store.settings.planning_window_start).toBe("09:00");
    expect(Object.keys(store.settings)).toHaveLength(7);
    expect(store.error).toBeNull();
  });

  it("updateSetting applies optimistically and keeps the value on success", async () => {
    const calls: Array<Record<string, unknown>> = [];
    mockIPC((cmd, args) => {
      if (cmd === "get_settings") return { ...FULL_MAP };
      if (cmd === "update_setting") {
        calls.push(args as Record<string, unknown>);
        return undefined; // CQS: commands return no data
      }
    });
    const store = useSettingsStore();
    await store.loadSettings();
    const promise = store.updateSetting("sound_volume", "0.5");
    expect(store.settings.sound_volume).toBe("0.5"); // optimistic, before the await
    await promise;
    expect(store.settings.sound_volume).toBe("0.5");
    expect(calls).toEqual([{ key: "sound_volume", value: "0.5" }]);
    expect(store.error).toBeNull();
  });

  it("updateSetting rolls back to the previous value on error", async () => {
    mockIPC((cmd) => {
      if (cmd === "get_settings") return { ...FULL_MAP };
      if (cmd === "update_setting")
        throw { code: "validation", message: "invalid value for sound_volume: expected a number between 0.0 and 1.0, got '1.5'" };
    });
    const store = useSettingsStore();
    await store.loadSettings();
    await store.updateSetting("sound_volume", "1.5");
    expect(store.settings.sound_volume).toBe("0.8"); // rolled back
    expect(store.error).toContain("sound_volume");
  });

  it("loadSettings records the error message on failure", async () => {
    mockIPC((cmd) => {
      if (cmd === "get_settings") throw { code: "db", message: "boom" };
    });
    const store = useSettingsStore();
    await store.loadSettings();
    expect(store.error).toBe("boom");
  });
});
```

- [ ] **Step 3: Run to verify it fails**

Run: `npm test -- --run`
Expected: FAIL — `./settingsStore` doesn't exist.

- [ ] **Step 4: Write `src/stores/settingsStore.ts`**

```ts
import { defineStore } from "pinia";
import { ipc } from "../ipc/client";
import type { IpcError, SettingsMap } from "../ipc/types";

export const useSettingsStore = defineStore("settings", {
  state: () => ({
    settings: {} as SettingsMap,
    loading: false,
    error: null as string | null,
  }),
  actions: {
    async loadSettings() {
      this.loading = true;
      this.error = null;
      try {
        this.settings = await ipc<SettingsMap>("get_settings");
      } catch (e) {
        this.error = (e as IpcError).message ?? String(e);
      } finally {
        this.loading = false;
      }
    },
    /** Optimistic: the UI reflects the new value immediately; a backend
     *  rejection (validation) rolls it back and surfaces the message. */
    async updateSetting(key: string, value: string) {
      const previous = this.settings[key];
      this.settings[key] = value;
      this.error = null;
      try {
        await ipc<void>("update_setting", { key, value });
      } catch (e) {
        this.settings[key] = previous;
        this.error = (e as IpcError).message ?? String(e);
      }
    },
  },
});
```

- [ ] **Step 5: Run to verify it passes**

Run: `npm test -- --run`
Expected: the 4 new tests pass; all pre-existing store tests still pass.

- [ ] **Step 6: Commit**

```bash
git add src/stores/settingsStore.ts src/stores/settingsStore.test.ts src/ipc/types.ts
git commit -m "feat: useSettingsStore with optimistic update and rollback on validation error"
```

---

### Task 8: Settings section components (Planning window, Audio, Notifications) `[medium]`

Three presentational components over the store. `@change` (not `@input`) on slider/number/time inputs — one IPC write per gesture, not per drag tick (KISS).

**Files:**
- Create: `src/components/settings/PlanningWindowSection.vue`, `src/components/settings/AudioSection.vue`, `src/components/settings/NotificationsSection.vue`

- [ ] **Step 1: Write `src/components/settings/PlanningWindowSection.vue`**

```vue
<script setup lang="ts">
import { useSettingsStore } from "../../stores/settingsStore";

const store = useSettingsStore();

function onChange(key: string, e: Event) {
  store.updateSetting(key, (e.target as HTMLInputElement).value);
}
</script>

<template>
  <section class="settings-section">
    <h2>Planning window</h2>
    <p class="hint">The planner schedules work blocks inside this window.</p>
    <div class="field-row">
      <label for="window-start">Start</label>
      <input
        id="window-start"
        type="time"
        :value="store.settings.planning_window_start"
        @change="onChange('planning_window_start', $event)"
      />
      <label for="window-end">End</label>
      <input
        id="window-end"
        type="time"
        :value="store.settings.planning_window_end"
        @change="onChange('planning_window_end', $event)"
      />
    </div>
  </section>
</template>
```

- [ ] **Step 2: Write `src/components/settings/AudioSection.vue`**

```vue
<script setup lang="ts">
import { ref } from "vue";
import { ipc } from "../../ipc/client";
import { useSettingsStore } from "../../stores/settingsStore";

const store = useSettingsStore();
const testing = ref(false);

function onVolumeChange(e: Event) {
  store.updateSetting("sound_volume", (e.target as HTMLInputElement).value);
}
function onToggle(e: Event) {
  store.updateSetting("sound_enabled", String((e.target as HTMLInputElement).checked));
}
async function playTestSound() {
  testing.value = true;
  try {
    await ipc<void>("play_test_sound");
  } catch {
    // ipc() already forwarded the failure to the log file; surface it inline:
    store.error = "Test sound failed — is an audio output device available?";
  } finally {
    testing.value = false;
  }
}
</script>

<template>
  <section class="settings-section">
    <h2>Audio</h2>
    <div class="field-row">
      <label for="sound-enabled">Completion sounds</label>
      <input
        id="sound-enabled"
        type="checkbox"
        :checked="store.settings.sound_enabled === 'true'"
        @change="onToggle"
      />
    </div>
    <div class="field-row">
      <label for="sound-volume">Volume</label>
      <input
        id="sound-volume"
        type="range"
        min="0"
        max="1"
        step="0.05"
        :value="store.settings.sound_volume"
        @change="onVolumeChange"
      />
      <span class="value">{{ Math.round(Number(store.settings.sound_volume ?? "0.8") * 100) }}%</span>
      <button :disabled="testing" @click="playTestSound">
        {{ testing ? "Playing…" : "Test sound" }}
      </button>
    </div>
    <p class="hint">Volume and toggles take effect on the next Start Day; the test button always uses the current volume.</p>
  </section>
</template>
```

- [ ] **Step 3: Write `src/components/settings/NotificationsSection.vue`**

```vue
<script setup lang="ts">
import { useSettingsStore } from "../../stores/settingsStore";

const store = useSettingsStore();

function onToggle(e: Event) {
  store.updateSetting("notifications_enabled", String((e.target as HTMLInputElement).checked));
}
function onMinutes(key: string, e: Event) {
  store.updateSetting(key, (e.target as HTMLInputElement).value);
}
</script>

<template>
  <section class="settings-section">
    <h2>Notifications</h2>
    <div class="field-row">
      <label for="notifications-enabled">System notifications</label>
      <input
        id="notifications-enabled"
        type="checkbox"
        :checked="store.settings.notifications_enabled === 'true'"
        @change="onToggle"
      />
    </div>
    <div class="field-row">
      <label for="work-warning">Work warning — minutes before a work block ends</label>
      <input
        id="work-warning"
        type="number"
        min="1"
        max="120"
        :value="store.settings.notify_work_warning_minutes"
        @change="onMinutes('notify_work_warning_minutes', $event)"
      />
    </div>
    <div class="field-row">
      <label for="break-warning">Break warning — minutes before a break ends</label>
      <input
        id="break-warning"
        type="number"
        min="1"
        max="120"
        :value="store.settings.notify_break_warning_minutes"
        @change="onMinutes('notify_break_warning_minutes', $event)"
      />
    </div>
  </section>
</template>
```

- [ ] **Step 4: Verify it typechecks**

Run: `npx vue-tsc --noEmit`
Expected: clean.

- [ ] **Step 5: Commit**

```bash
git add src/components/settings
git commit -m "feat: planning-window, audio (volume + test sound), and notifications settings sections"
```

---

### Task 9: Assemble the Settings view `[easy]`

The view becomes five grouped sections in this order: Planning window, Audio, Notifications, Pomodoro types (Phase 2's existing section), Data (Phase 5's existing export/import section). Keep the existing two exactly where they are — only insert the three new components above them and load the store on mount. Use the actual component/section names Phase 2/5 landed; if those sections live inline in `SettingsView.vue`, leave them inline.

**Files:**
- Modify: `src/views/SettingsView.vue`

- [ ] **Step 1: Wire the sections and the store**

Shape of the result (merge with the existing file, don't overwrite Phase 2/5 content):

```vue
<script setup lang="ts">
import { onMounted } from "vue";
import { useSettingsStore } from "../stores/settingsStore";
import PlanningWindowSection from "../components/settings/PlanningWindowSection.vue";
import AudioSection from "../components/settings/AudioSection.vue";
import NotificationsSection from "../components/settings/NotificationsSection.vue";
import ErrorBanner from "../components/ErrorBanner.vue";
// ...existing Phase 2 / Phase 5 imports stay...

const settingsStore = useSettingsStore();
onMounted(() => settingsStore.loadSettings());
</script>

<template>
  <section>
    <h1>Settings</h1>
    <ErrorBanner :message="settingsStore.error" @dismiss="settingsStore.error = null" />
    <p v-if="settingsStore.loading">Loading…</p>
    <template v-else>
      <PlanningWindowSection />
      <AudioSection />
      <NotificationsSection />
      <!-- existing Phase 2 Pomodoro types section -->
      <!-- existing Phase 5 Data (export/import) section -->
    </template>
  </section>
</template>
```

(`ErrorBanner` arrives in Task 10 — if executing strictly in order, do Task 10 Step 1 first or temporarily inline `<p class="error">`; the commit at the end of Task 10 must build either way.)

Add the shared section styles once (global stylesheet or `SettingsView.vue` non-scoped style):

```css
.settings-section { margin: 0 0 28px; padding: 16px 20px; background: #161b22; border: 1px solid #20242b; border-radius: 12px; }
.settings-section h2 { margin: 0 0 12px; font-size: 15px; font-weight: 600; }
.settings-section .hint { color: #9aa3b2; font-size: 13px; margin: 4px 0 12px; }
.field-row { display: flex; align-items: center; gap: 12px; margin: 10px 0; }
.field-row label { min-width: 120px; color: #c4ccd9; font-size: 14px; }
```

- [ ] **Step 2: Manual verification**

Run: `npm run tauri dev` → Settings.
Expected, in order:
1. All five sections render; planning window shows 09:00/17:00, volume 80%, all toggles on, warnings 5 and 1.
2. Drag volume to 30%, click **Test sound** → quieter two-note sound.
3. Set window end to 08:00 → the field snaps back (rollback) and the error banner shows "planning window must start before it ends".
4. `logs/focus-planner.log.$(date +%Y-%m-%d)` shows the full session: `get_settings … count=7 ok`, `setting sound_volume changed 0.8 -> 0.3`, `test sound played volume=0.3`, the WARN rejection line for the inverted window. A reader with zero context can reconstruct what you just did.

- [ ] **Step 3: Commit**

```bash
git add src/views/SettingsView.vue src/assets 2>/dev/null || git add src/views/SettingsView.vue
git commit -m "feat: Settings view assembled — planning window, audio, notifications + existing sections"
```

---

### Task 10: Shared `ErrorBanner.vue` + `EmptyState.vue` `[easy]`

Phases 2–5 each grew ad-hoc `<p class="error">`/empty markup. One component each, adopted everywhere in Task 11 (DRY now that the duplication has proven the shape — AHA satisfied).

**Files:**
- Create: `src/components/ErrorBanner.vue`, `src/components/EmptyState.vue`

- [ ] **Step 1: Write `src/components/ErrorBanner.vue`**

```vue
<script setup lang="ts">
defineProps<{ message: string | null }>();
defineEmits<{ dismiss: [] }>();
</script>

<template>
  <div v-if="message" class="error-banner" role="alert">
    <span>{{ message }}</span>
    <button class="dismiss" aria-label="Dismiss error" @click="$emit('dismiss')">×</button>
  </div>
</template>

<style scoped>
.error-banner {
  display: flex; align-items: center; justify-content: space-between; gap: 12px;
  background: #2a1518; border: 1px solid #5c2b31; color: #f2b8be;
  padding: 10px 14px; border-radius: 10px; margin: 0 0 16px; font-size: 14px;
}
.dismiss {
  background: none; border: none; color: #f2b8be; font-size: 18px;
  cursor: pointer; line-height: 1; padding: 2px 6px; border-radius: 6px;
}
.dismiss:hover { background: #3a1d21; }
</style>
```

- [ ] **Step 2: Write `src/components/EmptyState.vue`**

```vue
<script setup lang="ts">
defineProps<{ title: string; hint?: string }>();
</script>

<template>
  <div class="empty-state">
    <p class="title">{{ title }}</p>
    <p v-if="hint" class="hint">{{ hint }}</p>
    <slot />
  </div>
</template>

<style scoped>
.empty-state {
  text-align: center; padding: 48px 24px; color: #9aa3b2;
  border: 1px dashed #2a313c; border-radius: 12px;
}
.empty-state .title { font-size: 15px; color: #c4ccd9; margin: 0 0 6px; }
.empty-state .hint { font-size: 13px; margin: 0 0 16px; }
</style>
```

- [ ] **Step 3: Verify + commit**

Run: `npx vue-tsc --noEmit` — clean.

```bash
git add src/components/ErrorBanner.vue src/components/EmptyState.vue
git commit -m "feat: shared ErrorBanner and EmptyState components"
```

---

### Task 11: Consistent empty/error/loading states across all 4 views `[medium]`

Sweep Day, Backlog, Analytics, Settings: every store-backed surface shows `Loading…` while `loading`, `ErrorBanner` bound to the store's `error` (dismiss clears it), and `EmptyState` when loaded-but-empty. Replace the ad-hoc markup; do not change store logic.

**Files:**
- Modify: `src/views/DayView.vue`, `src/views/BacklogView.vue`, `src/views/AnalyticsView.vue`, `src/views/SettingsView.vue`

- [ ] **Step 1: BacklogView** — `EmptyState` titled "No projects yet" with hint "Create your first project to start building the backlog" and the create button in the slot; `ErrorBanner` above the tree bound to `useProjectStore().error`.

- [ ] **Step 2: DayView** — no plan for the selected date → `EmptyState` titled "No plan for this day" with hint "Generate a plan from your backlog" and the Generate button in the slot; `ErrorBanner` bound to `usePlanStore().error` (and the runtime store's error if Phase 4 added one).

- [ ] **Step 3: AnalyticsView** — no sessions in range → `EmptyState` titled "No focus sessions yet" with hint "Run a day to see your stats here"; `ErrorBanner` bound to the stats store's error.

- [ ] **Step 4: SettingsView** — already done in Task 9; verify it matches the same pattern.

- [ ] **Step 5: Manual verification**

Run with a fresh dev DB (`rm src-tauri/.dev/dev.sqlite && ./scripts/setup-db.sh`… note: the *app* DB is in app-data; to see empty states either use a fresh macOS app-data dir or temporarily delete `~/Library/Application Support/com.sramzz.focusplanner.dev/` if dev uses it — check `logs/` for the `opening database` path and clear that file).
Expected: all four views show their EmptyState on a virgin DB; kill the network/rename a command nothing — instead force an error by entering an invalid settings value: the banner appears and dismisses. `npm test -- --run` and `npx vue-tsc --noEmit` stay green.

- [ ] **Step 6: Commit**

```bash
git add src/views
git commit -m "feat: uniform loading/error/empty states across Day, Backlog, Analytics, Settings"
```

---

### Task 12: Shared `ConfirmDialog.vue` `[easy]`

**Files:**
- Create: `src/components/ConfirmDialog.vue`

- [ ] **Step 1: Write the component**

```vue
<script setup lang="ts">
defineProps<{
  open: boolean;
  title: string;
  message: string;
  confirmLabel?: string;
}>();
defineEmits<{ confirm: []; cancel: [] }>();
</script>

<template>
  <Teleport to="body">
    <div v-if="open" class="overlay" @click.self="$emit('cancel')" @keydown.esc="$emit('cancel')">
      <div class="dialog" role="alertdialog" :aria-label="title">
        <h3>{{ title }}</h3>
        <p>{{ message }}</p>
        <div class="actions">
          <button class="secondary" @click="$emit('cancel')">Cancel</button>
          <button class="danger" @click="$emit('confirm')">{{ confirmLabel ?? "Delete" }}</button>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.overlay {
  position: fixed; inset: 0; background: rgba(5, 8, 12, 0.6);
  backdrop-filter: blur(2px); display: grid; place-items: center; z-index: 100;
}
.dialog {
  width: min(420px, 90vw); background: #161b22; border: 1px solid #2a313c;
  border-radius: 14px; padding: 20px 24px; box-shadow: 0 16px 48px rgba(0, 0, 0, 0.5);
}
.dialog h3 { margin: 0 0 8px; font-size: 16px; }
.dialog p { margin: 0 0 20px; color: #9aa3b2; font-size: 14px; }
.actions { display: flex; justify-content: flex-end; gap: 10px; }
.actions .secondary { background: #1f2630; color: #c4ccd9; border: none; padding: 8px 16px; border-radius: 8px; cursor: pointer; }
.actions .danger { background: #8b2e38; color: #fff; border: none; padding: 8px 16px; border-radius: 8px; cursor: pointer; }
.actions .danger:hover { background: #a13743; }
</style>
```

- [ ] **Step 2: Verify + commit**

Run: `npx vue-tsc --noEmit` — clean.

```bash
git add src/components/ConfirmDialog.vue
git commit -m "feat: shared ConfirmDialog for destructive actions"
```

---

### Task 13: Confirm dialogs on every destructive action `[medium]`

Six wiring sites. Pattern at each: a local `confirming = ref<null | { … }>(…)` holding the pending action, the destructive button sets it, `@confirm` runs the existing store call then clears it, `@cancel` clears it. Archive actions are reversible — **no** dialog (POLA: don't cry wolf).

**Files:**
- Modify: `src/views/BacklogView.vue` (and/or its tree-node components from Phase 2), `src/views/DayView.vue`, the Phase 5 Data section component

- [ ] **Step 1: Backlog deletes** — delete project / goal / task / microtask each open the dialog. Message states the cascade, e.g. project: title `Delete project "{name}"?`, message `This permanently deletes the project and all of its goals, tasks, and microtasks. This cannot be undone.` (goal/task variants name their own cascade; microtask: `This permanently deletes the microtask.`)

- [ ] **Step 2: Day view — Clear plan** — title `Clear this day's plan?`, message `All work blocks for {date} will be removed. The backlog is not affected.`, confirmLabel `Clear plan`.

- [ ] **Step 3: Data section — Import** — title `Import data?`, message `Importing replaces ALL current data with the file's contents. Export a backup first if in doubt.`, confirmLabel `Import` — shown **before** opening the file picker.

- [ ] **Step 4: Manual verification**

Run: `npm run tauri dev`. For each of the six actions: trigger → dialog appears → Cancel leaves data untouched → trigger again → Confirm performs it. Esc and clicking the backdrop cancel. The log shows the underlying command only after Confirm.

- [ ] **Step 5: Commit**

```bash
git add src
git commit -m "feat: confirm dialogs on delete project/goal/task/microtask, clear plan, import"
```

---

### Task 14: Bundle Inter locally — no CDN `[easy]`

Spec §1 styles with Inter; Phase 1's CSS falls back to system-ui because the font was never shipped. A desktop app must not fetch fonts from the network (offline POLA). Bundle the variable font.

**Files:**
- Create: `src/assets/fonts/InterVariable.woff2`, `src/assets/fonts.css`
- Modify: `src/main.ts`

- [ ] **Step 1: Download the font into the repo**

```bash
mkdir -p src/assets/fonts
curl -L -o /tmp/inter.zip https://github.com/rsms/inter/releases/download/v4.1/Inter-4.1.zip
unzip -j -o /tmp/inter.zip "web/InterVariable.woff2" -d src/assets/fonts/
ls -la src/assets/fonts/InterVariable.woff2
```

Expected: the file exists, ~340 KB. Also copy Inter's license:

```bash
unzip -j -o /tmp/inter.zip "LICENSE.txt" -d src/assets/fonts/
```

- [ ] **Step 2: Write `src/assets/fonts.css` and import it**

```css
@font-face {
  font-family: "Inter";
  src: url("./fonts/InterVariable.woff2") format("woff2");
  font-weight: 100 900;
  font-style: normal;
  font-display: swap;
}
```

In `src/main.ts`, first import line: `import "./assets/fonts.css";`

- [ ] **Step 3: Verify**

Search for any CDN font usage and confirm none exists:

```bash
grep -rn "fonts.googleapis\|fonts.gstatic\|rsms.me" src index.html
```

Expected: no matches. Run `npm run tauri dev` with Wi-Fi off: the UI renders in Inter (compare a paragraph against system-ui — Inter's "g" is double-story).

- [ ] **Step 4: Commit**

```bash
git add src/assets/fonts src/assets/fonts.css src/main.ts
git commit -m "feat: bundle InterVariable locally (no CDN) with license"
```

---

### Task 15: Dark-mode polish — scrollbars, transitions, focus states `[easy]`

Spec §6: premium dark styling — custom scrollbars, smooth transitions; plus keyboard focus visibility (hardening scope).

**Files:**
- Modify: the global stylesheet (Phase 1 put global styles in `App.vue`'s non-scoped `<style>`; keep them there or extract to `src/assets/base.css` imported from `main.ts` — extraction preferred now that the block is growing)

- [ ] **Step 1: Add the polish block**

```css
/* Custom scrollbars (WebKit — the only engine Tauri ships on macOS) */
::-webkit-scrollbar { width: 10px; height: 10px; }
::-webkit-scrollbar-track { background: transparent; }
::-webkit-scrollbar-thumb {
  background: #2a313c; border-radius: 5px;
  border: 2px solid #111418; background-clip: padding-box;
}
::-webkit-scrollbar-thumb:hover { background: #3a4350; border: 2px solid #111418; background-clip: padding-box; }
::-webkit-scrollbar-corner { background: transparent; }

/* Smooth transitions on interactive elements */
button, input, select, a {
  transition: background-color 120ms ease, color 120ms ease,
    border-color 120ms ease, box-shadow 120ms ease, opacity 120ms ease;
}

/* Keyboard focus: visible ring for keyboard users only */
:focus-visible { outline: 2px solid #4f8cff; outline-offset: 2px; border-radius: 4px; }
:focus:not(:focus-visible) { outline: none; }

/* Range slider on dark */
input[type="range"] { accent-color: #4f8cff; }
input[type="checkbox"] { accent-color: #4f8cff; }
```

Confirm `:root { font-family: "Inter", system-ui, sans-serif; }` is still in effect (Phase 1 set it).

- [ ] **Step 2: Manual verification**

Run: `npm run tauri dev`.
Expected: Backlog with many items shows the slim dark scrollbar thumb; hover/active button color changes ease instead of snapping; pressing Tab repeatedly walks the sidebar and view controls with a visible blue ring; clicking a button shows **no** ring.

- [ ] **Step 3: Commit**

```bash
git add src
git commit -m "feat: dark-mode polish — custom scrollbars, eased transitions, focus-visible rings"
```

---

### Task 16: App icon — placeholder + `tauri icon` `[easy]`

No design exists yet. Generate a simple placeholder (clock mark on the app's dark palette) from an SVG, render it to a 1024 px PNG, and let `tauri icon` produce all platform sizes. **FLAGGED: placeholder — replace `design/icon.svg` with real branding before any public release** (tracked in the README task).

**Files:**
- Create: `design/icon.svg`, `design/icon-1024.png`
- Modify (generated): `src-tauri/icons/*`

- [ ] **Step 1: Write `design/icon.svg`**

```svg
<svg xmlns="http://www.w3.org/2000/svg" width="1024" height="1024">
  <rect width="1024" height="1024" rx="224" fill="#1f2630"/>
  <circle cx="512" cy="512" r="320" fill="none" stroke="#4f8cff" stroke-width="56"/>
  <line x1="512" y1="512" x2="512" y2="296" stroke="#e6e9ef" stroke-width="56" stroke-linecap="round"/>
  <line x1="512" y1="512" x2="668" y2="512" stroke="#e6e9ef" stroke-width="56" stroke-linecap="round"/>
</svg>
```

- [ ] **Step 2: Render to a 1024 px PNG (macOS QuickLook — no installs)**

```bash
qlmanage -t -s 1024 -o design design/icon.svg
mv design/icon.svg.png design/icon-1024.png
sips -g pixelWidth -g pixelHeight design/icon-1024.png
```

Expected: `pixelWidth: 1024`, `pixelHeight: 1024`. Fallback if qlmanage misrenders: `brew install imagemagick && magick design/icon.svg -resize 1024x1024 design/icon-1024.png`.

- [ ] **Step 3: Generate the platform icon set**

```bash
npm run tauri icon design/icon-1024.png
```

Expected output: `Appx Creating StoreLogo.png` … lines ending with icons written to `src-tauri/icons/` — including `icon.icns`, `icon.ico`, `32x32.png`, `128x128.png`, `128x128@2x.png`.

- [ ] **Step 4: Manual verification**

Run: `npm run tauri dev`.
Expected: the dock shows the clock placeholder instead of the Tauri default.

- [ ] **Step 5: Commit**

```bash
git add design src-tauri/icons
git commit -m "feat: placeholder app icon (FLAG: replace with real branding) + generated platform set"
```

---

### Task 17: Bundle config + release build `[medium]`

Identifier `com.sramzz.focusplanner` and productName `Focus Planner` were set in Phase 1. Pin the bundle targets and produce the artifacts. Ad-hoc signing is acceptable for M1 — Tauri signs with identity `-` when no `APPLE_SIGNING_IDENTITY` is configured.

**Files:**
- Modify: `src-tauri/tauri.conf.json`

- [ ] **Step 1: Pin the bundle section**

In `src-tauri/tauri.conf.json`, ensure:

```json
  "bundle": {
    "active": true,
    "targets": ["app", "dmg"],
    "icon": [
      "icons/32x32.png",
      "icons/128x128.png",
      "icons/128x128@2x.png",
      "icons/icon.icns",
      "icons/icon.ico"
    ]
  }
```

and `"version": "0.1.0"` (the scaffold default — M1 ships as 0.1.0).

- [ ] **Step 2: Build**

```bash
npm run tauri build
```

Expected (several minutes on first run): frontend `vite build` succeeds, `cargo build --release` succeeds, then:

```
    Bundling [src-tauri/target/release/bundle/macos/Focus Planner.app]
    Bundling [src-tauri/target/release/bundle/dmg/Focus Planner_0.1.0_aarch64.dmg]
    Finished 2 bundles at:
        src-tauri/target/release/bundle/macos/Focus Planner.app
        src-tauri/target/release/bundle/dmg/Focus Planner_0.1.0_aarch64.dmg
```

(`_x64.dmg` on an Intel Mac.)

- [ ] **Step 3: Verify the ad-hoc signature**

```bash
codesign -dv "src-tauri/target/release/bundle/macos/Focus Planner.app" 2>&1 | grep -E "Signature|Identifier"
```

Expected: `Identifier=com.sramzz.focusplanner` and `Signature=adhoc`.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/tauri.conf.json
git commit -m "build: pin app+dmg bundle targets; ad-hoc-signed release build verified"
```

---

### Task 18: Packaged-build smoke test — logs and DB in OS app dirs `[medium]`

Phase 1's `logging.rs` splits on `debug_assertions`: dev → repo `logs/`, packaged → `app_log_dir()`. On macOS with our identifier that is `~/Library/Logs/com.sramzz.focusplanner/`; `app_data_dir()` is `~/Library/Application Support/com.sramzz.focusplanner/`. This task proves the release paths actually work — the classic failure is a packaged app silently logging nowhere.

- [ ] **Step 1: Clean slate, then launch the packaged app**

```bash
rm -rf ~/Library/Logs/com.sramzz.focusplanner "~/Library/Application Support/com.sramzz.focusplanner" 2>/dev/null
rm -rf ~/Library/"Application Support"/com.sramzz.focusplanner
open "src-tauri/target/release/bundle/macos/Focus Planner.app"
```

Expected: the app opens with the placeholder icon. (Locally-built apps carry no quarantine attribute, so Gatekeeper does not fire here — it fires for *downloaded* dmgs; that caveat goes in the README, Task 19.)

- [ ] **Step 2: Verify the DB landed in app-data**

```bash
ls ~/Library/"Application Support"/com.sramzz.focusplanner/
sqlite3 ~/Library/"Application Support"/com.sramzz.focusplanner/focus-planner.sqlite \
  "SELECT name, work_minutes, rest_minutes FROM pomodoro_types;"
```

Expected: `focus-planner.sqlite` exists; `Standard|20|5`.

- [ ] **Step 3: Verify the log file landed in app-log dir and reads as a narrative**

```bash
cat ~/Library/Logs/com.sramzz.focusplanner/focus-planner.log.$(date +%Y-%m-%d)
```

Expected: `logging initialized` (with the Library/Logs path in the `logs_dir` field) → `opening database` (Application Support path) → `migrations applied`. No DEBUG lines (release default is `info`). Also confirm the repo's `logs/` got **no** new lines from this run.

- [ ] **Step 4: Exercise one settings change in the packaged app**

In the running packaged app: Settings → set volume to 50% → Test sound.
Expected: sound plays; the Library/Logs file gains `setting sound_volume changed 0.8 -> 0.5` and `test sound played volume=0.5`.

- [ ] **Step 5: Mount the dmg and spot-check**

```bash
open "src-tauri/target/release/bundle/dmg/Focus Planner_0.1.0_aarch64.dmg"
```

Expected: the dmg mounts showing `Focus Planner.app` with an Applications-folder drag target. Eject after checking.

- [ ] **Step 6: Record the result**

No commit (nothing changed) — tick this task's boxes and note any deviation as a `docs/lessons/` candidate (e.g. if the log dir was missing, that is a Phase 1 logging.rs bug to fix and record).

---

### Task 19: README — final M1 refresh + Gatekeeper caveat `[easy]`

PHILOSOPHY: any reader grasps the project in five minutes. The README currently describes an earlier phase; bring it to the finished-M1 state.

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Rewrite to the M1-complete state**

Sections, in order:
1. **What it is** — one paragraph: a local-first desktop focus planner — backlog (project → goal → task → microtask), deterministic day planning into pomodoro work/break blocks, a Rust-driven Start Day timer with sounds and notifications, stats, JSON export/import.
2. **Install (macOS)** — open the `.dmg`, drag to Applications, **and the Gatekeeper caveat verbatim:** "M1 builds are ad-hoc signed, not notarized. On first launch macOS will warn that the app is from an unidentified developer — right-click (Ctrl-click) **Focus Planner.app → Open → Open** to run it. This is expected until notarization lands post-M1."
3. **Develop** — prerequisites (Rust stable, Node 20+, `cargo install sqlx-cli --no-default-features --features sqlite`), then `./scripts/setup-db.sh && npm install && npm run tauri dev`. Link `docs/runbooks/dev-environment.md` for failures.
4. **Test** — `npm test -- --run` and `cargo test --manifest-path src-tauri/Cargo.toml`.
5. **Logs** — dev: `logs/` at the repo root; packaged: `~/Library/Logs/com.sramzz.focusplanner/`. "Every command, planner decision, and timer transition is in there — start debugging from the log file."
6. **Settings** — the 7-key table from Task 2 (key, default, what it does).
7. **Build a release** — `npm run tauri build` → `src-tauri/target/release/bundle/{macos,dmg}/`. **Note the flagged placeholder icon** (`design/icon.svg`) awaiting real branding.
8. **Docs map** — `PHILOSOPHY.md`, `docs/specs/`, `docs/superpowers/plans/`, `docs/db-context/`, `docs/runbooks/`, `docs/lessons/`, `legacy/` (the original vanilla prototype).

- [ ] **Step 2: The five-minute test**

Hand the README to someone (or an agent) with zero context: they must be able to say what the app does, run it in dev, and find the logs — using only the README. Fix whatever they stumble on.

- [ ] **Step 3: Commit**

```bash
git add README.md
git commit -m "docs: final M1 README — install with Gatekeeper caveat, dev/test/logs/settings/build"
```

---

### Task 20: Runbook — `docs/runbooks/dev-environment.md` `[easy]`

PHILOSOPHY: runbooks for recurring operational fixes. These three failures already bit during M1 phases — pay for each lesson once.

**Files:**
- Create: `docs/runbooks/dev-environment.md` (and the `docs/runbooks/` folder)

- [ ] **Step 1: Write the runbook**

```markdown
# Runbook — Dev Environment

## Fresh-clone setup
1. Prerequisites: Rust stable (`rustup`), Node 20+, Xcode Command Line Tools.
2. `cargo install sqlx-cli --no-default-features --features sqlite`
3. `./scripts/setup-db.sh`        # creates src-tauri/.dev/dev.sqlite and runs migrations
4. `npm install`
5. `npm run tauri dev`            # native window opens; logs stream to logs/

Verify: `cargo test --manifest-path src-tauri/Cargo.toml` and `npm test -- --run` both green.

## Common failures

### `error: no such command: sqlx` (missing sqlx-cli)
`scripts/setup-db.sh` and `cargo sqlx prepare` need the CLI.
**Fix:** `cargo install sqlx-cli --no-default-features --features sqlite`

### `sqlx::query!` compile errors after changing SQL or migrations (stale `.sqlx` cache)
Symptoms: `failed to find data for query …`, or macros compiling against old columns
even though the migration is correct. The committed `.sqlx/` offline cache is stale,
or your dev DB predates the new migration.
**Fix:**
1. `./scripts/setup-db.sh` (re-applies migrations to the dev DB)
2. `cd src-tauri && cargo sqlx prepare`
3. Commit the regenerated `.sqlx/` — CI builds with `SQLX_OFFLINE=true` and breaks otherwise.
Still failing? `rm src-tauri/.dev/dev.sqlite` and repeat from step 1.

### Notifications never appear (permission denied)
macOS asks for notification permission once; if denied, the app never re-prompts and
the engine's notifications silently go nowhere (the log still shows "notification fired" —
trust the log: the send happened, the OS suppressed it).
**Fix:** System Settings → Notifications → Focus Planner → Allow Notifications, then
restart the app. In dev, the entry may appear under the terminal/dev-host app instead.

### Log file empty or missing
Dev logs: `logs/focus-planner.log.<YYYY-MM-DD>` at the repo root. Packaged:
`~/Library/Logs/com.sramzz.focusplanner/`. If a packaged build logs nothing, the
`LogGuard` returned by `logging::init` was dropped — it must stay in Tauri managed
state for the process lifetime (`src-tauri/src/lib.rs` setup).
```

- [ ] **Step 2: Link it** — confirm the README (Task 19) links this file; add the link if you wrote the README first.

- [ ] **Step 3: Commit**

```bash
git add docs/runbooks
git commit -m "docs: dev-environment runbook — fresh-clone setup and the three recurring failures"
```

---

### Task 21: Harvest `docs/lessons/` from M1 execution `[trivial]`

PHILOSOPHY: we pay for a lesson once. Phases 1–6 each accumulated friction — make sure it is written down before the milestone closes.

- [ ] **Step 1: Sweep for unrecorded lessons** — review the phase branches/PR discussions and your own notes for: plan amendments made mid-phase (Tasks 5/6 of this plan are prime candidates if Phase 3/4 shapes differed), CI surprises, SQLx/Tauri/rodio gotchas, anything the runbook (Task 20) documents that wasn't in `docs/lessons/` yet.

- [ ] **Step 2: Write the entries** — one file per category (`docs/lessons/tauri.md`, `sqlx.md`, `frontend.md`, `planning.md`…), entry format per `docs/lessons/README.md`: date, what happened, root cause, the rule going forward.

- [ ] **Step 3: Commit**

```bash
git add docs/lessons
git commit -m "docs: harvest M1 lessons"
```

---

### Task 22: Full M1 manual QA checklist — the final gate `[medium]`

A scripted end-to-end walkthrough of every phase's demonstrable outcome, run in **dev** first, then the abbreviated packaged pass. Run it top to bottom in one sitting on a fresh database (`rm` the app-data DB or use a clean machine account). Every box must be ticked before the M1 PR merges. Anything that fails: fix, lesson entry if warranted, re-run from the affected section.

**Setup**
- [ ] Fresh clone + `docs/runbooks/dev-environment.md` setup works exactly as written
- [ ] `cargo test --manifest-path src-tauri/Cargo.toml` and `npm test -- --run` green; CI green on the branch

**Phase 1 — shell & plumbing**
- [ ] App launches as a native "Focus Planner" window; all 4 sidebar views navigate
- [ ] `logs/focus-planner.log.<today>` opens with: logging initialized → opening database → migrations applied

**Phase 2 — backlog**
- [ ] Create project → goal → task → 3 microtasks (estimates auto-compute pomodoro counts; one microtask on a custom PomodoroType created in Settings)
- [ ] Drag-reorder microtasks; order survives an app restart
- [ ] Complete the last open microtask of a task → roll-up completes the task; uncomplete reverses it; the roll-up chain appears in the log
- [ ] Delete a goal → confirm dialog → cascade removes its tasks/microtasks; Cancel leaves everything intact

**Phase 6 settings → Phase 3 planning**
- [ ] Settings: set planning window 10:00 → 15:00 (log shows `setting planning_window_start changed 09:00 -> 10:00`, then the end change)
- [ ] Attempt window end 08:00 → field rolls back, ErrorBanner explains, WARN in the log
- [ ] Generate a day plan → first block starts 10:00, breaks interleaved per type, long-break rule visible if the custom type defines one, nothing past 15:00
- [ ] Add a manual meeting block; regenerate → planner schedules around it
- [ ] Drag-reorder blocks, delete one, commit the plan

**Phase 6 settings → Phase 4 runtime**
- [ ] Settings: volume 30%, work warning 1 minute (for a fast test); Test sound plays at the lower volume
- [ ] Start Day → countdown ticks; "1 minutes remaining…" notification arrives 1 min before block end; completion sound at 30%; auto-advance into the break; break warning; skip break works; pause/resume works; End Day
- [ ] Set `sound_enabled` off and `notifications_enabled` off → run one more short block → silent, no notifications, but transitions still advance and the log shows the `suppressed` lines
- [ ] **Settings-change session reconstructable from logs:** `grep "setting " logs/focus-planner.log.<today>` replays every change above as `setting <key> changed <old> -> <new>` lines in order, interleaved with the `get_settings`/`update_setting` command envelopes — a zero-context reader can narrate the session
- [ ] One full Start Day run reads as a coherent narrative in the log (snapshot line → transitions → sessions written → day ended)

**Phase 5 — analytics & data**
- [ ] Analytics shows the sessions just run (totals, completion, per-project)
- [ ] Export to JSON → wipe the app-data DB → relaunch → Import (confirm dialog first) → backlog, plans, history, **and settings** all restored; Analytics matches pre-wipe

**Hardening**
- [ ] Fresh-DB pass: all 4 views show their EmptyState; loading flashes are graceful; forced validation error shows the ErrorBanner and dismisses
- [ ] Keyboard-only pass: Tab reaches sidebar, settings inputs, dialog buttons with a visible focus ring; Esc cancels the confirm dialog
- [ ] Visual pass: Inter renders (Wi-Fi off), custom scrollbars in long lists, transitions smooth, no light-mode artifacts

**Packaging (abbreviated re-run on the .app)**
- [ ] Task 18's packaged smoke test all green: DB in `~/Library/Application Support/com.sramzz.focusplanner/`, narrative log in `~/Library/Logs/com.sramzz.focusplanner/`, settings change + test sound logged there
- [ ] Dmg mounts; README's right-click-open Gatekeeper instructions are accurate as written
- [ ] Dock and Cmd-Tab show the placeholder icon (and the icon-replacement flag is still recorded in the README)

When every box is ticked: final commit of the checked-off plan, push, open the Phase 6 PR, and proceed per superpowers:finishing-a-development-branch.

---

## Plan self-review

- **Scope coverage:** roadmap Phase 6 row fully covered — settings commands + view (Tasks 2–9), empty/error states + dark polish (10–15), icon/identifier/dmg (16–17), packaged verification (18), QA across all phases (22); plus the spec-§7 logging deliverables threaded through Tasks 2–6 and the docs/runbook/lessons closure (19–21).
- **CQS audit:** `get_settings`/`get_setting`/`snapshot`/`planning_window` return data and never write; `update_setting` and `play_test_sound` return `Result<(), AppError>` and never return data. No command both mutates and returns.
- **Placeholder scan:** all code blocks are complete and compilable as written against Phase 1's landed conventions; the *only* intentionally adaptive pieces are the Phase 3/4 call-site names in Tasks 5–6 and the Phase 2/5 section names in Tasks 9/13, each marked with the amend-plan-plus-lesson rule (those plans' code lands before this plan runs). The app icon is an explicitly flagged placeholder asset, not a plan placeholder.
- **Phase 1 consistency:** `AppError` codes extend (`internal`) rather than change; commands use the exact `#[tracing::instrument(skip(db))]` + ok/failed envelope from Phase 1 Task 8; stores use `ipc<T>()`; tests use `#[sqlx::test]` and `mockIPC`; lib name `focus_planner_lib` carried over (same adjust-if-different note as Phase 1).
- **Key consistency:** the 7 registry keys match what the consumers read — `planning_window_start`/`planning_window_end` (Task 5 planner), `sound_volume`/`sound_enabled`/`notifications_enabled`/`notify_work_warning_minutes`/`notify_break_warning_minutes` (Task 6 snapshot) — and the snake_case keys pass the IPC boundary unchanged (map data, not struct fields).
- **Known cut:** no live mid-run settings push to the engine (snapshot-at-start per spec §4, POLA-noted in the Audio section UI); no notarization (documented caveat); cross-key window validation is the single deliberate exception to per-key validation.
