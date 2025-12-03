use sqlx_data::{
    Cursor, CursorData, CursorError, CursorSecureExtract, CursorValue, FilterValue, IntoParams, Pool, Result, dml, repo,
};

#[derive(Debug, sqlx::FromRow)]
pub struct User {
    pub id: i64,
    pub name: String,
}

impl CursorSecureExtract for User {
    fn extract_whitelisted_fields(&self, fields: &[String]) -> Result<Vec<CursorValue>> {
        let mut values = Vec::with_capacity(fields.len());
        for field in fields {
            match field.as_str() {
                "id" => values.push(self.id.into()),
                "name" => values.push(self.name.clone().into()),
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

#[repo]
trait UserRepo {
    #[dml("SELECT id, name FROM users")]
    async fn find_all(&self, params: impl IntoParams) -> Result<Cursor<User>, sqlx::Error>;

    #[dml("SELECT id, name FROM users WHERE age >= ?")]
    async fn find_by_min_age(
        &self,
        min_age: u8,
        params: impl IntoParams,
    ) -> Result<Cursor<User>, sqlx::Error>;

    #[dml("SELECT id, name FROM users WHERE name LIKE ?")]
    async fn find_by_name_pattern(
        &self,
        pattern: String,
        params: impl IntoParams,
    ) -> Result<Cursor<User>, sqlx::Error>;

    // Test with multiple filters
    #[dml("SELECT id, name FROM users WHERE age >= ? AND name LIKE ?")]
    async fn find_by_age_and_name(
        &self,
        min_age: u8,
        pattern: String,
        params: impl IntoParams,
    ) -> Result<Cursor<User>, sqlx::Error>;

    // Basic query without additional parameters for cursor testing
    #[dml("SELECT id, name FROM users")]
    async fn find_for_cursor(&self, params: impl IntoParams) -> Result<Cursor<User>, sqlx::Error>;
}

pub struct UserApp {
    pool: Pool,
}

impl UserRepo for UserApp {
    fn get_pool(&self) -> &Pool {
        &self.pool
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx_data::ParamsBuilder;

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_cursor_basic_functionality(pool: Pool) {
        let app = UserApp { pool };

        // First page with cursor
        let params = ParamsBuilder::new()
            .sort()
            .asc("id")
            .done()
            .cursor()
            .after(0)
            .done()
            .limit(5)
            .build();

        let cursor = app.find_all(params).await.unwrap();

        assert_eq!(cursor.data.len(), 5);
        assert!(cursor.has_next);
        assert_eq!(cursor.per_page, 5);

        // Verify first user
        assert_eq!(cursor.data[0].name, "Alice");
        assert_eq!(cursor.data[0].id, 1);
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_cursor_pagination_flow(pool: Pool) {
        let app = UserApp { pool };

        // First page
        let params = ParamsBuilder::new()
            .sort()
            .asc("id")
            .done()
            .cursor()
            .after(0)
            .done()
            .limit(3)
            .build();

        let first_page = app.find_for_cursor(params).await.unwrap();
        assert_eq!(first_page.data.len(), 3);
        assert!(first_page.has_next);
        assert!(first_page.next_cursor.is_some());

        let first_cursor = first_page.next_cursor.unwrap();

        // Second page using cursor
        let params = ParamsBuilder::new()
            .sort()
            .asc("id")
            .done()
            .cursor()
            .next_cursor::<User>(&first_cursor)
            .done()
            .limit(3)
            .build();

        let second_page = app.find_for_cursor(params).await.unwrap();
        assert_eq!(second_page.data.len(), 3);
        assert!(second_page.has_next);

        // Verify no overlap between pages
        let first_ids: Vec<i64> = first_page.data.iter().map(|u| u.id).collect();
        let second_ids: Vec<i64> = second_page.data.iter().map(|u| u.id).collect();

        for id in second_ids {
            assert!(!first_ids.contains(&id));
        }

        // Verify ordering (should be ascending by id)
        for i in 1..second_page.data.len() {
            assert!(second_page.data[i-1].id < second_page.data[i].id);
        }
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_cursor_with_filters(pool: Pool) {
        let app = UserApp { pool };

        // Test cursor with age filter
        let params = ParamsBuilder::new()
            .sort()
            .asc("id")
            .done()
            .cursor()
            .after(0)
            .done()
            .limit(3)
            .build();

        let cursor = app.find_by_min_age(25, params).await.unwrap();

        // Verify all users have age >= 25 (need to check actual data)
        assert!(cursor.data.len() <= 3);
        assert!(cursor.data.len() > 0);
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_cursor_with_name_pattern(pool: Pool) {
        let app = UserApp { pool };

        let params = ParamsBuilder::new()
            .sort()
            .asc("id")
            .done()
            .cursor()
            .after(0)
            .done()
            .limit(5)
            .build();

        let cursor = app
            .find_by_name_pattern("%a%".to_string(), params)
            .await
            .unwrap();

        // Should find users with 'a' in their names
        for user in &cursor.data {
            assert!(user.name.to_lowercase().contains('a'));
        }
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_cursor_multi_field_sorting(pool: Pool) {
        let app = UserApp { pool };

        // Test cursor with multiple sort fields
        let params = ParamsBuilder::new()
            .cursor()
                .after("a")
                .and_field(0)
                .done()
            .sort()
                .asc("name")
                .asc("id")
                .done()
            .limit(5)
            .build();

        let cursor = app.find_all(params).await.unwrap();

        // Verify sorting by name first, then id
        for i in 1..cursor.data.len() {
            let prev = &cursor.data[i-1];
            let curr = &cursor.data[i];

            if prev.name == curr.name {
                assert!(prev.id < curr.id);
            } else {
                assert!(prev.name < curr.name);
            }
        }
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_cursor_encoding_decoding(pool: Pool) {
        let app = UserApp { pool };

        let params = ParamsBuilder::new()
            .sort()
            .asc("id")
            .done()
            .cursor()
            .after(0)
            .done()
            .limit(3)
            .build();

        let first_page = app.find_for_cursor(params).await.unwrap();

        if let Some(cursor_str) = first_page.next_cursor {
            // Test that cursor can be decoded
            let decoded_filters = User::decode(&cursor_str).unwrap();
            assert!(!decoded_filters.is_empty());

            // Use the cursor for next page
            let params = ParamsBuilder::new()
                .sort()
                .asc("id")
                .done()
                .cursor()
                .next_cursor::<User>(&cursor_str)
                .done()
                .limit(3)
                .build();

            let second_page = app.find_for_cursor(params).await.unwrap();
            assert!(second_page.data.len() <= 3);
        }
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_cursor_with_multiple_params(pool: Pool) {
        let app = UserApp { pool };

        let params = ParamsBuilder::new()
            .sort()
            .asc("id")
            .done()
            .cursor()
            .after(0)
            .done()
            .limit(3)
            .build();

        let cursor = app
            .find_by_age_and_name(20, "%e%".to_string(), params)
            .await
            .unwrap();

        // Should find users with age >= 20 AND name containing 'e'
        for user in &cursor.data {
            assert!(user.name.to_lowercase().contains('e'));
        }
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_cursor_edge_cases(pool: Pool) {
        let app = UserApp { pool };

        // Test with limit 1
        let params = ParamsBuilder::new()
            .sort()
            .asc("id")
            .done()
            .cursor()
            .after(0)
            .done()
            .limit(1)
            .build();

        let cursor = app.find_all(params).await.unwrap();
        assert_eq!(cursor.data.len(), 1);
        assert!(cursor.has_next);

        // Test with very large limit
        let params = ParamsBuilder::new()
            .sort()
            .asc("id")
            .done()
            .cursor()
            .after(0)
            .done()
            .limit(1000)
            .build();

        let cursor = app.find_all(params).await.unwrap();
        assert_eq!(cursor.data.len(), 20); // All users
        assert!(!cursor.has_next);
        assert!(cursor.next_cursor.is_none());
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_cursor_consistency(pool: Pool) {
        let app = UserApp { pool };

        // Get all data using cursor pagination
        let mut all_users = Vec::new();
        let mut cursor_str = None;
        let limit = 4;

        loop {
            let params = if let Some(cursor) = cursor_str {
                ParamsBuilder::new()
                    .sort()
                    .asc("id")
                    .done()
                    .cursor()
                    .next_cursor::<User>(&cursor)
                    .done()
                    .limit(limit)
                    .build()
            } else {
                ParamsBuilder::new()
                    .sort()
                    .asc("id")
                    .done()
                    .cursor()
                    .after(0)
                    .done()
                    .limit(limit)
                    .build()
            };

            let page = app.find_for_cursor(params).await.unwrap();
            all_users.extend(page.data);

            if !page.has_next {
                break;
            }

            cursor_str = page.next_cursor;
        }

        // Should have all 20 users
        assert_eq!(all_users.len(), 20);

        // Verify no duplicates
        let mut ids: Vec<i64> = all_users.iter().map(|u| u.id).collect();
        ids.sort();
        for i in 1..ids.len() {
            assert!(ids[i-1] < ids[i], "Found duplicate or unordered ID");
        }
    }
}