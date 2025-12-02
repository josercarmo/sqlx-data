use sqlx_data::{QueryResult, Result, dml, repo};

#[derive(Debug, sqlx::FromRow)]
pub struct User {
    pub id: i64,
    pub name: String,
    pub email: String,
    pub age: i64,
    pub birth_year: Option<i64>,
}

#[repo]
trait UncheckedRepo {
    // Test unchecked scalar queries
    #[dml("SELECT COUNT(*) FROM users", unchecked)]
    async fn count_users(&self) -> Result<i64>;

    // Test unchecked struct queries
    #[dml(
        "SELECT id, name, email, age, birth_year FROM users WHERE id = $1",
        unchecked
    )]
    async fn find_user_by_id(&self, id: i64) -> Result<User>;

    // Test unchecked tuple queries
    #[dml("SELECT id, name, email FROM users WHERE age > $1", unchecked)]
    async fn find_user_tuples(&self, min_age: i64) -> Result<Vec<(i64, String, String)>>;

    // Test unchecked optional queries
    #[dml("SELECT id, name FROM users WHERE email = $1", unchecked)]
    async fn find_user_optional(&self, email: String) -> Result<Option<(i64, String)>>;

    // Test unchecked queries returning Vec<struct>
    #[dml(
        "SELECT id, name, email, age, birth_year FROM users WHERE age BETWEEN $1 AND $2",
        unchecked
    )]
    async fn find_users_in_age_range(&self, min_age: i64, max_age: i64) -> Result<Vec<User>>;

    // Test unchecked queries with execute (QueryResult)
    #[dml("INSERT INTO users (name, email, age, birth_year) VALUES ($1, $2, $3, $4)")]
    async fn insert_user(
        &self,
        name: String,
        email: String,
        age: i64,
        birth_year: Option<i64>,
    ) -> Result<QueryResult>;

    #[dml(
        "INSERT INTO users (name, email, age, birth_year) VALUES ($1, $2, $3, $4) RETURNING *",
        unchecked
    )]
    async fn insert_user_returning(
        &self,
        name: String,
        email: String,
        age: i64,
        birth_year: Option<i64>,
    ) -> Result<QueryResult>;

    // Test unchecked queries with Unit return (should work with unchecked)
    #[dml("DELETE FROM users WHERE id = $1", unchecked)]
    async fn delete_user(&self, id: i64) -> Result<()>;

    // Test unchecked queries with complex SQL that might not parse well
    #[dml(
        "SELECT id, name, email, age, (age * 365) as approximate_days FROM users WHERE birth_year > $1",
        unchecked
    )]
    async fn get_users_with_calculated_field(
        &self,
        min_birth_year: i64,
    ) -> Result<Vec<(i64, String, String, i64, i64)>>;

    // Test unchecked queries with database-specific functions
    #[dml(
        "SELECT id, name, email, age, birth_year, CASE WHEN age >= 65 THEN 'Senior' WHEN age >= 18 THEN 'Adult' ELSE 'Minor' END as category FROM users",
        unchecked
    )]
    async fn get_users_with_category(
        &self,
    ) -> Result<Vec<(i64, String, String, i64, Option<i64>, String)>>;

    // Compare with checked versions for same queries
    #[dml("SELECT COUNT(*) FROM users")]
    async fn count_users_checked(&self) -> Result<i64>;

    #[dml("SELECT id, name, email, age, birth_year FROM users WHERE id = $1")]
    async fn find_user_by_id_checked(&self, id: i64) -> Result<User>;

    #[dml(
        "INSERT INTO users (name, email, age, birth_year) VALUES ($1, $2, $3, $4)",
        unchecked
    )]
    async fn insert_user_checked(
        &self,
        name: String,
        email: String,
        age: i64,
        birth_year: Option<i64>,
    ) -> Result<QueryResult>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx_data::Pool;

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_scalar_query_with_unchecked(pool: Pool) {
        let repo = TestUncheckedRepo { pool: &pool };

        let count_unchecked = repo.count_users().await.unwrap();
        let count_checked = repo.count_users_checked().await.unwrap();

        assert_eq!(count_unchecked, count_checked);
        assert_eq!(count_unchecked, 20); // We have 3 test users
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_struct_query_with_unchecked(pool: Pool) {
        let repo = TestUncheckedRepo { pool: &pool };

        let user_unchecked = repo.find_user_by_id(1).await.unwrap();
        let user_checked = repo.find_user_by_id_checked(1).await.unwrap();

        assert_eq!(user_unchecked.id, user_checked.id);
        assert_eq!(user_unchecked.name, user_checked.name);
        assert_eq!(user_unchecked.email, user_checked.email);
        assert_eq!(user_unchecked.age, user_checked.age);
        assert_eq!(user_unchecked.birth_year, user_checked.birth_year);

        assert_eq!(user_unchecked.name, "Alice");
        assert_eq!(user_unchecked.email, "alice@example.com");
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_tuple_query_with_unchecked(pool: Pool) {
        let repo = TestUncheckedRepo { pool: &pool };

        let users = repo.find_user_tuples(20).await.unwrap();

        assert_eq!(users.len(), 19); // Alice (25) and Bob (30)

        let alice = users.iter().find(|(_, name, _)| name == "Alice").unwrap();
        assert_eq!(alice.0, 1); // id
        assert_eq!(alice.2, "alice@example.com"); // email

        let bob = users.iter().find(|(_, name, _)| name == "Bob").unwrap();
        assert_eq!(bob.0, 2); // id
        assert_eq!(bob.2, "bob@example.com"); // email
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_optional_query_with_unchecked(pool: Pool) {
        let repo = TestUncheckedRepo { pool: &pool };

        // Test finding existing user
        let user_found = repo
            .find_user_optional("alice@example.com".to_string())
            .await
            .unwrap();
        assert!(user_found.is_some());
        let (id, name) = user_found.unwrap();
        assert_eq!(id, 1);
        assert_eq!(name, "Alice");

        // Test finding non-existing user
        let user_not_found = repo
            .find_user_optional("nonexistent@example.com".to_string())
            .await
            .unwrap();
        assert!(user_not_found.is_none());
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_vec_struct_query_with_unchecked(pool: Pool) {
        let repo = TestUncheckedRepo { pool: &pool };

        let users = repo.find_users_in_age_range(20, 30).await.unwrap();

        assert_eq!(users.len(), 10);

        let alice = users.iter().find(|u| u.name == "Alice").unwrap();
        assert_eq!(alice.age, 30);
        assert_eq!(alice.email, "alice@example.com");

        let bob = users.iter().find(|u| u.name == "Bob").unwrap();
        assert_eq!(bob.age, 25);
        assert_eq!(bob.email, "bob@example.com");
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_insert_query_with_unchecked(pool: Pool) {
        let repo = TestUncheckedRepo { pool: &pool };

        let result_unchecked = repo
            .insert_user(
                "Dave".to_string(),
                "dave@example.com".to_string(),
                35,
                Some(1988),
            )
            .await
            .unwrap();

        let result_checked = repo
            .insert_user_checked(
                "Eve".to_string(),
                "eve1@example.com".to_string(),
                28,
                Some(1995),
            )
            .await
            .unwrap();

        assert!(result_unchecked.rows_affected() > 0);
        assert!(result_checked.rows_affected() > 0);

        // Verify both users were inserted
        let count = repo.count_users().await.unwrap();
        assert_eq!(count, 22); // 3 initial + 2 new
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_delete_query_with_unchecked(pool: Pool) {
        let repo = TestUncheckedRepo { pool: &pool };

        // Verify user exists before deletion
        let user_before = repo.find_user_by_id(3).await.unwrap();
        assert_eq!(user_before.name, "Charlie");

        // Delete the user
        let result = repo.delete_user(3).await;
        assert!(result.is_ok());

        // Verify user count decreased
        let count = repo.count_users().await.unwrap();
        assert_eq!(count, 19); // 3 - 1 = 2
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_complex_calculated_field_with_unchecked(pool: Pool) {
        let repo = TestUncheckedRepo { pool: &pool };

        let users = repo.get_users_with_calculated_field(1980).await.unwrap();

        // Should get users born after 1980
        assert!(!users.is_empty());

        for (id, name, email, age, approx_days) in &users {
            assert!(*age > 0);
            assert_eq!(*approx_days, age * 365);
            assert!(!name.is_empty());
            assert!(!email.is_empty());
            assert!(*id > 0);
        }
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_case_when_query_with_unchecked(pool: Pool) {
        let repo = TestUncheckedRepo { pool: &pool };

        let users = repo.get_users_with_category().await.unwrap();

        assert_eq!(users.len(), 20);

        for (id, name, email, age, birth_year, category) in &users {
            assert!(*id > 0);
            assert!(!name.is_empty());
            assert!(!email.is_empty());
            assert!(*age > 0);

            // Check category logic
            if *age >= 65 {
                assert_eq!(category, "Senior");
            } else if *age >= 18 {
                assert_eq!(category, "Adult");
            } else {
                assert_eq!(category, "Minor");
            }

            // Verify birth_year is consistent
            if let Some(by) = birth_year {
                assert!(*by > 1900 && *by < 2010);
            }
        }
    }

    struct TestUncheckedRepo<'a> {
        pool: &'a Pool,
    }

    impl<'a> UncheckedRepo for TestUncheckedRepo<'a> {
        fn get_pool(&self) -> &Pool {
            self.pool
        }
    }
}
