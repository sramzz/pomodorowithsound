-- Projects
CREATE TABLE projects (
    id TEXT PRIMARY KEY,               -- client-generated UUID
    name TEXT NOT NULL,
    description TEXT,
    status TEXT NOT NULL DEFAULT 'open', -- open | completed
    is_archived INTEGER NOT NULL DEFAULT 0,
    completed_at TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Goals (belong to a project)
CREATE TABLE goals (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    description TEXT,
    deadline TEXT,                      -- ISO 8601, nullable
    priority INTEGER NOT NULL DEFAULT 0,
    sort_order INTEGER NOT NULL DEFAULT 0,
    status TEXT NOT NULL DEFAULT 'open', -- open | completed
    is_archived INTEGER NOT NULL DEFAULT 0,
    completed_at TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Tasks (belong to a goal)
CREATE TABLE tasks (
    id TEXT PRIMARY KEY,
    goal_id TEXT NOT NULL REFERENCES goals(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    description TEXT,
    deadline TEXT,
    priority INTEGER NOT NULL DEFAULT 0,
    sort_order INTEGER NOT NULL DEFAULT 0,
    status TEXT NOT NULL DEFAULT 'open', -- open | completed
    is_archived INTEGER NOT NULL DEFAULT 0,
    completed_at TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Pomodoro Types (presets)
CREATE TABLE pomodoro_types (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    work_minutes INTEGER NOT NULL,
    rest_minutes INTEGER NOT NULL,
    long_break_minutes INTEGER,         -- nullable
    long_break_every INTEGER,           -- nullable (e.g., every 4 pomodoros)
    is_default INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Microtasks (the schedulable unit, belong to a task)
CREATE TABLE microtasks (
    id TEXT PRIMARY KEY,
    task_id TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    estimated_minutes INTEGER NOT NULL,
    pomodoro_count INTEGER NOT NULL DEFAULT 1,
    pomodoro_type_id TEXT REFERENCES pomodoro_types(id) ON DELETE SET NULL,
    deadline TEXT,
    priority INTEGER NOT NULL DEFAULT 0,
    sort_order INTEGER NOT NULL DEFAULT 0,
    status TEXT NOT NULL DEFAULT 'open', -- open | completed
    is_archived INTEGER NOT NULL DEFAULT 0,
    completed_at TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Plans (a day plan; exactly one per date — regenerating replaces the draft)
CREATE TABLE plans (
    id TEXT PRIMARY KEY,
    date TEXT NOT NULL UNIQUE,           -- YYYY-MM-DD, one plan per date
    status TEXT NOT NULL DEFAULT 'draft', -- draft | committed
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Work Blocks (belong to a plan)
CREATE TABLE work_blocks (
    id TEXT PRIMARY KEY,
    plan_id TEXT NOT NULL REFERENCES plans(id) ON DELETE CASCADE,
    block_type TEXT NOT NULL,            -- task | break | meeting
    microtask_id TEXT REFERENCES microtasks(id) ON DELETE SET NULL,
    calendar_event_id TEXT,              -- M4 event link, nullable
    start_time TEXT NOT NULL,            -- ISO 8601 UTC
    end_time TEXT NOT NULL,              -- ISO 8601 UTC
    pomodoro_index INTEGER,              -- Pomodoro index within the microtask
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Durable history: Focus Sessions (completed day runs)
CREATE TABLE focus_sessions (
    id TEXT PRIMARY KEY,
    plan_id TEXT NOT NULL REFERENCES plans(id),
    start_time TEXT NOT NULL,
    end_time TEXT NOT NULL,
    total_work_seconds INTEGER NOT NULL,
    total_break_seconds INTEGER NOT NULL,
    blocks_completed INTEGER NOT NULL,
    blocks_skipped INTEGER NOT NULL,
    created_at TEXT NOT NULL
);

-- Durable history: Pomodoro Sessions (individual completed pomodoros)
CREATE TABLE pomodoro_sessions (
    id TEXT PRIMARY KEY,
    focus_session_id TEXT REFERENCES focus_sessions(id),
    microtask_id TEXT REFERENCES microtasks(id),
    pomodoro_type_id TEXT REFERENCES pomodoro_types(id),
    work_minutes INTEGER NOT NULL,
    started_at TEXT NOT NULL,
    completed_at TEXT NOT NULL,
    was_completed INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL
);

-- App settings (key-value): planning window, audio volume, notification toggles
CREATE TABLE settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
