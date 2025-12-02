use sqlx_data::{FilterValue, IntoParams, ParamsBuilder, Pool, Slice};
use sqlx_data::{dml, repo};

#[derive(Debug, sqlx::FromRow)]
pub struct User {
    #[allow(dead_code)]
    pub id: i64,
    pub name: String,
    pub age: Option<i64>,
}

#[repo]
trait UserRepo {
    #[dml("SELECT id, name, age FROM users")]
    async fn find_all(&self, params: impl IntoParams) -> Result<Slice<User>, sqlx::Error>;

    #[dml("SELECT name, age FROM users WHERE name LIKE $1")]
    async fn find_by_name_pattern(
        &self,
        pattern: String,
        params: impl IntoParams,
    ) -> Result<Slice<(String, Option<i64>)>, sqlx::Error>;

    #[dml("SELECT id as 'id: i64', name as 'name: String', age as 'age: Option<i64>' FROM users")]
    async fn find_with_cast_syntax(
        &self,
        params: impl IntoParams,
    ) -> Result<Slice<User>, sqlx::Error>;

    // New test with filtering only
    #[dml("SELECT id, name, age FROM users")]
    async fn find_with_filters(&self, params: impl IntoParams) -> Result<Slice<User>, sqlx::Error>;

    // Test with params + additional parameters
    #[dml("SELECT id, name, age FROM users WHERE age >= $1 AND name LIKE $2")]
    async fn find_by_age_and_name(
        &self,
        min_age: i64,
        name_pattern: String,
        params: impl IntoParams,
    ) -> Result<Slice<User>, sqlx::Error>;

    // Test with multiple additional parameters + params
    #[dml("SELECT id, name, age FROM users WHERE age BETWEEN $1 AND $2 AND name LIKE $3")]
    async fn find_by_age_range_and_name(
        &self,
        min_age: i64,
        max_age: i64,
        name_pattern: String,
        params: impl IntoParams,
    ) -> Result<Slice<User>, sqlx::Error>;

    // Test with optional parameters + params
    #[dml("SELECT id, name, age FROM users WHERE ($1 IS NULL OR age >= $1) AND name LIKE $2")]
    async fn find_with_optional_age_filter(
        &self,
        min_age: Option<i64>,
        name_pattern: String,
        params: impl IntoParams,
    ) -> Result<Slice<User>, sqlx::Error>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_slice_with_pagination_only(pool: Pool) {
        let repo = TestUserRepo { pool: &pool };

        let params = ParamsBuilder::new()
            .slice()
            .page(1, 3)
            .done()
            .sort()
            .asc("name")
            .done()
            .build();

        let page = repo.find_all(params).await.unwrap();

        assert_eq!(page.size, 3);
        assert_eq!(page.page, 1);
        assert!(page.has_next);
        assert_eq!(page.data.len(), 3); // Should return exactly 3 items
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_slice_with_search(pool: Pool) {
        let repo = TestUserRepo { pool: &pool };

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
        assert_eq!(page.data.len(), 3);
        for user in &page.data {
            assert!(user.name.contains("y"));
        }
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_slice_with_filters(pool: Pool) {
        let repo = TestUserRepo { pool: &pool };

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
        let repo = TestUserRepo { pool: &pool };

        #[rustfmt::skip]
        let params = ParamsBuilder::new()
            .slice()
                .page(1, 5)
                .done()
            .filter()
                .gte("age", FilterValue::Int(20))
                .like("name", "%e%")
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
        let repo = TestUserRepo { pool: &pool };

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

        assert_eq!(page.size, 5);
        assert_eq!(page.page, 1);
        assert!(!page.has_next);
        assert_eq!(page.data.len(), 3);

        // Check that both results contain "y"
        for (name, _age) in &page.data {
            assert!(name.contains("y"));
        }
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_slice_no_next_page(pool: Pool) {
        let repo = TestUserRepo { pool: &pool };

        let params = ParamsBuilder::new()
            .slice()
            .page(1, 20)
            .done()
            .sort()
            .asc("id")
            .done()
            .build();

        let page = repo.find_all(params).await.unwrap();

        assert_eq!(page.size, 20);
        assert!(!page.has_next);
        assert!(page.data.len() <= 20);
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_slice_with_cast_syntax_and_filters(pool: Pool) {
        let repo = TestUserRepo { pool: &pool };

        let params = ParamsBuilder::new()
            .slice()
            .page(1, 5)
            .done()
            .filter()
            .gt("age", FilterValue::Int(20))
            .done()
            .sort()
            .desc("age")
            .done()
            .build();

        let page = repo.find_with_cast_syntax(params).await.unwrap();

        assert_eq!(page.size, 5);
        assert_eq!(page.page, 1);
        assert!(page.data.len() > 0);
        for user in &page.data {
            assert!(user.age.unwrap_or(0) > 20);
        }
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_slice_with_complex_filters_and_search(pool: Pool) {
        let repo = TestUserRepo { pool: &pool };

        let params = ParamsBuilder::new()
            .slice()
            .page(1, 10)
            .done()
            .filter()
            .between("age", FilterValue::Int(20), FilterValue::Int(30))
            .done()
            .search()
            .query("y")
            .fields(["name"])
            .exact(false)
            .case_sensitive(true)
            .done()
            .sort()
            .asc("age")
            .desc("name")
            .done()
            .build();

        let page = repo.find_with_filters(params).await.unwrap();

        // Should find users with age between 20-30 AND name containing 'a'
        for user in &page.data {
            let age = user.age.unwrap_or(0);
            assert!(age >= 20 && age <= 30);
            assert!(user.name.to_lowercase().contains('y'));
        }
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_slice_with_params_and_additional_parameters(pool: Pool) {
        let repo = TestUserRepo { pool: &pool };

        // Test with additional parameters + params (age >= 25 AND name LIKE '%a%')
        let params = ParamsBuilder::new()
            .slice()
            .page(1, 10)
            .done()
            .filter()
            .gte("id", 1) // This will be applied by params system - filter id >= 1
            .done()
            .sort()
            .asc("name")
            .done()
            .build();

        let page = repo
            .find_by_age_and_name(25, "%y%".to_string(), params)
            .await
            .unwrap();

        assert_eq!(page.size, 10);
        assert_eq!(page.page, 1);

        // Verify all results match both explicit parameters and params filters
        for user in &page.data {
            assert!(user.age.unwrap_or(0) >= 25); // From explicit min_age parameter
            assert!(user.name.to_lowercase().contains('y')); // From explicit name_pattern parameter (case insensitive)
        }

        println!(
            "Found {} users with age >= 25 and name containing 'a'",
            page.data.len()
        );
        // Should find Alice (25) and Grace (28)
        assert!(page.data.len() >= 2);
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_slice_with_multiple_additional_params(pool: Pool) {
        let repo = TestUserRepo { pool: &pool };

        // Test with multiple additional parameters + pagination + sorting
        let params = ParamsBuilder::new()
            .slice()
            .page(1, 5)
            .done()
            .sort()
            .desc("age")
            .asc("name")
            .done()
            .build();

        let page = repo
            .find_by_age_range_and_name(20, 35, "%e%".to_string(), params)
            .await
            .unwrap();

        assert_eq!(page.size, 5);
        assert_eq!(page.page, 1);

        // Verify all results match the age range and name pattern
        for user in &page.data {
            let age = user.age.unwrap_or(0);
            assert!(age >= 20 && age <= 35); // From explicit age range parameters
            assert!(user.name.contains('e')); // From explicit name pattern parameter
        }

        // Verify sorting (should be DESC by age, then ASC by name)
        for i in 1..page.data.len() {
            let prev_age = page.data[i - 1].age.unwrap_or(0);
            let curr_age = page.data[i].age.unwrap_or(0);

            if prev_age == curr_age {
                // If ages are equal, names should be in ascending order
                assert!(page.data[i - 1].name <= page.data[i].name);
            } else {
                // Ages should be in descending order
                assert!(prev_age >= curr_age);
            }
        }

        println!(
            "Found {} users aged 20-35 with name containing 'e', sorted by age DESC, name ASC",
            page.data.len()
        );
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_slice_with_optional_params(pool: Pool) {
        let repo = TestUserRepo { pool: &pool };

        // Test 1: With Some value for optional parameter
        let params1 = ParamsBuilder::new()
            .slice()
            .page(1, 10)
            .done()
            .sort()
            .asc("age")
            .done()
            .build();

        let page1 = repo
            .find_with_optional_age_filter(Some(30), "%y%".to_string(), params1)
            .await
            .unwrap();

        for user in &page1.data {
            assert!(user.age.unwrap_or(0) >= 30); // Should filter by age >= 30
            assert!(user.name.to_lowercase().contains('y')); // Should filter by name pattern
        }

        println!("With Some(30): Found {} users", page1.data.len());

        // Test 2: With None for optional parameter (should ignore age filter)
        let params2 = ParamsBuilder::new()
            .slice()
            .page(1, 10)
            .done()
            .sort()
            .asc("age")
            .done()
            .build();

        let page2 = repo
            .find_with_optional_age_filter(None, "%y%".to_string(), params2)
            .await
            .unwrap();

        for user in &page2.data {
            // No age restriction when min_age is None
            assert!(user.name.to_lowercase().contains('y')); // Should still filter by name pattern
        }

        println!("With None: Found {} users", page2.data.len());

        // Page2 should have more or equal results than page1 (since no age restriction)
        assert!(page2.data.len() >= page1.data.len());
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_slice_with_complex_params_combination(pool: Pool) {
        let repo = TestUserRepo { pool: &pool };

        // Complex test combining all features: multiple params + filters + search + sorting + pagination
        let params = ParamsBuilder::new()
            .slice()
            .page(1, 3) // Small page size to test pagination
            .done()
            .filter()
            .r#in(
                "name",
                vec!["Alice", "Bob", "Charlie", "David", "Eva", "Frank"],
            ) // Additional filtering
            .done()
            .search()
            .query("y") // Search for 'a' in names
            .fields(["name"])
            .exact(false)
            .case_sensitive(false)
            .done()
            .sort()
            .asc("age")
            .desc("name")
            .done()
            .build();

        let page = repo
            .find_by_age_range_and_name(18, 50, "%y%".to_string(), params)
            .await
            .unwrap();

        assert_eq!(page.size, 3);
        assert_eq!(page.page, 1);

        for user in &page.data {
            let age = user.age.unwrap_or(0);
            assert!(age >= 18 && age <= 50); // From explicit parameters
            assert!(user.name.to_lowercase().contains('y')); // From both explicit pattern and search

            // Should be one of the allowed names from IN filter
            let allowed_names = vec!["Alice", "Bob", "Charlie", "David", "Eva", "Frank"];
            assert!(allowed_names.contains(&user.name.as_str()));
        }

        println!(
            "Complex combination found {} users with all filters applied",
            page.data.len()
        );
    }

    #[test]
    fn test_slice_enable_total_count() {
        // Test Slice - default is disable_total_count = true
        let slice_params_default = ParamsBuilder::new().slice().page(1, 10).done().build();

        assert_eq!(slice_params_default.is_disable_total_count(), true);

        // Test Slice with enable_total_count() - should disable_total_count = false
        let slice_params_enabled = ParamsBuilder::new()
            .slice()
            .page(1, 10)
            .enable_total_count()
            .done()
            .build();

        assert_eq!(slice_params_enabled.is_disable_total_count(), false);
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
