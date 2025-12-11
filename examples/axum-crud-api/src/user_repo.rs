use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use sqlx_data::{repo, dml, Result, QueryResult, Pool};

#[derive(Deserialize)]
pub struct UserPayload {
    pub name: String,
    pub email: String,
}

#[derive(Serialize, FromRow)]
pub struct User {
    pub id: i32,
    pub name: String,
    pub email: String,
}

// Repository trait using sqlx-data
#[repo]
pub trait UserRepo {
    // List all users
    #[dml("SELECT * FROM users")]
    async fn list_users(&self) -> Result<Vec<User>>;

    // Create a new user and return it
    #[dml("INSERT INTO users (name, email) VALUES ($1, $2) RETURNING *")]
    async fn create_user(&self, name: String, email: String) -> Result<User>;

    // Get user by ID
    #[dml("SELECT * FROM users WHERE id = $1")]
    async fn get_user(&self, id: i32) -> Result<Option<User>>;

    // Update user and return the updated record
    #[dml("UPDATE users SET name = $1, email = $2 WHERE id = $3 RETURNING *")]
    async fn update_user(&self, name: String, email: String, id: i32) -> Result<Option<User>>;

    // Delete user by ID
    #[dml("DELETE FROM users WHERE id = $1")]
    async fn delete_user(&self, id: i32) -> Result<QueryResult>;

    // Count total users (useful for health checks)
    #[dml("SELECT COUNT(*) FROM users")]
    async fn count_users(&self) -> Result<Option<i64>>;
}

// Repository implementation struct
#[derive(Clone)]
pub struct UserRepoImpl {
    pub pool: Pool,
}

impl UserRepo for UserRepoImpl {
    fn get_pool(&self) -> &Pool {
        &self.pool
    }
}