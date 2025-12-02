use sqlx_data::{Pool, QueryResult, Result, dml};

// Example of a newtype for testing transparent types
#[derive(Clone, PartialEq, Eq, Debug, sqlx::Type)]
#[sqlx(transparent)]
pub struct Id(i64);

impl From<i64> for Id {
    fn from(value: i64) -> Self {
        Id(value)
    }
}

// User model for tests
#[derive(Debug, Clone, PartialEq)]
pub struct User {
    pub id: Id,
    pub name: String,
    pub email: String,
    pub age: u8,
    pub birth_year: Option<u16>,
}

// This is what the user writes with combined approach
#[sqlx_data::repo]
trait UserRepo {
    #[dml("SELECT 1")]
    async fn ping(&self) -> Result<i32>;

    #[dml(
        "SELECT id as 'id!: Id', name, email, age as 'age: u8', birth_year as 'birth_year: u16' FROM users WHERE id = $1"
    )]
    async fn find_by_id(&self, id: i64) -> Result<User>;

    #[dml(
        "SELECT id as 'id!: Id', name, email, age as 'age: u8', birth_year as 'birth_year: u16' FROM users WHERE id = $1"
    )]
    async fn find_optional_by_id(&self, id: i64) -> Result<Option<User>>;

    #[dml(
        "SELECT id as 'id!: Id', name, email, age as 'age: u8', birth_year as 'birth_year: u16' FROM users WHERE id = $1"
    )]
    async fn find_many_by_id(&self, id: i64) -> Result<Vec<User>>;

    //TODO Important: to document casting in struct mapping
    //#[dml("SELECT * FROM users WHERE id = $1")]
    //async fn find_by_id_struct(&self, id: i64) -> Result<User>;

    //#[dml("SELECT * FROM users WHERE age >= $1")]
    //async fn find_adults(&self, min_age: u8) -> Result<Vec<User>>;

    #[dml("SELECT COUNT(*) FROM users")]
    async fn count_users(&self) -> Result<u64>;

    #[dml("SELECT AVG(age) as 'avg_age?: f32' FROM users")]
    async fn avg_age(&self) -> Result<Option<f32>>;

    #[dml("SELECT MAX(age) FROM users")]
    async fn max_age(&self) -> Result<Option<u8>>;

    #[dml("SELECT name FROM users WHERE id = $1")]
    async fn get_name(&self, id: i64) -> Result<String>;

    #[dml("SELECT email FROM users WHERE id = $1")]
    async fn get_email(&self, id: i64) -> Result<String>;

    #[dml("SELECT age FROM users WHERE id = $1")]
    async fn get_age(&self, id: i64) -> Result<i64>;

    // Column is nullable and the return type is Option
    // fetch_optional add one layer of Option -> Result<Option<Option<i64>>>
    #[dml("SELECT birth_year FROM users WHERE id = $1")]
    async fn get_birth_year(&self, id: i64) -> Result<Option<u16>>;

    // 6. Complex mathematical expression
    #[dml("SELECT (age * 365) + COALESCE(birth_year, 0) as calculation FROM users WHERE id = $1")]
    async fn math_expression(&self, id: i64) -> Result<u32>;

    // 8. Data/timestamp casting (assumindo coluna timestamp)
    #[dml("SELECT strftime('%Y', id) as year FROM users WHERE id = $1")]
    async fn date_year_cast(&self, id: i64) -> Result<Option<String>>;

    // 9. Multiple parameters with different types
    #[dml("SELECT COUNT(*) FROM users WHERE age BETWEEN $1 AND $2 AND name LIKE $3")]
    async fn multi_params(&self, min_age: u8, max_age: u8, pattern: String) -> Result<u32>;

    // 10. Nullable with HAVING
    #[dml("SELECT COUNT(*) as cnt FROM users GROUP BY birth_year HAVING COUNT(*) > $1")]
    async fn group_having_count(&self, min_count: u8) -> Result<Vec<u16>>;

    // 10.1 Nullable with HAVING
    #[dml("SELECT COUNT(*) as cnt FROM users GROUP BY birth_year HAVING COUNT(*) > $1")]
    async fn group_having_count_no_casting(&self, min_count: u8) -> Result<Vec<i64>>;

    // 11. DISTINCT with casting
    #[dml("SELECT DISTINCT age FROM users ORDER BY age")]
    async fn distinct_ages(&self) -> Result<Vec<u8>>;

    // 12. UNION with different types
    #[dml(
        "SELECT age as value FROM users UNION SELECT birth_year as value FROM users WHERE birth_year IS NOT NULL"
    )]
    async fn union_mixed_types(&self) -> Result<Vec<Option<u16>>>;

    // 14. Nested subqueries
    #[dml(
        "SELECT (SELECT MAX(age) FROM users WHERE age < (SELECT AVG(age) FROM users)) as max_below_avg"
    )]
    async fn nested_subquery(&self) -> Result<Option<u8>>;

    // 16. Complex WHERE with multiple ORs
    #[dml("SELECT id FROM users WHERE age > $1 OR birth_year < $2 OR name LIKE $3")]
    async fn complex_where(&self, age: u8, year: u16, name: String) -> Result<Vec<i32>>;

    // 20. IN clause with parameters
    #[dml("SELECT COUNT(*) FROM users WHERE age IN ($1, $2, $3)")]
    async fn in_clause(&self, age1: u8, age2: u8, age3: u8) -> Result<u32>;

    // 21. Complex math with multiple columns
    #[dml("SELECT (age + COALESCE(birth_year, 0)) / 2 as avg_year FROM users WHERE id = $1")]
    async fn complex_math(&self, id: i64) -> Result<u32>;

    // 22. String concatenation
    #[dml("SELECT name || ' (' || email || ')' as display_name FROM users WHERE id = $1")]
    async fn string_concat(&self, id: i64) -> Result<String>;

    // 24. ABS function with casting (Important)
    #[dml("SELECT ABS(age - $1) as 'age_diff!: u8' FROM users WHERE id = $2")]
    async fn abs_function(&self, target_age: u8, id: i64) -> Result<u8>;

    // 25. Floating point precision
    #[dml("SELECT ROUND(AVG(age), 2) as 'precise_avg: f64' FROM users")]
    async fn float_precision(&self) -> Result<Option<f64>>;

    // 28. Multiple CTEs
    #[dml(
        r#"
        WITH young AS (SELECT * FROM users WHERE age < 30), 
             old AS (SELECT * FROM users WHERE age >= 30) 
        SELECT COUNT(*) FROM young "#
    )]
    async fn multiple_ctes(&self) -> Result<u32>;

    // 29. Recursive CTE (se suportado)
    #[dml(
        "WITH RECURSIVE cnt(x) AS (SELECT 1 UNION ALL SELECT x+1 FROM cnt WHERE x < 5) SELECT SUM(x) FROM cnt"
    )]
    async fn recursive_cte(&self) -> Result<u16>;

    // 30. JSON extraction (se SQLite compilado com JSON)
    //#[dml("SELECT json_extract(metadata, '$.score') as score FROM users WHERE id = $1")]
    //async fn json_extract(&self, id: i64) -> Result<Option<f32>>;

    // 31. Trigonometric functions
    //#[dml("SELECT SIN(age * 3.14159 / 180) as sin_age FROM users WHERE id = $1")]
    //async fn trig_functions(&self, id: i64) -> Result<f64>;

    // 32. String length with casting
    #[dml("SELECT (LENGTH(name) + LENGTH(email)) as total_chars FROM users WHERE id = $1")]
    async fn string_length_sum(&self, id: i64) -> Result<Option<String>>;

    // // 33. Date arithmetic(important)
    #[dml(
        "SELECT date('now', '-' || age || ' years') as 'birth_estimate!' FROM users WHERE id = $1"
    )]
    async fn date_arithmetic(&self, id: i64) -> Result<String>;

    // 34. Null comparison edge case
    #[dml("SELECT COUNT(*) FROM users WHERE birth_year <> $1 OR birth_year IS NULL")]
    async fn null_comparison(&self, year: u16) -> Result<u32>;

    // 35. Float division precision
    #[dml("SELECT CAST(age AS REAL) / 12.0 as age_months FROM users WHERE id = $1")]
    async fn float_division(&self, id: i64) -> Result<f32>;

    // 36. Multiple parameters in expression
    #[dml("SELECT (age - $1) * $2 + $3 as 'formula!' FROM users WHERE id = $4")]
    async fn multi_param_formula(&self, base: u8, mult: f32, add: i16, id: i64) -> Result<f32>;

    // 38. BETWEEN with different types
    #[dml("SELECT COUNT(*) FROM users WHERE CAST(birth_year AS INTEGER) BETWEEN $1 AND $2")]
    async fn between_cast(&self, start: u16, end: u16) -> Result<u32>;

    // 39. Nested aggregations
    #[dml(
        "SELECT MAX(cnt) as max_count FROM (SELECT birth_year, COUNT(*) as cnt FROM users GROUP BY birth_year)"
    )]
    async fn nested_aggregation(&self) -> Result<Option<u32>>;

    // 41. String functions chain - Important
    // ERROR: no built in mapping found for type NULL of column #1 ("short_name"); a type override may be required, see documentation for details
    //#[dml("SELECT SUBSTR(UPPER(TRIM(name)), 1, 3) as short_name FROM users WHERE id = $1")]
    #[dml(
        "SELECT SUBSTR(UPPER(TRIM(name)), 1, 3) as 'short_name: String' FROM users WHERE id = $1"
    )]
    async fn string_chain(&self, id: i64) -> Result<Option<String>>;

    // // 42. Mathematical constants
    // #[dml("SELECT PI() * age as pi_age FROM users WHERE id = $1")]
    // async fn math_constants(&self, id: i64) -> Result<f64>;

    // 43. Random with casting
    #[dml("SELECT ABS(RANDOM() % 100) as 'random_num: u8' FROM users LIMIT 1")]
    async fn random_cast(&self) -> Result<Option<u8>>;

    // 44. Timestamp functions
    #[dml("SELECT unixepoch('now') - unixepoch('now') as seconds_since FROM users WHERE id = $1")]
    async fn timestamp_diff(&self, id: i64) -> Result<Option<u32>>;

    // 45. Complex IN subquery
    #[dml(
        "SELECT COUNT(*) FROM users WHERE age IN (SELECT DISTINCT age FROM users WHERE birth_year > $1)"
    )]
    async fn in_subquery(&self, year: u16) -> Result<u32>;

    //TODO: Document - Clausule IN with Vec parameter does not work yet on SQLite or MySQL, use FilterParams instead
    // 46. IN clause with Vec parameter
    //#[dml("SELECT id, name FROM users WHERE id IN ($1)")]
    //async fn find_by_ids(&self, ids: Vec<i64>) -> Result<Vec<(i32, String)>>;

    // // 46. Multiple JOINs with complex casting
    // // #[dml("SELECT u.id, COUNT(p.id) as post_count, AVG(c.rating) as avg_rating FROM users u LEFT JOIN posts p ON u.id = p.user_id
    // //   LEFT JOIN comments c ON p.id = c.post_id WHERE u.id = $1 GROUP BY u.id")]
    // // async fn complex_joins(&self, id: i64) -> Result<(i32, u16, Option<f32>)>;

    // 48. Complex expression em WHERE
    #[dml("SELECT id FROM users WHERE (age * 2 + COALESCE(birth_year, 0) / 10) > $1")]
    async fn complex_where_expr(&self, threshold: u32) -> Result<Vec<i32>>;

    // 50. Boolean parameter test
    #[dml("SELECT COUNT(*) FROM users WHERE (age > 30) = $1")]
    async fn count_by_age_condition(&self, is_older: bool) -> Result<u32>;

    // // 49. Multiple UNION with casting
    // #[dml("SELECT 'user' as type, CAST(id AS TEXT) as identifier FROM users UNION SELECT 'admin', email FROM admins")]
    // async fn union_different_types(&self) -> Result<Vec<(String, String)>>;

    //#[dml("INSERT INTO users (name) VALUES ($1) RETURNING *")]
    //async fn create_user(name: &str) -> Result<User>; //Use new types or all columns
    //async fn create_user(&self, name: String) -> Result<i64, sqlx::Error>;

    #[dml("UPDATE users SET name = $2 WHERE id = $1")]
    async fn update_user(&self, id: i64, name: String) -> Result<QueryResult>;

    #[dml("DELETE FROM users WHERE id = $1")]
    async fn delete_user(&self, id: i64) -> Result<QueryResult>;

    // === TESTS FOR EXPLICIT CASTING NO DOUBLE CASTING ===

    // 51. Scalar query with explicit casting - should NOT do additional casting
    #[dml("SELECT COUNT(*) as 'user_count: u32' FROM users")]
    async fn count_users_explicit(&self) -> Result<u32>;

    // 52. Scalar query with nullable explicit casting
    #[dml("SELECT AVG(age) as 'avg_age: f64' FROM users")]
    async fn avg_age_explicit(&self) -> Result<Option<f64>>;

    // 53. Scalar query WITHOUT explicit casting - should do automatic casting
    #[dml("SELECT COUNT(*) FROM users")]
    async fn count_users_implicit(&self) -> Result<u32>;

    // 55. Vec scalar with explicit casting
    #[dml("SELECT LENGTH(name) as 'name_len: u8' FROM users")]
    async fn vec_explicit_casting(&self) -> Result<Vec<Option<u8>>>;

    // 56. Struct query with explicit casting (should work via query_as!)
    #[dml(
        "SELECT id as 'id!: Id', name, email, age as 'age: u8', birth_year as 'birth_year: u16' FROM users WHERE id = $1"
    )]
    async fn struct_explicit_casting(&self, id: i64) -> Result<User>;
}

// Example: User struct that implements SqlxRepo
pub struct MyApp {
    pool: Pool,
}

// User implements UserRepo
impl UserRepo for MyApp {
    // Override the default get_pool implementation
    fn get_pool(&self) -> &Pool {
        &self.pool
    }
    // All SQL methods (find_by_id, etc.) are implemented automatically! 🎉

    //override count_users to show custom implementation
    async fn count_users(&self) -> Result<u64> {
        let row = Self::count_users_query(&self).await?;
        Ok(row)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_find_by_id(pool: Pool) {
        let repo = MyApp { pool };

        let user = repo.find_by_id(1).await.unwrap();
        assert_eq!(user.name, "Alice");
        assert_eq!(user.email, "alice@example.com");
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_count_users(pool: Pool) {
        let repo = MyApp { pool };

        let count = repo.count_users().await.unwrap();
        assert_eq!(count, 20); // 20 users from fixture
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_get_name(pool: Pool) {
        let repo = MyApp { pool };

        let name = repo.get_name(1).await.unwrap();
        assert_eq!(name, "Alice".to_string());

        let missing = repo.get_name(999).await;
        assert!(missing.is_err());
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_nullable_column_handling(pool: Pool) {
        let repo = MyApp { pool };

        let result = repo.get_birth_year(3).await;
        assert!(result.is_ok(), "Optional version should handle NULL");
        assert_eq!(result.unwrap(), None);
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_auto_flatten_aggregates(pool: Pool) {
        let repo = MyApp { pool };

        let avg_result = repo.avg_age().await;
        assert!(avg_result.is_ok());
        let avg = avg_result.unwrap();
        assert!(avg.is_some()); // Should have average since we have data
        assert!(avg.unwrap() > 0.0);

        let max_result = repo.max_age().await;
        assert!(max_result.is_ok());
        let max_age = max_result.unwrap();
        assert!(max_age.is_some()); // Should have maximum since we have data
        assert!(max_age.unwrap() >= 35); // Maximum age from fixture data
    }

    // #[tokio::test]
    // async fn test_in_clause_with_vec() {
    //     let pool = setup_test_db().await;
    //     let repo = MyApp { pool };

    //     // Test IN clause with Vec parameter
    //     let ids_to_find = vec![1i64, 3i64];
    //     let result = repo.find_by_ids(ids_to_find).await;
    //     assert!(result.is_ok());

    //     let users = result.unwrap();
    //     assert_eq!(users.len(), 2);

    //     // Should find Alice (id=1) and Charlie (id=3)
    //     assert!(users.iter().any(|(id, name)| *id == 1 && name == "Alice"));
    //     assert!(users.iter().any(|(id, name)| *id == 3 && name == "Charlie"));

    //     // Test with empty Vec
    //     let empty_ids: Vec<i64> = vec![];
    //     let empty_result = repo.find_by_ids(empty_ids).await;
    //     assert!(empty_result.is_ok());
    //     assert_eq!(empty_result.unwrap().len(), 0);

    //     // Test with single ID
    //     let single_id = vec![2i64];
    //     let single_result = repo.find_by_ids(single_id).await;
    //     assert!(single_result.is_ok());
    //     let single_users = single_result.unwrap();
    //     assert_eq!(single_users.len(), 1);
    //     assert_eq!(single_users[0], (2, "Bob".to_string()));
    // }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_explicit_vs_implicit_casting(pool: Pool) {
        let repo = MyApp { pool };

        // Test explicit casting - should work without additional casting
        let explicit_count = repo.count_users_explicit().await.unwrap();
        assert_eq!(explicit_count, 20u32);

        // Test implicit casting - should apply automatic casting
        let implicit_count = repo.count_users_implicit().await.unwrap();
        assert_eq!(implicit_count, 20u32);

        // Both should produce the same result, but via different code paths
        assert_eq!(explicit_count, implicit_count);

        // Test nullable explicit casting
        let explicit_avg = repo.avg_age_explicit().await.unwrap();
        assert!(explicit_avg.is_some());
        let avg_value = explicit_avg.unwrap();
        assert!(avg_value > 0.0);
        assert!(avg_value < 100.0);
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_vec_explicit_casting(pool: Pool) {
        let repo = MyApp { pool };

        // Test Vec with explicit casting - should not do double casting
        let name_lengths = repo.vec_explicit_casting().await.unwrap();

        // Should have 20 lengths from fixture data
        assert_eq!(name_lengths.len(), 20);

        // Check that lengths are reasonable u8 values (all should be Some)
        for length_opt in &name_lengths {
            assert!(length_opt.is_some());
            let length = length_opt.unwrap();
            assert!(length > 0);
            assert!(length < 50); // Reasonable name length
        }

        // Alice = 5, Bob = 3, Charlie = 7
        assert!(name_lengths.contains(&Some(5))); // Alice
        assert!(name_lengths.contains(&Some(3))); // Bob
        assert!(name_lengths.contains(&Some(7))); // Charlie
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_struct_explicit_casting(pool: Pool) {
        let repo = MyApp { pool };

        // Test struct with explicit casting - query_as! should handle it correctly
        let user = repo.struct_explicit_casting(1).await.unwrap();

        assert_eq!(user.name, "Alice");
        assert_eq!(user.email, "alice@example.com");
        assert_eq!(user.age, 30u8);
        assert_eq!(user.birth_year, Some(1993u16));

        // The Id should be properly constructed
        assert_eq!(user.id, Id::from(1i64));
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_boolean_parameter(pool: Pool) {
        let repo = MyApp { pool };

        // Test with true - should count users older than 30
        let older_count = repo.count_by_age_condition(true).await.unwrap();

        // Test with false - should count users 30 or younger
        let younger_count = repo.count_by_age_condition(false).await.unwrap();

        // Total should equal sum of both conditions
        let total_count = repo.count_users().await.unwrap() as u32;
        assert_eq!(older_count + younger_count, total_count);

        // Should have at least some users in each category with fixture data
        assert!(older_count > 0, "Should have users older than 30");
        assert!(younger_count > 0, "Should have users 30 or younger");

        println!("Users older than 30: {}", older_count);
        println!("Users 30 or younger: {}", younger_count);
        println!("Total users: {}", total_count);
    }
}
