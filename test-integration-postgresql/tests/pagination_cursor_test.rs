#![cfg(all(feature = "json"))]
use sqlx_data::{
    Cursor, CursorData, CursorError, CursorSecureExtract, CursorValue, FilterValue, IntoParams, Pool, Result, dml, repo,
};

// PostgreSQL-adapted User struct with UUID type support
#[derive(Debug, sqlx::FromRow)]
pub struct User {
    pub id: i64, // PostgreSQL BIGINT
    pub name: String, // PostgreSQL TEXT
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
    // PostgreSQL-specific SQL with proper parameter syntax
    #[dml("SELECT id, name FROM users ORDER BY id")]
    async fn find_all(&self, params: impl IntoParams) -> Result<Cursor<User>>;
}

#[cfg(test)]
mod tests {
    use sqlx_data::ParamsBuilder;

    use super::*;

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_cursor_basic_functionality(pool: Pool) {
        let repo = TestUserRepo { pool: &pool };

        let params = ParamsBuilder::new()
            .sort()
            .asc("id")
            .done()
            .cursor()
            .after(5)
            .done()
            .limit(3)
            .build();

        let page = repo.find_all(params).await.unwrap();

        // Should return users with id > 5
        assert!(page.data.iter().all(|user| user.id > 5));
        assert_eq!(page.per_page, 3);
        assert_eq!(page.data.len(), 3);
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_cursor_from_encoded(pool: Pool) {
        let repo = TestUserRepo { pool: &pool };

        // First, create a cursor and get its encoded form
        let original_params = ParamsBuilder::new()
            .sort()
            .asc("id")
            .done()
            .cursor()
            .after(3)
            .done()
            .limit(2)
            .build();

        let first_page = repo.find_all(original_params).await.unwrap();
        println!("First page with cursor: {:?}", first_page);

        // Get the next cursor from the response
        let next_cursor_encoded = first_page.next_cursor.as_ref().unwrap().clone();
        println!("Next cursor encoded: {}", next_cursor_encoded);

        // Test ParamsBuilder::cursor().from_encoded()
        let decoded_params = ParamsBuilder::new()
            .sort()
            .asc("id")
            .done()
            .cursor()
            .next_cursor::<User>(&next_cursor_encoded)
            .done()
            .build();

        let second_page = repo.find_all(decoded_params).await.unwrap();
        println!("Second page from encoded cursor: {:?}", second_page);

        // Should get next set of users
        assert!(second_page.data.len() > 0);
        assert!(second_page.data[0].id > first_page.data.last().unwrap().id);
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_cursor_encode_decode_roundtrip(pool: Pool) {
        // Test encoding a cursor, then decoding it back and using it
        let repo = TestUserRepo { pool: &pool };

        // Get a real cursor from pagination to test encode/decode
        let initial_params = ParamsBuilder::new()
            .sort()
            .asc("id")
            .done()
            .cursor()
            .after(3)
            .done()
            .limit(2)
            .build();

        let page = repo.find_all(initial_params).await.unwrap();
        let encoded = page.next_cursor.as_ref().unwrap();
        println!("Encoded cursor from pagination: {}", encoded);

        // Test that we can decode it back using User's implementation
        let decoded_values = User::decode(encoded).unwrap();
        println!("Decoded values: {:?}", decoded_values);

        // Use decoded cursor in ParamsBuilder
        let params = ParamsBuilder::new()
            .sort()
            .asc("id")
            .done()
            .cursor()
            .next_cursor::<User>(encoded)
            .done()
            .limit(3)
            .build();

        let page = repo.find_all(params).await.unwrap();
        println!("Page from encoded cursor: {:?}", page);

        // Should return users with id > 4
        assert!(page.data.iter().all(|user| user.id > 4));
        assert_eq!(page.per_page, 3);
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_cursor_invalid_from_encoded(pool: Pool) {
        let repo = TestUserRepo { pool: &pool };

        let params = ParamsBuilder::new()
            .sort()
            .asc("id")
            .done()
            .cursor()
            .next_cursor::<User>("invalid-cursor-string")
            .done()
            .limit(3)
            .build();

        // Should work fine with default cursor (warns but doesn't crash)
        let page = repo.find_all(params).await.unwrap();

        // Should return first page with default cursor behavior
        assert!(page.data.len() > 0);
        assert_eq!(page.per_page, 3);
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_cursor_current_none_scenarios(pool: Pool) {
        let repo = TestUserRepo { pool: &pool };

        // Test with minimal cursor params (no initial cursor provided)
        let params = ParamsBuilder::new()
            .sort()
            .asc("id")
            .done()
            .cursor()
            .after(0) // Start from beginning
            .done()
            .limit(3)
            .build();

        let page = repo.find_all(params).await.unwrap();

        println!("Next cursor when starting: {:?}", page.next_cursor);

        // Test edge case: what happens with empty data?
        let beyond_end_params = ParamsBuilder::new()
            .sort()
            .asc("id")
            .done()
            .cursor()
            .after(1000) // Way beyond available data
            .done()
            .limit(5)
            .build();

        let empty_page = repo.find_all(beyond_end_params).await.unwrap();

        // With empty data, no next cursor is available
        assert_eq!(empty_page.data.len(), 0);
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_cursor_has_next(pool: Pool) {
        let repo = TestUserRepo { pool: &pool };

        // First page - should have has_next = true (we have 20 users, requesting 3)
        let params = ParamsBuilder::new()
            .sort()
            .asc("id")
            .done()
            .cursor()
            .after(0)
            .done()
            .limit(3)
            .build();

        let first_page = repo.find_all(params).await.unwrap();
        println!(
            "First page: has_next={}, data_len={}",
            first_page.has_next,
            first_page.data.len()
        );

        assert!(first_page.has_next, "First page should have next page");
        assert_eq!(first_page.data.len(), 3);
        assert_eq!(first_page.per_page, 3);
        assert!(
            first_page.has_prev,
            "First page with cursor has previous (cursor-based pagination)"
        );

        // Navigate to next page using the next_cursor
        let next_cursor_encoded = first_page.next_cursor.as_ref().unwrap().clone();
        let next_params = ParamsBuilder::new()
            .sort()
            .asc("id")
            .done()
            .cursor()
            .next_cursor::<User>(&next_cursor_encoded)
            .done()
            .limit(3)
            .build();

        let second_page = repo.find_all(next_params).await.unwrap();
        println!(
            "Second page: has_next={}, data_len={}",
            second_page.has_next,
            second_page.data.len()
        );

        assert!(
            second_page.has_next,
            "Second page should still have next page"
        );
        assert_eq!(second_page.data.len(), 3);
        assert!(second_page.has_prev, "Second page should have previous");

        // Test near end - request large chunk that goes beyond available data
        let near_end_params = ParamsBuilder::new()
            .sort()
            .asc("id")
            .done()
            .cursor()
            .after(18)
            .done()
            .limit(5) // Only 2 users left (id=19,20)
            .build();

        let near_end_page = repo.find_all(near_end_params).await.unwrap();
        println!(
            "Near end page: has_next={}, data_len={}",
            near_end_page.has_next,
            near_end_page.data.len()
        );

        assert!(
            !near_end_page.has_next,
            "Near end page should not have next page"
        );
        assert_eq!(near_end_page.data.len(), 2); // Only users 19 and 20
        assert!(near_end_page.has_prev, "Near end page should have previous");

        // Test exactly at end
        let end_params = ParamsBuilder::new()
            .sort()
            .asc("id")
            .done()
            .cursor()
            .after(20) // Beyond last user
            .done()
            .limit(3)
            .build();

        let end_page = repo.find_all(end_params).await.unwrap();
        println!(
            "End page: has_next={}, data_len={}",
            end_page.has_next,
            end_page.data.len()
        );

        assert!(!end_page.has_next, "End page should not have next page");
        assert_eq!(end_page.data.len(), 0);
        assert!(end_page.has_prev, "End page should have previous");
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_cursor_has_prev(pool: Pool) {
        let repo = TestUserRepo { pool: &pool };

        // Start from beginning - should have has_prev = false
        let first_params = ParamsBuilder::new()
            .sort()
            .asc("id")
            .done()
            .cursor()
            .after(0)
            .done()
            .limit(3)
            .build();

        let first_page = repo.find_all(first_params).await.unwrap();
        println!(
            "First page: has_prev={}, data_len={}",
            first_page.has_prev,
            first_page.data.len()
        );

        assert!(
            first_page.has_prev,
            "First page with cursor has previous (cursor-based pagination)"
        );
        assert!(first_page.has_next, "First page should have next");

        // Move to second page using next_cursor
        let next_cursor = first_page.next_cursor.as_ref().unwrap().clone();
        let second_params = ParamsBuilder::new()
            .sort()
            .asc("id")
            .done()
            .cursor()
            .next_cursor::<User>(&next_cursor)
            .done()
            .limit(3)
            .build();

        let second_page = repo.find_all(second_params).await.unwrap();
        println!(
            "Second page: has_prev={}, data_len={}",
            second_page.has_prev,
            second_page.data.len()
        );

        assert!(second_page.has_prev, "Second page should have previous");
        assert!(second_page.has_next, "Second page should have next");

        // Test using before direction - should also have has_prev
        let before_params = ParamsBuilder::new()
            .sort()
            .asc("id")
            .done()
            .cursor()
            .before(5)
            .done()
            .limit(3)
            .build();

        let before_page = repo.find_all(before_params).await.unwrap();
        println!(
            "Before page: has_prev={}, data_len={}",
            before_page.has_prev,
            before_page.data.len()
        );

        // When using before with id=5, we get users with id < 5 (users 1,2,3,4)
        assert!(
            before_page.has_prev,
            "Before page should have previous when cursor is provided"
        );
        assert_eq!(before_page.data.len(), 3); // Should get 3 users (1,2,3)

        // Test middle of dataset
        let middle_params = ParamsBuilder::new()
            .sort()
            .asc("id")
            .done()
            .cursor()
            .after(4)
            .done()
            .limit(3)
            .build();

        let middle_page = repo.find_all(middle_params).await.unwrap();
        println!(
            "Middle page: has_prev={}, data_len={}",
            middle_page.has_prev,
            middle_page.data.len()
        );

        assert!(middle_page.has_prev, "Middle page should have previous");
        assert!(middle_page.has_next, "Middle page should have next");
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_postgresql_mixed_sort_directions(pool: Pool) {
        let repo = TestUserRepo { pool: &pool };

        // Mixed ASC and DESC directions should work with PostgreSQL
        let params = ParamsBuilder::new()
            .sort()
                .asc("name")
                .desc("id")  // Mixed direction should work
                .done()
            .cursor()
                .first_page()
                .done()
            .limit(5)
            .build();

        let result = repo.find_all(params).await;

        // Should succeed with mixed sort directions
        assert!(result.is_ok(), "Mixed sort directions should work: {:?}", result.err());

        let page = result.unwrap();
        assert_eq!(page.per_page, 5);

        // Verify correct ordering: name ASC, then id DESC for same names
        if page.data.len() > 1 {
            for i in 0..page.data.len()-1 {
                let current = &page.data[i];
                let next = &page.data[i+1];

                // Either name is lexicographically smaller, or same name with larger id
                assert!(
                    current.name < next.name ||
                    (current.name == next.name && current.id > next.id),
                    "Data not properly ordered: {:?} should come before {:?}", current, next
                );
            }
        }
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_postgresql_collation_support(pool: Pool) {
        let repo = TestUserRepo { pool: &pool };

        // Test PostgreSQL case-insensitive collation for names
        let params = ParamsBuilder::new()
            .sort()
                .asc("name")
                .done()
            .cursor()
                .first_page()
                .done()
            .limit(5)
            .build();

        let result = repo.find_all(params).await;
        assert!(result.is_ok(), "PostgreSQL collation should work: {}", result.err().unwrap());

        let page = result.unwrap();
        assert!(page.data.len() > 0);

        // Verify names are properly sorted
        if page.data.len() > 1 {
            for i in 0..page.data.len()-1 {
                let current = &page.data[i];
                let next = &page.data[i+1];
                assert!(
                    current.name <= next.name,
                    "Names should be ordered: '{}' should come before '{}'",
                    current.name, next.name
                );
            }
        }
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_cursor_edge_cases_postgresql(pool: Pool) {
        let repo = TestUserRepo { pool: &pool };

        // Test with limit larger than total data
        let large_page_params = ParamsBuilder::new()
            .sort()
            .asc("id")
            .done()
            .cursor()
            .after(0)
            .done()
            .limit(30) // More than our 20 test users
            .build();

        let large_page = repo.find_all(large_page_params).await.unwrap();
        println!(
            "Large page: has_next={}, has_prev={}, data_len={}",
            large_page.has_next,
            large_page.has_prev,
            large_page.data.len()
        );

        assert!(!large_page.has_next, "Large page should not have next");
        assert!(
            large_page.has_prev,
            "Large page with cursor has previous (cursor-based pagination)"
        );
        assert_eq!(large_page.data.len(), 20); // All users

        // Test exact boundary - PostgreSQL BIGINT precision
        let boundary_params = ParamsBuilder::new()
            .sort()
            .asc("id")
            .done()
            .cursor()
            .after(17) // 3 users left (18,19,20)
            .done()
            .limit(3) // Exactly what's left
            .build();

        let boundary_page = repo.find_all(boundary_params).await.unwrap();
        println!(
            "Boundary page: has_next={}, has_prev={}, data_len={}",
            boundary_page.has_next,
            boundary_page.has_prev,
            boundary_page.data.len()
        );

        assert!(
            !boundary_page.has_next,
            "Boundary page should not have next"
        );
        assert!(boundary_page.has_prev, "Boundary page should have previous");
        assert_eq!(boundary_page.data.len(), 3);
    }
}

struct TestUserRepo<'a> {
    pool: &'a Pool,
}

impl<'a> UserRepo for TestUserRepo<'a> {
    fn get_pool(&self) -> &Pool {
        self.pool
    }
}