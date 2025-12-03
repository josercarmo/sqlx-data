use sqlx::types::BigDecimal;
use sqlx_data::{Pool, Result, dml};

// Use strong MySQL types
#[derive(Clone, PartialEq, Eq, Debug, sqlx::Type)]
#[sqlx(transparent)]
pub struct Id(i64);

impl From<i64> for Id {
    fn from(value: i64) -> Self {
        Id(value)
    }
}

// User model for MySQL tests with strong typing
#[derive(Debug, Clone, PartialEq)]
pub struct User {
    pub id: Id,
    pub name: String,
    pub email: String,
    pub age: u8,                 // MySQL TINYINT UNSIGNED
    pub birth_year: Option<u16>, // MySQL SMALLINT UNSIGNED
}

// Test trait for file-based SQL queries with MySQL-specific syntax
#[sqlx_data::repo]
trait UserFileRepo {
    // File-based query with MySQL parameter style (?)
    #[dml(file = "tests/fixtures/find_user_by_id.sql")]
    async fn find_by_id_from_file(&self, id: i64) -> Result<User>;

    // Count query using MySQL functions and unsigned types
    #[dml(file = "tests/fixtures/count_users_by_birth_year.sql")]
    async fn count_by_birth_year_from_file(&self, birth_year: u16) -> Result<i64>;

    // Tuple result with MySQL type casting
    #[dml(file = "tests/fixtures/get_user_info_tuple.sql")]
    async fn get_user_info_from_file(&self, id: i64) -> Result<(String, u8)>;

    // MySQL-specific: Complex aggregation with UNSIGNED types
    #[dml(file = "tests/fixtures/user_statistics.sql")]
    async fn get_user_statistics(&self) -> Result<(i64, Option<u8>, Option<u8>, Option<BigDecimal>)>;

    // MySQL-specific: Using LIMIT with ORDER BY
    #[dml(file = "tests/fixtures/top_users_by_age.sql")]
    async fn get_top_users_by_age(&self, limit_count: u32) -> Result<Vec<User>>;
}

pub struct MyFileApp {
    pool: Pool,
}

impl UserFileRepo for MyFileApp {
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
    async fn test_find_by_id_from_file(pool: Pool) {
        let repo = MyFileApp { pool };

        let user = repo.find_by_id_from_file(1).await.unwrap();
        assert_eq!(user.name, "Alice");
        assert_eq!(user.email, "alice@example.com");
        assert_eq!(user.age, 30);
        assert_eq!(user.birth_year, Some(1993));
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_count_by_birth_year_from_file(pool: Pool) {
        let repo = MyFileApp { pool };

        let count = repo.count_by_birth_year_from_file(1993).await.unwrap();
        assert_eq!(count, 2); // Alice and another user with birth year 1993
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_get_user_info_tuple_from_file(pool: Pool) {
        let repo = MyFileApp { pool };

        let (name, age) = repo.get_user_info_from_file(2).await.unwrap();
        assert_eq!(name, "Bob");
        assert_eq!(age, 25);
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_get_user_statistics_from_file(pool: Pool) {
        let repo = MyFileApp { pool };

        let (total_users, min_age, max_age, avg_age) = repo.get_user_statistics().await.unwrap();
        assert_eq!(total_users, 20); // 20 users in fixture
        assert_eq!(min_age, Some(19)); // Henry is youngest
        assert_eq!(max_age, Some(42)); // Eve is oldest
        assert!(avg_age.is_some());
        let avg = avg_age.unwrap();
        // Convert BigDecimal to f64 for comparison
        let avg_f64: f64 = avg.to_string().parse().unwrap();
        assert!(avg_f64 > 25.0 && avg_f64 < 35.0); // Average should be around 30
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_get_top_users_by_age_from_file(pool: Pool) {
        let repo = MyFileApp { pool };

        let users = repo.get_top_users_by_age(3).await.unwrap();
        assert_eq!(users.len(), 3);

        // Should be ordered by age DESC
        assert_eq!(users[0].name, "Eve"); // Oldest (42)
        assert_eq!(users[1].name, "Tina"); // Second oldest (40)
        assert!(users[0].age >= users[1].age);
        assert!(users[1].age >= users[2].age);
    }
}