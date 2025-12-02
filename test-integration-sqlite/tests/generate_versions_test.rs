use sqlx::sqlite::SqliteQueryResult;
use sqlx_data::{Pool, QueryResult, Result, generate_versions, dml, repo};

#[derive(Clone, PartialEq, Eq, Debug, sqlx::Type)]
#[sqlx(transparent)]
pub struct UserId(i64);

impl From<i64> for UserId {
    fn from(value: i64) -> Self {
        UserId(value)
    }
}

// User model for tests
#[derive(Debug, Clone, PartialEq, sqlx::FromRow)]
pub struct User {
    pub id: UserId,
    pub name: String,
    pub email: String,
    pub age: u8,
}

// Test trait with generate_versions macro
#[repo]
trait UserVariantRepo {
    // Regular method without variants
    #[dml("SELECT COUNT(*) FROM users")]
    async fn count_users(&self) -> Result<i64>;

    // Method with pool and transaction variants
    #[generate_versions(pool, tx)]
    #[dml("DELETE FROM users WHERE id = ?")]
    async fn delete_user(&self, id: i64) -> Result<QueryResult>;

    // Method with all variants and instrument macro
    #[generate_versions(pool, tx, conn, exec)]
    #[dml("UPDATE users SET name = ? WHERE id = ?")]
    #[instrument(skip(self))]
    async fn update_user_name(&self, name: String, id: i64) -> Result<QueryResult>;

    // Method with complex query and pool variant
    #[generate_versions(pool)]
    #[dml("SELECT id as 'id!: UserId', name, email, age as 'age: u8' FROM users WHERE age >= ?")]
    async fn find_adults(&self, min_age: u8) -> Result<Vec<User>>;

    // Method with tuple return, multiple variants, and instrument macro
    #[generate_versions(tx, conn)]
    #[dml("SELECT name, age as 'age: u8' FROM users WHERE id = ?")]
    #[instrument(skip(self))]
    async fn get_user_info(&self, id: i64) -> Result<(String, u8)>;

    // Insert method with executor variant
    #[generate_versions(exec)]
    #[dml("INSERT INTO users (name, email, age) VALUES (?, ?, ?)")]
    async fn create_user(&self, name: String, email: String, age: u8) -> Result<SqliteQueryResult>;
}

// Test implementation
pub struct TestVariantApp {
    pool: Pool,
}

impl UserVariantRepo for TestVariantApp {
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
    async fn test_original_methods_work(pool: Pool) {
        let repo = TestVariantApp { pool };

        // Test original method without variants
        let count = repo.count_users().await.unwrap();
        assert_eq!(count, 20); // From fixtures

        // Test original methods that have variants still work
        let adults = repo.find_adults(25).await.unwrap();
        assert!(adults.len() > 0);
        assert!(adults.iter().all(|u| u.age >= 25));
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_pool_variants(pool: Pool) {
        let repo = TestVariantApp { pool: pool.clone() };

        // Test delete_user_with_pool variant
        // First create a test user to delete
        let insert_result = repo.create_user(
            "TestDelete".to_string(),
            "testdelete@example.com".to_string(),
            30,
        ).await.unwrap();
        let new_id = insert_result.last_insert_rowid();

        // Now test the pool variant
        let result = repo.delete_user_with_pool(&pool, new_id).await.unwrap();
        assert_eq!(result.rows_affected(), 1);

        // Test find_adults_with_pool variant
        let adults = repo.find_adults_with_pool(&pool, 25).await.unwrap();
        assert!(adults.len() > 0);
        assert!(adults.iter().all(|u| u.age >= 25));
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_transaction_variants(pool: Pool) {
        let repo = TestVariantApp { pool: pool.clone() };
        let mut tx = pool.begin().await.unwrap();

        // Test update_user_name_with_tx variant
        let result = repo.update_user_name_with_tx(
            &mut tx,
            "UpdatedAlice".to_string(),
            1,
        ).await.unwrap();
        assert_eq!(result.rows_affected(), 1);

        // Test get_user_info_with_tx variant
        let (name, age) = repo.get_user_info_with_tx(&mut tx, 1).await.unwrap();
        assert_eq!(name, "UpdatedAlice");
        assert_eq!(age, 30);

        // Test delete_user_with_tx variant
        let result = repo.delete_user_with_tx(&mut tx, 2).await.unwrap();
        assert_eq!(result.rows_affected(), 1);

        tx.commit().await.unwrap();
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_connection_variants(pool: Pool) {
        let repo = TestVariantApp { pool: pool.clone() };
        let mut conn = pool.acquire().await.unwrap();

        // Test update_user_name_with_conn variant
        let result = repo.update_user_name_with_conn(
            &mut conn,
            "ConnUpdatedBob".to_string(),
            2,
        ).await.unwrap();
        assert_eq!(result.rows_affected(), 1);

        // Test get_user_info_with_conn variant
        let (name, age) = repo.get_user_info_with_conn(&mut conn, 2).await.unwrap();
        assert_eq!(name, "ConnUpdatedBob");
        assert_eq!(age, 25);
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_executor_variants(pool: Pool) {
        let repo = TestVariantApp { pool: pool.clone() };

        // Test update_user_name_with_executor variant using pool as executor
        let result = repo.update_user_name_with_executor(
            &pool,
            "ExecUpdatedCharlie".to_string(),
            3,
        ).await.unwrap();
        assert_eq!(result.rows_affected(), 1);

        // Test create_user_with_executor variant using pool as executor
        let result = repo.create_user_with_executor(
            &pool,
            "ExecCreated".to_string(),
            "execcreated@example.com".to_string(),
            28,
        ).await.unwrap();
        assert!(result.last_insert_rowid() > 0);

        // Test with transaction as executor
        let mut tx = pool.begin().await.unwrap();
        let result = repo.update_user_name_with_executor(
            &mut *tx,
            "TxExecUpdated".to_string(),
            4,
        ).await.unwrap();
        assert_eq!(result.rows_affected(), 1);
        tx.commit().await.unwrap();
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_attribute_copying_in_variants(pool: Pool) {
        let repo = TestVariantApp { pool: pool.clone() };

        // Test that generated variants work correctly
        // Methods with #[instrument] should have it copied to all variants
        let adults_original = repo.find_adults(30).await.unwrap();
        let adults_with_pool = repo.find_adults_with_pool(&pool, 30).await.unwrap();

        // Both should return the same results
        assert_eq!(adults_original.len(), adults_with_pool.len());
        for (original, with_pool) in adults_original.iter().zip(adults_with_pool.iter()) {
            assert_eq!(original.id.0, with_pool.id.0);
            assert_eq!(original.name, with_pool.name);
            assert_eq!(original.email, with_pool.email);
            assert_eq!(original.age, with_pool.age);
        }
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_all_variant_types_exist(pool: Pool) {
        let repo = TestVariantApp { pool: pool.clone() };

        // Test that all generated methods actually exist and can be called
        // This is mainly a compilation test

        // Pool variants (from delete_user and find_adults)
        let _ = repo.find_adults_with_pool(&pool, 18).await.unwrap();

        // Transaction variants (from delete_user, update_user_name, get_user_info)
        let mut tx = pool.begin().await.unwrap();
        let _ = repo.get_user_info_with_tx(&mut tx, 1).await.unwrap();
        tx.rollback().await.unwrap();

        // Connection variants (from update_user_name, get_user_info)
        let mut conn = pool.acquire().await.unwrap();
        let _ = repo.get_user_info_with_conn(&mut conn, 1).await.unwrap();

        // Executor variants (from update_user_name, create_user)
        let _ = repo.create_user_with_executor(
            &pool,
            "ExistenceTest".to_string(),
            "exist@test.com".to_string(),
            25,
        ).await.unwrap();
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_method_parameter_order(pool: Pool) {
        let repo = TestVariantApp { pool: pool.clone() };

        // Test that parameters are in correct order:
        // &self, variant_param, original_params...

        // Original: update_user_name(&self, name: String, id: i64)
        // Generated: update_user_name_with_pool(&self, pool: &Pool, name: String, id: i64)
        let result = repo.update_user_name_with_pool(
            &pool,           // variant parameter (pool)
            "OrderTest".to_string(),  // original parameter 1 (name)
            5,               // original parameter 2 (id)
        ).await.unwrap();
        assert_eq!(result.rows_affected(), 1);

        // Test transaction variant with same parameter order
        let mut tx = pool.begin().await.unwrap();
        let result = repo.update_user_name_with_tx(
            &mut tx,         // variant parameter (transaction)
            "TxOrderTest".to_string(), // original parameter 1 (name)
            6,               // original parameter 2 (id)
        ).await.unwrap();
        assert_eq!(result.rows_affected(), 1);
        tx.commit().await.unwrap();
    }
}
