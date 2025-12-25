# Chapter 4: Your First Repository

Now that you understand SQLx-Data's core concepts, let's build a complete, real-world example. We'll create a User Management API using Axum that demonstrates all the CRUD operations with proper error handling.

## Project Setup

Create a new Rust project and add the dependencies:

```toml
[dependencies]
sqlx-data = { version = "0.1.0", features = ["postgres"] }
sqlx = { version = "0.8", features = ["postgres", "runtime-tokio", "migrate"] }
axum = "0.7"
tokio = { version = "1", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
```

## Define Your Data Models

First, let's define our User struct and payload for API requests:

```rust
// src/user_repo.rs
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
```

## Create Your Repository

Now let's define a complete repository with all CRUD operations:

```rust
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
```

## Implement the Repository

The implementation is simple - just provide the database pool:

```rust
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
```

## Build the API

Create the main application with Axum:

```rust
// src/main.rs
use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use sqlx::postgres::PgPoolOptions;
use std::env;

mod user_repo;
use user_repo::{User, UserPayload, UserRepo, UserRepoImpl};

#[tokio::main]
async fn main() {
    let db_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = PgPoolOptions::new().connect(&db_url).await.expect("Failed to connect to DB");
    sqlx::migrate!().run(&pool).await.expect("Migrations failed");
    let repo = UserRepoImpl { pool };

    let app = Router::new()
        .route("/", get(root))
        .route("/users", post(create_user).get(list_users))
        .route("/users/{id}", get(get_user).put(update_user).delete(delete_user))
        .with_state(repo);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8000").await.unwrap();
    println!("🚀 Server running on port 8000");
    axum::serve(listener, app).await.unwrap();
}
```

## Endpoint Handlers

Implement the API handlers using our repository:

```rust
// Test endpoint
async fn root() -> &'static str {
    "Welcome to the User Management API!"
}

// GET /users - List all users
async fn list_users(State(repo): State<UserRepoImpl>) -> Result<Json<Vec<User>>, StatusCode> {
    repo.list_users().await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

// POST /users - Create new user
async fn create_user(
    State(repo): State<UserRepoImpl>,
    Json(payload): Json<UserPayload>
) -> Result<(StatusCode, Json<User>), StatusCode> {
    repo.create_user(payload.name, payload.email).await
        .map(|u| (StatusCode::CREATED, Json(u)))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

// GET /users/{id} - Get user by ID
async fn get_user(
    State(repo): State<UserRepoImpl>,
    Path(id): Path<i32>
) -> Result<Json<User>, StatusCode> {
    repo.get_user(id).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

// PUT /users/{id} - Update user
async fn update_user(
    State(repo): State<UserRepoImpl>,
    Path(id): Path<i32>,
    Json(payload): Json<UserPayload>
) -> Result<Json<User>, StatusCode> {
    repo.update_user(payload.name, payload.email, id).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

// DELETE /users/{id} - Delete user
async fn delete_user(
    State(repo): State<UserRepoImpl>,
    Path(id): Path<i32>
) -> Result<StatusCode, StatusCode> {
    let result = repo.delete_user(id).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if result.rows_affected() == 0 {
        Err(StatusCode::NOT_FOUND)
    } else {
        Ok(StatusCode::NO_CONTENT)
    }
}
```

## Database Setup

You'll need a PostgreSQL database and migrations. Create a migration file:

```sql
-- migrations/001_create_users.sql
CREATE TABLE users (
    id SERIAL PRIMARY KEY,
    name VARCHAR NOT NULL,
    email VARCHAR NOT NULL UNIQUE
);
```

## Running the Application

Set your database URL and run:

```bash
export DATABASE_URL="postgresql://username:password@localhost/dbname"
cargo run
```

## Testing Your API

Test the endpoints:

```bash
# Create a user
curl -X POST http://localhost:8000/users \
  -H "Content-Type: application/json" \
  -d '{"name": "Alice", "email": "alice@example.com"}'

# List all users
curl http://localhost:8000/users

# Get user by ID
curl http://localhost:8000/users/1

# Update user
curl -X PUT http://localhost:8000/users/1 \
  -H "Content-Type: application/json" \
  -d '{"name": "Alice Updated", "email": "alice.updated@example.com"}'

# Delete user
curl -X DELETE http://localhost:8000/users/1
```

## What SQLx-Data Generated

For each `#[dml]` method, SQLx-Data generates the actual implementation. For example:

```rust
// From #[dml("SELECT * FROM users")]
async fn list_users_query(&self) -> Result<Vec<User>> {
    sqlx::query_as!(User, "SELECT * FROM users")
        .fetch_all(self.get_pool())
        .await
}
```

## Key Benefits Demonstrated

1. **Zero Boilerplate**: No manual SQLx code, just trait definitions
2. **Type Safety**: Compile-time validation of SQL queries and types
3. **Different Return Types**: `Vec<T>`, `Option<T>`, `Result<T>`, and `QueryResult`
4. **Parameter Binding**: Automatic parameter binding by position
5. **Clean Architecture**: Business logic depends on traits, not implementations

## Next Steps

This foundation gives you a complete, production-ready repository pattern. In the next chapter, we'll explore more sophisticated features like:

- Named parameter binding
- Complex return types and tuples
- Error handling strategies
- Custom method implementations

Your repository is ready to scale!