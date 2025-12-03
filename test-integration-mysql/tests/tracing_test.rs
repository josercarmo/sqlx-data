use sqlx_data::{Pool, QueryResult, Result, dml, repo};
use tracing::{info, warn, debug};

#[derive(Debug, sqlx::FromRow)]
pub struct User {
    pub id: i64,
    pub name: String,
    pub email: String,
}

#[repo]
trait TracingRepo {
    #[dml("SELECT id, name, email FROM users WHERE id = ?")]
    async fn find_user(&self, id: i64) -> Result<Option<User>>;

    #[dml("INSERT INTO users (name, email, age) VALUES (?, ?, ?)")]
    async fn insert_user(&self, name: String, email: String, age: u8) -> Result<QueryResult>;

    #[dml("UPDATE users SET name = ? WHERE id = ?")]
    async fn update_user_name(&self, name: String, id: i64) -> Result<QueryResult>;

    #[dml("DELETE FROM users WHERE id = ?")]
    async fn delete_user(&self, id: i64) -> Result<QueryResult>;
}

pub struct TracingApp {
    pool: Pool,
}

impl TracingRepo for TracingApp {
    fn get_pool(&self) -> &Pool {
        &self.pool
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracing_subscriber;

    fn setup_tracing() {
        let _ = tracing_subscriber::fmt()
            .with_max_level(tracing::Level::DEBUG)
            .with_test_writer()
            .try_init();
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_tracing_find_operations(pool: Pool) {
        setup_tracing();
        let app = TracingApp { pool };

        info!("Starting user lookup test");

        let user = app.find_user(1).await.unwrap();
        assert!(user.is_some());

        let user = user.unwrap();
        debug!("Found user: {:?}", user);

        info!("User lookup test completed successfully");
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_tracing_insert_operations(pool: Pool) {
        setup_tracing();
        let app = TracingApp { pool };

        info!("Starting user insert test");

        let result = app
            .insert_user("Traced User".to_string(), "traced@example.com".to_string(), 25)
            .await
            .unwrap();

        debug!("Inserted user with ID: {}", result.last_insert_id());
        assert!(result.last_insert_id() > 0);

        let inserted_user = app.find_user(result.last_insert_id() as i64).await.unwrap();
        assert!(inserted_user.is_some());

        let user = inserted_user.unwrap();
        debug!("Retrieved inserted user: {:?}", user);
        assert_eq!(user.name, "Traced User");

        info!("User insert test completed successfully");
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_tracing_update_operations(pool: Pool) {
        setup_tracing();
        let app = TracingApp { pool };

        info!("Starting user update test");

        let original_user = app.find_user(1).await.unwrap().unwrap();
        debug!("Original user: {:?}", original_user);

        let results = app
            .update_user_name("Updated Name".to_string(), 1)
            .await
            .unwrap();

        debug!("Updated {} rows", results.rows_affected());
        assert_eq!(results.rows_affected(), 1);

        let updated_user = app.find_user(1).await.unwrap().unwrap();
        debug!("Updated user: {:?}", updated_user);
        assert_eq!(updated_user.name, "Updated Name");

        info!("User update test completed successfully");
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_tracing_error_scenarios(pool: Pool) {
        setup_tracing();
        let app = TracingApp { pool };

        warn!("Testing error scenarios");

        let non_existent_user = app.find_user(999999).await.unwrap();
        assert!(non_existent_user.is_none());
        debug!("Non-existent user lookup returned None as expected");

        let update_result = app
            .update_user_name("Should Not Update".to_string(), 999999)
            .await
            .unwrap();
        assert_eq!(update_result.rows_affected(), 0);
        debug!("Update of non-existent user affected 0 rows as expected");

        info!("Error scenario tests completed");
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_tracing_transaction_like_operations(pool: Pool) {
        setup_tracing();
        let app = TracingApp { pool };

        info!("Starting transaction-like operations test");

        let result = app
            .insert_user("Temp User".to_string(), "temp@example.com".to_string(), 30)
            .await
            .unwrap();
        debug!("Created temporary user with ID: {}", result.last_insert_id());
        let user = app.find_user(result.last_insert_id() as i64).await.unwrap();
        assert!(user.is_some());
        debug!("Verified user exists: {:?}", user);

        let results = app.delete_user(result.last_insert_id() as i64).await.unwrap();
        debug!("Deleted {} rows", results.rows_affected());
        assert_eq!(results.rows_affected(), 1);

        let deleted_user = app.find_user(result.last_insert_id() as i64).await.unwrap();
        assert!(deleted_user.is_none());
        debug!("Verified user was deleted");

        info!("Transaction-like operations test completed successfully");
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_tracing_performance_logging(pool: Pool) {
        setup_tracing();
        let app = TracingApp { pool };

        info!("Starting performance test");

        let start = std::time::Instant::now();

        for i in 0..5 {
            let user = app.find_user((i % 3) + 1).await.unwrap();
            debug!("Query {} completed, user found: {}", i, user.is_some());
        }

        let duration = start.elapsed();
        info!("Completed 5 queries in {:?}", duration);

        assert!(duration.as_secs() < 5, "Queries should complete in reasonable time");
    }
}