use sqlx_data::{Pool, QueryResult, Result, dml};

// Use same structure as integration_tests but adapted for MySQL
#[derive(Clone, PartialEq, Eq, Debug, sqlx::Type)]
#[sqlx(transparent)]
pub struct Id(i64);

impl From<i64> for Id {
    fn from(value: i64) -> Self {
        Id(value)
    }
}

impl From<Option<i64>> for Id {
    fn from(value: Option<i64>) -> Self {
        Id(value.unwrap_or_default())
    }
}

// User model for MySQL tests with proper types
#[derive(Debug, Clone, PartialEq)]
pub struct User {
    pub id: Id,
    pub name: String,
    pub email: String,
    pub age: u8,  // MySQL TINYINT UNSIGNED
    pub birth_year: Option<u16>,  // MySQL SMALLINT UNSIGNED
}

#[derive(Debug, Clone, PartialEq)]
pub struct UserCast {
    pub id: Id,
    pub name: String,
    pub email: String,
    pub age: u8,
    pub birth_year: Option<u16>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, sqlx::FromRow)]
pub struct UserSelect {
    pub id: Id,
    pub name: String,
}

// Test trait with alias definitions using MySQL syntax and strong types
#[rustfmt::skip]
#[sqlx_data::repo]
#[alias(columns = "id, name, email, age, birth_year")] // DRY columns alias
#[alias(columns_cast = "id as 'id!: Id', name, email, age as 'age: u8', birth_year as 'birth_year: u16'")] // DRY columns alias
#[alias(values = "(?, ?, ?, ?, ?)")] // DRY values alias for MySQL
trait UserCreateRepo {

    #[dml("INSERT INTO users (name, email, age, birth_year) VALUES (?, ?, ?, ?)")]
    async fn create_user_basic(&self, name: String, email: String, age: u8, birth_year: Option<u16>) -> Result<QueryResult>;

    // MySQL AUTO_INCREMENT insert - don't specify id
    #[dml("INSERT INTO users (name, email, age, birth_year) VALUES (?, ?, ?, ?)")]
    async fn create_user_auto_increment(&self, name: String, email: String, age: u8, birth_year: Option<u16>) -> Result<QueryResult>;

    async fn create_user_not_boring(&self, user: User) -> Result<QueryResult> {
        // For MySQL AUTO_INCREMENT, we don't pass the id
        self.create_user_basic(
            user.name,
            user.email,
            user.age,
            user.birth_year,
        ).await
    }

    // Test with alias
    #[dml("INSERT INTO users VALUES {{values}}")]
    async fn create_user_with_alias(&self, id: i64, name: String, email: String, age: u8, birth_year: Option<u16>) -> Result<QueryResult>;

    // Batch insert with MySQL syntax
    #[dml("INSERT INTO users (name, email, age, birth_year) VALUES (?, ?, ?, ?), (?, ?, ?, ?)")]
    async fn create_two_users(&self,
        name1: String, email1: String, age1: u8, birth_year1: Option<u16>,
        name2: String, email2: String, age2: u8, birth_year2: Option<u16>
    ) -> Result<QueryResult>;

    // Select after insert for testing
    #[dml("SELECT {{columns_cast}} FROM users WHERE email = ?")]
    async fn find_by_email(&self, email: String) -> Result<UserCast>;

    #[dml("SELECT {{columns_cast}} FROM users ORDER BY id DESC LIMIT 1")]
    async fn find_last_created(&self) -> Result<UserCast>;

    // MySQL-specific: Get last insert ID
    #[dml("SELECT LAST_INSERT_ID() as 'last_id!: u64'")]
    async fn get_last_insert_id(&self) -> Result<u64>;

    // Test insert with ON DUPLICATE KEY UPDATE (MySQL-specific)
    #[dml("INSERT INTO users (id, name, email, age, birth_year) VALUES (?, ?, ?, ?, ?) ON DUPLICATE KEY UPDATE age = VALUES(age)")]
    async fn upsert_user(&self, id: i64, name: String, email: String, age: u8, birth_year: Option<u16>) -> Result<QueryResult>;

    // Test insert with IGNORE (MySQL-specific)
    #[dml("INSERT IGNORE INTO users (name, email, age, birth_year) VALUES (?, ?, ?, ?)")]
    async fn create_user_ignore(&self, name: String, email: String, age: u8, birth_year: Option<u16>) -> Result<QueryResult>;

    // MySQL bulk insert with VALUES
    #[dml("INSERT INTO users (name, email, age, birth_year) VALUES (?, ?, ?, ?), (?, ?, ?, ?), (?, ?, ?, ?)")]
    async fn create_three_users_bulk(&self,
        name1: String, email1: String, age1: u8, birth_year1: Option<u16>,
        name2: String, email2: String, age2: u8, birth_year2: Option<u16>,
        name3: String, email3: String, age3: u8, birth_year3: Option<u16>
    ) -> Result<QueryResult>;

    // Test with MySQL unsigned types
    #[dml("INSERT INTO users (name, email, age, birth_year) VALUES (?, ?, ?, ?)")]
    async fn create_user_unsigned(&self, name: String, email: String, age: u8, birth_year: Option<u16>) -> Result<QueryResult>;

    // Count for verification
    #[dml("SELECT COUNT(*) FROM users")]
    async fn count_users(&self) -> Result<i64>;
}

// Test implementation
pub struct TestApp {
    pool: Pool,
}

impl UserCreateRepo for TestApp {
    fn get_pool(&self) -> &Pool {
        &self.pool
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_create_user_basic(pool: Pool) {
        let app = TestApp { pool };

        let result = app.create_user_basic(
            "John".to_string(),
            "john@example.com".to_string(),
            25,
            Some(1998)
        ).await.unwrap();

        // MySQL should return affected rows
        assert_eq!(result.rows_affected(), 1);
    }

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_create_user_auto_increment(pool: Pool) {
        let app = TestApp { pool };

        let result = app.create_user_auto_increment(
            "Jane".to_string(),
            "jane@example.com".to_string(),
            30,
            Some(1993)
        ).await.unwrap();

        // MySQL AUTO_INCREMENT should work
        assert_eq!(result.rows_affected(), 1);

        // Verify the user was created
        let user = app.find_by_email("jane@example.com".to_string()).await.unwrap();
        assert_eq!(user.name, "Jane");
        assert_eq!(user.age, 30);
        assert_eq!(user.birth_year, Some(1993));
    }

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_create_user_not_boring(pool: Pool) {
        let app = TestApp { pool };

        let user = User {
            id: Id(0), // Will be ignored due to AUTO_INCREMENT
            name: "Alice".to_string(),
            email: "alice@example.com".to_string(),
            age: 25,
            birth_year: Some(1998),
        };

        let result = app.create_user_not_boring(user).await.unwrap();
        assert_eq!(result.rows_affected(), 1);

        // Verify the user was created
        let created_user = app.find_by_email("alice@example.com".to_string()).await.unwrap();
        assert_eq!(created_user.name, "Alice");
        assert_eq!(created_user.age, 25);
    }

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_create_two_users(pool: Pool) {
        let app = TestApp { pool };

        let result = app.create_two_users(
            "User1".to_string(), "user1@example.com".to_string(), 20, Some(2003),
            "User2".to_string(), "user2@example.com".to_string(), 25, Some(1998)
        ).await.unwrap();

        // Should insert 2 rows
        assert_eq!(result.rows_affected(), 2);

        let count = app.count_users().await.unwrap();
        assert_eq!(count, 2);
    }

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_create_three_users_bulk(pool: Pool) {
        let app = TestApp { pool };

        let result = app.create_three_users_bulk(
            "BulkUser1".to_string(), "bulk1@example.com".to_string(), 22, Some(2001),
            "BulkUser2".to_string(), "bulk2@example.com".to_string(), 27, Some(1996),
            "BulkUser3".to_string(), "bulk3@example.com".to_string(), 32, None
        ).await.unwrap();

        // Should insert 3 rows
        assert_eq!(result.rows_affected(), 3);

        let count = app.count_users().await.unwrap();
        assert_eq!(count, 3);
    }

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_upsert_user(pool: Pool) {
        let app = TestApp { pool };

        // First insert
        let result1 = app.upsert_user(
            100,
            "UpsertUser".to_string(),
            "upsert@example.com".to_string(),
            25,
            Some(1998)
        ).await.unwrap();
        assert_eq!(result1.rows_affected(), 1);

        // Update same id with different age
        let result2 = app.upsert_user(
            100,
            "UpsertUser".to_string(),
            "upsert@example.com".to_string(),
            30, // Changed age
            Some(1998)
        ).await.unwrap();
        assert_eq!(result2.rows_affected(), 2); // MySQL reports 2 for updates in ON DUPLICATE KEY

        // Verify age was updated
        let user = app.find_by_email("upsert@example.com".to_string()).await.unwrap();
        assert_eq!(user.age, 30);
    }

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_create_user_ignore(pool: Pool) {
        let app = TestApp { pool };

        // First insert should succeed
        let result1 = app.create_user_ignore(
            "IgnoreUser".to_string(),
            "ignore@example.com".to_string(),
            25,
            Some(1998)
        ).await.unwrap();
        assert_eq!(result1.rows_affected(), 1);

        // Second insert with same email should be ignored (due to UNIQUE constraint)
        let result2 = app.create_user_ignore(
            "IgnoreUser2".to_string(),
            "ignore@example.com".to_string(),
            30,
            Some(1993)
        ).await.unwrap();
        assert_eq!(result2.rows_affected(), 0); // Should be ignored

        let count = app.count_users().await.unwrap();
        assert_eq!(count, 1); // Only one user should exist
    }

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_create_user_unsigned(pool: Pool) {
        let app = TestApp { pool };

        // Test with unsigned types at boundaries
        let result = app.create_user_unsigned(
            "UnsignedUser".to_string(),
            "unsigned@example.com".to_string(),
            255,  // Max TINYINT UNSIGNED
            Some(65535)  // Max SMALLINT UNSIGNED
        ).await.unwrap();

        assert_eq!(result.rows_affected(), 1);

        let user = app.find_by_email("unsigned@example.com".to_string()).await.unwrap();
        assert_eq!(user.age, 255);
        assert_eq!(user.birth_year, Some(65535));
    }

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_find_last_created(pool: Pool) {
        let app = TestApp { pool };

        // Create a user
        app.create_user_basic(
            "LastUser".to_string(),
            "last@example.com".to_string(),
            35,
            Some(1988)
        ).await.unwrap();

        // Find the last created user
        let user = app.find_last_created().await.unwrap();
        assert_eq!(user.name, "LastUser");
        assert_eq!(user.email, "last@example.com");
        assert_eq!(user.age, 35);
    }
}