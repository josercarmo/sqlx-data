# Chapter 5: Return Types and Parameter Binding

SQLx-Data provides intelligent return type handling and flexible parameter binding strategies. Understanding these features is essential for building sophisticated repositories that handle all kinds of data patterns.

## Return Type Intelligence

SQLx-Data analyzes your method's return type and automatically chooses the appropriate SQLx method:

### Single Results

```rust
#[repo]
trait UserRepo {
    // Result<T> -> uses fetch_one() - expects exactly one row
    #[dml("SELECT name FROM users WHERE id = ?")]
    async fn get_name(&self, id: i64) -> Result<String>;

    // Result<Option<T>> -> uses fetch_optional() - zero or one row
    #[dml("SELECT name FROM users WHERE id = ?")]
    async fn find_name(&self, id: i64) -> Result<Option<String>>;
}
```

Generated code:
```rust
// From Result<String>
async fn get_name_query(&self, id: i64) -> Result<String> {
    sqlx::query_scalar!("SELECT name FROM users WHERE id = ?", id)
        .fetch_one(self.get_pool())
        .await
}

// From Result<Option<String>>
async fn find_name_query(&self, id: i64) -> Result<Option<String>> {
    sqlx::query_scalar!("SELECT name FROM users WHERE id = ?", id)
        .fetch_optional(self.get_pool())
        .await
}
```

### Multiple Results

```rust
#[repo]
trait UserRepo {
    // Result<Vec<T>> -> uses fetch_all() - zero or more rows
    #[dml("SELECT name FROM users")]
    async fn all_names(&self) -> Result<Vec<String>>;

    // Return all users as structs
    #[dml("SELECT id, name, email, age FROM users")]
    async fn find_all(&self) -> Result<Vec<User>>;
}
```

### Tuples - Multiple Columns

SQLx-Data supports complex tuple return types from real tests:

```rust
#[repo]
trait TupleRepo {
    // Basic tuple - count and average
    #[dml("SELECT COUNT(id) as count, AVG(age) as avg_age FROM users")]
    async fn average_age(&self) -> Result<(i64, Option<f64>)>;

    // Multiple rows of tuples
    #[dml("SELECT id, age, name FROM users LIMIT 10")]
    async fn get_all_ages(&self) -> Result<Vec<(i64, u8, String)>>;

    // Single row tuple
    #[dml("SELECT name, birth_year FROM users WHERE id = ?")]
    async fn get_one_birth(&self, id: i64) -> Result<(String, Option<u16>)>;
}
```

### Streaming Results

For memory-efficient processing of large datasets:

```rust
use futures::Stream;

#[repo]
trait StreamRepo {
    // fn (not async) returns Stream for memory-efficient processing
    #[dml("SELECT id, name, age FROM users")]
    fn stream_users(&self) -> impl Stream<Item = Result<SimpleUser>> + Send;

    // Stream scalar values
    #[dml("SELECT name FROM users ORDER BY name")]
    fn stream_names(&self) -> impl Stream<Item = Result<String>> + Send;

    // Stream tuples
    #[dml("SELECT name, age FROM users WHERE age >= ?")]
    fn stream_user_info(&self, min_age: u8) -> impl Stream<Item = Result<(String, u8)>> + Send;
}
```

Usage example:
```rust
use futures::StreamExt;

async fn process_all_users(repo: &impl StreamRepo) -> Result<()> {
    let mut stream = repo.stream_users();

    while let Some(user_result) = stream.next().await {
        let user = user_result?;
        println!("Processing: {} ({})", user.name, user.age);
        // Process without loading all users into memory
    }

    Ok(())
}
```

### Modification Results

```rust
#[repo]
trait UserRepo {
    // Result<QueryResult> -> uses execute() - for INSERT/UPDATE/DELETE
    #[dml("DELETE FROM users WHERE id = ?")]
    async fn delete(&self, id: i64) -> Result<QueryResult>;

    // Check affected rows
    #[dml("UPDATE users SET age = ? WHERE id = ?")]
    async fn update_age(&self, age: u8, id: i64) -> Result<QueryResult>;
}
```

Usage:
```rust
async fn safe_delete(repo: &impl UserRepo, id: i64) -> Result<bool> {
    let result = repo.delete(id).await?;
    Ok(result.rows_affected() > 0)
}
```

## Parameter Binding Strategies

SQLx-Data supports both positional and named parameter binding:

### Positional Parameters

Parameters are bound by their position in the function signature:

```rust
#[repo]
trait UserRepo {
    // Parameters bound by position: name -> ?, email -> ?, age -> ?
    #[dml("INSERT INTO users (name, email, age) VALUES (?, ?, ?)")]
    async fn create(&self, name: String, email: String, age: u8) -> Result<QueryResult>;

    // Order matters in function signature
    #[dml("SELECT * FROM users WHERE age >= ? AND name LIKE ?")]
    async fn search_by_age_and_name(&self, min_age: u8, name_pattern: String) -> Result<Vec<User>>;
}
```

### Named Parameters

Use `@parameter_name` syntax for clearer, reusable parameters:

```rust
#[repo]
trait UserRepo {
    // Named parameters - can be in any order in function signature
    #[dml("SELECT * FROM users WHERE name = @name AND age >= @min_age")]
    async fn search(&self, min_age: u8, name: String) -> Result<Vec<User>>;

    // Parameter reuse (supported by PostgreSQL and SQLite)
    #[dml("SELECT * FROM users WHERE (name = @search OR email = @search) AND age > @min_age")]
    async fn search_user(&self, search: String, min_age: u8) -> Result<Vec<User>>;

    // Complex named parameters
    #[dml("UPDATE users SET name = @new_name, email = @new_email WHERE id = @user_id")]
    async fn update_user_info(&self, user_id: i64, new_name: String, new_email: String) -> Result<QueryResult>;
}
```

### Mixed Database-Specific Syntax

SQLx-Data respects each database's parameter syntax:

```rust
// PostgreSQL - numbered parameters
#[dml("INSERT INTO users (name, email) VALUES ($1, $2) RETURNING *")]
async fn create_user_pg(&self, name: String, email: String) -> Result<User>;

// MySQL - positional parameters
#[dml("INSERT INTO users (name, email) VALUES (?, ?)")]
async fn create_user_mysql(&self, name: String, email: String) -> Result<QueryResult>;

// SQLite - both supported
#[dml("SELECT * FROM users WHERE name = ? AND email = ?")]
async fn find_by_name_email_sqlite(&self, name: String, email: String) -> Result<Option<User>>;
```

## Type Casting and Conversion

SQLx-Data handles automatic type conversion between Rust and SQL types:

### SQLite Auto-Casting Examples

```rust
#[repo]
trait CastingRepo {
    // SQLite INTEGER -> Rust u8 with casting annotation
    #[dml("SELECT age as 'age: u8' FROM users WHERE id = ?")]
    async fn get_age_u8(&self, id: i64) -> Result<u8>;

    // Optional casting for nullable columns
    #[dml("SELECT birth_year as 'birth_year: Option<u16>' FROM users WHERE id = ?")]
    async fn get_birth_year(&self, id: i64) -> Result<Option<u16>>;

    // Complex tuple with mixed casting
    #[dml("SELECT name, birth_year as 'birth_year: Option<u16>' FROM users WHERE id = ?")]
    async fn get_name_and_birth(&self, id: i64) -> Result<(String, Option<u16>)>;
}
```

### PostgreSQL Strong Types

PostgreSQL provides strong type checking without casting annotations:

```rust
#[repo]
trait PostgresRepo {
    // PostgreSQL SMALLINT maps directly to i16
    #[dml("SELECT age FROM users WHERE id = $1")]
    async fn get_age(&self, id: i64) -> Result<i16>;

    // DECIMAL for precise calculations
    #[dml("SELECT AVG(age) FROM users")]
    async fn average_age(&self) -> Result<Option<Decimal>>;
}
```

## Complex Return Types

### Nested Structs

```rust
#[derive(sqlx::FromRow)]
pub struct UserProfile {
    pub id: i64,
    pub name: String,
    pub email: String,
    pub age: u8,
}

#[repo]
trait ProfileRepo {
    #[dml("SELECT id, name, email, age FROM users WHERE id = ?")]
    async fn get_profile(&self, id: i64) -> Result<Option<UserProfile>>;

    #[dml("SELECT id, name, email, age FROM users")]
    async fn all_profiles(&self) -> Result<Vec<UserProfile>>;
}
```

### Count Queries

```rust
#[repo]
trait CountRepo {
    // COUNT returns i64 in most databases
    #[dml("SELECT COUNT(*) FROM users")]
    async fn total_users(&self) -> Result<i64>;

    // Optional count (shouldn't happen but supported)
    #[dml("SELECT COUNT(*) FROM users")]
    async fn count_users(&self) -> Result<Option<i64>>;
}
```

## Error Handling with Return Types

Different return types provide different error behaviors:

```rust
// fetch_one() - fails if no rows or multiple rows
async fn get_exact_user(repo: &impl UserRepo, id: i64) -> Result<User> {
    repo.find_by_id(id).await // Panics if 0 or 2+ rows
}

// fetch_optional() - returns None if no rows, fails if multiple rows
async fn maybe_get_user(repo: &impl UserRepo, id: i64) -> Result<Option<User>> {
    repo.find_optional_by_id(id).await // None if not found, Some(user) if found
}

// fetch_all() - never fails for missing data, returns empty Vec
async fn get_all_users(repo: &impl UserRepo) -> Result<Vec<User>> {
    repo.find_all().await // Always succeeds, empty Vec if no users
}
```

## Practical Examples

### User Search with Multiple Return Types

```rust
#[repo]
trait UserSearchRepo {
    // Exact match - must exist
    #[dml("SELECT * FROM users WHERE email = ?")]
    async fn get_by_email(&self, email: String) -> Result<User>;

    // Safe lookup - might not exist
    #[dml("SELECT * FROM users WHERE email = ?")]
    async fn find_by_email(&self, email: String) -> Result<Option<User>>;

    // Search multiple - always returns vector
    #[dml("SELECT * FROM users WHERE name LIKE ?")]
    async fn search_by_name(&self, pattern: String) -> Result<Vec<User>>;

    // Count results
    #[dml("SELECT COUNT(*) FROM users WHERE age >= ?")]
    async fn count_adults(&self, min_age: u8) -> Result<i64>;

    // Stream large results
    #[dml("SELECT id, name FROM users ORDER BY id")]
    fn stream_all_ids(&self) -> impl Stream<Item = Result<(i64, String)>> + Send;
}
```

## Key Principles

### 1. Return Type Drives Behavior
The return type in your trait method signature determines which SQLx method is called.

### 2. Parameter Order Flexibility
Named parameters (`@name`) provide more flexibility than positional parameters (`?`).

### 3. Database-Specific Optimization
SQLx-Data leverages each database's parameter syntax and type system.

### 4. Memory Efficiency
Use streaming for large datasets to avoid loading everything into memory.

## Next Steps

With a solid understanding of return types and parameter binding, you're ready to explore SQLx-Data's advanced features. In the next chapter, we'll dive into sophisticated pagination strategies that work seamlessly with your existing queries.

The foundation you've built here - understanding how SQLx-Data handles different data patterns - will serve you well as we explore more complex scenarios like cursor pagination, filtering, and batch operations.