use futures::{Stream, StreamExt};
use sqlx_data::{Pool, Result, dml};

// User model for stream tests
#[derive(Debug, Clone, PartialEq, sqlx::FromRow)]
pub struct User {
    pub id: i64,
    pub name: String,
    pub age: u8,
}

// Basic repository with stream methods
#[sqlx_data::repo]
trait StreamTestRepo {
    // Stream scalar values
    #[dml("SELECT age FROM users WHERE age >= $1")]
    fn stream_ages(&self, min_age: u8) -> impl Stream<Item = Result<i32>>;

    // Stream struct values
    #[dml("SELECT id, name, age as 'age: u8' FROM users WHERE age >= $1")]
    fn stream_users(&self, min_age: u8) -> impl Stream<Item = Result<User>>;

    // Stream tuple values
    #[dml("SELECT id, name FROM users WHERE age >= $1")]
    fn stream_user_names(&self, min_age: u8) -> impl Stream<Item = Result<(i64, String)>>;
}

// Test implementation
pub struct TestStreamRepo {
    pool: Pool,
}

impl StreamTestRepo for TestStreamRepo {
    fn get_pool(&self) -> &Pool {
        &self.pool
    }
}

#[cfg(test)]
mod tests {
    use sqlx_data::Pool;

    use super::*;

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_stream_scalar_ages(pool: Pool) {
        let repo = TestStreamRepo { pool };

        let mut stream = repo.stream_ages(40);
        let mut ages = Vec::new();

        while let Some(result) = stream.next().await {
            ages.push(result.unwrap());
        }

        // Should get ages >= 40 (only Eve=42, Tina=40)
        ages.sort();
        assert_eq!(ages, vec![40, 42]);
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_stream_struct_users(pool: Pool) {
        let repo = TestStreamRepo { pool };

        let mut stream = repo.stream_users(40);
        let mut users = Vec::new();

        while let Some(result) = stream.next().await {
            users.push(result.unwrap());
        }

        // Should get users with age >= 40 (Eve=42, Tina=40)
        users.sort_by_key(|u| u.age);
        assert_eq!(users.len(), 2);
        assert_eq!(users[0].name, "Tina");
        assert_eq!(users[0].age, 40);
        assert_eq!(users[1].name, "Eve");
        assert_eq!(users[1].age, 42);
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_stream_tuple_names(pool: Pool) {
        let repo = TestStreamRepo { pool };

        let mut stream = repo.stream_user_names(40);
        let mut user_names = Vec::new();

        while let Some(result) = stream.next().await {
            user_names.push(result.unwrap());
        }

        // Should get (id, name) tuples for users with age >= 40 (Eve=id:5, Tina=id:20)
        user_names.sort_by_key(|(id, _)| *id);
        assert_eq!(user_names.len(), 2);
        assert_eq!(user_names[0], (5, "Eve".to_string()));
        assert_eq!(user_names[1], (20, "Tina".to_string()));
    }
}
