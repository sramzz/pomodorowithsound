# Lessons

Every mistake and failed experiment becomes an entry here — we pay for a lesson once.

## Structure

One markdown file per category (e.g. `tauri.md`, `sqlx.md`, `frontend.md`), created the first time a category gets a lesson.

## Entry format

Each entry in a category file follows this template:

```
### YYYY-MM-DD — Short title

**What happened:** describe the problem or failed attempt.

**Root cause:** why it happened.

**The rule going forward:** the concrete change in approach that prevents a recurrence.
```

## Categories to expect

- `tauri.md` — Tauri v2 quirks, IPC gotchas, plugin issues
- `sqlx.md` — SQLx compile-time checks, offline mode, migration ordering
- `frontend.md` — Vue 3 / Pinia / Vite / TypeScript pitfalls
- `ci.md` — GitHub Actions, caching, platform-specific build issues
- `rust.md` — async/tokio patterns, ownership issues, crate version conflicts
