use futures::{Stream, StreamExt};
use sqlx_data::{Pool, Result, dml, repo};

// Simple model for basic streaming tests
#[derive(Debug, sqlx::FromRow, PartialEq)]
pub struct SimpleUser {
    pub id: i64,
    pub name: String,
    pub age: u8, // MySQL TINYINT UNSIGNED
}

#[repo]
trait BasicStreamRepo {
    // Most basic stream query
    #[dml("SELECT id, name, age FROM users")]
    fn stream_users(&self) -> impl Stream<Item = Result<SimpleUser>> + Send;

    // Stream just names
    #[dml("SELECT name FROM users ORDER BY name")]
    fn stream_names(&self) -> impl Stream<Item = Result<String>> + Send;

    // Stream just ages
    #[dml("SELECT age FROM users WHERE age >= ?")]
    fn stream_ages(&self, min_age: u8) -> impl Stream<Item = Result<u8>> + Send;

    // Stream with simple filter
    #[dml("SELECT id, name, age FROM users WHERE id <= ?")]
    fn stream_first_users(&self, max_id: i64) -> impl Stream<Item = Result<SimpleUser>> + Send;

    // Stream single column with COUNT
    #[dml("SELECT COUNT(*) FROM users")]
    fn stream_count(&self) -> impl Stream<Item = Result<i64>> + Send;
}

pub struct BasicStreamApp {
    pool: Pool,
}

impl BasicStreamRepo for BasicStreamApp {
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
    async fn test_basic_user_stream(pool: Pool) {
        let app = BasicStreamApp { pool };

        let mut stream = app.stream_users();
        let mut users = Vec::new();

        while let Some(user_result) = stream.next().await {
            let user = user_result.unwrap();
            users.push(user);
        }

        assert_eq!(users.len(), 20); // From fixture

        // Verify first user
        assert_eq!(users[0].id, 1);
        assert_eq!(users[0].name, "Alice");
        assert_eq!(users[0].age, 30);
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_stream_names_only(pool: Pool) {
        let app = BasicStreamApp { pool };

        let mut stream = app.stream_names();
        let mut names = Vec::new();

        while let Some(result) = stream.next().await {
            names.push(result.unwrap());
        }

        assert_eq!(names.len(), 20);

        // Names should be sorted alphabetically
        for i in 1..names.len() {
            assert!(names[i - 1] <= names[i]);
        }

        // Should contain Alice
        assert!(names.contains(&"Alice".to_string()));
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_stream_ages_filtered(pool: Pool) {
        let app = BasicStreamApp { pool };

        let mut stream = app.stream_ages(30);
        let mut ages = Vec::new();

        while let Some(result) = stream.next().await {
            ages.push(result.unwrap());
        }

        // All ages should be >= 30
        for &age in &ages {
            assert!(age >= 30);
        }

        assert!(ages.len() > 0);
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_stream_first_few_users(pool: Pool) {
        let app = BasicStreamApp { pool };

        let mut stream = app.stream_first_users(5);
        let mut users = Vec::new();

        while let Some(result) = stream.next().await {
            users.push(result.unwrap());
        }

        assert_eq!(users.len(), 5);

        // All IDs should be <= 5
        for user in &users {
            assert!(user.id <= 5);
        }

        // Should include Alice (id = 1)
        assert!(users.iter().any(|u| u.name == "Alice" && u.id == 1));
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_stream_count(pool: Pool) {
        let app = BasicStreamApp { pool };

        let mut stream = app.stream_count();
        let mut counts = Vec::new();

        while let Some(result) = stream.next().await {
            counts.push(result.unwrap());
        }

        // Should have exactly one result
        assert_eq!(counts.len(), 1);
        assert_eq!(counts[0], 20);
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_stream_manual_iteration(pool: Pool) {
        let app = BasicStreamApp { pool };

        let mut stream = app.stream_first_users(3);

        // Manually iterate through first 2 items
        let first = stream.next().await;
        assert!(first.is_some());
        let first_user = first.unwrap().unwrap();
        assert_eq!(first_user.id, 1);
        assert_eq!(first_user.name, "Alice");

        let second = stream.next().await;
        assert!(second.is_some());
        let second_user = second.unwrap().unwrap();
        assert_eq!(second_user.id, 2);
        assert_eq!(second_user.name, "Bob");

        // Get remaining items
        let mut remaining = Vec::new();
        while let Some(result) = stream.next().await {
            remaining.push(result.unwrap());
        }
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].id, 3);
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_stream_take_and_skip(pool: Pool) {
        let app = BasicStreamApp { pool };

        let mut stream = app.stream_users();

        // Take only first 3 users
        let mut first_three = Vec::new();
        let mut count = 0;
        while let Some(result) = stream.next().await {
            if count >= 3 {
                break;
            }
            first_three.push(result.unwrap());
            count += 1;
        }

        assert_eq!(first_three.len(), 3);
        assert_eq!(first_three[0].name, "Alice");
        assert_eq!(first_three[1].name, "Bob");
        assert_eq!(first_three[2].name, "Charlie");
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_stream_enumerate(pool: Pool) {
        let app = BasicStreamApp { pool };

        let mut stream = app.stream_first_users(5);

        let mut index = 0;
        while let Some(user_result) = stream.next().await {
            let user = user_result.unwrap();

            // Verify that users come in order
            match index {
                0 => assert_eq!(user.name, "Alice"),
                1 => assert_eq!(user.name, "Bob"),
                2 => assert_eq!(user.name, "Charlie"),
                3 => assert_eq!(user.name, "Diana"),
                4 => assert_eq!(user.name, "Eve"),
                _ => panic!("Unexpected user at index {}", index),
            }

            index += 1;
        }

        assert_eq!(index, 5);
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_stream_find_specific(pool: Pool) {
        let app = BasicStreamApp { pool };

        let mut stream = app.stream_users();
        let mut charlie = None;

        // Find a specific user by name
        while let Some(result) = stream.next().await {
            let user = result.unwrap();
            if user.name == "Charlie" {
                charlie = Some(user);
                break;
            }
        }

        assert!(charlie.is_some());
        let charlie = charlie.unwrap();
        assert_eq!(charlie.name, "Charlie");
        assert_eq!(charlie.id, 3);
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_stream_any_condition(pool: Pool) {
        let app = BasicStreamApp { pool };

        let mut stream = app.stream_ages(20);
        let mut has_thirty = false;

        // Check if any user has age exactly 30
        while let Some(result) = stream.next().await {
            let age = result.unwrap();
            if age == 30 {
                has_thirty = true;
                break;
            }
        }

        assert!(has_thirty); // Alice is 30
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_stream_max_age(pool: Pool) {
        let app = BasicStreamApp { pool };

        let mut stream = app.stream_ages(0); // Get all ages
        let mut max_age = 0u8;

        // Find maximum age
        while let Some(result) = stream.next().await {
            let age = result.unwrap();
            max_age = max_age.max(age);
        }

        assert_eq!(max_age, 42); // Eve is the oldest at 42
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_empty_stream_result(pool: Pool) {
        let app = BasicStreamApp { pool };

        // Stream with impossible condition
        let mut stream = app.stream_ages(200); // No one is 200+ years old
        let mut ages = Vec::new();

        while let Some(result) = stream.next().await {
            ages.push(result.unwrap());
        }

        assert!(ages.is_empty());
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_stream_chunks(pool: Pool) {
        let app = BasicStreamApp { pool };

        let mut stream = app.stream_first_users(6);

        // Process in chunks of 2
        let mut chunk_count = 0;
        let mut total_processed = 0;
        let mut current_chunk = Vec::new();

        while let Some(result) = stream.next().await {
            let user = result.unwrap();
            current_chunk.push(user);

            if current_chunk.len() == 2 {
                chunk_count += 1;
                total_processed += current_chunk.len();
                assert!(current_chunk.len() <= 2);
                current_chunk.clear();
            }
        }

        // Handle remaining items
        if !current_chunk.is_empty() {
            chunk_count += 1;
            total_processed += current_chunk.len();
            assert!(current_chunk.len() <= 2);
        }

        assert_eq!(chunk_count, 3); // 6 users / 2 per chunk = 3 chunks
        assert_eq!(total_processed, 6);
    }
}
