use sqlx::prelude::FromRow;
use sqlx_data::{Pool, QueryResult, Result, Transaction, dml, repo};

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

// User model for tests (same as integration_tests)
#[derive(Debug, Clone, PartialEq, FromRow)]
pub struct User {
    pub id: Id,
    pub name: String,
    pub email: String,
    pub age: i64,
    pub birth_year: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, FromRow)]
pub struct UserCast {
    pub id: Id,
    pub name: String,
    pub email: String,
    pub age: u8,
    pub birth_year: Option<u16>,
}

// Test trait for batch insert operations
#[rustfmt::skip]
#[repo]
#[alias(values = "(?, ?, ?, ?, ?)")] // DRY values alias
trait BatchInsertRepo {
    // Batch insert with auto-generated IDs (using auto-detection)
    #[dml("INSERT INTO users (name, email, age, birth_year) VALUES (?, ?, ?, ?) RETURNING id")]
    async fn insert_users_auto_id(&self, rows: Vec<(String, String, u8, Option<u16>)>) -> Result<Vec<u64>>;

    // Batch insert with auto-generated IDs (explicit multi_insert for comparison)
    #[dml("INSERT INTO users (name, email, age, birth_year) VALUES (?, ?, ?, ?) RETURNING id")]
    async fn insert_users_auto_id_explicit(&self, rows: Vec<(String, String, u8, Option<u16>)>) -> Result<Vec<u64>>;

    // Basic batch insert without RETURNING
    #[dml("INSERT INTO users (id, name, email, age, birth_year) VALUES (?, ?, ?, ?, ?)")]
    async fn insert_users_batch(&self, rows: Vec<(i64, String, String, u8, Option<u16>)>) -> Result<QueryResult>;

    // Batch insert with RETURNING ids
    #[dml("INSERT INTO users (id, name, email, age, birth_year) VALUES {{values}} RETURNING id")]
    async fn insert_users_batch_returning_ids(&self, rows: Vec<(i64, String, String, u8, Option<u16>)>) -> Result<Vec<i64>>;

    // Batch insert with RETURNING all fields
    #[dml("INSERT INTO users (id, name, email, age, birth_year) VALUES (?,?,?,?,?) RETURNING id as 'id!: Id', name, email, age, birth_year")]
    async fn insert_users_batch_returning_all(&self,rows: Vec<(i64, String, String, u8, Option<u16>)>) -> Result<Vec<User>>;

    // Batch insert with cast in RETURNING
    #[dml("INSERT INTO users (id, name, email, age, birth_year) VALUES {{values}} RETURNING id as 'id!: Id', name, email, age as 'age: u8', birth_year as 'birth_year: u16'")]
    async fn insert_users_batch_with_cast(&self, rows: Vec<(i64, String, String, u8, Option<u16>)>) -> Result<Vec<UserCast>>;

    // Batch insert minimal fields
    #[dml("INSERT INTO users (name, email, age) VALUES (?, ?, ?) RETURNING id")]
    async fn insert_users_minimal(&self, rows: Vec<(String, String, u8)>) -> Result<Vec<i64>>;

    // Single insert for comparison
    #[dml("INSERT INTO users (id, name, email, age, birth_year) VALUES (?, ?, ?, ?, ?) RETURNING id")]
    async fn insert_single_user(&self, user: Vec<(i64, String, String, u8, Option<u16>)>) -> Result<i64>;

    // INSERT with ON CONFLICT (UPSERT) operations
    #[dml("INSERT INTO users (id, name, email, age, birth_year) VALUES (?, ?, ?, ?, ?) ON CONFLICT(id) DO UPDATE SET name = EXCLUDED.name, email = EXCLUDED.email")]
    async fn upsert_users_batch(&self, rows: Vec<(i64, String, String, u8, Option<u16>)>) -> Result<QueryResult>;

    #[dml("INSERT INTO users (id, name, email, age, birth_year) VALUES (?, ?, ?, ?, ?) ON CONFLICT(id) DO UPDATE SET name = EXCLUDED.name RETURNING id")]
    async fn upsert_users_returning(&self, rows: Vec<(i64, String, String, u8, Option<u16>)>) -> Result<Vec<i64>>;

    #[dml("INSERT INTO users (id, name, email, age, birth_year) VALUES (?, ?, ?, ?, ?) ON CONFLICT(email) DO NOTHING")]
    async fn insert_or_ignore_users(&self, rows: Vec<(i64, String, String, u8, Option<u16>)>) -> Result<QueryResult>;

    // INSERT with complex constraints returning tuple
    #[dml("INSERT INTO users (id, name, email, age, birth_year) VALUES (?, ?, ?, ?, ?) ON CONFLICT(email) DO UPDATE SET name = EXCLUDED.name, age = EXCLUDED.age WHERE users.age < EXCLUDED.age RETURNING id as \"id!: i64\", name")]
    async fn conditional_upsert_users(&self, rows: Vec<(i64, String, String, u8, Option<u16>)>) -> Result<Vec<(i64, String)>>;

    // INSERT with complex constraints returning tuple
    #[dml("INSERT INTO users (id, name, email, age, birth_year) VALUES (?, ?, ?, ?, ?) ON CONFLICT(email) DO UPDATE SET age = EXCLUDED.age WHERE users.age < EXCLUDED.age RETURNING id as \"id!: i64\", name")]
    async fn conditional_upsert_users_ids(&self, rows: Vec<(i64, String, String, u8, Option<u16>)>) -> Result<Vec<(Id, String)>>;

    // INSERT with different column orders
    #[dml("INSERT INTO users (email, age, name, id, birth_year) VALUES (?, ?, ?, ?, ?)")]
    async fn insert_users_reordered(&self, rows: Vec<(String, u8, String, i64, Option<u16>)>) -> Result<QueryResult>;

    // INSERT with calculated values
    #[dml("INSERT INTO users (id, name, email, age, birth_year) VALUES (?, ?, ?, ?, ?)")]
    async fn insert_users_with_functions(&self, rows: Vec<(i64, String, String, u8, Option<u16>)>) -> Result<QueryResult>;

    // Test with slice parameters
    #[dml("INSERT INTO users (id, name, email, age, birth_year) VALUES (?, ?, ?, ?, ?)")]
    async fn insert_users_with_slices(&self, rows: &[(i64, String, String, u8, Option<u16>)]) -> Result<QueryResult>;

    // Test with transaction parameter
    #[dml("INSERT INTO users (id, name, email, age, birth_year) VALUES (?, ?, ?, ?, ?)")]
    async fn insert_users_batch_with_transaction(&self, rows: Vec<(i64, String, String, u8, Option<u16>)>, tx: &mut Transaction<'_>) -> Result<QueryResult>;

    // Count users for transaction testing
    #[dml("SELECT COUNT(*) FROM users")]
    async fn count_users(&self) -> Result<i64>;

    // Test with array parameters
    #[dml("INSERT INTO users (id, name, email, age, birth_year) VALUES (?, ?, ?, ?, ?)")]
    async fn insert_users_with_arrays(&self, rows: [(i64, String, String, u8, Option<u16>); 3]) -> Result<QueryResult>;
    
    //Test with impl IntoIterator
    #[dml("INSERT INTO users (id, name, email, age, birth_year) VALUES (?, ?, ?, ?, ?)")]
    async fn insert_users_with_iterables(&self, rows: impl IntoIterator<Item = (i64, String, String, u8, Option<u16>)>) -> Result<QueryResult>;
    
    #[dml("INSERT INTO users (id, name, email, age, birth_year) VALUES (?, ?, ?, ?, ?)")]
    async fn insert_users_with_iterables_generics<I>(&self, rows: I) -> Result<QueryResult> where I: IntoIterator<Item = (i64, String, String, u8, Option<u16>)>;

    // #[dml(r#"  
    //     INSERT INTO users(id, name, email, age, birth_year)   
    //     SELECT * FROM UNNEST($1::int8[], $2::text[], $3::text[], $4::int2[], $5::int2[])  
    //     "#)]
    // async fn insert_users_with_functions(&self,ids: Vec<i64>,names: Vec<String>,emails: Vec<String>,ages: Vec<u8>,birth_years: Vec<Option<u16>>) -> Result<QueryResult>;
}

// Test implementation
pub struct TestBatchInsertApp {
    pool: Pool,
}

impl BatchInsertRepo for TestBatchInsertApp {
    fn get_pool(&self) -> &Pool {
        &self.pool
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_insert_users_auto_id(pool: Pool) {
        let repo = TestBatchInsertApp { pool };

        let users = vec![
            (
                "Alice".to_string(),
                "alice@example.com".to_string(),
                25u8,
                Some(1998u16),
            ),
            (
                "Bob".to_string(),
                "bob@example.com".to_string(),
                30u8,
                Some(1993u16),
            ),
            (
                "Charlie".to_string(),
                "charlie@example.com".to_string(),
                35u8,
                None,
            ),
        ];

        let result = repo.insert_users_auto_id(users).await.unwrap();

        assert_eq!(result.len(), 3);
        // IDs should be sequential starting from 1 (SQLite auto-increment)
        assert!(result.iter().all(|&id| id > 0));
    }

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_auto_detection_vs_explicit_flag(pool: Pool) {
        let repo = TestBatchInsertApp { pool };

        let users = vec![
            (
                "Alice".to_string(),
                "alice@autotest.com".to_string(),
                25u8,
                Some(1998u16),
            ),
            (
                "Bob".to_string(),
                "bob@autotest.com".to_string(),
                30u8,
                Some(1993u16),
            ),
        ];

        // Test auto-detection (no multi_insert flag)
        let result_auto = repo.insert_users_auto_id(users.clone()).await.unwrap();

        // Clean up for explicit test
        sqlx::query("DELETE FROM users")
            .execute(&repo.pool)
            .await
            .unwrap();

        // Test explicit flag
        let result_explicit = repo.insert_users_auto_id_explicit(users).await.unwrap();

        // Both methods should work identically
        assert_eq!(result_auto.len(), result_explicit.len());
        assert_eq!(result_auto.len(), 2);
    }

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_insert_users_batch(pool: Pool) {
        let repo = TestBatchInsertApp { pool };

        let users = vec![
            (
                1i64,
                "Alice".to_string(),
                "alice@test.com".to_string(),
                25u8,
                Some(1998u16),
            ),
            (
                2i64,
                "Bob".to_string(),
                "bob@test.com".to_string(),
                30u8,
                Some(1993u16),
            ),
        ];

        let result = repo.insert_users_batch(users).await.unwrap();
        assert_eq!(result.rows_affected(), 2);
    }

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_insert_users_batch_returning_ids(pool: Pool) {
        let repo = TestBatchInsertApp { pool };

        let users = vec![
            (
                10i64,
                "Charlie".to_string(),
                "charlie@test.com".to_string(),
                35u8,
                None,
            ),
            (
                20i64,
                "David".to_string(),
                "david@test.com".to_string(),
                40u8,
                Some(1983u16),
            ),
        ];

        let result = repo.insert_users_batch_returning_ids(users).await.unwrap();
        assert_eq!(result, vec![10i64, 20i64]);
    }

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_insert_users_batch_returning_all(pool: Pool) {
        let repo = TestBatchInsertApp { pool };

        let users = vec![
            (
                100i64,
                "Eve".to_string(),
                "eve@test.com".to_string(),
                28u8,
                Some(1995u16),
            ),
            (
                200i64,
                "Frank".to_string(),
                "frank@test.com".to_string(),
                32u8,
                None,
            ),
        ];

        let result = repo.insert_users_batch_returning_all(users).await.unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].id, Id(100));
        assert_eq!(result[0].name, "Eve");
        assert_eq!(result[1].id, Id(200));
        assert_eq!(result[1].name, "Frank");
    }

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_insert_users_batch_with_cast(pool: Pool) {
        let repo = TestBatchInsertApp { pool };

        let users = vec![(
            300i64,
            "Grace".to_string(),
            "grace@test.com".to_string(),
            25u8,
            Some(1998u16),
        )];

        let result = repo.insert_users_batch_with_cast(users).await.unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, Id(300));
        assert_eq!(result[0].age, 25u8);
        assert_eq!(result[0].birth_year, Some(1998u16));
    }

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_insert_users_minimal(pool: Pool) {
        let repo = TestBatchInsertApp { pool };

        let users = vec![
            ("Henry".to_string(), "henry@test.com".to_string(), 45u8),
            ("Ivy".to_string(), "ivy@test.com".to_string(), 33u8),
        ];

        let result = repo.insert_users_minimal(users).await.unwrap();
        assert_eq!(result.len(), 2);
        assert!(result.iter().all(|&id| id > 0));
    }

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_upsert_users_batch(pool: Pool) {
        let repo = TestBatchInsertApp { pool };

        // Insert initial user
        let initial_users = vec![(
            500i64,
            "Jack".to_string(),
            "jack@test.com".to_string(),
            30u8,
            Some(1993u16),
        )];
        repo.insert_users_batch(initial_users).await.unwrap();

        // Upsert with same ID but different data
        let upsert_users = vec![
            (
                500i64,
                "Jack Updated".to_string(),
                "jack@test.com".to_string(),
                31u8,
                Some(1992u16),
            ),
            (
                600i64,
                "Kelly".to_string(),
                "kelly@test.com".to_string(),
                29u8,
                Some(1994u16),
            ),
        ];

        let result = repo.upsert_users_batch(upsert_users).await.unwrap();
        assert_eq!(result.rows_affected(), 2);
    }

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_upsert_users_returning(pool: Pool) {
        let repo = TestBatchInsertApp { pool };

        let users = vec![
            (
                700i64,
                "Liam".to_string(),
                "liam@test.com".to_string(),
                26u8,
                Some(1997u16),
            ),
            (
                800i64,
                "Mia".to_string(),
                "mia@test.com".to_string(),
                24u8,
                Some(1999u16),
            ),
        ];

        let result = repo.upsert_users_returning(users).await.unwrap();
        assert_eq!(result, vec![700i64, 800i64]);
    }

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_insert_or_ignore_users(pool: Pool) {
        let repo = TestBatchInsertApp { pool };

        // Insert initial user
        let initial_users = vec![(
            900i64,
            "Noah".to_string(),
            "noah@test.com".to_string(),
            27u8,
            Some(1996u16),
        )];
        repo.insert_users_batch(initial_users).await.unwrap();

        // Try to insert with same email (should be ignored due to unique constraint)
        let ignore_users = vec![
            (
                901i64,
                "Noah Duplicate".to_string(),
                "noah@test.com".to_string(),
                28u8,
                Some(1995u16),
            ),
            (
                902i64,
                "Olivia".to_string(),
                "olivia@test.com".to_string(),
                25u8,
                Some(1998u16),
            ),
        ];

        let result = repo.insert_or_ignore_users(ignore_users).await.unwrap();
        // Only one row should be affected (Olivia), Noah duplicate should be ignored
        assert_eq!(result.rows_affected(), 1);
    }

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_conditional_upsert_users(pool: Pool) {
        let repo = TestBatchInsertApp { pool };

        // Insert initial user with lower age
        let initial_users = vec![(
            1000i64,
            "Paul".to_string(),
            "paul@test.com".to_string(),
            25u8,
            Some(1998u16),
        )];
        repo.insert_users_batch(initial_users).await.unwrap();

        // Try conditional upsert with higher age (should update)
        let upsert_users = vec![
            (
                1001i64,
                "Paul Updated".to_string(),
                "paul@test.com".to_string(),
                30u8,
                Some(1993u16),
            ),
            (
                1002i64,
                "Quinn".to_string(),
                "quinn@test.com".to_string(),
                22u8,
                Some(2001u16),
            ),
        ];

        let result = repo.conditional_upsert_users(upsert_users).await.unwrap();
        assert_eq!(result.len(), 2);
        assert!(
            result
                .iter()
                .any(|(id, name)| *id == 1000 && name == "Paul Updated")
        );
    }

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_insert_users_batch_with_transaction(pool: Pool) {
        let repo = TestBatchInsertApp { pool: pool.clone() };

        // Count inicial antes da transação
        let count_initial = repo.count_users().await.unwrap();

        // Start a transaction
        let mut tx = pool.begin().await.unwrap();

        // Insert first batch in transaction
        let users_batch1 = vec![
            (
                100i64,
                "Alice Transaction".to_string(),
                "alice_tx@test.com".to_string(),
                28u8,
                Some(1995u16),
            ),
            (
                101i64,
                "Bob Transaction".to_string(),
                "bob_tx@test.com".to_string(),
                32u8,
                Some(1991u16),
            ),
        ];

        let result1 = repo
            .insert_users_batch_with_transaction(users_batch1, &mut tx)
            .await
            .unwrap();
        assert_eq!(result1.rows_affected(), 2);

        // Insert second batch in same transaction
        let users_batch2 = vec![(
            102i64,
            "Charlie Transaction".to_string(),
            "charlie_tx@test.com".to_string(),
            25u8,
            Some(1998u16),
        )];

        let result2 = repo
            .insert_users_batch_with_transaction(users_batch2, &mut tx)
            .await
            .unwrap();
        assert_eq!(result2.rows_affected(), 1);

        // Commit transaction
        tx.commit().await.unwrap();

        // Count final após commit da transação
        let count_final = repo.count_users().await.unwrap();

        // Verificar que foram inseridos 3 usuários
        assert_eq!(count_final, count_initial + 3);
    }

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_insert_users_reordered(pool: Pool) {
        let repo = TestBatchInsertApp { pool };

        // Note: column order is (email, age, name, id, birth_year)
        let users = vec![
            (
                "rachel@test.com".to_string(),
                29u8,
                "Rachel".to_string(),
                1100i64,
                Some(1994u16),
            ),
            (
                "sam@test.com".to_string(),
                31u8,
                "Sam".to_string(),
                1200i64,
                Some(1992u16),
            ),
        ];

        let result = repo.insert_users_reordered(users).await.unwrap();
        assert_eq!(result.rows_affected(), 2);
    }

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_insert_users_with_functions(pool: Pool) {
        let repo = TestBatchInsertApp { pool };

        let users = vec![(
            1300i64,
            "tina".to_string(),
            "TINA@TEST.COM".to_string(),
            25u8,
            Some(1998u16),
        )];

        let result = repo.insert_users_with_functions(users).await.unwrap();
        assert_eq!(result.rows_affected(), 1);

        // Verify the values were inserted as provided (no SQL functions applied)
        let user = sqlx::query_as::<_, (i64, String, String, i64)>(
            "SELECT id, name, email, age FROM users WHERE id = 1300",
        )
        .fetch_one(&repo.pool)
        .await
        .unwrap();

        assert_eq!(user.1, "tina"); // name as provided
        assert_eq!(user.2, "TINA@TEST.COM"); // email as provided
        assert_eq!(user.3, 25); // age as provided
    }

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_insert_users_with_slices(pool: Pool) {
        let repo = TestBatchInsertApp { pool };

        let users = vec![
            (
                1400i64,
                "Uma".to_string(),
                "uma@test.com".to_string(),
                27u8,
                Some(1996u16),
            ),
            (
                1500i64,
                "Victor".to_string(),
                "victor@test.com".to_string(),
                33u8,
                None,
            ),
        ];

        let result = repo.insert_users_with_slices(&users).await.unwrap();
        assert_eq!(result.rows_affected(), 2);
    }

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_insert_users_with_arrays(pool: Pool) {
        let repo = TestBatchInsertApp { pool };

        let users = [
            (
                1600i64,
                "Wendy".to_string(),
                "wendy@test.com".to_string(),
                28u8,
                Some(1995u16),
            ),
            (
                1700i64,
                "Xavier".to_string(),
                "xavier@test.com".to_string(),
                35u8,
                Some(1988u16),
            ),
            (
                1800i64,
                "Yara".to_string(),
                "yara@test.com".to_string(),
                26u8,
                None,
            ),
        ];

        let result = repo.insert_users_with_arrays(users).await.unwrap();
        assert_eq!(result.rows_affected(), 3);
    }

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_insert_users_with_iterables(pool: Pool) {
        let repo = TestBatchInsertApp { pool };

        let users = vec![
            (
                1900i64,
                "Zoe".to_string(),
                "zoe@test.com".to_string(),
                24u8,
                Some(1999u16),
            ),
            (
                2000i64,
                "Adam".to_string(),
                "adam@test.com".to_string(),
                32u8,
                Some(1991u16),
            ),
        ];

        let result = repo.insert_users_with_iterables(users).await.unwrap();
        assert_eq!(result.rows_affected(), 2);
    }

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_insert_users_with_iterables_generics(pool: Pool) {
        let repo = TestBatchInsertApp { pool };

        let users = vec![
            (
                2100i64,
                "Bella".to_string(),
                "bella@test.com".to_string(),
                23u8,
                Some(2000u16),
            ),
            (
                2200i64,
                "Carlos".to_string(),
                "carlos@test.com".to_string(),
                29u8,
                Some(1994u16),
            ),
        ];

        let result = repo
            .insert_users_with_iterables_generics(users)
            .await
            .unwrap();
        assert_eq!(result.rows_affected(), 2);
    }
}
