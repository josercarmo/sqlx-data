use sqlx_data::{Pool, Result, dml, repo};

#[derive(Debug, Clone, PartialEq, sqlx::FromRow)]
pub struct User {
    pub id: i64,
    pub name: String,
    pub email: String,
    pub age: i16,
}

// Test repository with different generic signature patterns
#[repo]
trait GenericSignatureRepo {
    // Basic Into<String> test
    #[dml("INSERT INTO users (name, email, age) VALUES ($1, $2, $3) RETURNING id")]
    async fn create_user_into_string(
        &self,
        name: impl Into<String>,
        email: impl Into<String>,
        age: i16,
    ) -> Result<i64>;

    // AsRef<str> test
    #[dml("SELECT id, name, email, age FROM users WHERE name = $1")]
    async fn find_by_name_as_ref(&self, name: &str) -> Result<Vec<User>>;

    // Multiple generic parameters
    #[dml("SELECT COUNT(*) FROM users WHERE name LIKE $1 AND email LIKE $2")]
    async fn count_with_patterns(
        &self,
        name_pattern: impl Into<String>,
        email_pattern: &str,
    ) -> Result<Option<i64>>;

    // Generic with Option
    #[dml("SELECT id, name, email, age FROM users WHERE age > $1 AND ($2::TEXT IS NULL OR name LIKE $2)")]
    async fn find_with_optional_filter(
        &self,
        min_age: i16,
        name_filter: Option<impl Into<String>>,
    ) -> Result<Vec<User>>;

    // Multiple Into<String> with different positions
    #[dml(
        "UPDATE users SET name = $2, email = $3 WHERE id = $1 RETURNING id, name, email, age"
    )]
    async fn update_user_multiple_into(
        &self,
        id: i64,
        new_name: impl Into<String>,
        new_email: impl Into<String>,
    ) -> Result<User>;

    // Test with slice reference
    #[dml("SELECT COUNT(*) FROM users WHERE name = ANY($1)")]
    async fn count_users_in_names(&self, names: &[String]) -> Result<Option<i64>>;

    // Generic return with different types
    #[dml("SELECT name FROM users WHERE id = $1")]
    async fn get_name_generic(&self, id: i64) -> Result<String>;

    #[dml("SELECT age FROM users WHERE id = $1")]
    async fn get_age_generic(&self, id: i64) -> Result<i16>;

    // Complex generic combination
    #[dml(
        r#"
        SELECT id, name, email, age
        FROM users
        WHERE (name LIKE $1 OR email LIKE $2)
        AND age BETWEEN $3 AND $4
        ORDER BY name
        "#
    )]
    async fn complex_search_generics(
        &self,
        name_pattern: &str,
        email_pattern: impl Into<String>,
        min_age: i16,
        max_age: i16,
    ) -> Result<Vec<User>>;
}

pub struct TestGenericSignatureApp {
    pool: Pool,
}

impl GenericSignatureRepo for TestGenericSignatureApp {
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
    async fn test_into_string_params(pool: Pool) -> Result<()> {
        let app = TestGenericSignatureApp { pool };

        // Test with string literals
        let id1 = app
            .create_user_into_string("Alice Test", "alice.test@example.com", 28)
            .await?;
        assert!(id1 > 0);

        // Test with String values
        let name = String::from("Bob Test");
        let email = String::from("bob.test@example.com");
        let id2 = app.create_user_into_string(name, email, 32).await?;
        assert!(id2 > 0);

        // Test with &str
        let name_ref = "Charlie Test";
        let email_ref = "charlie.test@example.com";
        let id3 = app.create_user_into_string(name_ref, email_ref, 25).await?;
        assert!(id3 > 0);

        Ok(())
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_as_ref_str_params(pool: Pool) -> Result<()> {
        let app = TestGenericSignatureApp { pool };

        // Test with string literal
        let users1 = app.find_by_name_as_ref("Alice").await?;
        assert_eq!(users1.len(), 1);

        // Test with String
        let name = String::from("Bob");
        let users2 = app.find_by_name_as_ref(&name).await?;
        assert_eq!(users2.len(), 1);

        // Test with &String
        let name_string = String::from("Charlie");
        let users3 = app.find_by_name_as_ref(&name_string).await?;
        assert_eq!(users3.len(), 1);

        Ok(())
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_mixed_generic_params(pool: Pool) -> Result<()> {
        let app = TestGenericSignatureApp { pool };

        // Test with different combinations
        let count1 = app
            .count_with_patterns("%Alice%", "@example.com")
            .await?;
        assert!(count1 >= Some(0));

        let pattern1 = String::from("%Bob%");
        let pattern2 = "@example.com";
        let count2 = app.count_with_patterns(pattern1, pattern2).await?;
        assert!(count2 >= Some(0));

        Ok(())
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_optional_generic_params(pool: Pool) -> Result<()> {
        let app = TestGenericSignatureApp { pool };

        // Test with Some value
        let users1 = app
            .find_with_optional_filter(20, Some("Alice%"))
            .await?;
        assert!(!users1.is_empty());

        // Test with None
        let users2 = app
            .find_with_optional_filter(20, Option::<String>::None)
            .await?;
        assert!(!users2.is_empty());

        // Test with Some(String)
        let filter = Some(String::from("Bob%"));
        let users3 = app.find_with_optional_filter(20, filter).await?;
        assert!(!users3.is_empty());

        Ok(())
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_update_multiple_into(pool: Pool) -> Result<()> {
        let app = TestGenericSignatureApp { pool };

        // First get a user ID
        let users = app.find_by_name_as_ref("Alice").await?;
        assert!(!users.is_empty());
        let user_id = users[0].id;

        // Test update with string literals
        let updated = app
            .update_user_multiple_into(user_id, "Alice Updated", "alice.updated@example.com")
            .await?;

        assert_eq!(updated.name, "Alice Updated");
        assert_eq!(updated.email, "alice.updated@example.com");

        // Test update with String values
        let new_name = String::from("Alice Final");
        let new_email = String::from("alice.final@example.com");
        let updated2 = app
            .update_user_multiple_into(user_id, new_name, new_email)
            .await?;

        assert_eq!(updated2.name, "Alice Final");
        assert_eq!(updated2.email, "alice.final@example.com");

        Ok(())
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_slice_reference_params(pool: Pool) -> Result<()> {
        let app = TestGenericSignatureApp { pool };

        let names = vec![
            String::from("Alice"),
            String::from("Bob"),
            String::from("Charlie"),
        ];

        let count = app.count_users_in_names(&names).await?;
        assert!(count >= Some(3)); // Should find at least our fixture users

        // Test with empty slice
        let empty_names: Vec<String> = vec![];
        let count_empty = app.count_users_in_names(&empty_names).await?;
        assert_eq!(count_empty, Some(0));

        Ok(())
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_generic_returns(pool: Pool) -> Result<()> {
        let app = TestGenericSignatureApp { pool };

        // Get a user ID from fixtures
        let users = app.find_by_name_as_ref("Alice").await?;
        assert!(!users.is_empty());
        let user_id = users[0].id;

        // Test string return
        let name = app.get_name_generic(user_id).await?;
        assert!(!name.is_empty());

        // Test numeric return
        let age = app.get_age_generic(user_id).await?;
        assert!(age > 0);

        Ok(())
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_complex_generic_combination(pool: Pool) -> Result<()> {
        let app = TestGenericSignatureApp { pool };

        // Test with mixed generic types
        let users = app
            .complex_search_generics("%Alice%", String::from("@example.com"), 20, 40)
            .await?;

        assert!(!users.is_empty());
        for user in users {
            assert!(user.age >= 20 && user.age <= 40);
            assert!(user.name.contains("Alice") || user.email.contains("@example.com"));
        }

        // Test with string literals
        let users2 = app
            .complex_search_generics("Bob", "@example.com", 25, 35)
            .await?;

        // Should work without errors
        assert_eq!(users2.len(), 1);

        Ok(())
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_signature_type_inference(pool: Pool) -> Result<()> {
        let app = TestGenericSignatureApp { pool };

        // Test that Rust can infer types properly
        let name = "Type Test";
        let email = "type.test@example.com";

        // This should compile without explicit type annotations
        let id = app.create_user_into_string(name, email, 30).await?;
        assert!(id > 0);

        // This should also work with method chaining
        let users = app
            .find_by_name_as_ref("Type Test")
            .await?;

        assert_eq!(users.len(), 1);
        assert_eq!(users[0].name, "Type Test");

        Ok(())
    }
}