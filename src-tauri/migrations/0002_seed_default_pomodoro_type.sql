INSERT INTO pomodoro_types (id, name, work_minutes, rest_minutes, long_break_minutes, long_break_every, is_default, created_at, updated_at)
VALUES (
    'a0000000-0000-4000-8000-000000000001',
    'Standard',
    20, 5, NULL, NULL, 1,
    strftime('%Y-%m-%dT%H:%M:%SZ', 'now'),
    strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
);
