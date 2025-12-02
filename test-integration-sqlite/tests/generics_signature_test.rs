use sqlx::sqlite::SqliteQueryResult;
use sqlx_data::{DB, Executor, Pool, QueryResult, Result, dml};

// Test repository to validate that method signatures with generics are preserved correctly
#[sqlx_data::repo]
trait GenericSignatureRepo {
    // Baseline: method without generics (should work)
    #[dml("SELECT COUNT(*) FROM users")]
    async fn count_simple(&self) -> Result<i64>;

    // Test: Simple string parameter (no generics)
    #[dml("INSERT INTO users (name, email, age) VALUES (?, 'test@example.com', 25)")]
    async fn insert_string(&self, name: String) -> Result<QueryResult>;

    // Test 1: Generic with lifetime parameter
    #[dml("INSERT INTO users (name, email, age) VALUES (?, 'lifetime@example.com', 26)")]
    async fn insert_lifetime<'a>(&self, value: &'a str) -> Result<QueryResult>;

    // Test 2: Generic with executor trait bound (our main case)
    #[dml(
        "INSERT INTO users (id, name, email, age) VALUES (?, 'executor', 'exec@example.com', 27)"
    )]
    async fn insert_with_executor<'e, E>(
        &self,
        executor: E,
        value: i64,
    ) -> Result<SqliteQueryResult>
    where
        E: Executor<'e>;

    // Test 3: Multiple executor generics
    #[dml("INSERT INTO users (id, name, email, age) VALUES (?, ?, 'multi@example.com', 28)")]
    async fn insert_multi_executor<'e, EX>(
        &self,
        executor: EX,
        id: i64,
        name: String,
    ) -> Result<QueryResult>
    where
        EX: sqlx::Executor<'e, Database = sqlx::Sqlite>;

    // Test 4: Complex executor with additional constraints
    #[dml("INSERT INTO users (name, email, age) VALUES (?, 'complex@example.com', 29)")]
    async fn insert_complex_executor<'e, EX>(
        &self,
        executor: EX,
        name: String,
    ) -> Result<SqliteQueryResult>
    where
        EX: sqlx::Executor<'e, Database = sqlx::Sqlite> + Send; //not recommended way, here just to test
    //EX: Executor<'e> + Send; recommended way!!

    // Test 5: impl trait (should also be preserved)
    #[dml("INSERT INTO users (name, email, age) VALUES (?, 'impl@example.com', 30)")]
    async fn insert_impl_trait(
        &self,
        name: String,
        //executor: impl Executor<'_>, recommended way!!
        executor: impl sqlx::Executor<'_, Database = DB>, //not recommended way, here just to test
    ) -> Result<SqliteQueryResult>;
}

// Test implementation
pub struct TestGenericRepo {
    pool: Pool,
}

impl GenericSignatureRepo for TestGenericRepo {
    fn get_pool(&self) -> &Pool {
        &self.pool
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_simple_method_works(pool: Pool) {
        let repo = TestGenericRepo { pool };

        let count = repo.count_simple().await.unwrap();
        assert_eq!(count, 20); // Now we have 20 users from fixture
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_string_parameter_works(pool: Pool) {
        let repo = TestGenericRepo { pool };

        let result = repo.insert_string("test".to_string()).await.unwrap();
        assert_eq!(result.rows_affected(), 1);
    }

    // NOTE: These tests will fail to compile if generics are not preserved correctly
    // That's exactly what we want to catch!

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_multi_executor_compiles(pool: Pool) {
        let repo = TestGenericRepo { pool };

        let mut tx = repo.get_pool().begin().await.unwrap();
        let result = repo
            .insert_multi_executor(&mut *tx, 123, "test".to_string())
            .await
            .unwrap();
        assert_eq!(result.rows_affected(), 1);
        tx.commit().await.unwrap();
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_lifetime_generic_compiles(pool: Pool) {
        let repo = TestGenericRepo { pool };

        let name = "test";
        let result = repo.insert_lifetime(name).await.unwrap();
        assert_eq!(result.rows_affected(), 1);
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_executor_generic_compiles(pool: Pool) {
        let repo = TestGenericRepo { pool };

        // This is our critical test case - if generics are lost, this won't compile
        let mut tx = repo.get_pool().begin().await.unwrap();
        let result = repo.insert_with_executor(&mut *tx, 123).await.unwrap();
        assert_eq!(result.rows_affected(), 1);
        tx.commit().await.unwrap();
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_complex_executor_compiles(pool: Pool) {
        let repo = TestGenericRepo { pool };

        let mut tx = repo.get_pool().begin().await.unwrap();
        let result = repo
            .insert_complex_executor(&mut *tx, "complex test".to_string())
            .await
            .unwrap();
        assert_eq!(result.rows_affected(), 1);
        tx.commit().await.unwrap();
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_impl_trait_compiles(pool: Pool) {
        let repo = TestGenericRepo { pool };

        let mut tx = repo.get_pool().begin().await.unwrap();
        let result = repo
            .insert_impl_trait("impl test".to_string(), &mut *tx)
            .await
            .unwrap();
        assert_eq!(result.rows_affected(), 1);
        tx.commit().await.unwrap();
    }
}
