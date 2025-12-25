# Chapter 2: Quick Start

Let's get you up and running with SQLx-Data in under 10 minutes. By the end of this chapter, you'll have a working repository that performs CRUD operations with full type safety.

## Prerequisites

Before we begin, ensure you have:
- Rust 1.75+ installed
- A SQLite database (we'll create one as we go)
- Your favorite text editor or IDE

## Project Setup

Create a new Rust project:

```bash
cargo new sqlx-data-quickstart
cd sqlx-data-quickstart
```

Add SQLx-Data to your `Cargo.toml`:

```toml
[dependencies]
sqlx-data = { version = "0.1.0", features = ["sqlite"] }
sqlx = { version = "0.8", features = ["sqlite", "runtime-tokio"] }
tokio = { version = "1", features = ["full"] }
```

## Define Your Data Model

First, let's define a simple `User` struct:

```rust
// src/main.rs
use sqlx_data::{repo, dml, Pool, Result, QueryResult};

#[derive(sqlx::FromRow, Debug)]
struct User {
    id: i32,
    name: String,
    email: String,
    age: u8,
}
```

## Create Your First Repository

Now, let's define a repository trait using SQLx-Data:

```rust
#[repo]
trait UserRepo {
    #[dml("SELECT * FROM users WHERE id = ?")]
    async fn find_by_id(&self, id: i32) -> Result<Option<User>>;

    #[dml("SELECT * FROM users")]
    async fn find_all(&self) -> Result<Vec<User>>;

    #[dml("INSERT INTO users (name, email, age) VALUES (?, ?, ?)")]
    async fn create(&self, name: String, email: String, age: u8) -> Result<QueryResult>;

    #[dml("UPDATE users SET name = ?, email = ?, age = ? WHERE id = ?")]
    async fn update(&self, name: String, email: String, age: u8, id: i32) -> Result<QueryResult>;

    #[dml("DELETE FROM users WHERE id = ?")]
    async fn delete(&self, id: i32) -> Result<QueryResult>;
}
```

## Implement the Repository

With SQLx-Data, implementation is minimal – you just need to provide the database pool:

```rust
struct App {
    pool: Pool,
}

impl UserRepo for App {
    fn get_pool(&self) -> &Pool {
        &self.pool
    }
}
```

That's it! SQLx-Data generates all the method implementations automatically.

## Set Up the Database

Let's create a simple main function that sets up our database and tests our repository:

```rust
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create an in-memory SQLite database
    let pool = Pool::connect("sqlite::memory:").await?;

    // Create the users table
    sqlx::query(
        r#"
        CREATE TABLE users (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            email TEXT NOT NULL UNIQUE,
            age INTEGER NOT NULL
        )
        "#
    )
    .execute(&pool)
    .await?;

    // Create our repository
    let app = App { pool };

    // Test the repository
    demo_repository(&app).await?;

    Ok(())
}
```

## Test the Repository

Add a demo function to test all CRUD operations:

```rust
async fn demo_repository(repo: &App) -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 SQLx-Data Quick Start Demo");
    println!("============================");

    // CREATE: Insert some users
    println!("\n📝 Creating users...");
    repo.create("Alice Johnson".to_string(), "alice@example.com".to_string(), 30).await?;
    repo.create("Bob Smith".to_string(), "bob@example.com".to_string(), 25).await?;
    repo.create("Carol Davis".to_string(), "carol@example.com".to_string(), 35).await?;

    // READ: Find all users
    println!("\n👥 All users:");
    let users = repo.find_all().await?;
    for user in &users {
        println!("  • {} ({}) - {} years old", user.name, user.email, user.age);
    }

    // READ: Find specific user
    println!("\n🔍 Finding user with ID 1:");
    if let Some(user) = repo.find_by_id(1).await? {
        println!("  Found: {} ({})", user.name, user.email);
    }

    // UPDATE: Update a user
    println!("\n✏️  Updating Alice's age...");
    repo.update("Alice Johnson".to_string(), "alice@example.com".to_string(), 31, 1).await?;

    // Verify the update
    if let Some(user) = repo.find_by_id(1).await? {
        println!("  Alice is now {} years old", user.age);
    }

    // DELETE: Remove a user
    println!("\n🗑️  Deleting user with ID 2...");
    repo.delete(2).await?;

    // Final count
    let final_users = repo.find_all().await?;
    println!("\n📊 Final user count: {}", final_users.len());

    println!("\n✅ Demo completed successfully!");
    Ok(())
}
```

## Complete Example

Here's the complete `src/main.rs` file:

```rust
use sqlx_data::{repo, dml, Pool, Result, QueryResult};

#[derive(sqlx::FromRow, Debug)]
struct User {
    id: i32,
    name: String,
    email: String,
    age: u8,
}

#[repo]
trait UserRepo {
    #[dml("SELECT * FROM users WHERE id = ?")]
    async fn find_by_id(&self, id: i32) -> Result<Option<User>>;

    #[dml("SELECT * FROM users")]
    async fn find_all(&self) -> Result<Vec<User>>;

    #[dml("INSERT INTO users (name, email, age) VALUES (?, ?, ?)")]
    async fn create(&self, name: String, email: String, age: u8) -> Result<QueryResult>;

    #[dml("UPDATE users SET name = ?, email = ?, age = ? WHERE id = ?")]
    async fn update(&self, name: String, email: String, age: u8, id: i32) -> Result<QueryResult>;

    #[dml("DELETE FROM users WHERE id = ?")]
    async fn delete(&self, id: i32) -> Result<QueryResult>;
}

struct App {
    pool: Pool,
}

impl UserRepo for App {
    fn get_pool(&self) -> &Pool {
        &self.pool
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create an in-memory SQLite database
    let pool = Pool::connect("sqlite::memory:").await?;

    // Create the users table
    sqlx::query(
        r#"
        CREATE TABLE users (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            email TEXT NOT NULL UNIQUE,
            age INTEGER NOT NULL
        )
        "#
    )
    .execute(&pool)
    .await?;

    // Create our repository
    let app = App { pool };

    // Test the repository
    demo_repository(&app).await?;

    Ok(())
}

async fn demo_repository(repo: &App) -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 SQLx-Data Quick Start Demo");
    println!("============================");

    // CREATE: Insert some users
    println!("\n📝 Creating users...");
    repo.create("Alice Johnson".to_string(), "alice@example.com".to_string(), 30).await?;
    repo.create("Bob Smith".to_string(), "bob@example.com".to_string(), 25).await?;
    repo.create("Carol Davis".to_string(), "carol@example.com".to_string(), 35).await?;

    // READ: Find all users
    println!("\n👥 All users:");
    let users = repo.find_all().await?;
    for user in &users {
        println!("  • {} ({}) - {} years old", user.name, user.email, user.age);
    }

    // READ: Find specific user
    println!("\n🔍 Finding user with ID 1:");
    if let Some(user) = repo.find_by_id(1).await? {
        println!("  Found: {} ({})", user.name, user.email);
    }

    // UPDATE: Update a user
    println!("\n✏️  Updating Alice's age...");
    repo.update("Alice Johnson".to_string(), "alice@example.com".to_string(), 31, 1).await?;

    // Verify the update
    if let Some(user) = repo.find_by_id(1).await? {
        println!("  Alice is now {} years old", user.age);
    }

    // DELETE: Remove a user
    println!("\n🗑️  Deleting user with ID 2...");
    repo.delete(2).await?;

    // Final count
    let final_users = repo.find_all().await?;
    println!("\n📊 Final user count: {}", final_users.len());

    println!("\n✅ Demo completed successfully!");
    Ok(())
}
```

## Run Your Application

Execute your program:

```bash
cargo run
```

You should see output like:

```
🚀 SQLx-Data Quick Start Demo
============================

📝 Creating users...

👥 All users:
  • Alice Johnson (alice@example.com) - 30 years old
  • Bob Smith (bob@example.com) - 25 years old
  • Carol Davis (carol@example.com) - 35 years old

🔍 Finding user with ID 1:
  Found: Alice Johnson (alice@example.com)

✏️  Updating Alice's age...
  Alice is now 31 years old

🗑️  Deleting user with ID 2...

📊 Final user count: 2

✅ Demo completed successfully!
```

## What Just Happened?

In just a few lines of code, you:

1. **Defined a trait** with SQL queries using the `#[dml]` attribute
2. **Got automatic implementations** for all CRUD operations
3. **Achieved compile-time safety** – invalid SQL would be caught at build time
4. **Used proper parameter binding** – no SQL injection vulnerabilities
5. **Handled different return types** – `Option<T>`, `Vec<T>`, and `QueryResult`

## Key Observations

### Zero Boilerplate
Notice how the `UserRepo` implementation only required providing the pool. All query logic, parameter binding, and result mapping was generated automatically.

### Type Safety
The `#[dml]` macro validates your SQL at compile time. Try changing a column name in a query and see the compile error.

### Clean Abstractions
Your business logic works with the `UserRepo` trait, not the implementation details. This makes testing and refactoring much easier.

### Full SQLx Power
Under the hood, SQLx-Data uses `sqlx::query_as!` and friends, giving you all the benefits of SQLx's compile-time verification.

## Next Steps

Now that you've seen SQLx-Data in action, let's dive deeper into the core concepts that make this magic possible. In the next chapter, we'll explore:

- How the `#[repo]` and `#[dml]` macros work
- Different return type patterns
- Parameter binding strategies
- Error handling approaches

Ready to become a SQLx-Data expert? Let's continue!