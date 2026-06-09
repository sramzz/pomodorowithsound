# Focus Planner — Architecture & Command Vocabulary (latest)

## The one principle everything hangs on

**One component owns every mutation. Nothing else writes data.**

Call it the **Domain Core** (a.k.a. Command Service). Every create / update / delete /
state change in the system goes through it. The desktop UI, the CLI, the mobile app, the
web app, the future cloud endpoint, and any LLM agent are all just *callers* of the Core —
none of them touch SQLite or Postgres directly. Reads can go straight to the data (or
through the Core's read side), but **writes have exactly one door.**

That is the single source of truth for "the creation of anything." The command vocabulary
(Section 4) is simply the list of doors. Building the Core first, in M1, is what makes the
agent story later a matter of *exposing* the Core remotely rather than rewriting it.

---

## 1. Long-term architecture

```
Long-term target — Version 1 (Local-first core, thin cloud; promotable to a server command core)

PRINCIPLE: all writes pass through the Domain Core. Clients never write data directly.

Clients (callers of the Core)
├── Tauri Desktop App                          (hosts the Core in-process)
│   ├── SQLite local database                  (device source of truth)
│   ├── DOMAIN CORE  (Command Service)          ◀── single mutation authority
│   │   ├── command handlers                    (the vocabulary, Section 4)
│   │   ├── validation + business rules
│   │   ├── deterministic planner
│   │   └── Start Day runtime engine            (timer / pomodoro / rest state machine)
│   │       └── live session state: LOCAL only, never synced
│   ├── Desktop integration                     notifications + sounds,
│   │                                           focus / app-blocking adapter
│   ├── Command transports over the Core        Local CLI · Local HTTP API · MCP server
│   └── Sync client (PowerSync — local-only → connect() when cloud lands)
│
├── Expo / React Native Mobile App             (thin: capture + visibility)
│   ├── SQLite local database · sync client
│   ├── calls the Core's commands (subset)      — no direct writes
│   └── quick capture · deadline/priority edits · calendar visibility (+ on-device Apple)
│
└── Optional PWA / Web App                      (thin: cloud-backed)
    └── calls the Core's commands (subset) · simple day view · no Start Day execution

Cloud / Backend (intentionally minimal)
├── Auth & User Management                      OAuth · user + device identity ·
│                                               Row-Level Security (per-user isolation)
├── Postgres / Supabase                         durable records: projects, goals, tasks,
│                                               microtasks, pomodoro types, plans, work
│                                               blocks, completed focus/pomodoro sessions,
│                                               calendar links, provider connections,
│                                               sync metadata
├── Sync Engine (PowerSync — SELECTED, not hand-built)
│                                               offline queue · conflict resolution ·
│                                               change log · per-user partitions  [engine-provided]
├── Command Endpoint                            = remote door to the SAME Domain Core
│                                               (idempotent commands · permission model)
├── Calendar Sync Service                       Google + Outlook (server-side) ·
│                                               Apple = on-device EventKit ·
│                                               encrypted tokens (server-only) ·
│                                               webhook intake · incremental sync · reconciliation
├── Agent / LLM Integration                     hits the SAME Command Endpoint / MCP ·
│                                               LLM = optional plan proposer → Core validates
│                                               + persists · permission model · safe boundaries
└── Focus / App-Blocking Adapter contract       driven locally by the desktop runtime
```

---

## 2. Short-term architecture

```
Short-term delivery — milestones ordered by priority; each ships something usable; nothing throwaway

M1 — Standalone desktop app   (no backend, no agents, offline, single-user)
Tauri Desktop App
├── SQLite local database
├── DOMAIN CORE (Command Service)              ◀── BUILD THIS FIRST
│   │                                              single source of truth for every
│   │                                              create / update / delete / state change
│   ├── model: projects, goals, tasks, microtasks,
│   │           estimated_minutes, pomodoro_count, deadlines, priorities
│   ├── PomodoroType presets                    default "Standard" 20/5; fallback 20 min
│   ├── core actions                            the vocabulary — defined once here (Section 4)
│   ├── deterministic planner                   basic day plan
│   └── Start Day runtime                       timer engine · notifications · sounds ·
│       └── live state: LOCAL only               completion state
├── day view + Start Day button                UI calls Core actions, never writes DB directly
├── local import / export
└── Sync client in LOCAL-ONLY mode             PowerSync installed, not connected yet

M2 — Local agent surface   (transports over the SAME Core; still no server)
├── Local CLI                    → calls Core actions
└── Optional Local HTTP API + MCP → calls Core actions
    └── this is where idempotency keys + the local/remote permission split get added

M3 — Cloud foundation   (multi-user + multi-device arrive together)
├── Auth & User Management        OAuth · user + device identity · Row-Level Security
├── Postgres / Supabase           user-owned records · history · backup
└── Turn ON real sync             PowerSync connect() + Command Endpoint
    └── the SAME Core vocabulary, now reachable remotely (no second sync built)

M4 — Calendar
Calendar Sync Service
├── Google + Outlook connectors (server-side) · import meetings · create task blocks · reconcile
├── encrypted provider tokens (server-only)
└── Apple = on-device EventKit (desktop/mobile) — deferred / optional

M5 — Thin clients
├── Expo / RN mobile   capture + sync; calls Core actions (subset)
└── PWA / Web          entry + day view; calls Core actions (subset)
```

---

## 3. Domain model (the nouns)

```
Project
└── Goal              deadline · priority/rank within project
    └── Task          deadline · priority/rank within goal
        └── Microtask the schedulable unit
              · estimated_minutes
              · pomodoro_count
              · pomodoro_type_id?     → which PomodoroType (else the default)
              · deadline? · priority/rank · status

PomodoroType          a named preset you pick from
   · id · name
   · work_minutes · rest_minutes
   · long_break_minutes? · long_break_every?
   examples:  "Standard" 20 / 5   (the default)
              "Short"    15 / 3
              "Deep"     50 / 10
   if a microtask names no type → the default type
   if no default type is configured → fall back to 20 min work

Plan (a day)          status: draft | committed; belongs to a date
└── WorkBlock         type: task | break | meeting
      · microtask_id?        (task blocks)
      · calendar_event_id?   (meeting blocks, imported)
      · start · end · pomodoro_index?

Runtime (live, desktop-local, NOT synced)
└── active run of a committed Plan: current block · timer position · paused/running

Durable history (synced after the fact)
├── FocusSession      one completed "Start Day" run (summary)
└── PomodoroSession   one completed pomodoro (for stats)

Calendar
├── ProviderConnection   Google | Outlook | Apple — tokens server-side, encrypted
└── CalendarEventLink    links a WorkBlock/Microtask to an external event
```

---

## 4. Core actions — the single source of truth  (build in M1)

These are the named operations the Domain Core exposes. In M1 they are plain functions the
UI calls — **no idempotency keys, no permission tiers yet** (those are added in Section 6).
The discipline that matters now: *the UI never writes the database; it calls one of these.*

> Conventions for M1: `Create*` carries the new entity's client-generated `id` (so creates
> are naturally safe and references work immediately). `Update*` sets **absolute values**,
> not deltas. `Reorder*` takes the **full ordered list**. `field?` = optional.

### Structure — projects, goals, tasks, microtasks
- `createProject(id, name, description?)`
- `updateProject(id, name?, description?)`
- `archiveProject(id)` · `deleteProject(id)`
- `createGoal(id, projectId, title, deadline?, priority?, description?)`
- `updateGoal(id, title?, deadline?, priority?, description?)`
- `archiveGoal(id)` · `deleteGoal(id)`
- `createTask(id, goalId, title, deadline?, priority?, description?)`
- `updateTask(id, title?, deadline?, priority?, description?)`
- `archiveTask(id)` · `deleteTask(id)`
- `createMicrotask(id, taskId, title, estimatedMinutes, pomodoroCount, pomodoroTypeId?, deadline?, priority?)`
- `updateMicrotask(id, …any of the above fields)`
- `completeMicrotask(id)` · `uncompleteMicrotask(id)`
- `archiveMicrotask(id)` · `deleteMicrotask(id)`

*Handler rule: completing the last microtask of a task may roll up to complete the task /
goal. Estimates and pomodoro counts roll up for display. Lives inside the Core, so every
caller gets it for free.*

### Ranking
- `reorderGoals(projectId, orderedGoalIds[])`
- `reorderTasks(goalId, orderedTaskIds[])`
- `reorderMicrotasks(taskId, orderedMicrotaskIds[])`

### Pomodoro types
- `createPomodoroType(id, name, workMinutes, restMinutes, longBreakMinutes?, longBreakEvery?)`
- `updatePomodoroType(id, …fields)`
- `deletePomodoroType(id)`
- `setDefaultPomodoroType(id)`

### Planning — plans & work blocks
- `generateDayPlan(planId, date, strategy?, constraints?)` — deterministic planner over
  eligible microtasks (deadlines, priorities, pomodoro counts × type length, rest) + the
  day's meetings → a **draft** plan. (Also the LLM's main door later: it proposes, the
  planner validates and persists.)
- `addWorkBlock(id, planId, microtaskId, start, end)`
- `moveWorkBlock(id, newStart, newEnd?)`  *(reschedule)*
- `removeWorkBlock(id)`
- `reorderWorkBlocks(planId, orderedWorkBlockIds[])`
- `commitDayPlan(planId)` · `clearDayPlan(planId)`

### Day-running — live runtime (desktop only)
Drives the screen, timers, sounds, state machine. Their *outputs* (completed sessions) are
durable and sync; the *live state* does not.
- `startDay(planId)` — begin: first block, start timer + sound, auto-engage focus mode
- `pauseDay()` · `resumeDay()`
- `completeCurrentBlock()` — mark running microtask done, record a PomodoroSession, advance
- `skipToNextBlock()`
- `startBreak()` · `skipBreak()`
- `endDay()` — finalize; write FocusSession + remaining PomodoroSessions (these sync)

### Focus / app-blocking adapter (the seam)
Usually auto-invoked by the runtime; exposed as actions so the blocker is a clean contract.
- `startFocusMode(context?)` · `stopFocusMode()`

---

## 5. Queries (reads)

Read-only; they never change state. In M1 these are simply the read functions the UI calls.
- `listProjects(includeArchived?)`
- `getProjectTree(projectId)` — project → goals → tasks → microtasks
- `getMicrotask(id)`
- `getDayPlan(planId | date)` — plan + ordered work blocks (incl. meetings)
- `getToday(date?)` — committed plan + blocks + meetings for the day
- `getRunStatus()` — live runtime state (local only)
- `getStats(dateRange?)` — completed focus/pomodoro sessions, completion rates
- `getCalendar(dateRange, provider?)` — *added with M4*

---

## 6. Deferred — the agent & sync layer  (M2 / M3 onward)

Everything here is real, but it is **not** needed for the single-user offline tool. It is
added on top of the same Core actions when remote callers and sync appear — no rewrite,
because the actions are already the single door.

### Idempotency (added when retries exist — i.e. with sync / remote callers)
Every command additionally carries a client-generated `commandId`. The Core records
processed ids; a repeat is a no-op returning the prior result. Combined with the M1
conventions (client-supplied entity ids, absolute updates, full-order reorders), this makes
agent and sync retries safe.

### Permission tiers (added when a remote caller exists)
| Tier | Meaning |
|------|---------|
| **L**  | Local only. Never exposed remotely. Anything that drives the screen, sound, or machine: all of *Day-running* and *Focus mode*. |
| **A**  | Local + Agent. Safe to automate: most structure, ranking, pomodoro-type, and planning actions. |
| **A\*** | Local + Agent, **confirm by default**. Destructive or consequential: `delete*`, and the calendar publish actions below. Agent proposes; user confirms unless auto-approve is on. |
| **U**  | User-interactive. Needs a human flow (OAuth consent). |

### Transports (M2 → M3)
The same actions, reached different ways: Local CLI (M2) · Local HTTP API + MCP (M2) ·
Cloud Command Endpoint (M3). One vocabulary, many doors.

### Calendar commands (M4) — over the same Core
- `connectCalendarProvider(provider)` — **U** (OAuth; Apple via on-device EventKit)
- `disconnectCalendarProvider(provider)` — **A\***
- `syncCalendar(provider?, dateRange?)` — **A** (import + reconcile meetings)
- `publishDayPlanToCalendar(planId)` — **A\*** (writes to your real calendar → confirm)
- `unpublishDayPlanFromCalendar(planId)` · `removeCalendarEvent(eventLinkId)` — **A\***

### Worked example — the LLM endgame, all through the one Core
1. Agent (A): `createProject` → `createGoal` → `createTask` → `createMicrotask` (with
   estimates, pomodoro counts, chosen pomodoro type, deadlines, priorities).
2. Agent (A): `generateDayPlan(date)` → draft over your meetings; tweak with `moveWorkBlock`
   / `reorderWorkBlocks`; `commitDayPlan`.
3. `publishDayPlanToCalendar` — **A\***, so **you confirm** before it writes your calendar.
4. You press Start Day → `startDay(planId)` — **L**. An agent could never do this step; by
   design. The runtime advances blocks, plays sounds, fires the 5-minute warning, runs rest,
   and `startFocusMode` engages the blocker.
5. `endDay()` writes the durable session records, which sync to your other devices.

Same Core, many doors, one source of truth.