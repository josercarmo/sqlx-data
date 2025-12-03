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

#[derive(Debug, Clone, PartialEq, FromRow)]
pub struct User {
    pub id: Id,
    pub name: String,
    pub email: String,
    pub age: u8,
    pub birth_year: Option<u16>,
}

#[repo]
trait CountValidationRepo {
    #[dml("SELECT id, name, email, age, birth_year FROM users WHERE age BETWEEN ? AND ?")]
    async fn find_users_by_age_range(
        &self,
        min_age: u8,
        max_age: u8,
        parameter: impl IntoParams,
    ) -> Result<Serial<User>>;

    #[dml("SELECT id, name, email, age, birth_year FROM users WHERE age > ? AND name LIKE ?")]
    async fn find_users_complex_binds(
        &self,
        min_age: u8,
        name_pattern: String,
        params: impl IntoParams,
    ) -> Result<Serial<User>>;

    #[dml("SELECT COUNT(*) FROM users WHERE age BETWEEN ? AND ?")]
    async fn count_users_in_age_range(&self, min_age: u8, max_age: u8) -> Result<i64>;

    #[dml(
        r#"
        SELECT age, COUNT(*) as user_count, GROUP_CONCAT(name) as names
        FROM users
        WHERE age > ? AND email LIKE ?
        GROUP BY age
        HAVING COUNT(*) >= ?
        ORDER BY age DESC
        LIMIT ? OFFSET ?
    "#
    )]
    async fn find_age_groups_with_having(
        &self,
        min_age: u8,
        email_pattern: String,
        having_min_count: i64,
        limit_count: i64,
        offset_count: i64,
        params: impl IntoParams,
    ) -> Result<Serial<(u8, i64, Option<String>)>>;
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
    use super::*;
    use sqlx_data::FilterValue;

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_dynamic_filters_affect_count_results(pool: Pool) -> Result<()> {
        let app = TestCountValidationApp { pool };

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
                .gte("age", FilterValue::UInt(35))
                .done()
            .serial()
                .page(1, 5)
                .done()
            .build();

        let result_with_filter = app
            .find_users_by_age_range(20, 50, params_with_filter)
            .await?;

        assert!(
            result_with_filter.total_items < result_no_filter.total_items,
            "Dynamic filter should reduce count. No filter: {}, With filter: {}",
            result_no_filter.total_items,
            result_with_filter.total_items
        );

        #[rustfmt::skip]
        let params_with_name_filter = ParamsBuilder::new()
            .serial()
                .page(1, 5)
                .done()
            .filter()
                .gte("age", FilterValue::UInt(35))
                .like("name", "User1%")
                .done()
            .build();

        let result_with_name_filter = app
            .find_users_by_age_range(20, 50, params_with_name_filter)
            .await?;

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

        let params = ParamsBuilder::new()
            .filter()
            .lte("age", FilterValue::UInt(40))
            .done()
            .search()
            .query("User")
            .fields(["name", "email"])
            .done()
            .serial()
            .page(1, 10)
            .done()
            .build();

        let result = app
            .find_users_complex_binds(25, "%User%".to_string(), params)
            .await?;

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

        for (min_age, max_age) in [(20, 50), (25, 40), (30, 45)] {
            let direct_count = app.count_users_in_age_range(min_age, max_age).await?;

            let params = ParamsBuilder::new().serial().page(1, 100).done().build();

            let paginated_result = app
                .find_users_by_age_range(min_age, max_age, params)
                .await?;

            assert_eq!(
                direct_count, paginated_result.total_items,
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

        let params_no_search = ParamsBuilder::new().serial().page(1, 10).done().build();

        let result_no_search = app
            .find_users_by_age_range(20, 50, params_no_search)
            .await?;

        let params_with_search = ParamsBuilder::new()
            .search()
            .query("User1")
            .fields(["name"])
            .done()
            .serial()
            .page(1, 10)
            .done()
            .build();

        let result_with_search = app
            .find_users_by_age_range(20, 50, params_with_search)
            .await?;

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

        let params_baseline = ParamsBuilder::new().serial().page(1, 5).done().build();

        let result_baseline = app
            .find_age_groups_with_having(20, "%.com".to_string(), 1, 10, 0, params_baseline)
            .await?;

        #[rustfmt::skip]
        let params_with_filters = ParamsBuilder::new()
            .serial()
                .page(1, 3)
                .done()
            .filter()
                .gte("age", FilterValue::UInt(35))
                .done()
            .search()
                .query("User1")
                .fields(["name"])
                .done()
            .build();

        let result_with_filters = app
            .find_age_groups_with_having(20, "%.com".to_string(), 1, 8, 1, params_with_filters)
            .await?;

        assert!(
            result_with_filters.total_items <= result_baseline.total_items,
            "Dynamic filters should reduce or maintain count. Baseline: {}, Filtered: {}",
            result_baseline.total_items,
            result_with_filters.total_items
        );

        for (age, _count, names) in &result_with_filters.data {
            assert!(
                *age >= 35,
                "All ages should be >= 35 due to dynamic filter, got {}",
                age
            );
            assert!(
                names.as_deref().map_or(false, |n| n.contains("User1")),
                "Names should contain 'User1' due to search filter, got '{:?}'",
                names
            );
        }

        Ok(())
    }
}
