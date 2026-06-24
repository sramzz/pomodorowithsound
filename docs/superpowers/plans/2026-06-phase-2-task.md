# Task Status

- [/] Task 1: Atomic Child Creation in Backend
  - [/] Step 1: Write failing tests in `src-tauri/tests/task_service.rs`
  - [ ] Step 2: Run backend tests to verify failures
  - [ ] Step 3: Modify `create_task` in `src-tauri/src/core/task_service.rs`
  - [ ] Step 4: Verify task service tests pass
  - [ ] Step 5: Write failing tests in `src-tauri/tests/microtask_service.rs`
  - [ ] Step 6: Run backend tests to verify failures
  - [ ] Step 7: Modify `create_microtask` in `src-tauri/src/core/microtask_service.rs`
  - [ ] Step 8: Verify microtask service tests pass
  - [ ] Step 9: Commit backend atomic fixes

- [ ] Task 2: Return NotFound when reordering under a missing parent
  - [ ] Step 1: Write failing tests in `src-tauri/tests/reorder.rs`
  - [ ] Step 2: Run tests to verify failures
  - [ ] Step 3: Modify `reorder_goals` in `src-tauri/src/core/goal_service.rs`
  - [ ] Step 4: Modify `reorder_tasks` in `src-tauri/src/core/task_service.rs`
  - [ ] Step 5: Modify `reorder_microtasks` in `src-tauri/src/core/microtask_service.rs`
  - [ ] Step 6: Run cargo tests to verify all pass
  - [ ] Step 7: Commit backend reorder fixes

- [ ] Task 3: Expose Project Update and Archive in Store & UI
  - [ ] Step 1: Write failing store unit tests in `src/stores/projectStore.test.ts`
  - [ ] Step 2: Run store tests to verify failures
  - [ ] Step 3: Modify `archiveProject` and `deleteProject` in `src/stores/projectStore.ts`
  - [ ] Step 4: Run store tests to verify they pass
  - [ ] Step 5: Add UI controls to `src/views/BacklogView.vue`
  - [ ] Step 6: Write component UI test in `src/views/BacklogView.test.ts`
  - [ ] Step 7: Run all vitest tests
  - [ ] Step 8: Commit UI and frontend fixes
