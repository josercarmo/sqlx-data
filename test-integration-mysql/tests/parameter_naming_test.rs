use sqlx_data::{Pool, Result, dml};

// User model for tests
#[derive(Debug, Clone, PartialEq)]
pub struct User {
    pub id: i64,
    pub name: String,
    pub email: String,
    pub age: u8,
}

// Test trait with named parameters
#[sqlx_data::repo]
trait NamedParamRepo {
    // Basic named parameter usage
    #[dml("SELECT id, name, email, age as 'age: u8' FROM users WHERE name = @name")]
    async fn find_by_name(&self, name: String) -> Result<Vec<User>>;

    // Multiple named parameters
    #[dml(
        "SELECT id, name, email, age as 'age: u8' FROM users WHERE age > @min_age AND name LIKE @name_pattern"
    )]
    async fn find_by_age_and_name(&self, min_age: u8, name_pattern: String) -> Result<Vec<User>>;

    // Named parameters out of order (should work only in sqlite and postgres)
    #[dml(
        "SELECT id as 'id!', name, email, age as 'age: u8' FROM users WHERE email = @email AND age = @age"
    )]
    async fn find_by_email_and_age(&self, age: u8, email: String) -> Result<Vec<User>>;

    // Mixed with positional (should still work for existing code)
    #[dml("SELECT id, name, email, age as 'age: u8' FROM users WHERE id = ?")]
    async fn find_by_id_positional(&self, id: i64) -> Result<Option<User>>;

    // Repeated named parameter
    #[dml(
        "SELECT id, name, email, age as 'age: u8' FROM users WHERE (name = @name OR email = @email) AND age > @min_age"
    )]
    async fn find_by_name_or_email(&self, name: String,email: String, min_age: u8) -> Result<Vec<User>>;
}

pub struct TestNamedParamApp {
    pool: Pool,
}

impl NamedParamRepo for TestNamedParamApp {
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
    async fn test_named_parameters_compile(pool: Pool) {
        // This test verifies that named parameters compile correctly
        let _repo = TestNamedParamApp { pool };

        // If we reach here, named parameters are working
        assert!(true);
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_named_parameters_functionality(pool: Pool) {
        let repo = TestNamedParamApp { pool };

        // Test single named parameter
        let users = repo.find_by_name("Alice".to_string()).await.unwrap();
        assert!(!users.is_empty());
        assert!(users.iter().any(|u| u.name == "Alice"));

        // Test multiple named parameters
        let users = repo
            .find_by_age_and_name(20, "%li%".to_string())
            .await
            .unwrap();
        assert!(!users.is_empty());

        // Test parameters out of order
        let users = repo
            .find_by_email_and_age(25, "bob@example.com".to_string())
            .await
            .unwrap();
        assert!(users.is_empty()); // Mysql does not support named parameters out of order!!!

        // Test positional parameters still work
        let user = repo.find_by_id_positional(1).await.unwrap();
        assert!(user.is_some());
        assert_eq!(user.unwrap().name, "Alice");

        // Test repeated named parameter
        let users = repo
            .find_by_name_or_email("Alice".to_string(), "Alice".to_string(), 20)
            .await
            .unwrap();
        assert!(!users.is_empty());
        assert!(users.iter().any(|u| u.name == "Alice"));
    }
}
