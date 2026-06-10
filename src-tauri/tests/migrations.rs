use sqlx::SqlitePool;

#[sqlx::test]
async fn migrations_apply_and_seed_the_default_pomodoro_type(pool: SqlitePool) {
    let row = sqlx::query!(
        r#"SELECT name, work_minutes, rest_minutes, is_default FROM pomodoro_types"#
    )
    .fetch_one(&pool)
    .await
    .expect("seed row must exist");

    assert_eq!(row.name, "Standard");
    assert_eq!(row.work_minutes, 20);
    assert_eq!(row.rest_minutes, 5);
    assert_eq!(row.is_default, 1);
}

#[sqlx::test]
async fn one_plan_per_date_is_enforced(pool: SqlitePool) {
    let insert = |id: &'static str| {
        sqlx::query("INSERT INTO plans (id, date, status, created_at, updated_at) VALUES (?, '2026-06-09', 'draft', '2026-06-09T08:00:00Z', '2026-06-09T08:00:00Z')")
            .bind(id)
            .execute(&pool)
    };
    insert("p1").await.expect("first plan inserts");
    let err = insert("p2").await.expect_err("second plan for same date must violate UNIQUE");
    assert!(err.to_string().contains("UNIQUE"));
}
