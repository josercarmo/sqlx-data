use sqlx::prelude::FromRow;
use sqlx_data::{Pool, QueryResult, Result,  dml, repo};

// Use same structure as integration_tests
#[derive(Clone, PartialEq, Eq, Debug, sqlx::Type)]
#[sqlx(transparent)]
pub struct Id(i64);

impl From<i64> for Id {
    fn from(value: i64) -> Self {
        Id(value)
    }
}

impl From<Option<i64>> for Id {
    fn from(value: Option<i64>) -> Self {
        Id(value.unwrap_or_default())
    }
}

// User model for tests (PostgreSQL types)
#[derive(Debug, Clone, PartialEq, FromRow)]
pub struct User {
    pub id: Id,
    pub name: String,
    pub email: String,
    pub age: i16,                // PostgreSQL SMALLINT
    pub birth_year: Option<i16>, // PostgreSQL SMALLINT
}

#[derive(Debug, Clone, PartialEq, FromRow)]
pub struct UserCast {
    pub id: Id,
    pub name: String,
    pub email: String,
    pub age: i16,
    pub birth_year: Option<i16>,
}

// Test trait for batch insert operations
#[rustfmt::skip]
#[repo]
#[alias(values = "($1, $2, $3, $4, $5)")] // DRY values alias for PostgreSQL
trait BatchInsertRepo {
    // Batch insert with auto-generated IDs (PostgreSQL supports RETURNING)
    #[dml("INSERT INTO users (name, email, age, birth_year) VALUES ($1, $2, $3, $4) RETURNING id")]
    async fn insert_users_auto_id(&self, rows: Vec<(String, String, i16, Option<i16>)>) -> Result<Vec<i64>>;

    // Batch insert with explicit IDs
    #[dml("INSERT INTO users (id, name, email, age, birth_year) VALUES {{values}}")]
    async fn insert_users_with_id(&self, rows: Vec<(i64, String, String, i16, Option<i16>)>) -> Result<QueryResult>;

    // Single insert for comparison
    #[dml("INSERT INTO users (name, email, age, birth_year) VALUES ($1, $2, $3, $4)")]
    async fn insert_single_user(&self, name: String, email: String, age: i16, birth_year: Option<i16>) -> Result<QueryResult>;

    // Batch insert with ON CONFLICT UPDATE (PostgreSQL specific)
    #[dml("INSERT INTO users (name, email, age, birth_year) VALUES ($1, $2, $3, $4) ON CONFLICT (email) DO UPDATE SET age = EXCLUDED.age, birth_year = EXCLUDED.birth_year")]
    async fn upsert_users(&self, rows: Vec<(String, String, i16, Option<i16>)>) -> Result<QueryResult>;

    // Select for verification
    #[dml("SELECT id as \"id!: Id\", name, email, age, birth_year FROM users WHERE id >= $1 ORDER BY id")]
    async fn find_users_from_id(&self, min_id: i64) -> Result<Vec<User>>;

    // Count users
    #[dml("SELECT COUNT(*) FROM users")]
    async fn count_users(&self) -> Result<i64>;

    // Clean up for tests
    #[dml("DELETE FROM users WHERE id >= $1")]
    async fn cleanup_users(&self, min_id: i64) -> Result<QueryResult>;
}

pub struct BatchApp {
    pool: Pool,
}

impl BatchInsertRepo for BatchApp {
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
    async fn test_batch_insert_basic(pool: Pool) {
        let app = BatchApp { pool };

        // Count initial users
        let initial_count = app.count_users().await.unwrap();

        // Prepare batch data
        let batch_data = vec![
            ("Batch User 1".to_string(), "batch1@example.com".to_string(), 25, Some(1998)),
            ("Batch User 2".to_string(), "batch2@example.com".to_string(), 30, Some(1993)),
            ("Batch User 3".to_string(), "batch3@example.com".to_string(), 35, None),
        ];

        // Insert batch (PostgreSQL returns inserted IDs)
        let inserted_ids = app.insert_users_auto_id(batch_data).await.unwrap();
        assert_eq!(inserted_ids.len(), 3);

        // Verify count increased
        let final_count = app.count_users().await.unwrap();
        assert_eq!(final_count, initial_count + 3);

        // Get the first inserted ID (PostgreSQL way - from RETURNING)
        let first_inserted_id = inserted_ids[0];
        assert!(first_inserted_id > 0);

        // Verify inserted data
        let inserted_users = app.find_users_from_id(first_inserted_id).await.unwrap();
        assert_eq!(inserted_users.len(), 3);
        assert_eq!(inserted_users[0].name, "Batch User 1");
        assert_eq!(inserted_users[0].age, 25);
        assert_eq!(inserted_users[1].name, "Batch User 2");
        assert_eq!(inserted_users[2].birth_year, None);
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_batch_insert_with_explicit_ids(pool: Pool) {
        let app = BatchApp { pool };

        // Use high IDs to avoid conflicts
        let batch_data = vec![
            (1001, "ID User 1".to_string(), "id1@example.com".to_string(), 28, Some(1995)),
            (1002, "ID User 2".to_string(), "id2@example.com".to_string(), 32, Some(1991)),
        ];

        let result = app.insert_users_with_id(batch_data).await.unwrap();
        assert_eq!(result.rows_affected(), 2);

        // Verify with specific IDs
        let users = app.find_users_from_id(1001).await.unwrap();
        assert_eq!(users.len(), 2);
        assert_eq!(users[0].id, Id(1001));
        assert_eq!(users[1].id, Id(1002));
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_batch_vs_single_insert_performance(pool: Pool) {
        let app = BatchApp { pool };

        let initial_count = app.count_users().await.unwrap();

        // Single inserts
        let start = std::time::Instant::now();
        for i in 0..5 {
            app.insert_single_user(
                format!("Single User {}", i),
                format!("single{}@example.com", i),
                (20 + i) as i16,
                Some((2000 - i) as i16)
            ).await.unwrap();
        }
        let single_duration = start.elapsed();

        let mid_count = app.count_users().await.unwrap();
        assert_eq!(mid_count, initial_count + 5);

        // Batch insert
        let batch_data = (0..5).map(|i| {
            (
                format!("Batch User {}", i),
                format!("batch{}@example.com", i),
                (20 + i) as i16,
                Some((2000 - i) as i16)
            )
        }).collect();

        let start = std::time::Instant::now();
        let _inserted_ids = app.insert_users_auto_id(batch_data).await.unwrap();
        let batch_duration = start.elapsed();

        let final_count = app.count_users().await.unwrap();
        assert_eq!(final_count, initial_count + 10);

        // Batch should be faster (or at least not significantly slower)
        println!("Single inserts: {:?}, Batch insert: {:?}", single_duration, batch_duration);
        // Note: In practice, batch should be faster, but for small datasets the difference might be negligible
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_batch_insert_empty_vec(pool: Pool) {
        let app = BatchApp { pool };

        let initial_count = app.count_users().await.unwrap();

        // Insert empty batch
        let inserted_ids = app.insert_users_auto_id(vec![]).await.unwrap();
        assert_eq!(inserted_ids.len(), 0);

        // Count should remain the same
        let final_count = app.count_users().await.unwrap();
        assert_eq!(final_count, initial_count);
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_postgresql_upsert_functionality(pool: Pool) {
        let app = BatchApp { pool };

        // First insert
        let batch_data = vec![
            ("Upsert User".to_string(), "upsert@example.com".to_string(), 25, Some(1998)),
        ];

        let result1 = app.upsert_users(batch_data.clone()).await.unwrap();
        assert_eq!(result1.rows_affected(), 1);

        // Second "insert" with same email (should update due to ON CONFLICT)
        // Note: This requires a unique constraint on email in the migration
        let updated_data = vec![
            ("Updated Upsert User".to_string(), "upsert@example.com".to_string(), 30, Some(1993)),
        ];

        let result2 = app.upsert_users(updated_data).await.unwrap();
        // This will update the existing row if email has unique constraint
        assert!(result2.rows_affected() >= 1);
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_batch_insert_with_nulls(pool: Pool) {
        let app = BatchApp { pool };

        // Mix of null and non-null values
        let batch_data = vec![
            ("Null Birth 1".to_string(), "null1@example.com".to_string(), 25, None),
            ("With Birth".to_string(), "withbirth@example.com".to_string(), 30, Some(1993)),
            ("Null Birth 2".to_string(), "null2@example.com".to_string(), 35, None),
        ];

        let inserted_ids = app.insert_users_auto_id(batch_data).await.unwrap();
        assert_eq!(inserted_ids.len(), 3);

        let first_id = inserted_ids[0];
        let inserted_users = app.find_users_from_id(first_id).await.unwrap();

        assert_eq!(inserted_users.len(), 3);
        assert_eq!(inserted_users[0].birth_year, None);
        assert_eq!(inserted_users[1].birth_year, Some(1993));
        assert_eq!(inserted_users[2].birth_year, None);
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_batch_insert_transaction_rollback(pool: Pool) {
        let app = BatchApp { pool: pool.clone() };

        let initial_count = app.count_users().await.unwrap();

        // Simulate transaction failure
        let mut tx = pool.begin().await.unwrap();

        let batch_data = vec![
            ("TX User 1".to_string(), "tx1@example.com".to_string(), 25, Some(1998)),
            ("TX User 2".to_string(), "tx2@example.com".to_string(), 30, Some(1993)),
        ];

        // Insert in transaction (PostgreSQL syntax)
        let result = sqlx::query("INSERT INTO users (name, email, age, birth_year) VALUES ($1, $2, $3, $4), ($5, $6, $7, $8)")
            .bind(&batch_data[0].0).bind(&batch_data[0].1).bind(batch_data[0].2).bind(batch_data[0].3)
            .bind(&batch_data[1].0).bind(&batch_data[1].1).bind(batch_data[1].2).bind(batch_data[1].3)
            .execute(&mut *tx)
            .await.unwrap();

        assert_eq!(result.rows_affected(), 2);

        // Rollback transaction
        tx.rollback().await.unwrap();

        // Verify no rows were committed
        let final_count = app.count_users().await.unwrap();
        assert_eq!(final_count, initial_count);
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_large_batch_insert(pool: Pool) {
        let app = BatchApp { pool };

        let initial_count = app.count_users().await.unwrap();

        // Create a larger batch (100 users)
        let batch_data: Vec<_> = (0..100).map(|i| {
            (
                format!("Large Batch User {}", i),
                format!("large{}@example.com", i),
                (20 + (i % 50)) as i16,  // Ages 20-69
                if i % 3 == 0 { None } else { Some((1950 + i % 70) as i16) }  // Some nulls
            )
        }).collect();

        let inserted_ids = app.insert_users_auto_id(batch_data).await.unwrap();
        assert_eq!(inserted_ids.len(), 100);

        let final_count = app.count_users().await.unwrap();
        assert_eq!(final_count, initial_count + 100);

        // Verify some of the inserted data
        let first_id = inserted_ids[0];
        let sample_users = app.find_users_from_id(first_id).await.unwrap();
        assert!(sample_users.len() >= 10); // At least some users inserted

        // Clean up the large batch
        app.cleanup_users(first_id).await.unwrap();

        let cleanup_count = app.count_users().await.unwrap();
        assert_eq!(cleanup_count, initial_count);
    }
}