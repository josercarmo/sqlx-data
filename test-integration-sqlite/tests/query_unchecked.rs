#![cfg(all(feature = "json"))]

use futures::Stream;
use sqlx_data::{
    Cursor, CursorData, CursorError, CursorSecureExtract, CursorValue, FilterValue, IntoParams, ParamsBuilder, Pool, Result,
    Serial, Slice, dml, repo,
};

// Comprehensive test trait with one example of each type, all with unchecked
#[repo]
trait ComprehensiveUncheckedRepo {
    // 1. Serial pagination with unchecked
    #[dml(
        "SELECT id as 'id: i64', name, age FROM users WHERE age > $1 ORDER BY id",
        unchecked
    )]
    async fn get_users_serial(
        &self,
        min_age: i32,
        params: impl IntoParams,
    ) -> Result<Serial<(i64, String, i32)>>;

    // 2. Slice pagination with unchecked
    #[dml(
        "SELECT id, name, age FROM users WHERE name LIKE $1 ORDER BY name",
        unchecked
    )]
    async fn get_users_slice(
        &self,
        name_pattern: &str,
        params: impl IntoParams,
    ) -> Result<Slice<User>>;

    // 3. Cursor pagination with unchecked
    #[dml("SELECT id, name, age FROM users WHERE id > $1 ORDER BY id", unchecked)]
    async fn get_users_cursor(
        &self,
        after_id: i64,
        params: impl IntoParams,
    ) -> Result<Cursor<User>>;

    // 4. Tuple return with unchecked
    #[dml(
        "SELECT COUNT(*) as 'count: i64', AVG(age) as 'avg_age: f64' FROM users WHERE age > $1",
        unchecked
    )]
    async fn get_user_stats(&self, min_age: i32) -> Result<(i64, Option<f64>)>;

    // 5. Scalar return with unchecked
    #[dml("SELECT COUNT(*) FROM users WHERE age > $1", unchecked)]
    async fn count_users(&self, min_age: i32) -> Result<i64>;

    // 6. Struct return with unchecked
    #[dml("SELECT id, name, age FROM users WHERE id = $1", unchecked)]
    async fn get_user_by_id(&self, user_id: i64) -> Result<Option<User>>;

    // 7. Stream return with unchecked (note: not async)
    #[dml(
        "SELECT id, name, age FROM users WHERE age > $1 ORDER BY id",
        unchecked
    )]
    fn stream_users(&self, min_age: i32) -> impl Stream<Item = Result<User>>;

    // 8. Stream tuple return with unchecked
    #[dml("SELECT id, name FROM users WHERE age > $1 ORDER BY id", unchecked)]
    fn stream_user_tuples(&self, min_age: i32) -> impl Stream<Item = Result<(i64, String)>>;
}

// Simple User struct for testing
#[derive(Debug, PartialEq, sqlx::FromRow)]
pub struct User {
    pub id: i64,
    pub name: String,
    pub age: i64,
}

impl CursorSecureExtract for User {
    fn extract_whitelisted_fields(&self, fields: &[String]) -> Result<Vec<CursorValue>> {
        let mut values = Vec::with_capacity(fields.len());
        for field in fields {
            match field.as_str() {
                "id" => values.push(CursorValue::Int(self.id)),
                _ => return Err(CursorError::invalid_field(field.clone()).into()),
            }
        }
        Ok(values)
    }

    fn encode(cursor: &CursorData) -> Result<String> {
        use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD as BASE64};
        let json_bytes = serde_json::to_vec(&cursor)
            .map_err(|e| CursorError::encode_error(format!("JSON serialization failed: {}", e)))?;
        Ok(BASE64.encode(json_bytes))
    }

    fn decode(encoded: &str) -> Result<Vec<FilterValue>> {
        use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD as BASE64};
        let bytes = BASE64
            .decode(encoded)
            .map_err(|e| CursorError::decode_error(format!("Base64 decode failed: {}", e)))?;

        let cursor: CursorData = serde_json::from_slice(&bytes).map_err(|e| {
            CursorError::decode_error(format!("JSON deserialization failed: {}", e))
        })?;

        let filter_values: Vec<FilterValue> = cursor.entries.into_iter().map(|entry| {
            match entry.value {
                CursorValue::Int(v) => FilterValue::Int(v),
                CursorValue::UInt(v) => FilterValue::UInt(v),
                CursorValue::Float(v) => FilterValue::Float(v),
                CursorValue::Bool(v) => FilterValue::Bool(v),
                CursorValue::String(v) => v.into(),
            }
        }).collect();

        Ok(filter_values)
    }
}

// Test implementation
pub struct TestRepo {
    pool: Pool,
}

impl ComprehensiveUncheckedRepo for TestRepo {
    fn get_pool(&self) -> &Pool {
        &self.pool
    }
}

#[cfg(test)]
mod tests {
    use futures::StreamExt;

    use super::*;

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_with_pool(pool: Pool) -> sqlx::Result<()> {
        let repo = TestRepo { pool };

        let params = ParamsBuilder::new().serial().page(1, 3).done();
        let result = repo.get_users_serial(20, params).await;
        assert!(
            result.is_ok(),
            "Serial pagination should work with unchecked"
        );

        Ok(())
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_serial_pagination_unchecked(pool: Pool) {
        let repo = TestRepo { pool };

        let params = ParamsBuilder::new().serial().page(1, 3).done();
        let result = repo.get_users_serial(20, params).await;
        assert!(
            result.is_ok(),
            "Serial pagination should work with unchecked"
        );

        let serial = result.unwrap();
        println!("Serial result: {:?}", serial);
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_slice_pagination_unchecked(pool: Pool) {
        let repo = TestRepo { pool };

        let params = ParamsBuilder::new().slice().page(1, 2).done();
        let result = repo.get_users_slice("%A%", params).await;
        assert!(
            result.is_ok(),
            "Slice pagination should work with unchecked"
        );

        let slice = result.unwrap();
        println!("Slice result: {:?}", slice);
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_cursor_pagination_unchecked(pool: Pool) {
        let repo = TestRepo { pool };
        #[rustfmt::skip]
        let params = ParamsBuilder::new()
            .sort()
                .asc("id")
                .done()
            .cursor()
                .after(0i64)
                .done()
            .limit(2);
        let result = repo.get_users_cursor(0, params).await;
        assert!(
            result.is_ok(),
            "Cursor pagination should work with unchecked"
        );

        let cursor = result.unwrap();
        assert!(!cursor.data.is_empty());
        assert_eq!(cursor.data[0].name, "Alice");
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_tuple_return_unchecked(pool: Pool) {
        let repo = TestRepo { pool };

        let result = repo.get_user_stats(20).await;
        assert!(result.is_ok(), "Tuple return should work with unchecked");

        let (count, avg_age) = result.unwrap();
        println!("User stats: count={}, avg_age={}", count, avg_age.unwrap());
        assert!(count > 0);
        assert!(avg_age.unwrap() > 0.0);
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_scalar_return_unchecked(pool: Pool) {
        let repo = TestRepo { pool };

        let result = repo.count_users(20).await;
        assert!(result.is_ok(), "Scalar return should work with unchecked");

        let count = result.unwrap();
        println!("User count: {}", count);
        assert!(count > 0);
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_struct_return_unchecked(pool: Pool) {
        let repo = TestRepo { pool };

        let result = repo.get_user_by_id(1).await;
        assert!(result.is_ok(), "Struct return should work with unchecked");

        let user = result.unwrap();
        println!("User: {:?}", user);
        assert!(user.is_some());
        if let Some(u) = user {
            assert_eq!(u.id, 1);
            assert_eq!(u.name, "Alice");
        }
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_stream_return_unchecked(pool: Pool) {
        let repo = TestRepo { pool };

        let mut stream = repo.stream_users(20);

        let mut count = 0;
        while let Some(result) = StreamExt::next(&mut stream).await {
            match result {
                Ok(_user) => {
                    count += 1;
                }
                Err(e) => {
                    eprintln!("Stream error: {:?}", e);
                }
            }
        }

        println!("Streamed {} users", count);
        assert!(count > 0, "Should stream at least one user");
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_stream_tuple_return_unchecked(pool: Pool) {
        let repo = TestRepo { pool };

        let mut stream = repo.stream_user_tuples(35);
        let mut tuples = Vec::new();

        while let Some(result) = StreamExt::next(&mut stream).await {
            tuples.push(result.unwrap());
        }

        tuples.sort_by_key(|(id, _)| *id);
        assert_eq!(tuples.len(), 4);
        assert_eq!(tuples[0], (5, "Eve".to_string()));
        assert_eq!(tuples[1], (17, "Quinn".to_string()));
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_all_types_comprehensive(pool: Pool) {
        let repo = TestRepo { pool };

        // 1. Scalar
        let count = repo.count_users(0).await.unwrap();
        assert!(count > 0);
        assert_eq!(count, 20);

        // 2. Tuple stats
        let (active_count, avg_age) = repo.get_user_stats(20).await.unwrap();
        assert!(active_count > 0);
        assert!(avg_age.is_some());
        assert!(avg_age.unwrap() > 0.0);

        // 3. Single struct
        let user = repo.get_user_by_id(1).await.unwrap();
        assert!(user.is_some());
        let user = user.unwrap();
        assert_eq!(user.id, 1);
        assert_eq!(user.name, "Alice");

        // 4. Pagination types
        let serial_params = ParamsBuilder::new().serial().page(1, 2).done();
        let slice_params = ParamsBuilder::new().slice().page(1, 2).done();
        let cursor_params = ParamsBuilder::new()
            .sort()
                .asc("id")
                .done()
            .cursor()
                .after(0i64)
                .done()
            .limit(2);

        let serial = repo.get_users_serial(20, serial_params).await.unwrap();
        let slice = repo.get_users_slice("%", slice_params).await.unwrap();
        let cursor = repo.get_users_cursor(0, cursor_params).await.unwrap();

        assert!(!serial.data.is_empty());
        assert!(!slice.data.is_empty());
        assert!(!cursor.data.is_empty());

        // 5. Stream (collect first few items)
        let mut stream = repo.stream_users(20);
        let mut streamed_users = Vec::new();
        for _ in 0..3 {
            if let Some(result) = StreamExt::next(&mut stream).await {
                streamed_users.push(result.unwrap());
            }
        }
        assert!(!streamed_users.is_empty());
        assert!(streamed_users.len() <= 3);
    }
}
