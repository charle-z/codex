use super::*;
use crate::migrations::runtime_logs_migrator;
use crate::runtime::test_support::unique_temp_dir;
use codex_utils_absolute_path::AbsolutePathBuf;

#[tokio::test]
async fn locked_persistent_logs_db_falls_back_to_in_memory_store() {
    let home = unique_temp_dir();
    std::fs::create_dir_all(&home).expect("create sqlite test home");
    let sqlite_home =
        AbsolutePathBuf::try_from(home.clone()).expect("sqlite test home must be absolute");
    let config = SqliteConfig::new_for_testing(sqlite_home);
    let logs_path = config.logs_db_path();

    // Create the file without applying Codex migrations, then hold a write
    // transaction so the real logs migrator deterministically hits SQLITE_BUSY.
    let blocker_pool = config
        .open_read_write_pool(&logs_path)
        .await
        .expect("open lock-holder pool");
    sqlx::query("CREATE TABLE lock_holder (value INTEGER NOT NULL)")
        .execute(&blocker_pool)
        .await
        .expect("create lock-holder table");
    let mut blocker = blocker_pool.acquire().await.expect("acquire lock holder");
    sqlx::query("BEGIN IMMEDIATE")
        .execute(&mut *blocker)
        .await
        .expect("hold sqlite write lock");
    sqlx::query("INSERT INTO lock_holder (value) VALUES (1)")
        .execute(&mut *blocker)
        .await
        .expect("keep write transaction active");

    let fallback_pool = config
        .open_logs_db(&runtime_logs_migrator(), /*telemetry_override*/ None)
        .await
        .expect("locked log db should degrade to an in-memory log store");

    let fallback_has_logs: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'logs'",
    )
    .fetch_one(&fallback_pool)
    .await
    .expect("query fallback log schema");
    assert_eq!(fallback_has_logs, 1, "fallback store should be migrated");

    // Release the original database and verify fallback migration never touched
    // the locked persistent file.
    sqlx::query("ROLLBACK")
        .execute(&mut *blocker)
        .await
        .expect("release sqlite write lock");
    drop(blocker);
    blocker_pool.close().await;

    let persistent_pool = config
        .open_read_write_pool(&logs_path)
        .await
        .expect("reopen persistent log db");
    let persistent_has_logs: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'logs'",
    )
    .fetch_one(&persistent_pool)
    .await
    .expect("query persistent log schema");
    assert_eq!(
        persistent_has_logs, 0,
        "fallback must not mutate the locked persistent database"
    );

    persistent_pool.close().await;
    fallback_pool.close().await;
    std::fs::remove_dir_all(home).expect("remove sqlite test home");
}

#[tokio::test]
async fn corrupt_logs_db_still_returns_error_for_recovery() {
    let home = unique_temp_dir();
    std::fs::create_dir_all(&home).expect("create sqlite test home");
    let sqlite_home =
        AbsolutePathBuf::try_from(home.clone()).expect("sqlite test home must be absolute");
    let config = SqliteConfig::new_for_testing(sqlite_home);
    let logs_path = config.logs_db_path();
    std::fs::write(&logs_path, b"not a sqlite database").expect("write corrupt log db");

    let err = match config
        .open_logs_db(&runtime_logs_migrator(), /*telemetry_override*/ None)
        .await
    {
        Ok(pool) => {
            pool.close().await;
            panic!("corrupt log db unexpectedly degraded to in-memory storage");
        }
        Err(err) => err,
    };

    assert!(
        !sqlite_error_is_lock(&err),
        "corruption must not be classified as a lock: {err:#}"
    );
    assert!(
        crate::runtime::is_sqlite_corruption_error(&err),
        "corrupt log db should remain visible to the existing recovery path: {err:#}"
    );

    std::fs::remove_dir_all(home).expect("remove sqlite test home");
}
