use sqlx_data::{FilterValue, IntoParams, ParamsBuilder, Pool, Result, Slice};
use sqlx_data::{dml, repo};

#[derive(Debug, sqlx::FromRow)]
pub struct User {
    #[allow(dead_code)]
    pub id: i64,
    pub name: String,
    pub age: Option<u8>,  // MySQL TINYINT UNSIGNED
}

#[repo]
trait UserRepo {
    #[dml("SELECT id, name, age FROM users")]
    async fn find_all(&self, params: impl IntoParams) -> Result<Slice<User>>;

    #[dml("SELECT name, age FROM users WHERE name LIKE ?")]
    async fn find_by_name_pattern(
        &self,
        pattern: String,
        params: impl IntoParams,
    ) -> Result<Slice<(String, Option<u8>)>>;

    #[dml("SELECT id as 'id: i64', name as 'name: String', age as 'age: Option<u8>' FROM users")]
    async fn find_with_cast_syntax(
        &self,
        params: impl IntoParams,
    ) -> Result<Slice<User>>;

    // New test with filtering only
    #[dml("SELECT id, name, age FROM users")]
    async fn find_with_filters(&self, params: impl IntoParams) -> Result<Slice<User>>;

    // Test with params + additional parameters
    #[dml("SELECT id, name, age FROM users WHERE age >= ? AND name LIKE ?")]
    async fn find_by_age_and_name(
        &self,
        min_age: u8,
        name_pattern: String,
        params: impl IntoParams,
    ) -> Result<Slice<User>>;

    // Test with multiple additional parameters + params
    #[dml("SELECT id, name, age FROM users WHERE age BETWEEN ? AND ? AND name LIKE ?")]
    async fn find_by_age_range_and_name(
        &self,
        min_age: u8,
        max_age: u8,
        name_pattern: String,
        params: impl IntoParams,
    ) -> Result<Slice<User>>;

    // Test with optional parameters + params
    #[dml("SELECT id, name, age FROM users WHERE (? IS NULL OR age >= ?) AND name LIKE ?")]
    async fn find_with_optional_age_filter(
        &self,
        min_age: Option<u8>,
        max_age: Option<u8>,
        name_pattern: String,
        params: impl IntoParams,
    ) -> Result<Slice<User>>;
}

pub struct TestUserRepo {
    pool: Pool,
}

impl UserRepo for TestUserRepo {
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
    async fn test_slice_with_pagination_only(pool: Pool) {
        let repo = TestUserRepo { pool };

        let params = ParamsBuilder::new()
            .slice()
            .page(1, 5)
            .done()
            .sort()
            .asc("name")
            .done()
            .build();

        let page = repo.find_all(params).await.unwrap();

        assert_eq!(page.size, 5);
        assert_eq!(page.page, 1);
        assert!(page.has_next);
        assert_eq!(page.data.len(), 5);

        // Verify first user (sorted by name)
        assert_eq!(page.data[0].name, "Alice");
        assert_eq!(page.data[0].age, Some(30));
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_slice_with_search(pool: Pool) {
        let repo = TestUserRepo { pool };

        use tracing_subscriber;

        // Initialize tracing subscriber to see actual output (filter out sqlx noise)
        use tracing_subscriber::{EnvFilter, FmtSubscriber};

        let filter = EnvFilter::new("debug").add_directive("sqlx=debug".parse().unwrap());

        let subscriber = FmtSubscriber::builder()
            .with_max_level(tracing::Level::DEBUG)
            .with_env_filter(filter)
            .with_span_events(
                tracing_subscriber::fmt::format::FmtSpan::NEW
                    | tracing_subscriber::fmt::format::FmtSpan::CLOSE,
            )
            .finish();

        let _ = tracing::subscriber::set_global_default(subscriber);
        #[rustfmt::skip]
        let params = ParamsBuilder::new()
            .slice()
                .page(1, 5)
                .done()
            .search()
                .query("y")
                .fields(["name"])
                .exact(false)
                .case_sensitive(true)
                .done()
            .build();

        let page = repo.find_all(params).await.unwrap();

        assert_eq!(page.size, 5);
        assert_eq!(page.page, 1);
        for user in &page.data {
            assert!(user.name.contains("y"));
        }
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_slice_with_filters(pool: Pool) {
        let repo = TestUserRepo { pool };

        let params = ParamsBuilder::new()
            .slice()
            .page(1, 10)
            .done()
            .filter()
            .gt("age", 25)
            .done()
            .sort()
            .desc("age")
            .done()
            .build();

        let page = repo.find_with_filters(params).await.unwrap();

        assert_eq!(page.size, 10);
        assert_eq!(page.page, 1);
        for user in &page.data {
            if let Some(age) = user.age {
                assert!(age > 25);
            }
        }
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_slice_with_multiple_filters(pool: Pool) {
        let repo = TestUserRepo { pool };

        #[rustfmt::skip]
        let params = ParamsBuilder::new()
            .slice()
                .page(1, 5)
                .done()
            .filter()
                .gte("age", FilterValue::UInt(20))
                .like_pattern("name", "%e%")
                .done()
            .sort()
                .asc("name")
                .done()
            .build();

        let page = repo.find_with_filters(params).await.unwrap();

        // Should find users with age >= 20 AND name containing 'e'
        for user in &page.data {
            assert!(user.age.unwrap_or(0) >= 20);
            assert!(user.name.contains('e'));
        }
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_slice_with_tuple_and_search(pool: Pool) {
        let repo = TestUserRepo { pool };

        let params = ParamsBuilder::new()
            .slice()
            .page(1, 5)
            .done()
            .sort()
            .asc("name")
            .done()
            .build();

        let page = repo
            .find_by_name_pattern("%y%".to_string(), params)
            .await
            .unwrap();

        // Should find users with 'y' in their names
        for (name, _) in &page.data {
            assert!(name.to_lowercase().contains('y'));
        }
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_slice_with_age_and_name_params(pool: Pool) {
        let repo = TestUserRepo { pool };

        let params = ParamsBuilder::new()
            .slice()
                .page(1, 10)
                .done()
            .sort()
                .desc("age")
                .done()
            .build();

        let page = repo
            .find_by_age_and_name(25, "%e%".to_string(), params)
            .await
            .unwrap();

        //Should find users with age >= 25 AND name containing 'e'
        for user in &page.data {
            assert!(user.age.unwrap_or(0) >= 25);
            assert!(user.name.to_lowercase().contains('e'));
        }
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_slice_with_age_range_params(pool: Pool) {
        let repo = TestUserRepo { pool };

        let params = ParamsBuilder::new()
            .slice()
            .page(1, 10)
            .done()
            .sort()
            .asc("age")
            .done()
            .build();

        let page = repo
            .find_by_age_range_and_name(25, 40, "%a%".to_string(), params)
            .await
            .unwrap();

        // Should find users with age between 25-40 AND name containing 'a'
        for user in &page.data {
            let age = user.age.unwrap_or(0);
            assert!(age >= 25 && age <= 40);
            assert!(user.name.to_lowercase().contains('a'));
        }
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_slice_with_optional_filter_params(pool: Pool) {
        let repo = TestUserRepo { pool };

        // Test with Some(min_age)
        let params = ParamsBuilder::new()
            .slice()
            .page(1, 5)
            .done()
            .build();

        let page = repo
            .find_with_optional_age_filter(Some(30),None, "%e%".to_string(), params)
            .await
            .unwrap();

        for user in &page.data {
            assert!(user.age.unwrap_or(0) >= 30);
            assert!(user.name.to_lowercase().contains('e'));
        }

        // Test with None (no age filter)
        let params2 = ParamsBuilder::new()
            .slice()
            .page(1, 10)
            .done()
            .build();

        let page2 = repo
            .find_with_optional_age_filter(None,None, "%a%".to_string(), params2)
            .await
            .unwrap();

        for user in &page2.data {
            assert!(user.name.to_lowercase().contains('a'));
            // No age requirement when None
        }
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_slice_pagination_flow(pool: Pool) {
        let repo = TestUserRepo { pool };

        // First page
        let params = ParamsBuilder::new()
            .slice()
            .page(1, 3)
            .done()
            .sort()
            .asc("id")
            .done()
            .build();

        let first_page = repo.find_all(params).await.unwrap();
        assert_eq!(first_page.data.len(), 3);
        assert!(first_page.has_next);

        // Second page
        let params2 = ParamsBuilder::new()
            .slice()
            .page(2, 3)
            .done()
            .sort()
            .asc("id")
            .done()
            .build();

        let second_page = repo.find_all(params2).await.unwrap();
        assert_eq!(second_page.data.len(), 3);

        // Should not overlap
        let first_ids: Vec<i64> = first_page.data.iter().map(|u| u.id).collect();
        let second_ids: Vec<i64> = second_page.data.iter().map(|u| u.id).collect();

        for id in second_ids {
            assert!(!first_ids.contains(&id));
        }
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_slice_edge_cases(pool: Pool) {
        let repo = TestUserRepo { pool };

        // Test page beyond available data
        let params = ParamsBuilder::new()
            .slice()
            .page(10, 10)  // Page 10 with 20 total users
            .done()
            .build();

        let page = repo.find_all(params).await.unwrap();
        assert_eq!(page.data.len(), 0);
        assert!(!page.has_next);

        // Test very large page size
        let params2 = ParamsBuilder::new()
            .slice()
            .page(1, 1000)
            .done()
            .build();

        let page2 = repo.find_all(params2).await.unwrap();
        assert_eq!(page2.data.len(), 20); // All users
        assert!(!page2.has_next);
    }
}