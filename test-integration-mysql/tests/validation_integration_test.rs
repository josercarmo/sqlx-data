use sqlx_data::{IntoParams, ParamsBuilder, Pool, Result, Serial, dml, repo};

#[repo]
trait UserRepo {
    #[dml("SELECT id, name FROM users")]
    async fn find_users(&self, params: impl IntoParams) -> Result<Serial<(i64,String)>>;
}

struct TestRepo {
    pool: Pool,
}

impl UserRepo for TestRepo {
    fn get_pool(&self) -> &Pool {
        &self.pool
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx_data::{Pool, Result};

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_validation_allows_safe_sorts(pool: Pool) -> Result<()> {
        let repo = TestRepo { pool };

        // Safe sorts should work without validation errors
        let params = ParamsBuilder::new()
            .sort()
                .asc("id")  // compile-time safe
                .done()
            .serial()
                .page(1, 10)
                .done()
            .build();

        let result = repo.find_users(params).await;
        assert!(result.is_ok());

        let users = result.unwrap();
        assert!(!users.data.is_empty());

        Ok(())
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_validation_allows_valid_unsafe_sorts(pool: Pool) -> Result<()> {
        let repo = TestRepo { pool };

        // Unsafe sorts with valid whitelist should work
        let params = ParamsBuilder::new()
            .sort()
                .with_allowed_columns(&["id", "name"])
                .asc_unsafe("name".to_string())  // runtime validation - valid
                .done()
            .serial()
                .page(1, 10)
                .done()
            .build();

        let result = repo.find_users(params).await;
        assert!(result.is_ok());

        let users = result.unwrap();
        assert!(!users.data.is_empty());

        Ok(())
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_validation_rejects_invalid_unsafe_sorts(pool: Pool) -> Result<()> {
        let repo = TestRepo { pool };

        // Unsafe sorts with invalid field should be rejected
        let params = ParamsBuilder::new()
            .sort()
                .with_allowed_columns(&["id", "name"])
                .asc_unsafe("malicious_field".to_string())  // runtime validation - invalid
                .done()
            .serial()
                .page(1, 10)
                .done()
            .build();

        let result = repo.find_users(params).await;
        assert!(result.is_err());

        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("Sort validation failed"));
        assert!(error_msg.contains("malicious_field"));

        Ok(())
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_validation_mixed_safe_unsafe(pool: Pool) -> Result<()> {
        let repo = TestRepo { pool };

        // Mix of safe and valid unsafe sorts should work
        let params = ParamsBuilder::new()
            .sort()
                .asc("id")  // safe
                .with_allowed_columns(&["name", "id"])
                .desc_unsafe("name".to_string())  // unsafe but valid
                .done()
            .serial()
                .page(1, 10)
                .done()
            .build();

        let result = repo.find_users(params).await;
        assert!(result.is_ok());

        let users = result.unwrap();
        assert!(!users.data.is_empty());

        Ok(())
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_validation_rejects_invalid_unsafe_sorts_contains(pool: Pool) -> Result<()> {
        let repo = TestRepo { pool };

        // Unsafe sorts with invalid field should be rejected
        let params = ParamsBuilder::new()
            .sort()
                .with_allowed_columns(&["id", "name"])
                .asc_unsafe("name; DROP".to_string())  // runtime validation - invalid
                .done()
            .serial()
                .page(1, 10)
                .done()
            .build();

        let result = repo.find_users(params).await;
        assert!(result.is_err());

        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("Sort validation failed"));
        assert!(error_msg.contains("name; DROP"));
        Ok(())
    }
}