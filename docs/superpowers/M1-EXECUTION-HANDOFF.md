# M1 Execution Handoff

_Original snapshot created 2026-06-10. Updated **2026-06-23** after full branch audit._

---

## Branch map

| Branch | Based on | Local | Remote | Status |
|---|---|---|---|---|
| `main` | — | ✅ | ✅ `origin/main` | Contains all docs/specs. Tip: `16212647` |
| `docs/m1-spec-fixes-and-roadmap` | `main` | ✅ | ✅ `origin/docs/m1-spec-fixes-and-roadmap` | **Fully merged into `main`** — same SHA (`16212647`). Can be deleted. |
| `feat/m1-phase-1-scaffold` | `main` | ✅ | ❌ never pushed | **Complete.** 14 commits, tip: `35a7003d`. Previously approved. Not pushed. |
| `feat/m1-phase-2-backlog-domain` | `feat/m1-phase-1-scaffold` | ✅ | ✅ `origin/feat/m1-phase-2-backlog-domain` | **In progress.** 32 commits on top of Phase 1, tip: `b76af01c`. Local = remote (in sync). |

### Stack order

```
main (16212647)
 └─ feat/m1-phase-1-scaffold (35a7003d) — 14 commits
     └─ feat/m1-phase-2-backlog-domain (b76af01c) — 32 commits on top of Phase 1
```

> **Note:** `docs/m1-spec-fixes-and-roadmap` is at the same SHA as `main`. It is safe to delete both locally and on the remote — it carries no unique work.

---

## Current state (as of 2026-06-23)

- **Checked-out branch:** `feat/m1-phase-2-backlog-domain`
- **HEAD:** `b76af01c94a465914e61ac848eabe77bb42a9a04` (`docs: update philosophy document for clarity and detail; add opencode configuration file`)
- **Working tree:** clean, fully synced with remote.
- **Phase 1:** complete and previously approved.
- **Phase 2:** implementation and docs are present, but **four Important review findings remain open** (see below). Do not start Phase 3 until they are fixed and a reviewer approves.
- **Phases 3–6:** not started. Plans exist in `docs/superpowers/plans/`.
- **Push/PR/GUI QA:** `feat/m1-phase-2-backlog-domain` is pushed. `feat/m1-phase-1-scaffold` is **not pushed**. No PRs created. No GUI-dependent check was run.

### Commits after the original handoff (2026-06-10)

Two **docs-only** commits were added. Neither addresses the four open review findings:

1. `c2272ea1` — `docs: standardize cross-agent effort mapping` (added the effort table to all plans and this handoff file)
2. `b76af01c` — `docs: update philosophy document for clarity and detail; add opencode configuration file` (PHILOSOPHY.md rewrite + `opencode.json`)

---

## Last successful verification (from 2026-06-10 session)

- Rust: **55 tests passed**.
- Vitest: **19 tests passed**.
- `npx vue-tsc --noEmit`: passed.
- `git diff --check`: passed.
- A focused reviewer also reported the production frontend build passing.

> **Warning:** These results are 13 days old. Re-run the full gate before proceeding with any implementation work.

---

## Open Phase 2 review findings (BLOCKING Phase 3)

The phase-wide reviewer found four remaining Important issues. A worker was assigned to fix them on 2026-06-10, but execution was stopped before any code changes were made. **No partial patches exist. These are completely untouched.**

### 1. Make child creation atomic against parent state
- **Files:** `src-tauri/src/core/task_service.rs`, `src-tauri/src/core/microtask_service.rs`
- **Issue:** Current parent `SELECT` and later `INSERT` can race with completion/archive.
- **Fix:** Use a transaction with a conditional `INSERT ... SELECT ... WHERE status = 'open' AND is_archived = 0`, or equivalent atomic strategy.
- **Semantics:** missing parent → `AppError::NotFound`; completed/archived parent → `AppError::Validation`.
- **Tests:** Add regression tests for both services.

### 2. Expose project update and archive in the UI
- **Files:** `src/views/BacklogView.vue`, `src/stores/projectStore.ts`
- **Issue:** Project delete is exposed, but rename/update and archive remain store-only.
- **Fix:** Add compact project rename and archive controls plus focused UI tests.

### 3. Return NotFound when reordering under a missing parent
- **Files:** `src-tauri/src/core/goal_service.rs`, `src-tauri/src/core/task_service.rs`, `src-tauri/src/core/microtask_service.rs`
- **Issue:** Empty reorder lists currently allow a nonexistent parent to return success; nonempty lists return Validation.
- **Fix:** Explicitly verify the parent and return `AppError::NotFound` consistently.
- **Tests:** Add tests for goal, task, and microtask reorder services.

### 4. Preserve frontend state when project deletion fails
- **Files:** `src/stores/projectStore.ts`
- **Issue:** The active tree is currently cleared before IPC succeeds.
- **Fix:** Clear it only after successful deletion. On failure, retain the tree and expose the error.
- **Tests:** Add a store regression test.

---

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

---

## Exact next steps

1. **Confirm environment and re-run tests** (the last successful run was 2026-06-10):
   ```bash
   git switch feat/m1-phase-2-backlog-domain && git status --short --branch
   SQLX_OFFLINE=true cargo test --manifest-path src-tauri/Cargo.toml
   npm test -- --run
   npx vue-tsc --noEmit
   ```
2. **Fix the four open findings** listed above. Dispatch one high-effort worker using TDD. Commit as:
   `fix: harden phase 2 mutation edge cases`
   with the Codex co-author trailer.
3. **Spec-compliance review** of the fix commit (medium effort). Resolve every gap and re-review.
4. **Phase-wide review** of the full diff `feat/m1-phase-1-scaffold..HEAD` (high effort). Fix every Critical or Important finding and repeat until approved.
5. **Run the complete Phase 2 gate:**
   ```bash
   SQLX_OFFLINE=true cargo test --manifest-path src-tauri/Cargo.toml
   npm test -- --run
   npx vue-tsc --noEmit
   git diff --check feat/m1-phase-1-scaffold..HEAD
   ```
6. Record Phase 2 manual QA as deferred: native drag/restart persistence, confirmation/radio behavior, and the complete `logs/` narrative.
7. **Push `feat/m1-phase-1-scaffold`** — it has never been pushed.
8. Only after Phase 2 approval, create `feat/m1-phase-3-day-planning` from Phase 2 HEAD and execute `docs/superpowers/plans/2026-06-09-m1-phase-3-day-planning.md` task by task.
9. Continue stacked Phases 4–6 using their existing plans and the same review/verification gates.

---

## Branch cleanup opportunity

`docs/m1-spec-fixes-and-roadmap` is identical to `main`. To clean up:
```bash
git branch -d docs/m1-spec-fixes-and-roadmap
git push origin --delete docs/m1-spec-fixes-and-roadmap
```

---

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

---

## Work completed in prior sessions (reference log)

### Phase 2 documentation (2026-06-10)

1. `b673decd` — Updated `README.md` for Phase 2 state; added `list_pomodoro_types()` to spec §5.
2. `347d48e7` — Corrected README wording so it does not overstate user-facing readiness.
3. Task-level spec and quality reviews approved these documentation changes.

### Phase 2 review fixes (2026-06-10)

Commit `6300aa63` fixed the first phase-wide review findings:
- Reorder operations reject duplicate IDs at goal/task/microtask levels.
- Child creation rejects completed or archived parents.
- Added project deletion, goal/task/microtask mutation controls, and pomodoro preset editing in the UI.
- Cleared long-break inputs are normalized to `null` before IPC.
- Project tree refresh errors are no longer erased by a successful project-list refresh.
- Project store tests now receive isolated Pinia state.
- Added backend and frontend regression tests and refreshed SQLx metadata.

### Post-handoff docs (2026-06-10 → 2026-06-22)

- `c2272ea1` — Standardized cross-agent effort mapping across all plan docs and this handoff.
- `b76af01c` — Rewrote `PHILOSOPHY.md` for clarity; added `opencode.json`.
