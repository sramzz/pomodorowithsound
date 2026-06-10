# Migration History

Every schema change is appended here. If you ran `./scripts/setup-db.sh` on a fresh clone, all of these have been applied.

| Migration file | Date | What it did | Why |
|----------------|------|-------------|-----|
| `0001_initial_schema.sql` | 2026-06-09 | Created the 10 tables: `projects`, `goals`, `tasks`, `microtasks`, `pomodoro_types`, `plans`, `work_blocks`, `focus_sessions`, `pomodoro_sessions`, `settings` | Initial M1 schema — establishes the full domain model |
| `0002_seed_default_pomodoro_type.sql` | 2026-06-09 | Inserted the "Standard" pomodoro type (20 min work / 5 min rest, `is_default = 1`) | The planner and runtime engine require a default type to fall back on when a microtask specifies none |

---

_Every future schema change — new column, new table, index, constraint — gets a new numbered migration file and a new row here._
