use sqlx_data::{Pool, Result, dml, repo};

#[derive(Debug)]
pub struct DynamicResult {
    pub id: i64,
    pub name: String,
    pub computed_value: i64,
}

#[repo]
trait UncheckedRepo {
    #[dml("SELECT 1 as id, 'test' as name, 42 as computed_value", unchecked)]
    async fn simple_unchecked(&self) -> Result<(i64, String, i64)>;

    #[dml("SELECT ? as dynamic_id, ? as dynamic_name", unchecked)]
    async fn parameterized_unchecked(&self, id: i64, name: String) -> Result<(i64, String)>;

    #[dml("SHOW TABLES", unchecked)]
    async fn show_tables(&self) -> Result<Vec<Vec<u8>>>;

    #[dml(
        "SELECT COUNT(*) FROM information_schema.tables WHERE table_schema = DATABASE()",
        unchecked
    )]
    async fn count_tables(&self) -> Result<i64>;

    #[dml("SELECT CAST(NOW() AS CHAR)", unchecked)]
    async fn current_timestamp(&self) -> Result<String>;
}

pub struct UncheckedApp {
    pool: Pool,
}

impl UncheckedRepo for UncheckedApp {
    fn get_pool(&self) -> &Pool {
        &self.pool
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_simple_unchecked_query(pool: Pool) {
        let app = UncheckedApp { pool };

        let result = app.simple_unchecked().await.unwrap();
        assert_eq!(result.0, 1);
        assert_eq!(result.1, "test");
        assert_eq!(result.2, 42);
    }

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_parameterized_unchecked_query(pool: Pool) {
        let app = UncheckedApp { pool };

        let result = app
            .parameterized_unchecked(100, "dynamic".to_string())
            .await
            .unwrap();
        assert_eq!(result.0, 100);
        assert_eq!(result.1, "dynamic");
    }

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_show_tables_unchecked(pool: Pool) {
        let app = UncheckedApp { pool };

        let tables = app.show_tables().await.unwrap();
        assert!(!tables.is_empty());

        // MySQL SHOW TABLES returns table names as VARBINARY (Vec<u8>)
        let users_bytes = b"users".to_vec();
        assert!(tables.contains(&users_bytes));

        // Also check for json_users table
        let json_users_bytes = b"json_users".to_vec();
        assert!(tables.contains(&json_users_bytes));
    }

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_count_tables_unchecked(pool: Pool) {
        let app = UncheckedApp { pool };

        let count = app.count_tables().await.unwrap();
        assert!(count > 0);
    }

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_current_timestamp_unchecked(pool: Pool) {
        let app = UncheckedApp { pool };

        let timestamp = app.current_timestamp().await.unwrap();
        assert!(!timestamp.is_empty());
        assert!(timestamp.len() > 10);
    }

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_multiple_unchecked_calls(pool: Pool) {
        let app = UncheckedApp { pool };

        for i in 0..3 {
            let result = app
                .parameterized_unchecked(i, format!("test_{}", i))
                .await
                .unwrap();
            assert_eq!(result.0, i);
            assert_eq!(result.1, format!("test_{}", i));
        }
    }
}
