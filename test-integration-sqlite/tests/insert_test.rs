use sqlx_data::{Pool, QueryResult, Result, dml};

// Use same structure as integration_tests
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

// User model for tests (same as integration_tests)
#[derive(Debug, Clone, PartialEq)]
pub struct User {
    pub id: Id,
    pub name: String,
    pub email: String,
    pub age: i64,
    pub birth_year: Option<i64>,
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

// Test trait with alias definitions using the same table structure
#[rustfmt::skip]
#[sqlx_data::repo]
#[alias(columns = "id, name, email, age, birth_year")] // DRY columns alias
#[alias(columns_cast = "id as 'id!: Id', name, email, age as 'age: u8', birth_year as 'birth_year: u16'")] // DRY columns alias
#[alias(values = "(?, ?, ?, ?, ?)")] // DRY values alias
trait UserCreateRepo {

    #[dml("INSERT INTO users (id, name, email, age, birth_year) VALUES (?, ?, ?, ?, ?)")]
    async fn create_user_boring(&self, id: i64, name: String, email: String, age: u8, birth_year: Option<u16>) -> Result<QueryResult>;

    async fn create_user_not_boring(&self, user: User) -> Result<QueryResult> {
        self.create_user_boring(
            user.id.0,
            user.name,
            user.email,
            user.age as u8,
            user.birth_year.map(|by| by as u16),
        ).await
    }

    // Basic alias substitution - using same format as integration_tests
    #[dml("INSERT INTO users ({{columns}}) VALUES {{values}} RETURNING id")]
    async fn create_user_retuning_id(&self, id: i64, name: String, email: String, age: u8, birth_year: Option<u16>) -> Result<u64>;

    #[dml("INSERT INTO users ({{columns}}) VALUES {{values}} RETURNING *")]
    async fn create_user_retuning_all(&self, id: i64, name: String, email: String, age: u8, birth_year: Option<u16>) -> Result<User>;

    #[dml("INSERT INTO users ({{columns}}) VALUES {{values}} RETURNING {{columns_cast}}")]
    async fn create_user_retuning_all_cast(&self, id: i64, name: String, email: String, age: u8, birth_year: Option<u16>) -> Result<UserCast>;

    #[dml("INSERT INTO users ({{columns}}) VALUES {{values}} ON CONFLICT(id) DO UPDATE SET name = excluded.name, email = excluded.email RETURNING id")]
    async fn upsert_user_returning_id(&self, id: i64, name: String, email: String, age: u8, birth_year: Option<u16>) -> Result<u64>;

    #[dml("INSERT INTO users ({{columns}}) VALUES {{values}} ON CONFLICT(id) DO UPDATE SET name = excluded.name, email = excluded.email, age = excluded.age, birth_year = excluded.birth_year RETURNING *")]
    async fn upsert_user_returning_all(&self, id: i64, name: String, email: String, age: u8, birth_year: Option<u16>) -> Result<User>;

    #[dml("INSERT INTO users (name, age) SELECT $1, $2 WHERE NOT EXISTS (SELECT 1 FROM users WHERE name = $1)")]
    async fn insert_if_not_exists_basic(&self, name: String, age: u8) -> Result<QueryResult>;

    #[dml("INSERT INTO users (name, age) SELECT $1, $2 WHERE NOT EXISTS (SELECT 1 FROM users WHERE name = $1) RETURNING id")]
    async fn insert_if_not_exists_returning_id(&self, name: String, age: u8) -> Result<u64>;

    #[dml("INSERT INTO users (name, age) SELECT $1, $2 WHERE NOT EXISTS (SELECT 1 FROM users WHERE name = $1) RETURNING id as \"id: Option<u64>\"")]
    async fn insert_if_not_exists_returning_option_id(&self, name: String, age: u8) -> Result<Option<u64>>;
    
    #[dml("INSERT INTO users (id, name, email, age, birth_year) SELECT $1, $2, $3, $4, $5 WHERE NOT EXISTS (SELECT 1 FROM users WHERE name = $2) RETURNING *")]
    async fn insert_if_not_exists_returning_all(&self, id: i64, name: String, email: String, age: u8, birth_year: Option<u16>) -> Result<Option<User>>;
    
    #[dml("INSERT INTO users ({{columns}}) VALUES {{values}}")]
    async fn create_user_simple(&self, id: i64, name: String, email: String, age: u8, birth_year: Option<u16>) -> Result<QueryResult>;
    
    #[dml("INSERT INTO users (name, email, age) SELECT $1, $2, $3 WHERE NOT EXISTS (SELECT 1 FROM users WHERE name = $1) RETURNING id as \"id: Option<u64>\"")]
    async fn insert_if_not_exists_returning_id_optional(&self, name: &str, email: String, age: u8) -> Result<Option<u64>>;

    #[dml("INSERT OR IGNORE INTO users ({{columns}}) VALUES {{values}}")]
    async fn insert_or_ignore_basic(&self, id: i64, name: String, email: String, age: u8, birth_year: Option<u16>) -> Result<QueryResult>;

    #[dml("INSERT OR IGNORE INTO users ({{columns}}) VALUES {{values}} RETURNING id")]
    async fn insert_or_ignore_returning_one_id(&self, id: i64, name: String, email: String, age: u8, birth_year: Option<u16>) -> Result<Option<u64>>; // Without Option, it errors if nothing inserted
    
    #[dml("INSERT OR IGNORE INTO users ({{columns}}) VALUES {{values}} RETURNING id")]
    async fn insert_or_ignore_returning_many_id(&self, id: i64, name: String, email: String, age: u8, birth_year: Option<u16>) -> Result<Vec<Option<u64>>>; // Without Option, it errors if nothing inserted
    
    #[dml("INSERT OR IGNORE INTO users ({{columns}}) VALUES {{values}} RETURNING id")]
    async fn insert_or_ignore_returning_all(&self, id: i64, name: String, email: String, age: u8, birth_year: Option<u16>) -> Result<Vec<Option<u64>>>; // Without Option, it errors if nothing inserted

    #[dml("INSERT OR IGNORE INTO users ({{columns}}) VALUES {{values}} RETURNING {{columns_cast}}")]
    async fn insert_or_ignore_returning_one_cast(&self, id: i64, name: String, email: String, age: u8, birth_year: Option<u16>) -> Result<Vec<UserCast>>;
    
    #[dml("INSERT OR IGNORE INTO users ({{columns}}) VALUES {{values}} RETURNING {{columns}}")]
    async fn insert_or_ignore_returning_one(&self, id: i64, name: String, email: String, age: u8, birth_year: Option<u16>) -> Result<Vec<User>>;
    
    #[dml("INSERT OR IGNORE INTO users (id, name) VALUES (1, 'Alice'), (2, 'Bob'), (3, 'Carol') RETURNING id, name")]
    async fn insert_or_ignore_returning_struct(&self) -> Result<Vec<UserSelect>>;
    
    #[dml("INSERT OR IGNORE INTO users ({{columns}}) VALUES {{values}} RETURNING id as \"id!: i64\", name")]
    async fn insert_or_ignore_returning_tuple(&self, id: i64, name: String, email: String, age: u8, birth_year: Option<u16>) -> Result<(i64, String)>;
    
    #[dml("INSERT OR IGNORE INTO users ({{columns}}) VALUES {{values}} RETURNING id,name")]
    async fn insert_or_ignore_returning_tuple_option(&self, id: i64, name: &str, email: String, age: u8, birth_year: Option<u16>) -> Result<(Option<i64>, String)>;
    
    #[dml("INSERT INTO users (id, name, email, age, birth_year) VALUES (1, 'Alice', 'alice@example.com', 30, 1993)",unchecked)]
    async fn insert_no_return_unchecked(&self) -> Result<()>;

    #[dml("INSERT INTO users (id, name, email, age, birth_year) VALUES (?, ?, ?, ?, ?)",unchecked)]
    async fn create_user_boring_erro(&self, id: i64, name: String, email: String, age: u8, birth_year: Option<u16>) -> Result<()>;


}

// Test implementation
pub struct TestUserCreateApp {
    pool: Pool,
}

impl UserCreateRepo for TestUserCreateApp {
    fn get_pool(&self) -> &Pool {
        &self.pool
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_create_user_simple(pool: Pool) {
        let repo = TestUserCreateApp { pool };

        let result = repo
            .insert_or_ignore_returning_tuple_option(
                1,
                "Alice",
                "alice@example.com".to_string(),
                30,
                Some(1993),
            )
            .await
            .unwrap();
        assert_eq!(result, (Some(1), "Alice".to_string()));
    }

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_create_user_returning_id(pool: Pool) {
        let repo = TestUserCreateApp { pool };

        let id = repo
            .create_user_retuning_id(
                2,
                "Bob".to_string(),
                "bob@example.com".to_string(),
                25,
                Some(1998),
            )
            .await
            .unwrap();
        assert_eq!(id, 2);
    }

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_create_user_returning_all(pool: Pool) {
        let repo = TestUserCreateApp { pool };

        let user = repo
            .create_user_retuning_all(
                3,
                "Carol".to_string(),
                "carol@example.com".to_string(),
                35,
                Some(1988),
            )
            .await
            .unwrap();
        assert_eq!(user.id, Id(3));
        assert_eq!(user.name, "Carol");
        assert_eq!(user.email, "carol@example.com");
        assert_eq!(user.age, 35);
        assert_eq!(user.birth_year, Some(1988));
    }

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_upsert_user_new_record(pool: Pool) {
        let repo = TestUserCreateApp { pool };

        // Insert new record
        let id = repo
            .upsert_user_returning_id(
                5,
                "Eve".to_string(),
                "eve@example.com".to_string(),
                28,
                Some(1995),
            )
            .await
            .unwrap();
        assert_eq!(id, 5);
    }

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_upsert_user_conflict_update(pool: Pool) {
        let repo = TestUserCreateApp { pool };

        // First insert
        let _user1 = repo
            .create_user_retuning_all(
                6,
                "Frank".to_string(),
                "frank@example.com".to_string(),
                30,
                Some(1993),
            )
            .await
            .unwrap();

        // Upsert with same ID - should update
        let updated_user = repo
            .upsert_user_returning_all(
                6,
                "Franklin".to_string(),             // Updated name
                "franklin@example.com".to_string(), // Updated email
                31,                                 // Updated age
                Some(1992),                         // Updated birth year
            )
            .await
            .unwrap();

        assert_eq!(updated_user.id, Id(6));
        assert_eq!(updated_user.name, "Franklin"); // Updated
        assert_eq!(updated_user.email, "franklin@example.com"); // Updated
        assert_eq!(updated_user.age, 31); // Updated
        assert_eq!(updated_user.birth_year, Some(1992)); // Updated
    }

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_insert_if_not_exists_basic_new_user(pool: Pool) {
        let repo = TestUserCreateApp { pool };

        // This will fail because email is NOT NULL but not provided in the INSERT
        let result = repo
            .insert_if_not_exists_basic("Grace".to_string(), 26)
            .await;

        // Should return an error because email column is missing
        assert!(result.is_err());
        let error = result.unwrap_err().to_string();
        assert!(error.contains("NOT NULL"));
    }

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_insert_if_not_exists_returning_id_with_cast(pool: Pool) {
        let repo = TestUserCreateApp { pool };

        // This will fail because email is NOT NULL but not provided in the INSERT
        let result = repo
            .insert_if_not_exists_returning_id("Henry".to_string(), 29)
            .await;

        // Should return an error because email column is missing
        assert!(result.is_err());
        let error = result.unwrap_err().to_string();
        assert!(error.contains("NOT NULL constraint failed: users.email"));
    }

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_insert_if_not_exists_returning_all(pool: Pool) {
        let repo = TestUserCreateApp { pool };

        // Insert new user
        let user = repo
            .insert_if_not_exists_returning_all(
                7,
                "Isabella".to_string(),
                "isabella@example.com".to_string(),
                27,
                Some(1996),
            )
            .await
            .unwrap();

        assert!(user.is_some());
        let user = user.unwrap();
        assert_eq!(user.name, "Isabella");
        assert_eq!(user.age, 27);

        // Try to insert again with same name - should return None
        let user2 = repo
            .insert_if_not_exists_returning_all(
                8,                      // Different ID
                "Isabella".to_string(), // Same name
                "isabella2@example.com".to_string(),
                30,
                Some(1993),
            )
            .await
            .unwrap();

        assert!(user2.is_none());
    }

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_insert_if_not_exists_returning_optional(pool: Pool) {
        let repo = TestUserCreateApp { pool };

        // Insert new user (birth_year will be NULL since not provided)
        let id = repo
            .insert_if_not_exists_returning_id_optional(&"Jack", "jack@example.com".to_string(), 32)
            .await
            .unwrap();

        assert!(id.is_some());
        // The id will be auto-generated

        // Try to insert again with same name - should return None
        let id2 = repo
            .insert_if_not_exists_returning_id_optional(
                &"Jack",
                "jack2@example.com".to_string(),
                35,
            )
            .await
            .unwrap();

        assert!(id2.is_none());
    }

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_insert_or_ignore_basic(pool: Pool) {
        let repo = TestUserCreateApp { pool };

        // First insert should succeed
        let result1 = repo
            .insert_or_ignore_basic(
                10,
                "Mike".to_string(),
                "mike@example.com".to_string(),
                40,
                Some(1983),
            )
            .await
            .unwrap();

        assert_eq!(result1.rows_affected(), 1);

        // Second insert with same ID should be ignored
        let result2 = repo
            .insert_or_ignore_basic(
                10,                                // Same ID
                "Michael".to_string(),             // Different name
                "michael@example.com".to_string(), // Different email
                45,                                // Different age
                Some(1978),                        // Different birth year
            )
            .await
            .unwrap();

        assert_eq!(result2.rows_affected(), 0); // Should be ignored
    }

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_insert_or_ignore_returning_id(pool: Pool) {
        let repo = TestUserCreateApp { pool };

        // First insert should return the ID
        let ids1 = repo
            .insert_or_ignore_returning_all(
                11,
                "Sarah".to_string(),
                "sarah@example.com".to_string(),
                33,
                Some(1990),
            )
            .await
            .unwrap();

        assert_eq!(ids1.len(), 1);
        assert_eq!(ids1[0], Some(11));

        // Second insert with same ID should return empty vector
        let ids2 = repo
            .insert_or_ignore_returning_all(
                11, // Same ID
                "Sarah Jane".to_string(),
                "sarahjane@example.com".to_string(),
                34,
                Some(1989),
            )
            .await
            .unwrap();

        assert_eq!(ids2.len(), 0); // Should be empty - nothing inserted
    }

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_insert_or_ignore_returning_all(pool: Pool) {
        let repo = TestUserCreateApp { pool };

        // First insert should return the user
        let users1 = repo
            .insert_or_ignore_returning_one_cast(
                12,
                "Tom".to_string(),
                "tom@example.com".to_string(),
                28,
                Some(1995),
            )
            .await
            .unwrap();

        assert_eq!(users1.len(), 1);
        assert_eq!(users1[0].name, "Tom");
        assert_eq!(users1[0].age, 28);

        // Second insert with same ID should return empty vector
        let users2 = repo
            .insert_or_ignore_returning_one_cast(
                12, // Same ID
                "Thomas".to_string(),
                "thomas@example.com".to_string(),
                30,
                Some(1993),
            )
            .await
            .unwrap();

        assert_eq!(users2.len(), 0); // Should be empty - nothing inserted
    }
}
