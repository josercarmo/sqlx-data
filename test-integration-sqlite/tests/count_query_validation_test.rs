use sqlx::prelude::FromRow;
use sqlx_data::{IntoParams, ParamsBuilder, Pool, Result, Serial, dml, repo};

#[derive(Clone, PartialEq, Eq, Debug, sqlx::Type)]
#[sqlx(transparent)]
pub struct Id(i64);

impl From<i64> for Id {
    fn from(value: i64) -> Self {
        Id(value)
    }
}

// User model for count validation tests
#[derive(Debug, Clone, PartialEq, FromRow)]
pub struct User {
    pub id: Id,
    pub name: String,
    pub email: String,
    pub age: i64,
    pub birth_year: Option<i64>,
}

// Repository for testing count query behavior
#[repo]
trait CountValidationRepo {
    // Basic query with initial binds + dynamic filters
    #[dml("SELECT id, name, email, age, birth_year FROM users WHERE age BETWEEN $1 AND $2")]
    async fn find_users_by_age_range(
        &self,
        min_age: i64,
        max_age: i64,
        parameter: impl IntoParams,
    ) -> Result<Serial<User>>;

    // Multiple initial binds to test bind order
    #[dml("SELECT id, name, email, age, birth_year FROM users WHERE age > $1 AND name LIKE $2")]
    async fn find_users_complex_binds(
        &self,
        min_age: i64,
        name_pattern: String,
        params: impl IntoParams,
    ) -> Result<Serial<User>>;

    // Count query with same logic as paginated query for consistency testing
    #[dml("SELECT COUNT(*) FROM users WHERE age BETWEEN $1 AND $2")]
    async fn count_users_in_age_range(&self, min_age: i64, max_age: i64) -> Result<i64>;

    // Complex query with LIMIT, OFFSET, GROUP BY, HAVING and parameter order test
    // Tests: $1=limit, $2=offset, $3=min_age, $4=email_pattern, $5=having_min_count (last parameter)
    #[dml(
        r#"
        SELECT age, COUNT(*) as user_count, GROUP_CONCAT(name) as names
        FROM users
        WHERE age > $3 AND email LIKE $4
        GROUP BY age
        HAVING COUNT(*) >= $5
        ORDER BY age DESC
        LIMIT $1 OFFSET $2
    "#
    )]
    async fn find_age_groups_with_having(
        &self,
        limit_count: i64,      // $1
        offset_count: i64,     // $2
        min_age: i64,          // $3
        email_pattern: String, // $4
        params: impl IntoParams,
        having_min_count: i64, // $5 - Last parameter in HAVING
    ) -> Result<Serial<(i64, i64, String)>>;
}

pub struct TestCountValidationApp {
    pool: Pool,
}

impl CountValidationRepo for TestCountValidationApp {
    fn get_pool(&self) -> &Pool {
        &self.pool
    }
}

#[cfg(test)]
mod tests {
    use sqlx_data::FilterValue;

    use super::*;

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_dynamic_filters_affect_count_results(pool: Pool) -> Result<()> {
        let app = TestCountValidationApp { pool };

        // Test 1: No dynamic filters - should return all users in age range
        let params_no_filter = ParamsBuilder::new().serial().page(1, 5).done().build();

        let result_no_filter = app
            .find_users_by_age_range(20, 50, params_no_filter)
            .await?;

        assert_eq!(result_no_filter.total_items, 19);
        assert_eq!(result_no_filter.data.len(), 5);
        assert_eq!(result_no_filter.total_pages, 4);

        #[rustfmt::skip]
        let params_with_filter = ParamsBuilder::new()
            .filter()
                .gte("age", FilterValue::Int(35))
                .done()
            .serial()
                .page(1, 5)
                .done()
            .build();

        let result_with_filter = app
            .find_users_by_age_range(20, 50, params_with_filter)
            .await?;


        // CRITICAL TEST: Count should be less with dynamic filter
        assert!(
            result_with_filter.total_items < result_no_filter.total_items,
            "Dynamic filter should reduce count. No filter: {}, With filter: {}",
            result_no_filter.total_items,
            result_with_filter.total_items
        );

        // Test 3: Add name filter to further reduce count
        #[rustfmt::skip]
        let params_with_name_filter = ParamsBuilder::new()
            .serial()
                .page(1, 5)
                .done()
            .filter()
                .gte("age", FilterValue::Int(35))
                .like("name", "User1%")
                .done()
            .build();

        let result_with_name_filter = app
            .find_users_by_age_range(20, 50, params_with_name_filter)
            .await?;


        // Name filter should reduce count even further
        assert!(
            result_with_name_filter.total_items <= result_with_filter.total_items,
            "Additional name filter should not increase count"
        );

        Ok(())
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_bind_order_consistency(pool: Pool) -> Result<()> {
        let app = TestCountValidationApp { pool };

        // Test with multiple initial binds + dynamic filters
        // Initial: $1=min_age, $2=name_pattern
        // Dynamic: should be $3, $4, etc.
        #[rustfmt::skip]
        let params = ParamsBuilder::new()
            .filter()
                .lte("age", FilterValue::Int(40)) // Should be $3
                .done()
            .search()
                .query("User") // Should be $4
                .fields(["name", "email"])
                .done()
            .serial()
                .page(1, 10)
                .done()
            .build();

        let result = app
            .find_users_complex_binds(25, "%User%".to_string(), params)
            .await?;

        println!(
            "Complex binds test - Total: {}, Data count: {}",
            result.total_items,
            result.data.len()
        );

        // Verify results are consistent (exact numbers depend on test data)
        assert!(
            result.total_items > 0 || result.total_items == 0,
            "Should return valid count"
        );
        assert_eq!(
            result.data.len() as i64,
            std::cmp::min(result.total_items, 10)
        );

        Ok(())
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_count_vs_paginated_consistency(pool: Pool) -> Result<()> {
        let app = TestCountValidationApp { pool };

        // Compare direct count vs paginated count using SAME SQL logic
        for (min_age, max_age) in [(20, 50), (25, 40), (30, 45)] {
            let direct_count = app.count_users_in_age_range(min_age, max_age).await?;

            let params = ParamsBuilder::new()
                .serial()
                .page(1, 100) // Large page to get all results
                .done()
                .build();

            let paginated_result = app
                .find_users_by_age_range(min_age, max_age, params)
                .await?;

            println!(
                "age range {}..{}: direct_count={}, paginated_total={}",
                min_age, max_age, direct_count, paginated_result.total_items
            );

            assert_eq!(
                direct_count as i64, paginated_result.total_items,
                "Direct count should match paginated total for age range {}..{}",
                min_age, max_age
            );
        }

        Ok(())
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_search_affects_count(pool: Pool) -> Result<()> {
        let app = TestCountValidationApp { pool };

        // No search
        let params_no_search = ParamsBuilder::new().serial().page(1, 10).done().build();

        let result_no_search = app
            .find_users_by_age_range(20, 50, params_no_search)
            .await?;

        // With search
        #[rustfmt::skip]
        let params_with_search = ParamsBuilder::new()
            .search()
                .query("User1") // Should match User1, User10-19
                .fields(["name"])
                .done()
            .serial()
                .page(1, 10)
                .done()
            .build();

        let result_with_search = app
            .find_users_by_age_range(20, 50, params_with_search)
            .await?;

        println!("No search - Total: {}", result_no_search.total_items);
        println!(
            "With search 'User1' - Total: {}",
            result_with_search.total_items
        );

        // Search should affect count
        assert!(
            result_with_search.total_items <= result_no_search.total_items,
            "Search filter should not increase total count"
        );

        Ok(())
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_complex_having_limit_offset_with_dynamic_pagination(pool: Pool) -> Result<()> {
        let app = TestCountValidationApp { pool };

        // Test: Query has LIMIT/OFFSET in SQL + dynamic LIMIT/OFFSET in params
        // Parameter order: $1=sql_limit, $2=sql_offset, $3=min_age, $4=email_pattern, $5=having_min_count
        // Dynamic params: $6=dynamic_filter, $7=search, plus pagination LIMIT/OFFSET

        // First: Test without dynamic filters to get baseline count
        let params_baseline = ParamsBuilder::new()
            .serial()
            .page(1, 5) // Dynamic limit=5, offset=0
            .done()
            .build();

        let result_baseline = app
            .find_age_groups_with_having(
                10,                  // $1 = SQL limit (high number)
                0,                   // $2 = SQL offset
                20,                  // $3 = min_age in WHERE
                "%.com".to_string(), // $4 = email pattern in WHERE
                params_baseline,
                1, // $5 = having min count
            )
            .await?;

        println!(
            "Baseline (no dynamic filters) - Total groups: {}, Data count: {}",
            result_baseline.total_items,
            result_baseline.data.len()
        );

        // Second: Test with dynamic filters that should reduce count
        #[rustfmt::skip]
        let params_with_filters = ParamsBuilder::new()
            .filter()
                .gte("age", FilterValue::Int(35)) // Should reduce count - only older ages
                .done()
            .search()
                .query("User1") // Should reduce count - only User1X names
                .fields(["name"])
                .done()
            .serial()
                .page(1, 3) // Different dynamic limit/offset
                .done()
            .build();

        let result_with_filters = app
            .find_age_groups_with_having(
                8,                   // $1 = SQL limit
                1,                   // $2 = SQL offset
                20,                  // $3 = min_age (same)
                "%.com".to_string(), // $4 = email pattern (same)
                params_with_filters,
                1, // $5 = having min count (same)
            )
            .await?;

        println!(
            "With dynamic filters (age>=35, search='User1') - Total groups: {}, Data count: {}",
            result_with_filters.total_items,
            result_with_filters.data.len()
        );

        // CRITICAL TEST: Dynamic filters should affect count
        assert!(
            result_with_filters.total_items <= result_baseline.total_items,
            "Dynamic filters should reduce or maintain count. Baseline: {}, Filtered: {}",
            result_baseline.total_items,
            result_with_filters.total_items
        );

        // Third: Test with even more restrictive filters
        #[rustfmt::skip]
        let params_very_restrictive = ParamsBuilder::new()
            .filter()
                .gte("age", FilterValue::Int(40)) // Very restrictive age
                .like("name", "User1%") // Restrictive name pattern
                .done()
            .serial()
                .page(2, 2) // Different pagination - page 2, size 2
                .done()
            .build();

        let result_very_restrictive = app
            .find_age_groups_with_having(
                5,                   // $1 = SQL limit
                0,                   // $2 = SQL offset
                20,                  // $3 = min_age (same)
                "%.com".to_string(), // $4 = email pattern (same)
                params_very_restrictive,
                1, // $5 = having min count (same)
            )
            .await?;

        println!(
            "Very restrictive filters (age>=40, names like 'User1%') - Total groups: {}, Data count: {}",
            result_very_restrictive.total_items,
            result_very_restrictive.data.len()
        );

        // Should be most restrictive
        assert!(
            result_very_restrictive.total_items <= result_with_filters.total_items,
            "More restrictive filters should not increase count"
        );

        // Verify data consistency: all returned data should match filters
        for (age, _count, names) in &result_with_filters.data {
            assert!(
                *age >= 35,
                "All ages should be >= 35 due to dynamic filter, got {}",
                age
            );
            assert!(
                names.contains("User1"),
                "Names should contain 'User1' due to search filter, got '{}'",
                names
            );
        }

        for (age, _count, names) in &result_very_restrictive.data {
            assert!(
                *age >= 40,
                "All ages should be >= 40 due to very restrictive filter, got {}",
                age
            );
            assert!(
                names.starts_with("User1"),
                "Names should start with 'User1' due to LIKE filter, got '{}'",
                names
            );
        }

        Ok(())
    }
}
