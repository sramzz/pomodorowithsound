# Spec: Phase 2 Review Fixes

## 1. Goal & Context
The goal is to address the four remaining Important issues from the Phase 2 code review to unblock Phase 3. 

These issues are:
1. **Child creation atomicity against parent state**: Ensure task/microtask creation is atomic and cannot race with parent goal/task completion or archiving.
2. **Project update and archive in the UI**: Expose the ability to rename/update and archive projects in the backlog view, with associated UI tests.
3. **NotFound on reordering under a missing parent**: Reordering goals/tasks/microtasks under a nonexistent project/goal/task must return a `NotFound` error, even if the reorder list is empty.
4. **Preserve frontend state when project deletion fails**: Do not clear the active project tree in `projectStore` before the delete IPC call succeeds. Retain the tree and expose the error on failure.

---

## 2. Approach & Architecture

### Finding 1: Atomic Child Creation
We will use a conditional insert statement in Rust with a fallback query on failure:
* **Tasks**:
  ```sql
  INSERT INTO tasks (id, goal_id, title, description, deadline, priority, sort_order, status, is_archived, created_at, updated_at)
  SELECT ?1, id, ?2, ?3, ?4, ?5, (SELECT COALESCE(MAX(sort_order) + 1, 0) FROM tasks WHERE goal_id = ?6), 'open', 0, ?7, ?8
  FROM goals WHERE id = ?9 AND status = 'open' AND is_archived = 0
  ```
  If `rows_affected() == 0`, we run:
  ```sql
  SELECT id, status, is_archived FROM goals WHERE id = ?
  ```
  If no goal is found, return `AppError::NotFound`. If the goal is found but is closed or archived, return `AppError::Validation`.
* **Microtasks**:
  Apply the same pattern for `tasks` as the parent of `microtasks`.

### Finding 2: Project Update & Archive in UI
* **UI Controls**: 
  In the `BacklogView.vue` sidebar, next to each project item:
  * Add a "Rename" button (ghost icon or text). When clicked, prompt the user via `window.prompt` with the current name. If a valid name is provided, invoke `store.updateProject(id, name, description)`.
  * Add an "Archive" button. When clicked, invoke `store.archiveProject(id)`.
* **Store Logic**:
  Update `archiveProject(id)` in `projectStore.ts` to clear `activeProjectTree` if the archived project is the currently active one (similar to `deleteProject`).

### Finding 3: Reordering Under Missing Parent
In `reorder_goals`, `reorder_tasks`, and `reorder_microtasks`:
* Perform an explicit parent check before/inside the transaction:
  * For goals: `SELECT id FROM projects WHERE id = ?`
  * For tasks: `SELECT id FROM goals WHERE id = ?`
  * For microtasks: `SELECT id FROM tasks WHERE id = ?`
* If the query returns `None`, immediately return `AppError::NotFound`.

### Finding 4: Preserve State on Deletion Failure
In `projectStore.ts`:
* In `deleteProject(id)` and `archiveProject(id)`:
  * Keep track of whether the deleted/archived project is the active one (`const wasActive = this.activeProjectTree?.id === id;`).
  * Call `await this.mutate(...)`.
  * Only clear the active tree (`this.activeProjectTree = null`) if `wasActive && !this.error`.

---

## 3. Data Flow & IPC

* **IPC Channels**:
  * No new IPC channels are required. We are leveraging existing channels (`update_project`, `archive_project`, `create_task`, `create_microtask`, `reorder_goals`, `reorder_tasks`, `reorder_microtasks`, `delete_project`).
* **Client-side**:
  * UI triggers action -> Store executes mutation -> If success, tree reloads -> UI updates.
  * If mutation fails, error is logged in `store.error` and state is preserved.

---

## 4. Testing & Verification

### Backend (Rust Regression Tests)
* **Atomic Creation**:
  * Add tests where a task is created under a nonexistent/completed/archived goal.
  * Add tests where a microtask is created under a nonexistent/completed/archived task.
* **Reordering**:
  * Test that reordering goals/tasks/microtasks under a missing parent returns `AppError::NotFound`.
  * Test reordering with empty lists.

### Frontend (Vitest Tests)
* **Store Deletion/Archiving Failures**:
  * Add tests in `projectStore.test.ts` mocking IPC failures to verify `activeProjectTree` is preserved.
* **UI Controls**:
  * Add component tests in `BacklogView.test.ts` (or equivalent) to verify rename and archive buttons are present and function as expected.
