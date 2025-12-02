use sqlx_data::{Pool, Result, dml};

#[derive(Clone, PartialEq, Eq, Debug, sqlx::Type)]
#[sqlx(transparent)]
pub struct Id(i64);

impl From<i64> for Id {
    fn from(value: i64) -> Self {
        Id(value)
    }
}

// User model for tests
#[derive(Debug, Clone, PartialEq)]
pub struct User {
    pub id: Id,
    pub name: String,
    pub email: String,
    pub age: i64,
}

// Test trait for file-based SQL queries
#[sqlx_data::repo]
trait UserFileRepo {
    #[dml(file = "tests/fixtures/find_user_by_id.sql")]
    async fn find_by_id_from_file(&self, id: i64) -> Result<User>;

    #[dml(file = "tests/fixtures/count_users_by_birth_year.sql")]
    async fn count_by_birth_year_from_file(&self, birth_year: i64) -> Result<i64>;

    #[dml(file = "tests/fixtures/get_user_info_tuple.sql")]
    async fn get_user_info_from_file(&self, id: i64) -> Result<(String, i64)>;
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
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_count_by_birth_year_from_file(pool: Pool) {
        let repo = MyFileApp { pool };

        let count = repo.count_by_birth_year_from_file(1993).await.unwrap();
        assert_eq!(count, 2); // Alice and Noah have birth year 1993
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
}
