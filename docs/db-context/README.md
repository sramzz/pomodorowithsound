# DB Context

This folder keeps database knowledge current so any newcomer — or a future AI agent — can understand the schema and its history without digging through migration files.

**Rule:** whenever the schema changes, update both files here in the same commit as the migration.

## Contents

| File | Purpose |
|------|---------|
| [schema-for-dummies.md](schema-for-dummies.md) | Plain-language walk of all 10 tables — what each one is for and how they relate |
| [migration-history.md](migration-history.md) | Chronological log of every migration: what changed and why |
