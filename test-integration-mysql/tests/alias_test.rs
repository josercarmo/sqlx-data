use sqlx::types::BigDecimal;
use sqlx_data::{Pool, Result, dml, repo};

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

// Test trait with alias definitions using MySQL-specific types and SQL
#[repo]
#[alias(
    user_columns = "id, name, email, age, birth_year"
)]
#[alias(user_table = "users")]
#[alias(count_query = "SELECT COUNT(*) FROM users")]
#[alias(avg_query = "SELECT AVG(age) FROM users")]
#[alias(where_clause = "WHERE age >= ?")]
trait AliasUserRepo {
    // Basic alias substitution with MySQL parameter style
    #[dml("SELECT {{user_columns}} FROM {{user_table}} WHERE id = ?")]
    async fn find_by_id(&self, id: i64) -> Result<User>;

    // Optional result with alias
    #[dml("SELECT {{user_columns}} FROM {{user_table}} WHERE id = ?")]
    async fn find_optional_by_id(&self, id: i64) -> Result<Option<User>>;

    // Vector result with alias
    #[dml("SELECT {{user_columns}} FROM {{user_table}} {{where_clause}}")]
    async fn find_users_by_age(&self, min_age: u8) -> Result<Vec<User>>;

    // Complete query as alias with strong typing
    #[dml("{{count_query}}")]
    async fn count_all_users(&self) -> Result<i64>;

    // Scalar query with alias
    #[dml("{{avg_query}}")]
    async fn average_age(&self) -> Result<Option<BigDecimal>>;

    // Simple field selection with alias
    #[dml("SELECT name FROM {{user_table}} WHERE id = ?")]
    async fn get_user_name(&self, id: i64) -> Result<String>;

    // Tuple result with alias
    #[dml("SELECT name, email FROM {{user_table}} WHERE age >= ?")]
    async fn get_name_email_by_age(&self, min_age: u8) -> Result<Vec<(String, String)>>;

    // Mixed aliases in different parts of query
    #[dml("SELECT name FROM {{user_table}} {{where_clause}}")]
    async fn get_names_by_age(&self, min_age: u8) -> Result<Vec<String>>;

    // File-based query with alias
    #[dml(file = "tests/fixtures/alias_test_query.sql")]
    async fn find_users_from_file(&self) -> Result<Vec<User>>;

    // MySQL-specific: Using LIMIT with strong types
    #[dml("SELECT {{user_columns}} FROM {{user_table}} ORDER BY age DESC LIMIT ?")]
    async fn find_oldest_users(&self, limit_count: u32) -> Result<Vec<User>>;

    // MySQL-specific: Using UNSIGNED INT types
    #[dml("SELECT MIN(age) as 'min_age!: u8', MAX(age) as 'max_age!: u8' FROM {{user_table}}")]
    async fn get_age_range(&self) -> Result<(u8, u8)>;
}

// Test implementation
pub struct TestAliasApp {
    pool: Pool,
}

impl AliasUserRepo for TestAliasApp {
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
    async fn test_basic_alias_substitution(pool: Pool) {
        let repo = TestAliasApp { pool };

        let user = repo.find_by_id(1).await.unwrap();
        assert_eq!(user.name, "Alice");
        assert_eq!(user.email, "alice@example.com");
        assert_eq!(user.age, 30);
        assert_eq!(user.birth_year, Some(1993));
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_optional_result_with_alias(pool: Pool) {
        let repo = TestAliasApp { pool };

        let user = repo.find_optional_by_id(1).await.unwrap();
        assert!(user.is_some());
        assert_eq!(user.unwrap().name, "Alice");

        let user = repo.find_optional_by_id(999).await.unwrap();
        assert!(user.is_none());
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_vector_result_with_alias(pool: Pool) {
        let repo = TestAliasApp { pool };

        let users = repo.find_users_by_age(25).await.unwrap();
        assert_eq!(users.len(), 16); // All users with age >= 25
        assert!(users.iter().any(|u| u.name == "Alice"));
        assert!(users.iter().any(|u| u.name == "Bob"));
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_complete_query_alias(pool: Pool) {
        let repo = TestAliasApp { pool };

        let count = repo.count_all_users().await.unwrap();
        assert_eq!(count, 20); // All users in fixture
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_scalar_query_alias(pool: Pool) {
        let repo = TestAliasApp { pool };

        let avg_age = repo.average_age().await.unwrap();
        assert!(avg_age.is_some());
        let avg = avg_age.unwrap();
        // Convert BigDecimal to f64 for comparison
        let avg_f64: f64 = avg.to_string().parse().unwrap();
        // Average of ages should be around 30
        assert!((avg_f64 - 30.0).abs() < 5.0);
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_simple_field_selection_with_alias(pool: Pool) {
        let repo = TestAliasApp { pool };

        let name = repo.get_user_name(1).await.unwrap();
        assert_eq!(name, "Alice".to_string());

        let name = repo.get_user_name(999).await;
        assert!(name.is_err());
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_tuple_result_with_alias(pool: Pool) {
        let repo = TestAliasApp { pool };

        let user_info = repo.get_name_email_by_age(25).await.unwrap();
        assert_eq!(user_info.len(), 16); // All users with age >= 25

        let names: Vec<&String> = user_info.iter().map(|(name, _)| name).collect();
        assert!(names.contains(&&"Alice".to_string()));
        assert!(names.contains(&&"Bob".to_string()));
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_mixed_aliases_in_query(pool: Pool) {
        let repo = TestAliasApp { pool };

        let names = repo.get_names_by_age(30).await.unwrap();
        assert_eq!(names.len(), 10); // All users with age >= 30
        assert!(names.contains(&"Alice".to_string()));
        assert!(names.contains(&"Charlie".to_string()));
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_file_based_query_with_alias(pool: Pool) {
        let repo = TestAliasApp { pool };

        let users = repo.find_users_from_file().await.unwrap();
        assert_eq!(users.len(), 10); // Users with age >= 30
        assert!(users.iter().any(|u| u.name == "Alice"));
        assert!(users.iter().any(|u| u.name == "Eve"));

        // Should be ordered by age DESC (Eve: 42, Tina: 40)
        assert_eq!(users[0].name, "Eve");
        assert_eq!(users[1].name, "Tina");
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_mysql_specific_limit(pool: Pool) {
        let repo = TestAliasApp { pool };

        let users = repo.find_oldest_users(3).await.unwrap();
        assert_eq!(users.len(), 3);
        // Should be ordered by age DESC
        assert!(users[0].age >= users[1].age);
        assert!(users[1].age >= users[2].age);
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_mysql_unsigned_types(pool: Pool) {
        let repo = TestAliasApp { pool };

        let (min_age, max_age) = repo.get_age_range().await.unwrap();
        assert!(min_age <= max_age);
        assert_eq!(min_age, 19); // Henry is the youngest
        assert_eq!(max_age, 42); // Eve is the oldest
    }
}
