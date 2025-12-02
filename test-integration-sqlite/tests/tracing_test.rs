use sqlx_data::{Connection, Pool, QueryResult, Result, dml, repo};

#[allow(dead_code)]
type Blob = Vec<u8>;
// Test trait to verify tracing feature behavior - using simple trait for testing
#[repo(instrument = true)] //Turn on 
trait TracingTestRepo {
    // This method should automatically get #[instrument] when tracing feature is enabled
    #[dml("SELECT id, name FROM users WHERE id = ?")]
    async fn find_user_auto_instrument(
        &self,
        conn: &mut Connection,
        id: i64,
    ) -> Result<(i64, String)>;

    // This method has explicit #[instrument] so should not get duplicated
    #[dml("SELECT COUNT(*) FROM users WHERE id = ?")]
    #[instrument(skip(self))]
    async fn count_users_explicit_instrument(&self, conn: &mut Connection, id: i64) -> Result<i64>;

    // Another auto-instrumented method with different return type
    #[dml("INSERT INTO users (name, email) VALUES (?, ?) RETURNING id")]
    async fn create_user_auto_instrument(
        &self,
        conn: &mut Connection,
        name: String,
        email: String,
    ) -> Result<i64>;

    // This method uses explicit #[instrument] with skip_all to disable tracing
    #[dml("SELECT email as 'email: Option<String>' FROM users WHERE id = ?")]
    #[instrument(skip_all)]
    async fn get_user_email_no_trace(
        &self,
        conn: &mut Connection,
        id: i64,
    ) -> Result<Option<String>>;
}

// Trait WITHOUT instrument=true (no automatic tracing)
#[repo]
trait NoTracingRepo {
    #[dml("SELECT COUNT(*) FROM users")]
    async fn count_all_users_no_trace(&self, conn: &mut Connection) -> Result<i64>;
}

// Trait WITH instrument=true (automatic tracing for all methods)
#[repo(instrument = true)]
trait AutoTracingRepo {
    // This method gets automatic tracing from trait level
    #[dml("SELECT id, name FROM users WHERE name = ?")]
    async fn find_user_auto_traced(
        &self,
        conn: &mut Connection,
        name: String,
    ) -> Result<(i64, String)>;

    // This method also gets automatic tracing
    #[dml("UPDATE users SET name = ? WHERE id = ?")]
    async fn update_user_auto_traced(
        &self,
        conn: &mut Connection,
        name: String,
        id: i64,
    ) -> Result<QueryResult>;

    // This method should skip Vec<u8> parameter in tracing
    #[dml("INSERT INTO users (name, email) VALUES (?, ?)")]
    async fn insert_user_with_blob(
        &self,
        conn: &mut Connection,
        name: String,
        data: Blob,
    ) -> Result<QueryResult>;

    // This method has explicit #[instrument] which should win over automatic
    #[dml("DELETE FROM users WHERE id = ?")]
    #[instrument(skip_all)]
    async fn delete_user_explicit_trace(
        &self,
        conn: &mut Connection,
        id: i64,
    ) -> Result<QueryResult>;
}

struct TestRepo {
    pool: Pool,
}

impl TracingTestRepo for TestRepo {
    fn get_pool(&self) -> &Pool {
        &self.pool
    }
}

struct NoTracingTestRepo {
    pool: Pool,
}

impl NoTracingRepo for NoTracingTestRepo {
    fn get_pool(&self) -> &Pool {
        &self.pool
    }
}

struct AutoTracingTestRepo {
    pool: Pool,
}

impl AutoTracingRepo for AutoTracingTestRepo {
    fn get_pool(&self) -> &Pool {
        &self.pool
    }
}

#[cfg(test)]
mod tests {
    use sqlx_data::Pool;

    use super::*;

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_tracing_methods_compile(pool: Pool) {
        let repo = TestRepo { pool };
        let mut conn = repo.get_pool().acquire().await.unwrap();

        // Test auto-instrumented find method
        let result = repo.find_user_auto_instrument(&mut conn, 1).await;
        assert!(result.is_ok());
        let (id, name) = result.unwrap();
        assert_eq!(id, 1);
        assert_eq!(name, "Alice");

        // Test explicit instrumented count method
        let count = repo
            .count_users_explicit_instrument(&mut conn, 1)
            .await
            .unwrap();
        assert_eq!(count, 1);

        // Test method with skip_all (disabled tracing)
        let email = repo.get_user_email_no_trace(&mut conn, 1).await.unwrap();
        assert!(email.is_some());
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_no_tracing_functionality(pool: Pool) {
        let repo = NoTracingTestRepo { pool };
        let mut conn = repo.get_pool().acquire().await.unwrap();

        // Test method without tracing (should compile without instrument)
        let count = repo.count_all_users_no_trace(&mut conn).await.unwrap();
        assert!(count > 0);
        assert_eq!(count, 20); // Should have 20 users from fixture
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_auto_tracing_functionality(pool: Pool) {
        let repo = AutoTracingTestRepo { pool };
        let mut conn = repo.get_pool().acquire().await.unwrap();

        // Test method with automatic tracing (from trait level)
        let result = repo
            .find_user_auto_traced(&mut conn, "Alice".to_string())
            .await;
        assert!(result.is_ok());
        let (id, name) = result.unwrap();
        assert_eq!(id, 1);
        assert_eq!(name, "Alice");

        // Test method with automatic tracing
        let result = repo
            .update_user_auto_traced(&mut conn, "Updated".to_string(), 1)
            .await;
        assert!(result.is_ok());

        // Test explicit tracing method (should override automatic)
        let result = repo.delete_user_explicit_trace(&mut conn, 999).await;
        assert!(result.is_ok()); // Should succeed even if user doesn't exist
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_tracing_output_live(pool: Pool) {
        use tracing_subscriber;

        // Initialize tracing subscriber to see actual output (filter out sqlx noise)
        use tracing_subscriber::{EnvFilter, FmtSubscriber};

        let filter = EnvFilter::new("info").add_directive("sqlx=warn".parse().unwrap());

        let subscriber = FmtSubscriber::builder()
            .with_max_level(tracing::Level::INFO)
            .with_env_filter(filter)
            .with_span_events(
                tracing_subscriber::fmt::format::FmtSpan::NEW
                    | tracing_subscriber::fmt::format::FmtSpan::CLOSE,
            )
            .finish();

        let _ = tracing::subscriber::set_global_default(subscriber);

        let repo = AutoTracingTestRepo { pool };
        let mut conn = repo.get_pool().acquire().await.unwrap();

        // This should create visible tracing spans
        tracing::info!("About to call find_user_auto_traced");
        let _result = repo
            .find_user_auto_traced(&mut conn, "Alice".to_string())
            .await;
        tracing::info!("find_user_auto_traced completed");

        // Test with Vec<u8> parameter (should be skipped)
        let large_data = vec![1u8; 1000]; // Large binary data
        tracing::info!("About to call insert_user_with_blob");
        let _result = repo
            .insert_user_with_blob(&mut conn, "BlobUser".to_string(), large_data)
            .await;
        tracing::info!("insert_user_with_blob completed");
    }
}
