use sqlx::types::Uuid;
use sqlx_data::{Pool, QueryResult, Result, dml, repo};

// PostgreSQL insert operations repository with strong typing
#[repo]
trait InsertRepo {
    // Basic insert operations
    #[dml("INSERT INTO users (name, email, age) VALUES ($1, $2, $3)")]
    async fn insert_user(&self, name: String, email: String, age: i16) -> Result<QueryResult>;

    // PostgreSQL-specific: RETURNING clause
    #[dml("INSERT INTO users (name, email, age, birth_year) VALUES ($1, $2, $3, $4) RETURNING id")]
    async fn insert_user_returning_id(
        &self,
        name: String,
        email: String,
        age: i16,
        birth_year: Option<i16>,
    ) -> Result<i64>;

    #[dml("INSERT INTO users (name, email, age) VALUES ($1, $2, $3) RETURNING id, name, email")]
    async fn insert_user_returning_multiple(
        &self,
        name: String,
        email: String,
        age: i16,
    ) -> Result<(i64, String, String)>;

    // Batch inserts using VALUES
    #[dml("INSERT INTO users (name, email, age) VALUES ($1, $2, $3), ($4, $5, $6)")]
    async fn insert_two_users(
        &self,
        name1: String,
        email1: String,
        age1: i16,
        name2: String,
        email2: String,
        age2: i16,
    ) -> Result<QueryResult>;

    // Insert with DEFAULT values
    #[dml("INSERT INTO files (name) VALUES ($1)")]
    async fn insert_file_with_defaults(&self, name: String) -> Result<QueryResult>;

    // Insert with PostgreSQL-specific functions
    #[dml("INSERT INTO files (name, content, size) VALUES ($1, $2, LENGTH($2::bytea))")]
    async fn insert_file_with_calculated_size(
        &self,
        name: String,
        content: Vec<u8>,
    ) -> Result<QueryResult>;

    // Conditional insert (PostgreSQL ON CONFLICT)
    #[dml(
        "INSERT INTO users (name, email, age) VALUES ($1, $2, $3) ON CONFLICT (email) DO NOTHING"
    )]
    async fn insert_user_ignore_duplicate(
        &self,
        name: String,
        email: String,
        age: i16,
    ) -> Result<QueryResult>;

    #[dml(
        "INSERT INTO users (name, email, age) VALUES ($1, $2, $3) ON CONFLICT (email) DO UPDATE SET name = EXCLUDED.name, age = EXCLUDED.age"
    )]
    async fn insert_or_update_user(
        &self,
        name: String,
        email: String,
        age: i16,
    ) -> Result<QueryResult>;

    // Insert with type casting
    #[dml(
        "INSERT INTO users (name, email, age, birth_year) VALUES ($1, $2, $3::SMALLINT, $4::SMALLINT)"
    )]
    async fn insert_user_with_casting(
        &self,
        name: String,
        email: String,
        age: i16,
        birth_year: Option<i16>,
    ) -> Result<QueryResult>;

    // Insert with PostgreSQL-specific types
    #[dml("INSERT INTO files (name, created_at) VALUES ($1, NOW())")]
    async fn insert_file_with_timestamp(&self, name: String) -> Result<QueryResult>;

    // Insert with JSON/JSONB
    #[dml("INSERT INTO json_users (name, profile_json) VALUES ($1, $2)")]
    async fn insert_json_user(
        &self,
        name: String,
        profile: sqlx::types::JsonValue,
    ) -> Result<QueryResult>;

    #[dml(
        "INSERT INTO json_users (name, profile_json, preferences) VALUES ($1, $2, $3) RETURNING id"
    )]
    async fn insert_json_user_returning_id(
        &self,
        name: String,
        profile: sqlx::types::JsonValue,
        preferences: Option<sqlx::types::JsonValue>,
    ) -> Result<i64>;

    // Insert with array types
    #[dml("INSERT INTO test_arrays (name, numbers, texts) VALUES ($1, $2, $3)")]
    async fn insert_with_arrays(
        &self,
        name: String,
        numbers: Vec<i32>,
        texts: Vec<String>,
    ) -> Result<QueryResult>;

    // Insert with subquery
    #[dml(
        "INSERT INTO user_stats (user_id, avg_age) SELECT $1, AVG(age) FROM users WHERE id <> $1"
    )]
    async fn insert_user_stats(&self, user_id: i64) -> Result<QueryResult>;

    // Insert with CASE expression
    #[dml(
        "INSERT INTO user_categories (user_id, category) VALUES ($1, CASE WHEN $2 < 25 THEN 'young' WHEN $2 < 35 THEN 'middle' ELSE 'senior' END)"
    )]
    async fn insert_user_category(&self, user_id: i64, age: i32) -> Result<QueryResult>;

    // Insert multiple rows using RETURNING
    #[dml("INSERT INTO users (name, email, age) VALUES ($1, $2, $3), ($4, $5, $6) RETURNING id")]
    async fn insert_multiple_users_returning_ids(
        &self,
        name1: String,
        email1: String,
        age1: i16,
        name2: String,
        email2: String,
        age2: i16,
    ) -> Result<Vec<i64>>;

    // PostgreSQL-specific: Insert with UUID
    #[dml(
        "INSERT INTO uuid_records (uuid_id, name) VALUES (gen_random_uuid(), $1) RETURNING uuid_id"
    )]
    async fn insert_with_uuid(&self, name: String) -> Result<Uuid>;

    // Insert with complex expressions
    #[dml("INSERT INTO computed_values (input_value, computed) VALUES ($1, $1::int * $1::int + $2)")]
    async fn insert_computed_value(&self, input_value: i32, offset: i32) -> Result<QueryResult>;

    // Insert with PostgreSQL interval
    #[dml(
        "INSERT INTO time_records (name, event_date, event_time) VALUES ($1, CURRENT_DATE, CURRENT_TIME)"
    )]
    async fn insert_current_time_record(&self, name: String) -> Result<QueryResult>;

    // Cleanup operations for testing
    #[dml("DELETE FROM users WHERE email LIKE 'test%'")]
    async fn cleanup_test_users(&self) -> Result<QueryResult>;

    #[dml("DELETE FROM files WHERE name LIKE 'test%'")]
    async fn cleanup_test_files(&self) -> Result<QueryResult>;

    #[dml("DELETE FROM json_users WHERE name LIKE 'test%'")]
    async fn cleanup_test_json_users(&self) -> Result<QueryResult>;
}

pub struct InsertApp {
    pool: Pool,
}

impl InsertRepo for InsertApp {
    fn get_pool(&self) -> &Pool {
        &self.pool
    }
}

#[allow(clippy::unwrap_used)]
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_basic_insert(pool: Pool) {
        let app = InsertApp { pool };

        let result = app
            .insert_user("Test User".to_string(), "test@example.com".to_string(), 25)
            .await
            .unwrap();

        assert_eq!(result.rows_affected(), 1);

        // Cleanup
        app.cleanup_test_users().await.unwrap();
    }

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_insert_returning_id(pool: Pool) {
        let app = InsertApp { pool };

        let user_id = app
            .insert_user_returning_id(
                "Test User".to_string(),
                "test@example.com".to_string(),
                30,
                Some(1994),
            )
            .await
            .unwrap();

        assert!(user_id > 0);

        // Cleanup
        app.cleanup_test_users().await.unwrap();
    }

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_insert_returning_multiple(pool: Pool) {
        let app = InsertApp { pool };

        let (id, name, email) = app
            .insert_user_returning_multiple(
                "Test User".to_string(),
                "test@example.com".to_string(),
                28,
            )
            .await
            .unwrap();

        assert!(id > 0);
        assert_eq!(name, "Test User");
        assert_eq!(email, "test@example.com");

        // Cleanup
        app.cleanup_test_users().await.unwrap();
    }

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_batch_insert(pool: Pool) {
        let app = InsertApp { pool };

        let result = app
            .insert_two_users(
                "User One".to_string(),
                "test1@example.com".to_string(),
                25,
                "User Two".to_string(),
                "test2@example.com".to_string(),
                30,
            )
            .await
            .unwrap();

        assert_eq!(result.rows_affected(), 2);

        // Cleanup
        app.cleanup_test_users().await.unwrap();
    }

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_file_inserts(pool: Pool) {
        let app = InsertApp { pool };

        let result = app
            .insert_file_with_defaults("test_file.txt".to_string())
            .await
            .unwrap();
        assert_eq!(result.rows_affected(), 1);

        let content = b"Hello, PostgreSQL!";
        let result = app
            .insert_file_with_calculated_size("test_calculated.txt".to_string(), content.to_vec())
            .await
            .unwrap();
        assert_eq!(result.rows_affected(), 1);

        let result = app
            .insert_file_with_timestamp("test_timestamp.txt".to_string())
            .await
            .unwrap();
        assert_eq!(result.rows_affected(), 1);

        // Cleanup
        app.cleanup_test_files().await.unwrap();
    }

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_conflict_handling(pool: Pool) {
        let app = InsertApp { pool };

        // First insert should succeed
        let result1 = app
            .insert_user("Test User".to_string(), "test@example.com".to_string(), 25)
            .await
            .unwrap();
        assert_eq!(result1.rows_affected(), 1);

        // Second insert with same email should be ignored
        let result2 = app
            .insert_user_ignore_duplicate(
                "Different User".to_string(),
                "test@example.com".to_string(),
                30,
            )
            .await
            .unwrap();
        assert_eq!(result2.rows_affected(), 0); // ON CONFLICT DO NOTHING

        // Third insert should update existing record
        let result3 = app
            .insert_or_update_user(
                "Updated User".to_string(),
                "test@example.com".to_string(),
                35,
            )
            .await
            .unwrap();
        assert_eq!(result3.rows_affected(), 1); // ON CONFLICT DO UPDATE

        // Cleanup
        app.cleanup_test_users().await.unwrap();
    }

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_type_casting_insert(pool: Pool) {
        let app = InsertApp { pool };

        let result = app
            .insert_user_with_casting(
                "Cast Test".to_string(),
                "test@example.com".to_string(),
                25i16,         // i32 to be cast to SMALLINT
                Some(1999i16), // i32 to be cast to SMALLINT
            )
            .await
            .unwrap();

        assert_eq!(result.rows_affected(), 1);

        // Cleanup
        app.cleanup_test_users().await.unwrap();
    }

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_json_inserts(pool: Pool) {
        let app = InsertApp { pool };

        let profile = json!({
            "age": 28,
            "city": "Test City",
            "skills": ["Rust", "PostgreSQL"]
        });

        let result = app
            .insert_json_user("JSON Test User".to_string(), profile.clone())
            .await
            .unwrap();
        assert_eq!(result.rows_affected(), 1);

        let preferences = Some(json!({
            "theme": "dark",
            "notifications": true
        }));

        let user_id = app
            .insert_json_user_returning_id("JSON Test User 2".to_string(), profile, preferences)
            .await
            .unwrap();
        assert!(user_id > 0);

        // Cleanup
        app.cleanup_test_json_users().await.unwrap();
    }

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_multiple_inserts_returning_ids(pool: Pool) {
        let app = InsertApp { pool };

        let ids = app
            .insert_multiple_users_returning_ids(
                "Multi User 1".to_string(),
                "test1@example.com".to_string(),
                25,
                "Multi User 2".to_string(),
                "test2@example.com".to_string(),
                30,
            )
            .await
            .unwrap();

        assert_eq!(ids.len(), 2);
        assert!(ids[0] > 0);
        assert!(ids[1] > 0);
        assert_ne!(ids[0], ids[1]); // Should be different IDs

        // Cleanup
        app.cleanup_test_users().await.unwrap();
    }

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_uuid_insert(pool: Pool) {
        let app = InsertApp { pool };

        let uuid_id = app.insert_with_uuid("UUID Test".to_string()).await.unwrap();

        // UUID should be valid
        assert_eq!(uuid_id.to_string().len(), 36);
        assert!(uuid_id.to_string().contains('-'));
    }

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_computed_value_insert(pool: Pool) {
        let app = InsertApp { pool };

        let result = app.insert_computed_value(5, 10).await.unwrap();
        assert_eq!(result.rows_affected(), 1);
        // computed = 5 * 5 + 10 = 35
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_subquery_insert(pool: Pool) {
        let app = InsertApp { pool };

        let result = app.insert_user_stats(1).await.unwrap();
        assert_eq!(result.rows_affected(), 1);
    }

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_case_expression_insert(pool: Pool) {
        let app = InsertApp { pool };

        // First insert a user to categorize
        let user_id = app
            .insert_user_returning_id(
                "Category Test".to_string(),
                "test@example.com".to_string(),
                22,
                None,
            )
            .await
            .unwrap();

        let result = app.insert_user_category(user_id, 22).await.unwrap();
        assert_eq!(result.rows_affected(), 1);

        // Cleanup
        app.cleanup_test_users().await.unwrap();
    }

    #[sqlx::test(migrations = "tests/migrations_datetime")]
    async fn test_time_record_insert(pool: Pool) {
        let app = InsertApp { pool };

        let result = app
            .insert_current_time_record("Time Test".to_string())
            .await
            .unwrap();
        assert_eq!(result.rows_affected(), 1);
    }

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_error_handling_inserts(pool: Pool) {
        let app = InsertApp { pool };

        // Test duplicate email without ON CONFLICT
        let result1 = app
            .insert_user(
                "User 1".to_string(),
                "duplicate@example.com".to_string(),
                25,
            )
            .await
            .unwrap();
        assert_eq!(result1.rows_affected(), 1);

        // This should fail due to unique constraint on email
        let result2 = app
            .insert_user(
                "User 2".to_string(),
                "duplicate@example.com".to_string(),
                30,
            )
            .await;
        assert!(result2.is_err());

        // Cleanup
        app.cleanup_test_users().await.unwrap();
    }

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_array_insert(pool: Pool) {
        let app = InsertApp { pool };

        let numbers = vec![1, 2, 3, 4, 5];
        let texts = vec!["hello".to_string(), "world".to_string()];

        let result = app
            .insert_with_arrays("Array Test".to_string(), numbers, texts)
            .await
            .unwrap();

        assert_eq!(result.rows_affected(), 1);
    }

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_null_handling_inserts(pool: Pool) {
        let app = InsertApp { pool };

        // Insert with NULL birth_year
        let user_id = app
            .insert_user_returning_id(
                "Null Test".to_string(),
                "test@example.com".to_string(),
                25,
                None,
            )
            .await
            .unwrap();
        assert!(user_id > 0);

        // Insert with non-NULL birth_year
        let user_id2 = app
            .insert_user_returning_id(
                "Non-Null Test".to_string(),
                "test2@example.com".to_string(),
                30,
                Some(1994),
            )
            .await
            .unwrap();
        assert!(user_id2 > 0);
        assert_ne!(user_id, user_id2);

        // Cleanup
        app.cleanup_test_users().await.unwrap();
    }
}
