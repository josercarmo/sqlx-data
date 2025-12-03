use sqlx::types::BigDecimal;
use sqlx_data::{IntoParams, ParamsBuilder, Pool, Result, Serial, dml, repo};

#[derive(Clone, PartialEq, Eq, Debug, sqlx::Type)]
#[sqlx(transparent)]
pub struct Id(i64);

impl From<i64> for Id {
    fn from(value: i64) -> Self {
        Id(value)
    }
}

// Simple user struct for basic tests with MySQL types
#[derive(Debug, sqlx::FromRow)]
pub struct SimpleUser {
    pub id: i64,
    pub name: String,
    pub age: Option<u8>, // MySQL TINYINT UNSIGNED
}

// Production user struct with MySQL strong typing
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct User {
    pub id: Id,
    pub name: String,
    pub email: String,
    pub age: u8,                 
    pub birth_year: Option<u16>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct UserStats {
    pub name: String,
    pub age: u8,
    pub years_since_birth: Option<u16>,
    pub email_domain: String,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AgeGroup {
    pub age_bracket: String,
    pub user_count: i64,
    pub avg_birth_year: Option<BigDecimal>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct UserProfile {
    pub id: Id,
    pub display_name: String,
    pub age_category: String,
    pub has_birth_year: bool,
}

#[repo]
trait UserRepo {
    // Basic queries
    #[dml("SELECT id, name, age FROM users")]
    async fn find_all(&self, params: impl IntoParams) -> Result<Serial<SimpleUser>>;

    #[dml("SELECT COUNT(*) as count FROM users")]
    async fn count_all_users(&self) -> Result<i64>;

    #[dml("SELECT id, name, age FROM users WHERE age >= ?")]
    async fn find_adults(&self, min_age: u8, params: impl IntoParams)
    -> Result<Serial<SimpleUser>>;

    #[dml("SELECT name, age as 'age: u8' FROM users WHERE name LIKE ?")]
    async fn find_by_name_pattern(
        &self,
        pattern: String,
        params: impl IntoParams,
    ) -> Result<Serial<(String, Option<u8>)>>;

    #[dml(
        "SELECT id as 'id!: Id', name, email, age, birth_year
           FROM users
           WHERE age BETWEEN ? AND ?
           AND name LIKE ?
           AND (birth_year IS NULL OR birth_year > ?)"
    )]
    #[instrument(skip(self))]
    async fn find_users_complex_filter(
        &self,
        min_age: u8,
        max_age: u8,
        name_pattern: String,
        min_birth_year: u16,
        params: impl IntoParams,
    ) -> Result<Serial<User>>;

    #[dml(
        "SELECT
            CASE
                WHEN age < 25 THEN 'young'
                WHEN age < 40 THEN 'middle'
                ELSE 'senior'
            END as age_bracket,
            COUNT(*) as user_count,
            AVG(birth_year) as avg_birth_year
         FROM users
         WHERE age IS NOT NULL
         GROUP BY age_bracket"
    )]
    async fn get_age_groups(&self, params: impl IntoParams) -> Result<Serial<AgeGroup>>;

    #[dml(
        "SELECT
            name,
            age,
            (2024 - birth_year) as 'years_since_birth?: u16',
            SUBSTRING_INDEX(email, '@', -1) as 'email_domain!: String'
         FROM users
         WHERE birth_year IS NOT NULL"
    )]
    async fn get_user_stats(&self, params: impl IntoParams) -> Result<Serial<UserStats>>;

    #[dml(
        "SELECT
            id,
            CONCAT(name, ' (', age, ')') as 'display_name!: String',
            IF(age >= 30, 'adult', 'young') as age_category,
            (birth_year IS NOT NULL) as 'has_birth_year!: bool'
         FROM users"
    )]
    async fn get_user_profiles(&self, params: impl IntoParams) -> Result<Serial<UserProfile>>;

    #[dml(
        "SELECT id as 'id!: Id', name, email, age as 'age: u8', birth_year as 'birth_year: u16'
         FROM users
         ORDER BY
             CASE WHEN birth_year IS NULL THEN 1 ELSE 0 END,
             birth_year DESC,
             age ASC"
    )]
    async fn find_users_custom_sort(&self, params: impl IntoParams) -> Result<Serial<User>>;

    #[dml(
        "SELECT id as 'id!: Id', name, email, age, birth_year FROM users WHERE age < 25
         UNION ALL
         SELECT id as 'id!: Id', name, email, age, birth_year FROM users WHERE age > 40
         ORDER BY age"
    )]
    #[instrument(skip(self))]
    async fn find_young_and_senior(&self, params: impl IntoParams) -> Result<Serial<User>>;
}

// Test application
pub struct TestApp {
    pool: Pool,
}

impl UserRepo for TestApp {
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
    async fn test_basic_serial_pagination(pool: Pool) {
        let repo = TestApp { pool };
        // Test first page
        let params = ParamsBuilder::new().serial().page(1, 5).done().build();

        let result = repo.find_all(params).await.unwrap();
        assert_eq!(result.page, 1);
        assert!(result.page < result.total_pages); // has_next_page
        assert_eq!(result.page, 1); // has_previous_page = false for page 1
        assert_eq!(result.total_pages, 4); // 20 users / 5 per page
        assert_eq!(result.data.len(), 5);

        // Test second page
        let params = ParamsBuilder::new().serial().page(2, 5).done().build();

        let result = repo.find_all(params).await.unwrap();
        assert_eq!(result.data.len(), 5);
        assert_eq!(result.page, 2);
        assert!(result.page < result.total_pages); // has_next_page
        assert!(result.page > 1); // has_previous_page = true for page > 1
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_filtered_serial_pagination(pool: Pool) {
        let repo = TestApp { pool };

        // Filter adults (age >= 25) with MySQL unsigned comparison
        let params = ParamsBuilder::new().serial().page(1, 3).done().build();

        let result = repo.find_adults(25, params).await.unwrap();
        assert!(result.data.len() > 0);
        assert!(result.data.len() <= 3);

        // All returned users should be adults
        for user in &result.data {
            if let Some(age) = user.age {
                assert!(age >= 25);
            }
        }
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_name_pattern_search_mysql(pool: Pool) {
        let repo = TestApp { pool };

        let params = ParamsBuilder::new().serial().page(1, 10).done().build();

        // MySQL LIKE with wildcard
        let result = repo
            .find_by_name_pattern("A%".to_string(), params)
            .await
            .unwrap();

        for (name, _age) in &result.data {
            assert!(name.starts_with('A'));
        }
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_complex_filter_pagination_mysql(pool: Pool) {
        use tracing_subscriber;

        // Initialize tracing subscriber to see actual output (filter out sqlx noise)
        //use tracing_subscriber::{EnvFilter, FmtSubscriber};
        use tracing_subscriber::FmtSubscriber;

        //let filter = EnvFilter::new("debug").add_directive("sqlx=debug".parse().unwrap());

        let subscriber = FmtSubscriber::builder()
            .with_max_level(tracing::Level::INFO)
        //    .with_env_filter(filter)
            .with_span_events(
                tracing_subscriber::fmt::format::FmtSpan::NEW
                    | tracing_subscriber::fmt::format::FmtSpan::CLOSE,
            )
            .finish();

        let _ = tracing::subscriber::set_global_default(subscriber);

        let repo = TestApp { pool };
        let params = ParamsBuilder::new().serial().page(1, 5).done().build();

        // Complex MySQL filter with multiple conditions
        let result = repo
            .find_users_complex_filter(
                20,                // min_age
                40,                // max_age
                "%a%".to_string(), // name contains 'a'
                1990,              // min_birth_year
                params,
            )
            .await
            .unwrap();

        for user in &result.data {
            assert!(user.age >= 20 && user.age <= 40);
            assert!(user.name.to_lowercase().contains('a'));
            if let Some(birth_year) = user.birth_year {
                assert!(birth_year > 1990);
            }
        }
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_mysql_aggregation_pagination(pool: Pool) {
        let repo = TestApp { pool };

        let params = ParamsBuilder::new().serial().page(1, 10).done().build();

        let result = repo.get_age_groups(params).await.unwrap();
        assert!(result.data.len() > 0);

        for group in &result.data {
            assert!(!group.age_bracket.is_empty());
            assert!(group.user_count > 0);
            // avg_birth_year might be None for some groups
        }
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_mysql_computed_fields(pool: Pool) {
        let repo = TestApp { pool };

        let params = ParamsBuilder::new().serial().page(1, 5).done().build();

        let result = repo.get_user_stats(params).await.unwrap();

        for stats in &result.data {
            assert!(!stats.name.is_empty());
            assert!(stats.age > 0);
            assert!(!stats.email_domain.is_empty());

            if let Some(years) = stats.years_since_birth {
                assert!(years >= stats.age as u16); // Years since birth >= current age
            }
        }
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_mysql_conditional_expressions(pool: Pool) {
        let repo = TestApp { pool };

        let params = ParamsBuilder::new().serial().page(1, 5).done().build();

        let result = repo.get_user_profiles(params).await.unwrap();

        for profile in &result.data {
            assert!(profile.display_name.contains('('));
            assert!(profile.display_name.contains(')'));
            assert!(profile.age_category == "adult" || profile.age_category == "young");
            // has_birth_year is boolean - no specific assertion needed
        }
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_mysql_custom_sort_pagination(pool: Pool) {
        let repo = TestApp { pool };

        let params = ParamsBuilder::new().serial().page(1, 10).done().build();

        let result = repo.find_users_custom_sort(params).await.unwrap();

        // Verify MySQL-specific sorting: NULL birth_years last, then by birth_year DESC, then age ASC
        let mut has_non_null_birth_year = false;
        let mut last_birth_year: Option<u16> = None;
        let mut last_age_for_same_birth_year: Option<u8> = None;

        for user in &result.data {
            if user.birth_year.is_some() {
                has_non_null_birth_year = true;

                if let Some(last_by) = last_birth_year {
                    if user.birth_year.unwrap() == last_by {
                        // Same birth year - age should be ascending
                        if let Some(last_age) = last_age_for_same_birth_year {
                            assert!(user.age >= last_age);
                        }
                    } else {
                        // Different birth year - should be descending
                        assert!(user.birth_year.unwrap() <= last_by);
                    }
                }

                last_birth_year = user.birth_year;
                last_age_for_same_birth_year = Some(user.age);
            } else {
                // NULL birth years should come after non-NULL ones
                assert!(!has_non_null_birth_year || last_birth_year.is_some());
            }
        }
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_mysql_union_queries(pool: Pool) {
        let repo = TestApp { pool };

        let params = ParamsBuilder::new()
            .serial()
                .page(1, 20)
                .done()
            .build();

        let result = repo.find_young_and_senior(params).await.unwrap();

        // Should only have users under 25 or over 40
        for user in &result.data {
            assert!(user.age < 25 || user.age > 40);
        }

        // Should be sorted by age due to ORDER BY in UNION
        let mut last_age: Option<u8> = None;
        for user in &result.data {
            if let Some(last) = last_age {
                assert!(user.age >= last);
            }
            last_age = Some(user.age);
        }
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_mysql_pagination_edge_cases(pool: Pool) {
        let repo = TestApp { pool };

        // Test large page size
        let params = ParamsBuilder::new().serial().page(1, 100).done().build();

        let result = repo.find_all(params).await.unwrap();
        assert_eq!(result.data.len(), 20); // All 20 users
        assert_eq!(result.page, 1);
        assert_eq!(result.page, result.total_pages); // has_next_page = false when page == total_pages

        // Test page beyond data
        let params = ParamsBuilder::new().serial().page(10, 5).done().build();

        let result = repo.find_all(params).await.unwrap();
        assert_eq!(result.data.len(), 0);
        assert_eq!(result.page, 10);
        assert_eq!(result.total_pages, 4); // 20 users / 5 per page = 4 total pages
    }
}
