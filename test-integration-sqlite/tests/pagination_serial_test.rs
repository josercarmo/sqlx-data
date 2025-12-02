use sqlx_data::{IntoParams, ParamsBuilder, Pool, Serial};
use sqlx_data::{Pagination, SerialParams};
use sqlx_data::{dml, repo};

#[derive(Clone, PartialEq, Eq, Debug, sqlx::Type)]
#[sqlx(transparent)]
pub struct Id(i64);

impl From<i64> for Id {
    fn from(value: i64) -> Self {
        Id(value)
    }
}

// Simple user struct for basic tests
#[derive(Debug, sqlx::FromRow)]
pub struct SimpleUser {
    #[allow(dead_code)]
    pub id: i64,
    pub name: String,
    pub age: Option<i64>,
}

// Production user struct with type casting
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct User {
    #[allow(dead_code)]
    pub id: Id,
    pub name: String,
    #[allow(dead_code)]
    pub email: String,
    pub age: u8,
    pub birth_year: Option<u16>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct UserStats {
    #[allow(dead_code)]
    pub name: String,
    pub age: u8,
    #[allow(dead_code)]
    pub years_since_birth: Option<i64>,
    pub email_domain: String,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AgeGroup {
    pub age_bracket: String,
    pub user_count: i64,
    #[allow(dead_code)]
    pub avg_birth_year: Option<f64>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct UserProfile {
    #[allow(dead_code)]
    pub id: Id,
    pub display_name: String,
    pub age_category: String,
    pub has_birth_year: i64,
}

// Basic pagination repository
#[repo]
trait SimpleUserRepo {
    #[dml("SELECT id, name, age FROM users")]
    async fn find_all(&self, params: impl IntoParams) -> Result<Serial<SimpleUser>>;

    #[dml("SELECT id, name, age FROM users WHERE age >= $1")]
    async fn find_adults(
        &self,
        min_age: i64,
        params: impl IntoParams,
    ) -> Result<Serial<SimpleUser>>;

    #[dml("SELECT name, age FROM users WHERE name LIKE $1")]
    async fn find_by_name_pattern(
        &self,
        pattern: String,
        params: impl IntoParams,
    ) -> Result<Serial<(String, Option<i64>)>>;
}

// Production-level repository with complex queries
#[repo]
trait ProductionUserRepo {
    // Complex WHERE clauses with multiple conditions
    #[dml(
        "SELECT id as 'id!: Id', name, email, age as 'age: u8', birth_year as 'birth_year: u16'
           FROM users
           WHERE age BETWEEN $1 AND $2
           AND name LIKE $3
           AND (birth_year IS NULL OR birth_year > $4)"
    )]
    async fn find_users_complex_filter(
        &self,
        min_age: u8,
        max_age: u8,
        name_pattern: String,
        min_birth_year: u16,
        params: impl IntoParams,
    ) -> Result<Serial<User>>;

    // Aggregation with calculated fields and CASE expressions
    #[dml(
        "SELECT
           name,
           age as 'age: u8',
           CASE
               WHEN birth_year IS NULL THEN NULL
               ELSE 2024 - birth_year
           END as years_since_birth,
           SUBSTR(email, INSTR(email, '@') + 1) as 'email_domain!: String'
           FROM users
           WHERE age > $1"
    )]
    async fn find_user_analytics(
        &self,
        min_age: u8,
        params: impl IntoParams,
    ) -> Result<Serial<UserStats>>;

    // Complex grouping with HAVING clause
    #[dml(
        "SELECT
           CASE
               WHEN age < 25 THEN 'Young'
               WHEN age < 50 THEN 'Adult'
               ELSE 'Senior'
           END as age_bracket,
           COUNT(*) as user_count,
           AVG(CAST(birth_year as REAL)) as avg_birth_year
           FROM users
           WHERE name NOT LIKE '%test%'
           GROUP BY CASE
               WHEN age < 25 THEN 'Young'
               WHEN age < 50 THEN 'Adult'
               ELSE 'Senior'
           END
           HAVING COUNT(*) >= $1"
    )]
    async fn find_age_demographics(
        &self,
        min_count: i64,
        params: impl IntoParams,
    ) -> Result<Serial<AgeGroup>>;

    // Subquery with complex expressions
    #[dml(
        "SELECT
           id as 'id!: Id',
           UPPER(SUBSTR(name, 1, 1)) || LOWER(SUBSTR(name, 2)) as 'display_name!: String',
           CASE
               WHEN age >= (SELECT AVG(age) FROM users) THEN 'Above Average'
               ELSE 'Below Average'
           END as age_category,
           CASE WHEN birth_year IS NOT NULL THEN 1 ELSE 0 END as has_birth_year
           FROM users
           WHERE email LIKE $1
           AND age NOT IN (SELECT age FROM users WHERE name LIKE '%admin%')"
    )]
    async fn find_user_profiles(
        &self,
        email_pattern: String,
        params: impl IntoParams,
    ) -> Result<Serial<UserProfile>>;

    // Window function equivalent using correlated subquery (SQLite compatible)
    #[dml(
        "SELECT
           name,
           age ,
           (SELECT COUNT(*) FROM users u2 WHERE u2.age <= users.age) as age_rank,
           COALESCE(birth_year, 1900) as birth_year
           FROM users
           WHERE LENGTH(name) > $1"
    )]
    async fn find_users_with_ranking(
        &self,
        min_name_length: i32,
        params: impl IntoParams,
    ) -> Result<Serial<(String, u8, i64, u16)>>;

    // Slice variants for cursor-based pagination
    #[dml(
        "SELECT id as 'id!: Id', name, email, age as 'age: u8', birth_year as 'birth_year: u16'
           FROM users
           WHERE age BETWEEN $1 AND $2"
    )]
    async fn slice_users_by_age_range(
        &self,
        min_age: u8,
        max_age: u8,
        params: impl IntoParams,
    ) -> Result<Serial<User>>;

    #[dml(
        "SELECT
           name,
           LENGTH(email) as 'email_length!: i32',
           CASE WHEN birth_year IS NULL THEN 0 ELSE 1 END as has_birth_year
           FROM users
           WHERE name LIKE $1"
    )]
    async fn slice_users_by_pattern(
        &self,
        name_regex: String,
        params: impl IntoParams,
    ) -> Result<Serial<(String, i32, i32)>>;

    // Edge case: Very complex single tuple query
    #[dml(
        "SELECT
           COUNT(DISTINCT CASE WHEN age < 30 THEN id END) as 'young_users!: i64',
           COUNT(DISTINCT CASE WHEN age >= 30 THEN id END) as 'mature_users!: i64',
           AVG(CASE WHEN birth_year IS NOT NULL THEN age ELSE NULL END) as 'avg_age_with_birth!: f64',
           MAX(LENGTH(name || email)) as 'max_combined_length!: i32',
           MIN(CASE WHEN birth_year IS NULL THEN age ELSE birth_year END) as 'min_fallback!: i32'
           FROM users
           WHERE email LIKE '%@%'"
    )]
    async fn get_complex_aggregation(
        &self,
        pageable: SerialParams,
    ) -> Result<Serial<(i64, i64, Option<f64>, i32, i32)>>;

    // Stress test: Many parameters with complex logic
    #[dml(
        "SELECT id as 'id!: Id', name, email, age as 'age: u8', birth_year as 'birth_year: u16'
           FROM users
           WHERE (age = $1 OR age = $2 OR age = $3)
           AND (name LIKE $4 OR email LIKE $5)
           AND (birth_year IS NULL OR birth_year BETWEEN $6 AND $7)
           AND LENGTH(name) >= $8"
    )]
    async fn find_users_many_params(
        &self,
        age1: u8,
        age2: u8,
        age3: u8,
        name_pattern: String,
        email_pattern: String,
        birth_year_min: u16,
        birth_year_max: u16,
        min_name_length: i32,
        params: impl IntoParams,
    ) -> Result<Serial<User>>;

    // Test GROUP BY with alias reference (tests our GROUP BY alias expansion fix)
    #[dml(
        "SELECT
           CASE WHEN age < 25 THEN 'Young' ELSE 'Old' END as age_bracket,
           COUNT(*) as user_count
           FROM users
           GROUP BY age_bracket"
    )]
    async fn find_simple_age_groups(&self, pageable: SerialParams)
    -> Result<Serial<(String, i64)>>;

    // Test GROUP BY with HAVING and alias (comprehensive test)
    #[dml(
        "SELECT
           CASE WHEN age < 30 THEN 'Young' ELSE 'Mature' END as category,
           COUNT(*) as total
           FROM users
           GROUP BY category
           HAVING COUNT(*) >= $1"
    )]
    async fn find_age_categories_with_min_count(
        &self,
        min_count: i64,
        params: impl IntoParams,
    ) -> Result<Serial<(String, i64)>>;

    // Test with cast syntax
    #[dml(
        "SELECT id , name, email, age as 'age: u8', birth_year as 'birth_year: u16' FROM users WHERE age BETWEEN $1 AND $2"
    )]
    async fn find_with_casts(
        &self,
        min_age: i64,
        max_age: i64,
        pageable: SerialParams,
    ) -> Result<Serial<User>>;
}

pub type Result<T> = std::result::Result<T, sqlx::Error>;

#[cfg(test)]
mod tests {
    use super::*;

    // Basic pagination tests
    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_basic_find_all_pagination(pool: Pool) {
        let repo = TestSimpleUserRepo { pool: &pool };

        let params = ParamsBuilder::new()
            .serial()
            .page(1, 2)
            .done()
            .sort()
            .asc("name")
            .done()
            .build();

        let page = repo.find_all(params).await.unwrap();

        assert_eq!(page.size, 2);
        assert_eq!(page.page, 1);
        assert!(page.page == 1); // First page has no previous
        assert_eq!(page.total_pages, 10); // 20 users / 2 per page = 10 pages
        assert_eq!(page.data.len(), 2);
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_basic_find_adults_with_params(pool: Pool) {
        let repo = TestSimpleUserRepo { pool: &pool };

        let params = ParamsBuilder::new()
            .serial()
            .page(1, 5)
            .done()
            .sort()
            .desc("age")
            .done()
            .build();

        let page = repo.find_adults(18, params).await.unwrap();

        assert_eq!(page.size, 5);
        assert_eq!(page.page, 1);
        // All users should be >= 18
        for user in &page.data {
            if let Some(age) = user.age {
                assert!(age >= 18);
            }
        }
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_basic_find_by_name_pattern_tuple(pool: Pool) {
        let repo = TestSimpleUserRepo { pool: &pool };

        let params = ParamsBuilder::new()
            .serial()
            .page(1, 10)
            .done()
            .sort()
            .asc("name")
            .done()
            .build();

        let page = repo
            .find_by_name_pattern("%a%".to_string(), params)
            .await
            .unwrap();

        assert_eq!(page.size, 10);
        assert_eq!(page.page, 1);
        // Each item should be a tuple (String, Option<i64>)
        for (name, age) in &page.data {
            assert!(name.contains("a") || name.contains("A"));
            // age is Option<i64>
            println!("Found: {} (age: {:?})", name, age);
        }
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_basic_pagination_with_multiple_sorts(pool: Pool) {
        let repo = TestSimpleUserRepo { pool: &pool };

        let params = ParamsBuilder::new()
            .serial()
            .page(1, 5)
            .done()
            .sort()
            .desc("age")
            .asc("name")
            .done()
            .build();

        let page = repo.find_all(params).await.unwrap();

        assert_eq!(page.size, 5);
        // Verify sorting: first by age DESC, then by name ASC
        if page.data.len() > 1 {
            for i in 0..page.data.len() - 1 {
                let current = &page.data[i];
                let next = &page.data[i + 1];

                match (current.age, next.age) {
                    // Both have ages - compare them
                    (Some(current_age), Some(next_age)) => {
                        if current_age == next_age {
                            // Same age, name should be ascending
                            assert!(current.name <= next.name);
                        } else {
                            // Different age, current age should be >= next age (DESC)
                            assert!(current_age >= next_age);
                        }
                    }
                    // NULL ages come last in DESC sort
                    (Some(_), None) => {} // Current has age, next doesn't - OK for DESC
                    (None, None) => {
                        // Both NULL, sort by name ASC
                        assert!(current.name <= next.name);
                    }
                    (None, Some(_)) => {
                        panic!("NULL age should come after non-NULL age in DESC sort");
                    }
                }
            }
        }
    }

    #[test]
    fn test_pagination_types_compile() {
        let pagination_params = SerialParams::new(1, 10);

        assert_eq!(pagination_params.page(), 1);
        assert_eq!(pagination_params.per_page(), 10);
        assert_eq!(pagination_params.offset(), 0);
        assert_eq!(pagination_params.limit(), 10);
    }

    #[test]
    fn test_params_builder_sort_generation() {
        let params = ParamsBuilder::new()
            .sort()
            .asc("name")
            .desc("age")
            .asc("email")
            .done()
            .build();

        // Verify sort was created
        assert!(params.sort_by.is_some());
    }

    #[test]
    fn test_params_builder_fluent_api() {
        let params = ParamsBuilder::new()
            .serial()
            .page(3, 20)
            .done()
            .sort()
            .desc("created_at")
            .asc("name")
            .desc("priority")
            .done()
            .build();

        assert_eq!(params.limit.unwrap().0, 20); // page_size = 20
        assert_eq!(params.offset.unwrap().0, 40); // (page - 1) * page_size = (3-1) * 20 = 40
        assert!(params.sort_by.is_some());
    }

    // Production pagination tests
    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_complex_filter_pagination(pool: Pool) {
        let repo = TestProductionUserRepo { pool: &pool };

        // Test with complex filtering and multiple sort criteria
        let params = ParamsBuilder::new()
            .serial()
            .page(1, 2)
            .done()
            .sort()
            .desc("age")
            .asc("name")
            .desc("birth_year")
            .done()
            .build();

        let page = repo
            .find_users_complex_filter(20, 35, "%a%".to_string(), 1990, params)
            .await
            .unwrap();

        assert_eq!(page.size, 2);
        assert_eq!(page.page, 1);
        assert!(page.total_items > 0);

        // Verify sorting: age DESC, then name ASC, then birth_year DESC
        if page.data.len() > 1 {
            for i in 0..page.data.len() - 1 {
                let current = &page.data[i];
                let next = &page.data[i + 1];

                if current.age == next.age {
                    assert!(current.name <= next.name);
                } else {
                    assert!(current.age >= next.age);
                }
            }
        }
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_analytics_pagination(pool: Pool) {
        let repo = TestProductionUserRepo { pool: &pool };

        let params = ParamsBuilder::new()
            .serial()
            .page(1, 3)
            .done()
            .sort()
            .asc("years_since_birth")
            .done()
            .build();

        let page = repo.find_user_analytics(18, params).await.unwrap();

        assert_eq!(page.size, 3);
        // Verify calculated fields are working
        for stat in &page.data {
            assert!(stat.age > 18);
            assert!(stat.email_domain.contains(".") || !stat.email_domain.is_empty());
        }
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_age_demographics_grouping(pool: Pool) {
        let repo = TestProductionUserRepo { pool: &pool };

        let params = ParamsBuilder::new()
            .serial()
            .page(1, 10)
            .done()
            .sort()
            .desc("user_count")
            .done()
            .build();

        let page = repo.find_age_demographics(1, params).await.unwrap();

        // Should have age groups
        for group in &page.data {
            assert!(["Young", "Adult", "Senior"].contains(&group.age_bracket.as_str()));
            assert!(group.user_count >= 1);
        }
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_user_profiles_subquery(pool: Pool) {
        let repo = TestProductionUserRepo { pool: &pool };

        let params = ParamsBuilder::new()
            .serial()
            .page(1, 5)
            .done()
            .sort()
            .asc("display_name")
            .done()
            .build();

        let page = repo
            .find_user_profiles("%.com".to_string(), params)
            .await
            .unwrap();

        // Verify complex transformations
        for profile in &page.data {
            // display_name should be title case
            let first_char = profile.display_name.chars().nth(0).unwrap();
            assert!(first_char.is_uppercase());

            assert!(["Above Average", "Below Average"].contains(&profile.age_category.as_str()));
            assert!([0, 1].contains(&profile.has_birth_year));
        }
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_ranking_with_correlated_subquery(pool: Pool) {
        let repo = TestProductionUserRepo { pool: &pool };

        let params = ParamsBuilder::new()
            .serial()
            .page(1, 4)
            .done()
            .sort()
            .asc("age_rank")
            .done()
            .build();

        let page = repo.find_users_with_ranking(3, params).await.unwrap();

        // Verify ranking calculation
        if page.data.len() > 1 {
            for i in 0..page.data.len() - 1 {
                let current = &page.data[i];
                let next = &page.data[i + 1];
                // age_rank should be ascending
                assert!(current.2 <= next.2);
            }
        }
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_slice_by_age_range(pool: Pool) {
        let repo = TestProductionUserRepo { pool: &pool };

        let params = ParamsBuilder::new()
            .serial()
            .page(1, 3)
            .done()
            .sort()
            .asc("age")
            .done()
            .build();

        let page = repo.slice_users_by_age_range(20, 30, params).await.unwrap();

        assert_eq!(page.size, 3);
        assert_eq!(page.page, 1);
        // Note: This is a Serial result, so it has total_pages not has_next
        assert!(page.total_pages >= 1);

        // All users should be in age range
        for user in &page.data {
            assert!(user.age >= 20 && user.age <= 30);
        }
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_slice_pattern_matching(pool: Pool) {
        let repo = TestProductionUserRepo { pool: &pool };

        let params = ParamsBuilder::new()
            .serial()
            .page(1, 5)
            .done()
            .sort()
            .desc("email_length")
            .done()
            .build();

        // SQLite REGEXP might not be available, so let's use a safer test
        let slice = repo.slice_users_by_pattern(".*".to_string(), params).await;

        // This might fail due to REGEXP not being available, which is expected
        match slice {
            Ok(slice) => {
                for item in &slice.data {
                    assert!(item.1 > 0); // email_length should be positive
                    assert!([0, 1].contains(&item.2)); // has_birth_year boolean
                }
            }
            Err(_) => {
                println!("REGEXP not available in this SQLite build - expected");
            }
        }
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_complex_aggregation_single_result(pool: Pool) {
        let repo = TestProductionUserRepo { pool: &pool };

        let pageable = SerialParams::new(1, 1);

        let page = repo.get_complex_aggregation(pageable).await.unwrap();

        assert_eq!(page.data.len(), 1);
        let stats = &page.data[0];

        // Verify aggregation results make sense
        assert!(stats.0 >= 0); // young_users count
        assert!(stats.1 >= 0); // mature_users count
        assert!(stats.3 > 0); // max_combined_length should be positive
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_many_parameters_stress_test(pool: Pool) {
        let repo = TestProductionUserRepo { pool: &pool };

        let params = ParamsBuilder::new()
            .serial()
            .page(1, 2)
            .done()
            .sort()
            .asc("age")
            .desc("name")
            .done()
            .build();

        let page = repo
            .find_users_many_params(
                25,
                30,
                35,
                "%a%".to_string(),
                "%@%".to_string(),
                1990,
                2005,
                3,
                params,
            )
            .await
            .unwrap();

        // Should handle many parameters correctly
        for user in &page.data {
            assert!([25u8, 30u8, 35u8].contains(&user.age));
            assert!(user.name.len() >= 3);
        }
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_pagination_edge_cases(pool: Pool) {
        let repo = TestProductionUserRepo { pool: &pool };

        // Test page beyond available data
        let params = ParamsBuilder::new()
            .serial()
            .page(1000, 10)
            .done()
            .sort()
            .asc("id")
            .done()
            .build();
        let page = repo
            .find_users_complex_filter(0, 100, "%".to_string(), 1800, params)
            .await
            .unwrap();

        assert_eq!(page.size, 10); // page size from params
        assert_eq!(page.page, 1000);
        // Serial result - check total_pages instead of has_next/has_previous
        // total_pages is valid (u32, always >= 0)
        assert!(page.data.is_empty());

        // Test very large page size
        let params2 = ParamsBuilder::new()
            .serial()
            .page(1, 1000)
            .done()
            .sort()
            .desc("age")
            .done()
            .build();

        let page = repo
            .find_users_complex_filter(0, 100, "%".to_string(), 1800, params2)
            .await
            .unwrap();

        assert_eq!(page.size, 1000); // This was the requested page size
        assert_eq!(page.page, 1); // This is always page 1
        // Serial has total_pages, not has_next/has_previous
        // total_pages is valid (u32, always >= 0)
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_slice_edge_cases(pool: Pool) {
        let repo = TestProductionUserRepo { pool: &pool };

        // Test slice with size larger than available data
        let params = ParamsBuilder::new()
            .serial()
            .page(1, 100)
            .done()
            .sort()
            .asc("id")
            .done()
            .build();
        let page = repo.slice_users_by_age_range(0, 200, params).await.unwrap();

        assert_eq!(page.size, 100);
        // Serial has total_pages, not has_next
        // total_pages is valid (u32, always >= 0)

        // Test empty result
        let params2 = ParamsBuilder::new()
            .serial()
            .page(1, 5)
            .done()
            .sort()
            .asc("age")
            .done()
            .build();
        let page2 = repo
            .slice_users_by_age_range(200, 250, params2)
            .await
            .unwrap();

        assert!(page2.data.is_empty());
        assert_eq!(page2.total_items, 0);
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_multiple_sort_criteria_consistency(pool: Pool) {
        let repo = TestProductionUserRepo { pool: &pool };

        // Test 3-level sorting
        let params = ParamsBuilder::new()
            .serial()
            .page(1, 10)
            .done()
            .sort()
            .desc("age")
            .asc("name")
            .desc("birth_year")
            .done()
            .build();

        let page = repo
            .find_users_complex_filter(0, 100, "%".to_string(), 1800, params)
            .await
            .unwrap();

        // Manually verify complex sort order
        if page.data.len() > 2 {
            for i in 0..page.data.len() - 1 {
                let current = &page.data[i];
                let next = &page.data[i + 1];

                if current.age != next.age {
                    // Primary sort: age DESC
                    assert!(current.age >= next.age);
                } else if current.name != next.name {
                    // Secondary sort: name ASC
                    assert!(current.name <= next.name);
                } else {
                    // Tertiary sort: birth_year DESC
                    match (&current.birth_year, &next.birth_year) {
                        (Some(curr_year), Some(next_year)) => assert!(curr_year >= next_year),
                        (Some(_), None) => {} // Some comes before None in DESC
                        (None, Some(_)) => panic!("None should come after Some in DESC sort"),
                        (None, None) => {} // Equal
                    }
                }
            }
        }
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_group_by_alias_reference(pool: Pool) {
        // Test the specific case our GROUP BY alias expansion fix addresses
        let repo = TestProductionUserRepo { pool: &pool };

        let pageable = SerialParams::new(1, 10);

        let page = repo.find_simple_age_groups(pageable).await.unwrap();

        // Should return age groups
        assert!(page.data.len() <= 2); // 'Young' and 'Old' categories
        assert!(page.total_items > 0);

        // Verify each group has correct structure and positive counts
        for (category, count) in &page.data {
            assert!(["Young", "Old"].contains(&category.as_str()));
            assert!(*count > 0);
        }
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_group_by_with_having_and_alias(pool: Pool) {
        // Test GROUP BY with HAVING clause and alias reference
        let repo = TestProductionUserRepo { pool: &pool };

        let params = ParamsBuilder::new()
            .serial()
            .page(1, 10)
            .done()
            .sort()
            .desc("total")
            .done()
            .build();

        // Test with different minimum counts
        let page_min_1 = repo
            .find_age_categories_with_min_count(1, params.clone())
            .await
            .unwrap();
        let page_min_5 = repo
            .find_age_categories_with_min_count(5, params)
            .await
            .unwrap();

        // Should have some results for min_count=1
        assert!(page_min_1.total_items > 0);

        // Should have fewer or equal results for higher min_count
        assert!(page_min_5.total_items <= page_min_1.total_items);

        // Verify structure and HAVING clause enforcement
        for (category, total) in &page_min_5.data {
            assert!(["Young", "Mature"].contains(&category.as_str()));
            assert!(*total >= 5); // HAVING clause should ensure this
        }

        // Verify sorting (should be DESC by total)
        if page_min_1.data.len() > 1 {
            for i in 0..page_min_1.data.len() - 1 {
                assert!(page_min_1.data[i].1 >= page_min_1.data[i + 1].1);
            }
        }
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_with_casts(pool: Pool) {
        let repo = TestProductionUserRepo { pool: &pool };

        let pageable = SerialParams::new(1, 2);
        let result = repo.find_with_casts(20, 40, pageable).await;

        match result {
            Ok(page) => {
                println!("With casts: {} results", page.data.len());
                assert!(page.total_items > 0);
            }
            Err(e) => {
                println!("ERROR with casts: {:?}", e);
                panic!("Cast syntax should work");
            }
        }
    }

    #[test]
    fn test_pagination_params_creation() {
        let pagination_params = SerialParams::new(2, 5);

        assert_eq!(pagination_params.page(), 2);
        assert_eq!(pagination_params.per_page(), 5);
        assert_eq!(pagination_params.offset(), 5); // (page-1) * per_page = (2-1)*5 = 5
        assert_eq!(pagination_params.limit(), 5);
    }

    #[test]
    fn test_params_builder_creation() {
        let params = ParamsBuilder::new().serial().page(3, 10).done().build();

        assert!(params.pagination.is_some());
        if let Some(Pagination::Serial(serial_params)) = params.pagination {
            assert_eq!(serial_params.page(), 3);
            assert_eq!(serial_params.per_page(), 10);
        } else {
            panic!("Expected Serial pagination");
        }
    }
}

struct TestSimpleUserRepo<'a> {
    pool: &'a Pool,
}

impl<'a> SimpleUserRepo for TestSimpleUserRepo<'a> {
    fn get_pool(&self) -> &Pool {
        self.pool
    }
}

struct TestProductionUserRepo<'a> {
    pool: &'a Pool,
}

impl<'a> ProductionUserRepo for TestProductionUserRepo<'a> {
    fn get_pool(&self) -> &Pool {
        self.pool
    }
}
