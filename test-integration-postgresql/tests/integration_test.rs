use sqlx::types::BigDecimal;
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

// User model for PostgreSQL tests with strong typing
#[derive(Debug, Clone, PartialEq, sqlx::FromRow)]
pub struct User {
    pub id: Id,
    pub name: String,
    pub email: String,
    pub age: i16,  // PostgreSQL SMALLINT
    pub birth_year: Option<i16>,  // PostgreSQL SMALLINT
}

// This is what the user writes with combined approach for PostgreSQL
#[sqlx_data::repo]
trait UserRepo {
    #[dml("SELECT 1 as \"ping!\"")]
    async fn ping(&self) -> Result<i32>;

    #[dml(
        "SELECT id as \"id!: Id\", name, email, age as \"age: i16\", birth_year as \"birth_year: i16\" FROM users WHERE id = $1"
    )]
    async fn find_by_id(&self, id: i64) -> Result<User>;

    #[dml(
        "SELECT id as \"id!: Id\", name, email, age as \"age: i16\", birth_year as \"birth_year: i16\" FROM users WHERE id = $1"
    )]
    async fn find_optional_by_id(&self, id: i64) -> Result<Option<User>>;

    #[dml(
        "SELECT id as \"id!: Id\", name, email, age as \"age: i16\", birth_year as \"birth_year: i16\" FROM users WHERE id = $1"
    )]
    async fn find_many_by_id(&self, id: i64) -> Result<Vec<User>>;

    // PostgreSQL-specific queries with strong types
    #[dml("SELECT id as \"id!: Id\", name, email, age as \"age: i16\", birth_year as \"birth_year: i16\" FROM users WHERE age >= $1 ORDER BY age ASC")]
    async fn find_adults(&self, min_age: i16) -> Result<Vec<User>>;

    #[dml("SELECT name FROM users WHERE id = $1")]
    async fn get_user_name(&self, id: i64) -> Result<String>;

    #[dml("SELECT COUNT(*) as \"count!\" FROM users WHERE age >= $1")]
    async fn count_adults(&self, min_age: i16) -> Result<i64>;

    // Test nullable fields with PostgreSQL types
    #[dml("SELECT name FROM users WHERE birth_year IS NULL")]
    async fn get_names_without_birth_year(&self) -> Result<Vec<String>>;

    // PostgreSQL-specific: Using LIMIT with strong typing
    #[dml("SELECT id as \"id!: Id\", name, email, age as \"age: i16\", birth_year as \"birth_year: i16\" FROM users ORDER BY age DESC LIMIT $1")]
    async fn get_oldest_users(&self, limit_count: i64) -> Result<Vec<User>>;

    // PostgreSQL aggregations
    #[dml("SELECT AVG(age) as avg_age FROM users")]
    async fn get_average_age(&self) -> Result<Option<BigDecimal>>;

    #[dml("SELECT MIN(age) as \"min_age!\", MAX(age) as \"max_age!\" FROM users WHERE age > 0")]
    async fn get_age_bounds(&self) -> Result<(i16, i16)>;

    // Insert with PostgreSQL
    #[dml("INSERT INTO users (name, email, age, birth_year) VALUES ($1, $2, $3, $4)")]
    async fn create_user(&self, name: String, email: String, age: i16, birth_year: Option<i16>) -> Result<QueryResult>;

    // Update with PostgreSQL syntax
    #[dml("UPDATE users SET age = $1 WHERE id = $2")]
    async fn update_user_age(&self, age: i16, id: i64) -> Result<QueryResult>;

    // Delete with PostgreSQL syntax
    #[dml("DELETE FROM users WHERE id = $1")]
    async fn delete_user(&self, id: i64) -> Result<QueryResult>;
}

// Test implementation
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
    async fn test_ping(pool: Pool) {
        let app = TestApp { pool };
        let result = app.ping().await.unwrap();
        assert_eq!(result, 1);
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_find_by_id(pool: Pool) {
        let app = TestApp { pool };
        let user = app.find_by_id(1).await.unwrap();

        assert_eq!(user.id, Id(1));
        assert_eq!(user.name, "Alice");
        assert_eq!(user.email, "alice@example.com");
        assert_eq!(user.age, 30);
        assert_eq!(user.birth_year, Some(1993));
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_find_optional_by_id(pool: Pool) {
        let app = TestApp { pool };

        let user = app.find_optional_by_id(1).await.unwrap();
        assert!(user.is_some());
        assert_eq!(user.unwrap().name, "Alice");

        let user = app.find_optional_by_id(999).await.unwrap();
        assert!(user.is_none());
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_find_many_by_id(pool: Pool) {
        let app = TestApp { pool };
        let users = app.find_many_by_id(1).await.unwrap();

        assert_eq!(users.len(), 1);
        assert_eq!(users[0].name, "Alice");
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_find_adults(pool: Pool) {
        let app = TestApp { pool };
        let adults = app.find_adults(25).await.unwrap();

        // Should have 16 users with age >= 25
        assert_eq!(adults.len(), 16);
        assert!(adults.iter().all(|user| user.age >= 25));

        // Should be ordered by age ASC
        for i in 1..adults.len() {
            assert!(adults[i-1].age <= adults[i].age);
        }
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_get_user_name(pool: Pool) {
        let app = TestApp { pool };
        let name = app.get_user_name(1).await.unwrap();
        assert_eq!(name, "Alice");
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_count_adults(pool: Pool) {
        let app = TestApp { pool };
        let count = app.count_adults(25).await.unwrap();
        assert_eq!(count, 16);
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_get_names_without_birth_year(pool: Pool) {
        let app = TestApp { pool };
        let names = app.get_names_without_birth_year().await.unwrap();

        // Charlie, Paul, Quinn have NULL birth_year
        assert_eq!(names.len(), 3);
        assert!(names.contains(&"Charlie".to_string()));
        assert!(names.contains(&"Paul".to_string()));
        assert!(names.contains(&"Quinn".to_string()));
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_get_oldest_users(pool: Pool) {
        let app = TestApp { pool };
        let users = app.get_oldest_users(3).await.unwrap();

        assert_eq!(users.len(), 3);
        // Should be ordered by age DESC
        assert!(users[0].age >= users[1].age);
        assert!(users[1].age >= users[2].age);

        // Eve (42) should be first
        assert_eq!(users[0].name, "Eve");
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_get_average_age(pool: Pool) {
        let app = TestApp { pool };
        let avg_age = app.get_average_age().await.unwrap();

        assert!(avg_age.is_some());
        let avg = avg_age.unwrap();
        // Should be around 30 based on our test data
        assert!(avg > BigDecimal::from(25) && avg < BigDecimal::from(35));
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_get_age_bounds(pool: Pool) {
        let app = TestApp { pool };
        let (min_age, max_age) = app.get_age_bounds().await.unwrap();

        assert_eq!(min_age, 19); // Henry
        assert_eq!(max_age, 42); // Eve
    }
}
