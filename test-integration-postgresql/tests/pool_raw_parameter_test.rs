use sqlx_data::{
    Connection, Cursor, CursorData, CursorError, CursorSecureExtract, CursorValue, Executor, FilterValue, IntoParams, Pool, QueryResult, Result, Serial, Slice, Transaction, dml
};

#[derive(Clone, PartialEq, Eq, Debug, sqlx::Type)]
#[sqlx(transparent)]
pub struct Id(i64);

impl From<i64> for Id {
    fn from(value: i64) -> Self {
        Id(value)
    }
}

// User model for tests
#[derive(Debug, Clone, PartialEq, sqlx::FromRow)]
pub struct User {
    pub id: Id,
    pub name: String,
    pub email: String,
    pub age: i16,
}

impl CursorSecureExtract for User {
    fn extract_whitelisted_fields(&self, fields: &[String]) -> Result<Vec<CursorValue>> {
        let mut values = Vec::with_capacity(fields.len());
        for field in fields {
            match field.as_str() {
                "id" => values.push(self.id.0.into()),
                _ => {
                    return Err(CursorError::invalid_field(field.clone()).into());
                }
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

        // Convert CursorValue to FilterValue
        let filter_values: Vec<FilterValue> = cursor
            .entries
            .into_iter()
            .map(|entry| match entry.value {
                CursorValue::Int(v) => FilterValue::Int(v),
                CursorValue::UInt(v) => FilterValue::UInt(v),
                CursorValue::Float(v) => FilterValue::Float(v),
                CursorValue::Bool(v) => FilterValue::Bool(v),
                CursorValue::String(v) => v.into(),
            })
            .collect();

        Ok(filter_values)
    }
}

// Test trait for pool parameter functionality
#[sqlx_data::repo]
trait UserPoolRepo {
    // Method without pool parameter (uses get_pool())
    #[dml("SELECT COUNT(*) FROM users")]
    async fn count_users(&self) -> Result<i64>;

    // Method with pool parameter (uses provided pool)
    #[dml("SELECT COUNT(*) FROM users")]
    async fn count_users_with_pool(&self, pool: &Pool) -> Result<i64>;

    // Method with both query params and pool param
    #[dml("SELECT id, name, email, age FROM users WHERE id = $1")]
    async fn find_by_id_with_pool(&self, id: i64, pool: &Pool) -> Result<User>;

    // Tuple with pool parameter
    #[dml("SELECT name, age FROM users WHERE id = $1")]
    async fn get_user_info_with_pool(&self, id: i64, pool: &Pool) -> Result<(String, i16)>;

    // Connection parameters
    #[dml("SELECT COUNT(*) FROM users")]
    async fn count_users_with_connection(&self, conn: &mut Connection) -> Result<i64>;

    #[dml("SELECT id, name, email, age FROM users WHERE id = $1")]
    async fn find_by_id_with_connection(&self, id: i64, conn: &mut Connection) -> Result<User>;

    // Transaction parameters
    #[dml("SELECT COUNT(*) FROM users")]
    async fn count_users_with_transaction(&self, tx: &mut Transaction<'_>) -> Result<i64>;

    #[dml("SELECT id, name, email, age FROM users WHERE id = $1")]
    async fn find_by_id_with_transaction(
        &self,
        id: i64,
        tx: &mut Transaction<'_>,
    ) -> Result<User>;

    // Serial pagination with Connection and Transaction
    #[dml("SELECT id, name, email, age FROM users")]
    async fn find_users_serial_with_connection(
        &self,
        params: impl IntoParams,
        conn: &mut Connection,
    ) -> Result<Serial<User>>;

    #[dml("SELECT id, name, email, age FROM users")]
    async fn find_users_serial_with_transaction(
        &self,
        params: impl IntoParams,
        tx: &mut Transaction<'_>,
    ) -> Result<Serial<User>>;

    // Cursor pagination with Connection and Transaction
    #[dml("SELECT id, name, email, age FROM users")]
    async fn find_users_cursor_with_connection(
        &self,
        params: impl IntoParams,
        conn: &mut Connection,
    ) -> Result<Cursor<User>>;

    #[dml("SELECT id, name, email, age FROM users")]
    async fn find_users_cursor_with_transaction(
        &self,
        params: impl IntoParams,
        tx: &mut Transaction<'_>,
    ) -> Result<Cursor<User>>;

    // Slice pagination with Connection and Transaction
    #[dml("SELECT id, name, email, age FROM users")]
    async fn find_users_slice_with_connection(
        &self,
        params: impl IntoParams,
        conn: &mut Connection,
    ) -> Result<Slice<User>>;

    #[dml("SELECT id, name, email, age FROM users")]
    async fn find_users_slice_with_transaction(
        &self,
        params: impl IntoParams,
        tx: &mut Transaction<'_>,
    ) -> Result<Slice<User>>;

    #[dml("INSERT INTO users (id) VALUES ($1)")]
    async fn insert_with_executor<'e, E>(&self, executor: E, value: i64) -> Result<QueryResult>
    where
        E: Executor<'e, Database = sqlx::Postgres>;

    #[dml("INSERT INTO users (id, name) VALUES ($1, $2)")]
    async fn insert_multi_executor<'e, EX>(
        &self,
        executor: EX,
        id: i64,
        name: String,
    ) -> Result<QueryResult>
    where
        EX: sqlx::Executor<'e, Database = sqlx::Postgres>;

    #[dml("INSERT INTO users (name) VALUES ($1)")]
    async fn insert_complex_executor<'e, EX>(
        &self,
        executor: EX,
        name: String,
    ) -> Result<QueryResult>
    where
        EX: sqlx::Executor<'e, Database = sqlx::Postgres> + Send;

    #[dml("INSERT INTO users (name) VALUES ($1)")]
    async fn insert_impl_trait(
        &self,
        name: String,
        executor: impl Executor<'_>,
    ) -> Result<QueryResult>;
}

pub struct MyPoolApp {
    pool: Pool,
}

impl UserPoolRepo for MyPoolApp {
    // Override the default get_pool implementation
    fn get_pool(&self) -> &Pool {
        &self.pool
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx_data::{ParamsBuilder, SerialParams, SliceParams};

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_count_without_pool(pool: Pool) {
        let repo = MyPoolApp { pool };

        let count = repo.count_users().await.unwrap();
        assert_eq!(count, 20); // From fixtures
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_count_with_pool(pool: Pool) {
        let repo = MyPoolApp {
            pool: pool.clone(),
        };

        let count = repo.count_users_with_pool(&pool).await.unwrap();
        assert_eq!(count, 20); // From fixtures
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_find_by_id_with_pool(pool: Pool) {
        let repo = MyPoolApp {
            pool: pool.clone(),
        };

        let user = repo.find_by_id_with_pool(1, &pool).await.unwrap();
        assert_eq!(user.name, "Alice");
        assert_eq!(user.email, "alice@example.com");
        assert_eq!(user.age, 30);
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_tuple_with_pool(pool: Pool) {
        let repo = MyPoolApp {
            pool: pool.clone(),
        };

        let (name, age) = repo.get_user_info_with_pool(2, &pool).await.unwrap();
        assert_eq!(name, "Bob");
        assert_eq!(age, 25);
    }

    // Test struct that doesn't override get_pool (uses default unimplemented!)
    pub struct AppWithoutPool;

    impl UserPoolRepo for AppWithoutPool {
        // Uses default get_pool implementation (unimplemented!)
        // Only works with methods that have pool parameters
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    #[should_panic(
        expected = "not implemented: Implement get_pool() to use methods without pool parameters, or pass pool explicitly via method parameters"
    )]
    async fn test_default_get_pool_panics(_pool: Pool) {
        let app = AppWithoutPool;

        // This should panic because get_pool() is unimplemented!
        let _count = app.count_users().await;
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_methods_with_pool_param_work_without_get_pool(pool: Pool) {
        let app = AppWithoutPool;

        // This should work because it uses pool parameter, not get_pool()
        let count = app.count_users_with_pool(&pool).await.unwrap();
        assert_eq!(count, 20);
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_transaction_parameters(pool: Pool) {
        let app = AppWithoutPool;

        // Start a transaction
        let mut tx = pool.begin().await.unwrap();

        // Use transaction parameter directly - the macro should handle &mut *tx automatically
        let count = app.count_users_with_transaction(&mut tx).await.unwrap();
        assert_eq!(count, 20);

        let user = app.find_by_id_with_transaction(1, &mut tx).await.unwrap();
        assert_eq!(user.name, "Alice");
        assert_eq!(user.age, 30);

        // Commit transaction
        tx.commit().await.unwrap();
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_connection_parameters(pool: Pool) {
        let app = AppWithoutPool;

        let mut conn = pool.acquire().await.unwrap();

        let count = app.count_users_with_connection(&mut conn).await.unwrap();
        assert_eq!(count, 20);

        let user = app.find_by_id_with_connection(2, &mut conn).await.unwrap();
        assert_eq!(user.name, "Bob");
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_simple_connection_only(pool: Pool) {
        let app = AppWithoutPool;

        let mut conn = pool.acquire().await.unwrap();
        let count = app.count_users_with_connection(&mut conn).await.unwrap();
        assert_eq!(count, 20);
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_serial_pagination_with_connection(pool: Pool) {
        let app = AppWithoutPool;

        let mut conn = pool.acquire().await.unwrap();
        let params = SerialParams::new(1, 2);
        let result = app
            .find_users_serial_with_connection(params, &mut conn)
            .await
            .unwrap();

        assert_eq!(result.page, 1);
        assert_eq!(result.size, 2);
        assert_eq!(result.total_items, 20);
        assert_eq!(result.data.len(), 2);
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_serial_pagination_with_transaction(pool: Pool) {
        let app = AppWithoutPool;

        let mut tx = pool.begin().await.unwrap();
        let params = SerialParams::new(1, 2);
        let result = app
            .find_users_serial_with_transaction(params, &mut tx)
            .await
            .unwrap();

        assert_eq!(result.page, 1);
        assert_eq!(result.size, 2);
        assert_eq!(result.total_items, 20);
        assert_eq!(result.data.len(), 2);

        tx.commit().await.unwrap();
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_cursor_pagination_with_connection(pool: Pool) {
        let app = AppWithoutPool;

        let mut conn = pool.acquire().await.unwrap();
        let params = ParamsBuilder::new()
            .sort()
            .asc("id")
            .done()
            .cursor()
            .first_page()
            .done()
            .limit(2)
            .build();
        let result = app
            .find_users_cursor_with_connection(params, &mut conn)
            .await
            .unwrap();

        assert_eq!(result.data.len(), 2);
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_cursor_pagination_with_transaction(pool: Pool) {
        let app = AppWithoutPool;

        let mut tx = pool.begin().await.unwrap();
        let params = ParamsBuilder::default()
            .cursor()
            .first_page()
            .done()
            .sort()
            .asc("id")
            .done()
            .limit(2)
            .build();
        let result = app
            .find_users_cursor_with_transaction(params, &mut tx)
            .await
            .unwrap();

        assert_eq!(result.data.len(), 2);

        tx.commit().await.unwrap();
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_slice_pagination_with_connection(pool: Pool) {
        let app = AppWithoutPool;

        let mut conn = pool.acquire().await.unwrap();
        let params = SliceParams::new(1, 2);
        let result = app
            .find_users_slice_with_connection(params, &mut conn)
            .await
            .unwrap();

        assert_eq!(result.page, 1);
        assert_eq!(result.size, 2);
        assert_eq!(result.data.len(), 2);
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_slice_pagination_with_transaction(pool: Pool) {
        let app = AppWithoutPool;

        let mut tx = pool.begin().await.unwrap();
        let params = SliceParams::new(1, 2);
        let result = app
            .find_users_slice_with_transaction(params, &mut tx)
            .await
            .unwrap();

        assert_eq!(result.page, 1);
        assert_eq!(result.size, 2);
        assert_eq!(result.data.len(), 2);

        tx.commit().await.unwrap();
    }
}