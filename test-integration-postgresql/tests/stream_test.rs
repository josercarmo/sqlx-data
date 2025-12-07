use futures::{Stream, StreamExt};
use sqlx_data::{Pool, Result, dml, repo};

#[derive(Debug, Clone, PartialEq, sqlx::FromRow)]
pub struct User {
    pub id: i64,
    pub name: String,
    pub email: String,
    pub age: i16,
    pub birth_year: Option<i16>,
}

#[derive(Debug, PartialEq)]
pub struct UserInfo {
    pub name: String,
    pub age: i16,
}

#[repo]
trait UserStreamRepo {
    // Basic stream query
    #[dml("SELECT id, name, email, age, birth_year FROM users")]
    fn stream_all_users(&self) -> impl Stream<Item = Result<User>> + Send;

    // Stream with WHERE clause
    #[dml("SELECT id, name, email, age, birth_year FROM users WHERE age >= $1")]
    fn stream_users_by_min_age(&self, min_age: i16) -> impl Stream<Item = Result<User>> + Send;

    // Stream with ORDER BY
    #[dml("SELECT id, name, email, age, birth_year FROM users ORDER BY age DESC")]
    fn stream_users_by_age_desc(&self) -> impl Stream<Item = Result<User>> + Send;

    // Stream with LIMIT
    #[dml("SELECT id, name, email, age, birth_year FROM users LIMIT $1")]
    fn stream_users_limited(&self, limit: i64) -> impl Stream<Item = Result<User>> + Send;

    // Stream tuple results
    #[dml("SELECT name, age FROM users WHERE age BETWEEN $1 AND $2")]
    fn stream_user_info(&self, min_age: i16, max_age: i16) -> impl Stream<Item = Result<(String, i16)>> + Send;

    // Stream with complex WHERE
    #[dml("SELECT id, name, email, age, birth_year FROM users WHERE age >= $1 AND name LIKE $2")]
    fn stream_users_by_age_and_name(&self, min_age: i16, name_pattern: String) -> impl Stream<Item = Result<User>> + Send;

    // Stream with nullable fields
    #[dml("SELECT id, name, email, age, birth_year FROM users WHERE birth_year IS NOT NULL")]
    fn stream_users_with_birth_year(&self) -> impl Stream<Item = Result<User>> + Send;

    // Stream with subquery
    #[dml("SELECT DISTINCT name FROM users WHERE age IN (SELECT age FROM users WHERE age > $1)")]
    fn stream_names_by_subquery(&self, min_age: i16) -> impl Stream<Item = Result<String>> + Send;
}

pub struct StreamApp {
    pool: Pool,
}

impl UserStreamRepo for StreamApp {
    fn get_pool(&self) -> &Pool {
        &self.pool
    }
}

#[allow(clippy::unwrap_used)]
#[cfg(test)]
mod tests {
    use super::*;

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_stream_all_users(pool: Pool) {
        let app = StreamApp { pool };

        let mut stream = app.stream_all_users();
        let mut users = Vec::new();

        while let Some(user_result) = stream.next().await {
            users.push(user_result.unwrap());
        }

        assert_eq!(users.len(), 20); // From fixture
        assert_eq!(users[0].name, "Alice");
        assert_eq!(users[0].age, 30);
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_stream_with_filter(pool: Pool) {
        let app = StreamApp { pool };

        let mut stream = app.stream_users_by_min_age(30);
        let mut users = Vec::new();

        while let Some(user_result) = stream.next().await {
            users.push(user_result.unwrap());
        }

        // All users should have age >= 30
        assert!(!users.is_empty());
        for user in users {
            assert!(user.age >= 30);
        }
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_stream_with_ordering(pool: Pool) {
        let app = StreamApp { pool };

        let mut stream = app.stream_users_by_age_desc();
        let mut users = Vec::new();

        while let Some(user_result) = stream.next().await {
            users.push(user_result.unwrap());
        }

        // Verify descending order by age
        for i in 1..users.len() {
            assert!(users[i-1].age >= users[i].age);
        }

        // First user should be Eve (42, oldest)
        assert_eq!(users[0].name, "Eve");
        assert_eq!(users[0].age, 42);
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_stream_with_limit(pool: Pool) {
        let app = StreamApp { pool };

        let mut stream = app.stream_users_limited(5);
        let mut count = 0;

        while let Some(_user_result) = stream.next().await {
            count += 1;
        }

        assert_eq!(count, 5);
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_stream_tuple_results(pool: Pool) {
        let app = StreamApp { pool };

        let mut stream = app.stream_user_info(25, 35);
        let mut user_infos = Vec::new();

        while let Some(info_result) = stream.next().await {
            let (name, age) = info_result.unwrap();
            user_infos.push(UserInfo { name, age });
        }

        assert!(!user_infos.is_empty());
        for info in user_infos {
            assert!(info.age >= 25 && info.age <= 35);
        }
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_stream_with_multiple_filters(pool: Pool) {
        let app = StreamApp { pool };

        let mut stream = app.stream_users_by_age_and_name(25, "%e%".to_string());
        let mut users = Vec::new();

        while let Some(user_result) = stream.next().await {
            users.push(user_result.unwrap());
        }

        // All users should have age >= 25 AND name containing 'e'
        for user in users {
            assert!(user.age >= 25);
            assert!(user.name.to_lowercase().contains('e'));
        }
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_stream_with_nullable_filter(pool: Pool) {
        let app = StreamApp { pool };

        let mut stream = app.stream_users_with_birth_year();
        let mut users = Vec::new();

        while let Some(user_result) = stream.next().await {
            users.push(user_result.unwrap());
        }

        // All users should have non-null birth_year
        for user in users {
            assert!(user.birth_year.is_some());
        }
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_stream_collect_to_vec(pool: Pool) {
        let app = StreamApp { pool };

        let mut stream = app.stream_users_limited(3);
        let mut users = Vec::new();

        while let Some(user_result) = stream.next().await {
            users.push(user_result.unwrap());
        }

        assert_eq!(users.len(), 3);
        assert_eq!(users[0].name, "Alice");
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_stream_filter_map(pool: Pool) {
        let app = StreamApp { pool };

        let mut stream = app.stream_all_users();
        let mut adult_names = Vec::new();

        while let Some(user_result) = stream.next().await {
            let user = user_result.unwrap();
            if user.age >= 21 {
                adult_names.push(user.name);
            }
        }

        assert!(!adult_names.is_empty());
        assert!(adult_names.contains(&"Alice".to_string()));
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_stream_take_while(pool: Pool) {
        let app = StreamApp { pool };

        let mut stream = app.stream_users_by_age_desc();
        let mut older_users = Vec::new();

        // Take users while age >= 35
        while let Some(user_result) = stream.next().await {
            let user = user_result.unwrap();
            if user.age >= 35 {
                older_users.push(user);
            } else {
                break;
            }
        }

        assert!(!older_users.is_empty());
        for user in &older_users {
            assert!(user.age >= 35);
        }

        // Should start with the oldest users
        if older_users.len() > 1 {
            assert!(older_users[0].age >= older_users[1].age);
        }
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_stream_fold(pool: Pool) {
        let app = StreamApp { pool };

        let mut stream = app.stream_users_limited(5);
        let mut total_age = 0i32;

        while let Some(user_result) = stream.next().await {
            let user = user_result.unwrap();
            total_age += user.age as i32;
        }

        assert!(total_age > 0);
        println!("Total age of first 5 users: {}", total_age);
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_stream_for_each(pool: Pool) {
        let app = StreamApp { pool };

        let mut stream = app.stream_users_limited(3);
        let mut processed_count = 0;
        let mut name_lengths = Vec::new();

        while let Some(user_result) = stream.next().await {
            let user = user_result.unwrap();
            processed_count += 1;
            name_lengths.push(user.name.len());
        }

        assert_eq!(processed_count, 3);
        assert_eq!(name_lengths.len(), 3);
        assert!(name_lengths.iter().all(|&len| len > 0));
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_stream_names_subquery(pool: Pool) {
        let app = StreamApp { pool };

        let mut stream = app.stream_names_by_subquery(25);
        let mut names = Vec::new();

        while let Some(name_result) = stream.next().await {
            names.push(name_result.unwrap());
        }

        // Should get unique names of users whose age appears in users with age > 25
        assert!(!names.is_empty());
        for name in &names {
            assert!(!name.is_empty());
        }

        // Names should be unique due to DISTINCT
        let mut sorted_names = names.clone();
        sorted_names.sort();
        sorted_names.dedup();
        assert_eq!(names.len(), sorted_names.len());
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_stream_error_handling(pool: Pool) {
        let app = StreamApp { pool };

        let mut stream = app.stream_users_limited(1);
        let mut results = Vec::new();

        while let Some(result) = stream.next().await {
            results.push(result);
        }

        assert_eq!(results.len(), 1);
        assert!(results[0].is_ok());
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_empty_stream(pool: Pool) {
        let app = StreamApp { pool };

        // Query that should return no results
        let mut stream = app.stream_users_by_min_age(200); // Age > 200
        let mut count = 0;

        while let Some(_result) = stream.next().await {
            count += 1;
        }

        assert_eq!(count, 0);
    }
}
