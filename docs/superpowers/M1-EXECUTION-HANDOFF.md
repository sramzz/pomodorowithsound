# M1 Execution Handoff

_Status snapshot created after execution was explicitly stopped on 2026-06-10._

## Current state

- **Branch:** `feat/m1-phase-2-backlog-domain`, stacked on `feat/m1-phase-1-scaffold`.
- **HEAD:** `6300aa637688c91246d4007950e145f498668af8` (`fix: close phase 2 review gaps`).
- **Working tree:** clean except this untracked handoff file. The interrupted worker produced no uncommitted changes.
- **Phase 1:** complete and previously approved.
- **Phase 2:** implementation and docs are present, but the phase-wide review still has four Important findings. Do not start Phase 3 until they are fixed and the reviewer approves.
- **Phases 3-6:** not started. Their existing plans remain authoritative.
- **Push/PR/GUI QA:** deferred. Nothing was pushed, no PR was created, and no GUI-dependent check was run.

## Required execution conventions

- Use `superpowers:subagent-driven-development`: fresh implementer, spec review, then code-quality review; fix and re-review before proceeding.
- Use the cross-agent effort mapping below for every task and review.

  | Task difficulty | Codex | Claude Code |
  |---|---|---|
  | Trivial / Easy | GPT-5.5, low reasoning | Sonnet, low thinking |
  | Medium | GPT-5.5, medium reasoning | Sonnet, high thinking |
  | Difficult / Hard | GPT-5.5, high reasoning | Opus, medium thinking |

  `[hard]` is equivalent to Difficult / Hard. Phase-wide and broad architectural reviews use this tier.
- Each phase is a stacked branch and must be merged in order.
- New commits use:
  `Co-Authored-By: Codex <noreply@openai.com>`
- Preserve the untracked handoff and local artifacts. Do not revert unrelated user work.
- Do not launch `npm run tauri dev`, push, create PRs, or claim CI success automatically.
- For SQLx checked-query changes, run:
  `cd src-tauri && cargo sqlx prepare -- --all-targets && cd ..`
- SQLx offline tests must use `SQLX_OFFLINE=true`.

## Work completed in this session

### Phase 2 documentation

1. `b673decdefd7a06ea8416d3d0c4f4504f9d83a6c`
   - Updated `README.md` for Phase 2 state.
   - Added `list_pomodoro_types()` to spec section 5.
   - Verified no Phase 2 schema migration change.
2. `347d48e745a5bb80579d6dc53048723eb8f8b778`
   - Corrected README wording so it does not overstate user-facing readiness.
3. Task-level spec and quality reviews approved these documentation changes.

### Phase 2 review fixes

Commit `6300aa637688c91246d4007950e145f498668af8` fixed the first phase-wide review findings:

- Reorder operations reject duplicate IDs at goal/task/microtask levels.
- Child creation rejects completed or archived parents.
- Added project deletion, goal/task/microtask mutation controls, and pomodoro preset editing in the UI.
- Cleared long-break inputs are normalized to `null` before IPC.
- Project tree refresh errors are no longer erased by a successful project-list refresh.
- Project store tests now receive isolated Pinia state.
- Added backend and frontend regression tests and refreshed SQLx metadata.

The focused compliance review approved this commit.

### Last successful verification

- Rust: **55 tests passed**.
- Vitest: **19 tests passed**.
- `npx vue-tsc --noEmit`: passed.
- `git diff --check`: passed.
- A focused reviewer also reported the production frontend build passing.

## Open Phase 2 review findings

The high-effort phase-wide reviewer re-reviewed `feat/m1-phase-1-scaffold..HEAD` and found four remaining Important issues. A worker was asked to fix them, but the user stopped execution before it changed files. The worker was closed and left no partial patch.

1. **Make child creation atomic against parent state**
   - `src-tauri/src/core/task_service.rs`
   - `src-tauri/src/core/microtask_service.rs`
   - Current parent `SELECT` and later `INSERT` can race with completion/archive.
   - Use a transaction with a conditional `INSERT ... SELECT ... WHERE status = 'open' AND is_archived = 0`, or an equivalent atomic strategy.
   - Preserve semantics: missing parent = `AppError::NotFound`; completed/archived parent = `AppError::Validation`.
   - Add regression tests for both services.

2. **Expose project update and archive in the UI**
   - `src/views/BacklogView.vue`
   - `src/stores/projectStore.ts`
   - Project delete is exposed, but rename/update and archive remain store-only.
   - Add compact project rename and archive controls plus focused UI tests.

3. **Return NotFound when reordering under a missing parent**
   - `src-tauri/src/core/goal_service.rs`
   - `src-tauri/src/core/task_service.rs`
   - `src-tauri/src/core/microtask_service.rs`
   - Empty reorder lists currently allow a nonexistent parent to return success; nonempty lists return Validation.
   - Explicitly verify the parent and return `AppError::NotFound` consistently.
   - Add tests for goal, task, and microtask reorder services.

4. **Preserve frontend state when project deletion fails**
   - `src/stores/projectStore.ts`
   - The active tree is currently cleared before IPC succeeds.
   - Clear it only after successful deletion. On failure, retain the tree and expose the error.
   - Add a store regression test.

## Exact next steps

1. Confirm branch and clean state:
   `git switch feat/m1-phase-2-backlog-domain && git status --short --branch`
2. Dispatch one GPT-5.5 high-effort worker for only the four findings above, using TDD. Commit as:
   `fix: harden phase 2 mutation edge cases`
   with the Codex co-author trailer.
3. Run a medium-effort focused spec-compliance review of that fix commit. Resolve every gap and re-review.
4. Ask the existing/new GPT-5.5 high-effort reviewer to re-review the full diff:
   `feat/m1-phase-1-scaffold..HEAD`
   Fix every Critical or Important finding and repeat until approved.
5. Run the complete Phase 2 gate:
   - `SQLX_OFFLINE=true cargo test --manifest-path src-tauri/Cargo.toml`
   - `npm test -- --run`
   - `npx vue-tsc --noEmit`
   - `git diff --check feat/m1-phase-1-scaffold..HEAD`
6. Record Phase 2 manual QA as deferred: native drag/restart persistence, confirmation/radio behavior, and the complete `logs/` narrative.
7. Only after Phase 2 approval, create `feat/m1-phase-3-day-planning` from Phase 2 HEAD and execute `docs/superpowers/plans/2026-06-09-m1-phase-3-day-planning.md` task by task.
8. Continue stacked Phases 4-6 using their existing plans and the same review/verification gates.

## Remaining phase sequence

- **Phase 3:** deterministic pure planner, plan persistence/commands, `usePlanStore`, draft Day timeline. Planner tasks require high effort.
- **Phase 4:** tokio actor runtime, ticks, warnings, sessions, sound/notification adapters, runtime UI. Runtime engine tasks require high effort.
- **Phase 5:** SQL analytics plus versioned transactional export/import. Stats/import hard tasks require high effort.
- **Phase 6:** settings registry/integration, shared UI states and confirms, styling, packaging, runbook, and final M1 QA.

## Manual QA and delivery still owed

- Phase 1 GUI smoke test and log narrative.
- Phase 2 full backlog/settings workflow, drag persistence, roll-up, and logging checks.
- Phase 4 physical sound and notification checks.
- Phase 6 packaged `.app`/`.dmg` smoke test, OS app-directory DB/log checks, and full M1 walkthrough.
- Push stacked branches, create ordered PRs, and confirm CI only when explicitly requested.
