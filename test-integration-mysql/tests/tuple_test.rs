use sqlx::types::BigDecimal;
use sqlx_data::{Decimal, Pool, Result, dml};

// Example of a newtype for testing transparent types with MySQL
#[allow(dead_code)]
#[derive(Clone, PartialEq, Eq, Debug, sqlx::Type)]
#[sqlx(transparent)]
pub struct Id(i64);

#[allow(dead_code)]
impl From<i64> for Id {
    fn from(value: i64) -> Self {
        Id(value)
    }
}

// User model for MySQL tests
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub struct User {
    pub id: Id,
    pub name: String,
    pub email: String,
    pub age: u8,                 // MySQL TINYINT UNSIGNED
    pub birth_year: Option<u16>, // MySQL SMALLINT UNSIGNED
}

// Trait focused on tuple return types and tuple-based operations for MySQL
#[sqlx_data::repo]
trait TupleRepo {
    // === TUPLE RETURN TYPE TESTS ===

    // Basic tuple queries with type casting using MySQL syntax
    #[dml("SELECT COUNT(id) as count, AVG(age) as avg_age FROM users")]
    async fn average_age(&self) -> Result<(i64, Option<Decimal>)>;

    #[dml("SELECT id, age, name FROM users LIMIT 2")]
    async fn get_all_ages(&self) -> Result<Vec<(i64, u8, String)>>;

    // Test optimization: no casting needed (i64, String)
    #[dml("SELECT id, name FROM users LIMIT 2")]
    async fn get_id_names(&self) -> Result<Vec<(i64, String)>>;

    #[dml("SELECT id, age, name FROM users LIMIT 1")]
    async fn get_one_age(&self) -> Result<(i64, u8, String)>;

    // CTE with tuple return using MySQL syntax
    #[dml("WITH temp AS (SELECT 1) SELECT name, birth_year FROM users WHERE id = ?")]
    async fn get_one_birth(&self, id: i64) -> Result<(String, Option<u16>)>;

    // Tuple with casting
    #[dml("SELECT name, birth_year FROM users WHERE id = ?")]
    async fn get_one_birth_with_cast(&self, id: i64) -> Result<(String, Option<u16>)>;

    // Complex aggregates with casting in tuple using MySQL unsigned types
    #[dml(
        "SELECT MIN(age) as min_age, MAX(age) as max_age, COUNT(*) as count, SUM(age) as sum FROM users"
    )]
    async fn stats_all_types(&self) -> Result<(Option<u8>, Option<u8>, i64, Option<BigDecimal>)>;

    // MySQL-specific: Use UNSIGNED types
    #[dml(
        "SELECT COUNT(*) as 'user_count!: u64', AVG(age) as min_avg FROM users WHERE age >= ?"
    )]
    async fn count_and_avg_age(&self, min_age: u8) -> Result<(u64, Option<BigDecimal>)>;

    // MySQL tuple with AUTO_INCREMENT id
    #[dml("SELECT LAST_INSERT_ID() as last_id, ROW_COUNT() as affected")]
    async fn get_insert_info(&self) -> Result<(u64, i64)>;

    // Test nullable fields in tuples
    #[dml("SELECT name, birth_year FROM users WHERE birth_year IS NULL LIMIT 1")]
    async fn get_user_without_birth_year(&self) -> Result<Option<(String, Option<u16>)>>;

    // MySQL UNSIGNED INTEGER operations
    #[dml(
        "SELECT MIN(age) as 'min!: u8', MAX(age) as 'max!: u8', COUNT(DISTINCT age) as 'unique_ages!: u64' FROM users"
    )]
    async fn age_statistics(&self) -> Result<(u8, u8, u64)>;

    // === TUPLE PARAMETER TESTS ===

    // Test tuple destructuring in parameters (simulated)
    #[dml("SELECT name FROM users WHERE age BETWEEN ? AND ? ORDER BY name")]
    async fn find_names_by_age_range(&self, min_age: u8, max_age: u8) -> Result<Vec<String>>;

    // MySQL-specific: Test with LIMIT using u32
    #[dml("SELECT id, name FROM users ORDER BY age DESC LIMIT ?")]
    async fn get_top_users_by_age(&self, limit_count: u32) -> Result<Vec<(i64, String)>>;
}

// Test implementation
pub struct TestApp {
    pool: Pool,
}

impl TupleRepo for TestApp {
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
    async fn test_average_age(pool: Pool) {
        let app = TestApp { pool };
        let (count, avg) = app.average_age().await.unwrap();

        assert_eq!(count, 20); // 20 users in fixture
        assert!(avg.is_some());
        let average = avg.unwrap();
        // Convert BigDecimal to f64 for comparison
        let avg_f64: f64 = average.to_string().parse().unwrap();
        assert!(avg_f64 > 20.0 && avg_f64 < 40.0); // Reasonable age range
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_get_all_ages(pool: Pool) {
        let app = TestApp { pool };
        let ages = app.get_all_ages().await.unwrap();

        assert_eq!(ages.len(), 2);
        for (id, age, name) in ages {
            assert!(id > 0);
            assert!(age > 0);
            assert!(!name.is_empty());
        }
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_get_id_names(pool: Pool) {
        let app = TestApp { pool };
        let id_names = app.get_id_names().await.unwrap();

        assert_eq!(id_names.len(), 2);
        for (id, name) in id_names {
            assert!(id > 0);
            assert!(!name.is_empty());
        }
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_get_one_age(pool: Pool) {
        let app = TestApp { pool };
        let (id, age, name) = app.get_one_age().await.unwrap();

        assert_eq!(id, 1);
        assert_eq!(age, 30); // Alice's age
        assert_eq!(name, "Alice");
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_get_one_birth(pool: Pool) {
        let app = TestApp { pool };
        let (name, birth_year) = app.get_one_birth(1).await.unwrap();

        assert_eq!(name, "Alice");
        assert_eq!(birth_year, Some(1993));
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_get_one_birth_with_cast(pool: Pool) {
        let app = TestApp { pool };
        let (name, birth_year) = app.get_one_birth_with_cast(1).await.unwrap();

        assert_eq!(name, "Alice");
        assert_eq!(birth_year, Some(1993));
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_stats_all_types(pool: Pool) {
        let app = TestApp { pool };
        let (min_age, max_age, count, sum) = app.stats_all_types().await.unwrap();

        assert_eq!(min_age, Some(19)); // Henry
        assert_eq!(max_age, Some(42)); // Eve
        assert_eq!(count, 20);
        assert!(sum.is_some());
        // Convert BigDecimal to i64 for comparison
        let sum_i64: i64 = sum.unwrap().to_string().parse().unwrap();
        assert!(sum_i64 > 500); // Sum of all ages should be > 500
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_count_and_avg_age(pool: Pool) {
        let app = TestApp { pool };
        let (count, avg) = app.count_and_avg_age(25).await.unwrap();

        assert_eq!(count, 16); // Users with age >= 25
        assert!(avg.is_some());
        let average = avg.unwrap();
        // Convert BigDecimal to f64 for comparison
        let avg_f64: f64 = average.to_string().parse().unwrap();
        assert!(avg_f64 >= 25.0);
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_get_user_without_birth_year(pool: Pool) {
        let app = TestApp { pool };
        let result = app.get_user_without_birth_year().await.unwrap();

        assert!(result.is_some());
        let (name, birth_year) = result.unwrap();
        assert!(!name.is_empty());
        assert!(birth_year.is_none());
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_age_statistics(pool: Pool) {
        let app = TestApp { pool };
        let (min_age, max_age, unique_ages) = app.age_statistics().await.unwrap();

        assert_eq!(min_age, 19); // Henry
        assert_eq!(max_age, 42); // Eve
        assert!(unique_ages > 1); // Should have multiple unique ages
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_find_names_by_age_range(pool: Pool) {
        let app = TestApp { pool };
        let names = app.find_names_by_age_range(25, 35).await.unwrap();

        assert!(names.len() > 0);
        assert!(names.contains(&"Alice".to_string())); // Age 30
        assert!(names.contains(&"Bob".to_string())); // Age 25
        assert!(names.contains(&"Charlie".to_string())); // Age 35
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_get_top_users_by_age(pool: Pool) {
        let app = TestApp { pool };
        let users = app.get_top_users_by_age(3).await.unwrap();

        assert_eq!(users.len(), 3);
        // Should be ordered by age DESC
        assert_eq!(users[0].1, "Eve"); // Oldest
        assert_eq!(users[1].1, "Tina"); // Second oldest
    }
}
