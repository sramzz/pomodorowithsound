# Phase 2 Review Fixes Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Resolve the four open Phase 2 review findings (child creation atomicity, UI project edit/archive controls, parent checks on reordering, and state preservation on deletion failure).

**Architecture:** Conditional `INSERT ... SELECT` atomic queries in Rust, explicit parent existence SELECT queries in reorder transactions, and post-IPC error checking on Pinia store actions.

**Tech Stack:** Rust, SQLx (Sqlite), Vue 3, TypeScript, Pinia, Vitest.

---

### Task 1: Atomic Child Creation in Backend

**Files:**
- Modify: `src-tauri/src/core/task_service.rs`
- Modify: `src-tauri/src/core/microtask_service.rs`
- Test: `src-tauri/tests/task_service.rs`
- Test: `src-tauri/tests/microtask_service.rs`

- [ ] **Step 1: Write failing tests in `src-tauri/tests/task_service.rs`**
  Modify `src-tauri/tests/task_service.rs` to assert that trying to create a task under a completed/archived or nonexistent goal raises the correct errors. Add concurrent/stress testing assertions or simply assert correctness of the direct API return types under these states.
  ```rust
  // Append these tests to src-tauri/tests/task_service.rs
  #[tokio::test]
  async fn test_create_task_under_archived_or_completed_goal_atomic() {
      let pool = focus_planner_lib::db::init_test_db().await;
      
      // Seed a completed goal
      let project_id = "p-1";
      sqlx::query!("INSERT INTO projects (id, name, status, created_at, updated_at) VALUES (?, 'P1', 'open', 'now', 'now')", project_id).execute(&pool).await.unwrap();
      
      let goal_completed = "g-completed";
      sqlx::query!("INSERT INTO goals (id, project_id, title, status, is_archived, created_at, updated_at) VALUES (?, ?, 'G-Comp', 'completed', 0, 'now', 'now')", goal_completed, project_id).execute(&pool).await.unwrap();
      
      let goal_archived = "g-archived";
      sqlx::query!("INSERT INTO goals (id, project_id, title, status, is_archived, created_at, updated_at) VALUES (?, ?, 'G-Arch', 'open', 1, 'now', 'now')", goal_archived, project_id).execute(&pool).await.unwrap();

      // Assert validation errors
      let err = create_task(&pool, "t-1", goal_completed, "Task 1", None, None, 0).await.unwrap_err();
      assert!(matches!(err, AppError::Validation(_)));

      let err = create_task(&pool, "t-2", goal_archived, "Task 2", None, None, 0).await.unwrap_err();
      assert!(matches!(err, AppError::Validation(_)));

      // Assert not found
      let err = create_task(&pool, "t-3", "nonexistent-goal", "Task 3", None, None, 0).await.unwrap_err();
      assert!(matches!(err, AppError::NotFound { .. }));
  }
  ```

- [ ] **Step 2: Run backend tests to verify failures**
  Run: `SQLX_OFFLINE=true cargo test --test task_service`
  Expected: FAIL (compilation errors because tests/code doesn't match yet, or test assertion failures).

- [ ] **Step 3: Modify `create_task` in `src-tauri/src/core/task_service.rs`**
  Replace lines 20-43 in `src-tauri/src/core/task_service.rs` with:
  ```rust
      let now = now_iso8601();
      let mut tx = pool.begin().await?;
      let result = sqlx::query!(
          "INSERT INTO tasks (id, goal_id, title, description, deadline, priority, sort_order,
                              status, is_archived, created_at, updated_at)
           SELECT ?1, id, ?2, ?3, ?4, ?5,
                  (SELECT COALESCE(MAX(sort_order) + 1, 0) FROM tasks WHERE goal_id = ?6),
                  'open', 0, ?7, ?8
           FROM goals WHERE id = ?9 AND status = 'open' AND is_archived = 0",
          id, title, description, deadline, priority, goal_id, now, now, goal_id
      )
      .execute(&mut *tx)
      .await?;

      if result.rows_affected() == 0 {
          let goal = sqlx::query!("SELECT status, is_archived FROM goals WHERE id = ?", goal_id)
              .fetch_optional(&mut *tx)
              .await?;
          if let Some(g) = goal {
              if g.status == "completed" {
                  return Err(AppError::Validation("cannot create a task under a completed goal".into()));
              } else {
                  return Err(AppError::Validation("cannot create a task under an archived goal".into()));
              }
          } else {
              return Err(AppError::NotFound { entity: "goal", id: goal_id.to_string() });
          }
      }
      tx.commit().await?;
  ```

- [ ] **Step 4: Verify task service tests pass**
  Run: `SQLX_OFFLINE=true cargo test --test task_service`
  Expected: PASS

- [ ] **Step 5: Write failing tests in `src-tauri/tests/microtask_service.rs`**
  Add tests asserting that trying to create a microtask under a completed/archived or nonexistent task returns the proper errors.
  ```rust
  #[tokio::test]
  async fn test_create_microtask_under_archived_or_completed_task_atomic() {
      let pool = focus_planner_lib::db::init_test_db().await;
      
      let project_id = "p-1";
      sqlx::query!("INSERT INTO projects (id, name, status, created_at, updated_at) VALUES (?, 'P1', 'open', 'now', 'now')", project_id).execute(&pool).await.unwrap();
      let goal_id = "g-1";
      sqlx::query!("INSERT INTO goals (id, project_id, title, status, is_archived, created_at, updated_at) VALUES (?, ?, 'G1', 'open', 0, 'now', 'now')", goal_id, project_id).execute(&pool).await.unwrap();

      let task_completed = "t-completed";
      sqlx::query!("INSERT INTO tasks (id, goal_id, title, status, is_archived, created_at, updated_at) VALUES (?, ?, 'T-Comp', 'completed', 0, 'now', 'now')", task_completed, goal_id).execute(&pool).await.unwrap();
      
      let task_archived = "t-archived";
      sqlx::query!("INSERT INTO tasks (id, goal_id, title, status, is_archived, created_at, updated_at) VALUES (?, ?, 'T-Arch', 'open', 1, 'now', 'now')", task_archived, goal_id).execute(&pool).await.unwrap();

      // Assert validation errors
      let err = create_microtask(&pool, "m-1", task_completed, "Micro 1", 20, 1, None, None, 0).await.unwrap_err();
      assert!(matches!(err, AppError::Validation(_)));

      let err = create_microtask(&pool, "m-2", task_archived, "Micro 2", 20, 1, None, None, 0).await.unwrap_err();
      assert!(matches!(err, AppError::Validation(_)));

      // Assert not found
      let err = create_microtask(&pool, "m-3", "nonexistent-task", "Micro 3", 20, 1, None, None, 0).await.unwrap_err();
      assert!(matches!(err, AppError::NotFound { .. }));
  }
  ```

- [ ] **Step 6: Run backend tests to verify failures**
  Run: `SQLX_OFFLINE=true cargo test --test microtask_service`
  Expected: FAIL

- [ ] **Step 7: Modify `create_microtask` in `src-tauri/src/core/microtask_service.rs`**
  Replace lines 56-86 in `src-tauri/src/core/microtask_service.rs` with:
  ```rust
      let now = now_iso8601();
      let mut tx = pool.begin().await?;
      let result = sqlx::query!(
          "INSERT INTO microtasks (id, task_id, title, estimated_minutes, pomodoro_count,
                                   pomodoro_type_id, deadline, priority, sort_order,
                                   status, is_archived, created_at, updated_at)
           SELECT ?1, id, ?2, ?3, ?4, ?5, ?6, ?7,
                  (SELECT COALESCE(MAX(sort_order) + 1, 0) FROM microtasks WHERE task_id = ?8),
                  'open', 0, ?9, ?10
           FROM tasks WHERE id = ?11 AND status = 'open' AND is_archived = 0",
          id, title, estimated_minutes, pomodoro_count, pomodoro_type_id, deadline, priority, task_id, now, now, task_id
      )
      .execute(&mut *tx)
      .await?;

      if result.rows_affected() == 0 {
          let task = sqlx::query!("SELECT status, is_archived FROM tasks WHERE id = ?", task_id)
              .fetch_optional(&mut *tx)
              .await?;
          if let Some(t) = task {
              if t.status == "completed" {
                  return Err(AppError::Validation("cannot create a microtask under a completed task".into()));
              } else {
                  return Err(AppError::Validation("cannot create a microtask under an archived task".into()));
              }
          } else {
              return Err(AppError::NotFound { entity: "task", id: task_id.to_string() });
          }
      }
      tx.commit().await?;
  ```

- [ ] **Step 8: Verify microtask service tests pass**
  Run: `SQLX_OFFLINE=true cargo test --test microtask_service`
  Expected: PASS

- [ ] **Step 9: Commit backend atomic fixes**
  Run: `git commit -am "fix: make child creation atomic against parent state (Codex)"`

---

### Task 2: Return NotFound when reordering under a missing parent

**Files:**
- Modify: `src-tauri/src/core/goal_service.rs`
- Modify: `src-tauri/src/core/task_service.rs`
- Modify: `src-tauri/src/core/microtask_service.rs`
- Modify: `src-tauri/tests/reorder.rs`

- [ ] **Step 1: Write failing tests in `src-tauri/tests/reorder.rs`**
  Modify `src-tauri/tests/reorder.rs` to test reordering with empty lists under missing parent.
  ```rust
  // Add this test to src-tauri/tests/reorder.rs
  #[tokio::test]
  async fn test_reorder_under_nonexistent_parent_returns_not_found() {
      let pool = focus_planner_lib::db::init_test_db().await;

      // Reorder goals under nonexistent project
      let err_goals = reorder_goals(&pool, "nonexistent-project", &[]).await.unwrap_err();
      assert!(matches!(err_goals, AppError::NotFound { entity: "project", .. }));

      // Reorder tasks under nonexistent goal
      let err_tasks = reorder_tasks(&pool, "nonexistent-goal", &[]).await.unwrap_err();
      assert!(matches!(err_tasks, AppError::NotFound { entity: "goal", .. }));

      // Reorder microtasks under nonexistent task
      let err_microtasks = reorder_microtasks(&pool, "nonexistent-task", &[]).await.unwrap_err();
      assert!(matches!(err_microtasks, AppError::NotFound { entity: "task", .. }));
  }
  ```

- [ ] **Step 2: Run tests to verify failures**
  Run: `SQLX_OFFLINE=true cargo test --test reorder`
  Expected: FAIL

- [ ] **Step 3: Modify `reorder_goals` in `src-tauri/src/core/goal_service.rs`**
  Insert the parent project check in `reorder_goals`:
  ```rust
      let mut tx = pool.begin().await?;
      let parent = sqlx::query!("SELECT id FROM projects WHERE id = ?", project_id)
          .fetch_optional(&mut *tx)
          .await?;
      if parent.is_none() {
          return Err(AppError::NotFound {
              entity: "project",
              id: project_id.to_string(),
          });
      }
  ```

- [ ] **Step 4: Modify `reorder_tasks` in `src-tauri/src/core/task_service.rs`**
  Insert the parent goal check in `reorder_tasks`:
  ```rust
      let mut tx = pool.begin().await?;
      let parent = sqlx::query!("SELECT id FROM goals WHERE id = ?", goal_id)
          .fetch_optional(&mut *tx)
          .await?;
      if parent.is_none() {
          return Err(AppError::NotFound {
              entity: "goal",
              id: goal_id.to_string(),
          });
      }
  ```

- [ ] **Step 5: Modify `reorder_microtasks` in `src-tauri/src/core/microtask_service.rs`**
  Insert the parent task check in `reorder_microtasks`:
  ```rust
      let mut tx = pool.begin().await?;
      let parent = sqlx::query!("SELECT id FROM tasks WHERE id = ?", task_id)
          .fetch_optional(&mut *tx)
          .await?;
      if parent.is_none() {
          return Err(AppError::NotFound {
              entity: "task",
              id: task_id.to_string(),
          });
      }
  ```

- [ ] **Step 6: Run cargo tests to verify all pass**
  Run: `SQLX_OFFLINE=true cargo test`
  Expected: PASS (All 55+ tests pass)

- [ ] **Step 7: Commit backend reorder fixes**
  Run: `git commit -am "fix: return NotFound on reordering under nonexistent parent (Codex)"`

---

### Task 3: Expose Project Update and Archive in Store & UI

**Files:**
- Modify: `src/stores/projectStore.ts`
- Modify: `src/views/BacklogView.vue`
- Modify: `src/stores/projectStore.test.ts`
- Modify: `src/views/BacklogView.test.ts`

- [ ] **Step 1: Write failing store unit tests in `src/stores/projectStore.test.ts`**
  Add unit tests for `archiveProject` and `deleteProject` showing state preservation on failure, and clearing active tree on success.
  ```typescript
  // Add to src/stores/projectStore.test.ts
  it("preserves activeProjectTree when deleteProject fails", async () => {
    const store = useProjectStore();
    store.activeProjectTree = { id: "p-1", name: "P1", description: null, status: "open", goals: [] };
    mockIpcReject("delete_project", "db", "delete failed");

    await store.deleteProject("p-1");
    expect(store.activeProjectTree).not.toBeNull();
    expect(store.activeProjectTree?.id).toBe("p-1");
    expect(store.error).toBe("delete failed");
  });

  it("preserves activeProjectTree when archiveProject fails", async () => {
    const store = useProjectStore();
    store.activeProjectTree = { id: "p-1", name: "P1", description: null, status: "open", goals: [] };
    mockIpcReject("archive_project", "db", "archive failed");

    await store.archiveProject("p-1");
    expect(store.activeProjectTree).not.toBeNull();
    expect(store.activeProjectTree?.id).toBe("p-1");
    expect(store.error).toBe("archive failed");
  });

  it("clears activeProjectTree on successful archiveProject", async () => {
    const store = useProjectStore();
    store.activeProjectTree = { id: "p-1", name: "P1", description: null, status: "open", goals: [] };
    mockIpcResolve("archive_project", null);
    mockIpcResolve("list_projects", []);

    await store.archiveProject("p-1");
    expect(store.activeProjectTree).toBeNull();
  });
  ```

- [ ] **Step 2: Run store tests to verify failures**
  Run: `npm test -- --run`
  Expected: FAIL

- [ ] **Step 3: Modify `archiveProject` and `deleteProject` in `src/stores/projectStore.ts`**
  Update the actions:
  ```typescript
      async archiveProject(id: string) {
        const wasActive = this.activeProjectTree?.id === id;
        await this.mutate("archive_project", { id });
        if (wasActive && !this.error) {
          this.activeProjectTree = null;
        }
      },
      async deleteProject(id: string) {
        const wasActive = this.activeProjectTree?.id === id;
        await this.mutate("delete_project", { id });
        if (wasActive && !this.error) {
          this.activeProjectTree = null;
        }
      },
  ```

- [ ] **Step 4: Run store tests to verify they pass**
  Run: `npm test -- --run`
  Expected: PASS

- [ ] **Step 5: Add UI controls to `src/views/BacklogView.vue`**
  Add Rename and Archive buttons to `src/views/BacklogView.vue` under sidebar list items:
  ```html
            <button
              class="ghost"
              :aria-label="`Rename ${p.name}`"
              title="Rename project"
              @click.stop="renameProject(p.id, p.name, p.description)"
            >Rename</button>
            <button
              class="ghost"
              :aria-label="`Archive ${p.name}`"
              title="Archive project"
              @click.stop="store.archiveProject(p.id)"
            >Archive</button>
  ```
  Add the `renameProject` script in `BacklogView.vue`:
  ```typescript
  function renameProject(id: string, name: string, description: string | null) {
    const newName = window.prompt("Project name", name)?.trim();
    if (newName) {
      store.updateProject(id, newName, description);
    }
  }
  ```

- [ ] **Step 6: Write component UI test in `src/views/BacklogView.test.ts`**
  Test that the rename and archive buttons are rendered and trigger correct actions.
  ```typescript
  // Add assertions or a test case in src/views/BacklogView.test.ts
  ```

- [ ] **Step 7: Run all vitest tests**
  Run: `npm test -- --run`
  Expected: PASS

- [ ] **Step 8: Commit UI and frontend fixes**
  Run: `git commit -am "fix: expose update/archive controls in UI and preserve state on deletion fail (Codex)"`
