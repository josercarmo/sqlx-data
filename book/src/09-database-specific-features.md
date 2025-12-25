# Chapter 9: Database-Specific Features

SQLx-Data leverages the unique capabilities of each database engine while maintaining a unified programming interface. This chapter explores how to use database-specific SQL features like RETURNING clauses, UPSERT operations, and type systems effectively.

## PostgreSQL Features

PostgreSQL offers the most advanced SQL features, which SQLx-Data fully supports.

### RETURNING Clauses

PostgreSQL's RETURNING clause allows you to get back data from INSERT, UPDATE, or DELETE operations:

```rust
use sqlx_data::{Pool, QueryResult, Result, dml, repo};

#[derive(Clone, PartialEq, Eq, Debug, sqlx::Type)]
#[sqlx(transparent)]
pub struct Id(i64);

#[derive(Debug, Clone, PartialEq, sqlx::FromRow)]
pub struct User {
    pub id: Id,
    pub name: String,
    pub email: String,
    pub age: i16,                // PostgreSQL SMALLINT
    pub birth_year: Option<i16>, // PostgreSQL SMALLINT
}

#[repo]
#[alias(values = "($1, $2, $3, $4, $5)")] // PostgreSQL numbered parameters
trait PostgreSQLRepo {
    // Batch insert with RETURNING (PostgreSQL only)
    #[dml("INSERT INTO users (name, email, age, birth_year) VALUES ($1, $2, $3, $4) RETURNING id")]
    async fn insert_users_auto_id(&self, rows: Vec<(String, String, i16, Option<i16>)>) -> Result<Vec<i64>>;

    // Batch insert with explicit IDs
    #[dml("INSERT INTO users (id, name, email, age, birth_year) VALUES {{values}}")]
    async fn insert_users_with_id(&self, rows: Vec<(i64, String, String, i16, Option<i16>)>) -> Result<QueryResult>;

    // Select for verification with PostgreSQL casting
    #[dml("SELECT id as \"id!: Id\", name, email, age, birth_year FROM users WHERE id >= $1 ORDER BY id")]
    async fn find_users_from_id(&self, min_id: i64) -> Result<Vec<User>>;
}
```

### PostgreSQL UPSERT (ON CONFLICT)

PostgreSQL's sophisticated conflict resolution:

```rust
#[repo]
trait PostgreSQLUpsertRepo {
    // UPSERT with ON CONFLICT (PostgreSQL specific)
    #[dml("INSERT INTO users (name, email, age, birth_year) VALUES ($1, $2, $3, $4) ON CONFLICT (email) DO UPDATE SET age = EXCLUDED.age, birth_year = EXCLUDED.birth_year")]
    async fn upsert_users(&self, rows: Vec<(String, String, i16, Option<i16>)>) -> Result<QueryResult>;
}
```

Usage example from real tests:

```rust
async fn test_postgresql_upsert_functionality(pool: Pool) -> Result<()> {
    let app = PostgreSQLRepo { pool };

    // First insert
    let batch_data = vec![
        ("Upsert User".to_string(), "upsert@example.com".to_string(), 25, Some(1998)),
    ];

    let result1 = app.upsert_users(batch_data.clone()).await?;
    assert_eq!(result1.rows_affected(), 1);

    // Second "insert" with same email (should update due to ON CONFLICT)
    let updated_data = vec![
        ("Updated Upsert User".to_string(), "upsert@example.com".to_string(), 30, Some(1993)),
    ];

    let result2 = app.upsert_users(updated_data).await?;
    // This will update the existing row if email has unique constraint
    assert!(result2.rows_affected() >= 1);

    Ok(())
}
```

### PostgreSQL Strong Types

PostgreSQL provides native strong typing without casting annotations:

```rust
#[repo]
trait PostgreSQLTypesRepo {
    // PostgreSQL SMALLINT maps directly to i16
    #[dml("SELECT age FROM users WHERE id = $1")]
    async fn get_age(&self, id: i64) -> Result<i16>;

    // DECIMAL for precise calculations
    #[dml("SELECT AVG(age) FROM users")]
    async fn average_age(&self) -> Result<Option<rust_decimal::Decimal>>;
}
```

## MySQL Features

MySQL offers its own set of unique capabilities that SQLx-Data supports.

### ON DUPLICATE KEY UPDATE

MySQL's UPSERT mechanism:

```rust
#[derive(Debug, Clone, PartialEq, sqlx::FromRow)]
pub struct User {
    pub id: Id,
    pub name: String,
    pub email: String,
    pub age: u8,                 // MySQL TINYINT UNSIGNED
    pub birth_year: Option<u16>, // MySQL SMALLINT UNSIGNED
}

#[repo]
#[alias(values = "(?, ?, ?, ?, ?)")] // MySQL positional parameters
trait MySQLRepo {
    // Batch insert with auto-generated IDs (MySQL doesn't support RETURNING)
    #[dml("INSERT INTO users (name, email, age, birth_year) VALUES (?, ?, ?, ?)")]
    async fn insert_users_auto_id(&self, rows: Vec<(String, String, u8, Option<u16>)>) -> Result<QueryResult>;

    // MySQL UPSERT with ON DUPLICATE KEY UPDATE
    #[dml("INSERT INTO users (name, email, age, birth_year) VALUES (?, ?, ?, ?) ON DUPLICATE KEY UPDATE age = VALUES(age), birth_year = VALUES(birth_year)")]
    async fn upsert_users(&self, rows: Vec<(String, String, u8, Option<u16>)>) -> Result<QueryResult>;
}
```

Usage example from real tests:

```rust
async fn test_mysql_upsert_functionality(pool: Pool) -> Result<()> {
    let app = MySQLRepo { pool };

    // First insert
    let batch_data = vec![
        ("Upsert User".to_string(), "upsert@example.com".to_string(), 25, Some(1998)),
    ];

    let result1 = app.upsert_users(batch_data.clone()).await?;
    assert_eq!(result1.rows_affected(), 1);

    // Second "insert" with same email (should update if email is unique)
    let updated_data = vec![
        ("Updated Upsert User".to_string(), "upsert@example.com".to_string(), 30, Some(1993)),
    ];

    let result2 = app.upsert_users(updated_data).await?;
    // MySQL reports 2 for updates in ON DUPLICATE KEY
    assert!(result2.rows_affected() >= 1);

    Ok(())
}
```

### MySQL Type System

MySQL uses different type mappings:

```rust
#[repo]
trait MySQLTypesRepo {
    // MySQL TINYINT UNSIGNED maps to u8
    #[dml("SELECT age FROM users WHERE id = ?")]
    async fn get_age(&self, id: i64) -> Result<u8>;

    // MySQL AUTO_INCREMENT handling
    #[dml("INSERT INTO users (name, email, age) VALUES (?, ?, ?)")]
    async fn create_user(&self, name: String, email: String, age: u8) -> Result<QueryResult>;
}

async fn mysql_auto_increment_example(repo: &impl MySQLTypesRepo) -> Result<()> {
    let result = repo.create_user("Alice".to_string(), "alice@example.com".to_string(), 30).await?;

    // Get the first inserted ID (MySQL way)
    let first_inserted_id = result.last_insert_id();
    assert!(first_inserted_id > 0);

    Ok(())
}
```

## SQLite Features

SQLite provides excellent flexibility and unique capabilities that SQLx-Data leverages.

### SQLite UPSERT Operations

SQLite supports multiple UPSERT syntaxes:

```rust
#[derive(Debug, Clone, PartialEq, sqlx::FromRow)]
pub struct User {
    pub id: Id,
    pub name: String,
    pub email: String,
    pub age: i64,                // SQLite INTEGER
    pub birth_year: Option<i64>, // SQLite INTEGER
}

#[repo]
#[alias(values = "(?, ?, ?, ?, ?)")] // SQLite positional parameters
trait SQLiteRepo {
    // Basic batch insert without RETURNING
    #[dml("INSERT INTO users (id, name, email, age, birth_year) VALUES (?, ?, ?, ?, ?)")]
    async fn insert_users_batch(&self, rows: Vec<(i64, String, String, u8, Option<u16>)>) -> Result<QueryResult>;

    // SQLite with RETURNING (SQLite 3.35+)
    #[dml("INSERT INTO users (name, email, age, birth_year) VALUES (?, ?, ?, ?) RETURNING id")]
    async fn insert_users_auto_id(&self, rows: Vec<(String, String, u8, Option<u16>)>) -> Result<Vec<u64>>;

    // UPSERT with ON CONFLICT
    #[dml("INSERT INTO users (id, name, email, age, birth_year) VALUES (?, ?, ?, ?, ?) ON CONFLICT(id) DO UPDATE SET name = EXCLUDED.name, email = EXCLUDED.email")]
    async fn upsert_users_batch(&self, rows: Vec<(i64, String, String, u8, Option<u16>)>) -> Result<QueryResult>;

    // ON CONFLICT with RETURNING
    #[dml("INSERT INTO users (id, name, email, age, birth_year) VALUES (?, ?, ?, ?, ?) ON CONFLICT(id) DO UPDATE SET name = EXCLUDED.name RETURNING id")]
    async fn upsert_users_returning(&self, rows: Vec<(i64, String, String, u8, Option<u16>)>) -> Result<Vec<i64>>;

    // INSERT OR IGNORE
    #[dml("INSERT INTO users (id, name, email, age, birth_year) VALUES (?, ?, ?, ?, ?) ON CONFLICT(email) DO NOTHING")]
    async fn insert_or_ignore_users(&self, rows: Vec<(i64, String, String, u8, Option<u16>)>) -> Result<QueryResult>;
}
```

### Complex SQLite Constraints

SQLite allows sophisticated conflict resolution:

```rust
#[repo]
trait SQLiteAdvancedRepo {
    // Conditional UPSERT with WHERE clause
    #[dml("INSERT INTO users (id, name, email, age, birth_year) VALUES (?, ?, ?, ?, ?) ON CONFLICT(email) DO UPDATE SET name = EXCLUDED.name, age = EXCLUDED.age WHERE users.age < EXCLUDED.age RETURNING id as \"id!: i64\", name")]
    async fn conditional_upsert_users(&self, rows: Vec<(i64, String, String, u8, Option<u16>)>) -> Result<Vec<(i64, String)>>;
}
```

Usage from real tests:

```rust
async fn test_conditional_upsert_users(pool: Pool) -> Result<()> {
    let repo = SQLiteAdvancedRepo { pool };

    // Insert initial user with lower age
    let initial_users = vec![(
        1000i64,
        "Paul".to_string(),
        "paul@test.com".to_string(),
        25u8,
        Some(1998u16),
    )];
    repo.insert_users_batch(initial_users).await?;

    // Try conditional upsert with higher age (should update)
    let upsert_users = vec![
        (
            1001i64,
            "Paul Updated".to_string(),
            "paul@test.com".to_string(),
            30u8,
            Some(1993u16),
        ),
        (
            1002i64,
            "Quinn".to_string(),
            "quinn@test.com".to_string(),
            22u8,
            Some(2001u16),
        ),
    ];

    let result = repo.conditional_upsert_users(upsert_users).await?;
    assert_eq!(result.len(), 2);
    assert!(
        result
            .iter()
            .any(|(id, name)| *id == 1000 && name == "Paul Updated")
    );

    Ok(())
}
```

### SQLite Type Casting

SQLite's flexible type system with explicit casting:

```rust
#[derive(Debug, Clone, PartialEq, sqlx::FromRow)]
pub struct UserCast {
    pub id: Id,
    pub name: String,
    pub email: String,
    pub age: u8,
    pub birth_year: Option<u16>,
}

#[repo]
trait SQLiteCastingRepo {
    // Batch insert with cast in RETURNING
    #[dml("INSERT INTO users (id, name, email, age, birth_year) VALUES {{values}} RETURNING id as 'id!: Id', name, email, age as 'age: u8', birth_year as 'birth_year: u16'")]
    async fn insert_users_batch_with_cast(&self, rows: Vec<(i64, String, String, u8, Option<u16>)>) -> Result<Vec<UserCast>>;
}
```

## Database Parameter Syntax

Each database uses different parameter syntax, which SQLx-Data respects:

### PostgreSQL - Numbered Parameters

```rust
#[repo]
trait PostgreSQLParams {
    // $1, $2, $3... numbering
    #[dml("SELECT * FROM users WHERE name = $1 AND age >= $2")]
    async fn find_by_name_and_age(&self, name: String, min_age: i16) -> Result<Vec<User>>;
}
```

### MySQL - Positional Parameters

```rust
#[repo]
trait MySQLParams {
    // ? placeholders in order
    #[dml("SELECT * FROM users WHERE name = ? AND age >= ?")]
    async fn find_by_name_and_age(&self, name: String, min_age: u8) -> Result<Vec<User>>;
}
```

### SQLite - Both Syntaxes Supported

```rust
#[repo]
trait SQLiteParams {
    // Positional parameters
    #[dml("SELECT * FROM users WHERE name = ? AND age >= ?")]
    async fn find_by_name_and_age_positional(&self, name: String, min_age: i64) -> Result<Vec<User>>;

    // Named parameters also work
    #[dml("SELECT * FROM users WHERE name = @name AND age >= @min_age")]
    async fn find_by_name_and_age_named(&self, name: String, min_age: i64) -> Result<Vec<User>>;
}
```

## Transaction Handling

All databases support transactions through SQLx-Data:

```rust
use sqlx_data::Transaction;

#[repo]
trait TransactionRepo {
    // Method that accepts transaction parameter
    #[dml("INSERT INTO users (id, name, email, age, birth_year) VALUES (?, ?, ?, ?, ?)")]
    async fn insert_users_batch_with_transaction(&self, rows: Vec<(i64, String, String, u8, Option<u16>)>, tx: &mut Transaction<'_>) -> Result<QueryResult>;

    #[dml("SELECT COUNT(*) FROM users")]
    async fn count_users(&self) -> Result<i64>;
}

async fn transaction_example(repo: &impl TransactionRepo, pool: &Pool) -> Result<()> {
    // Count initial before transaction
    let count_initial = repo.count_users().await?;

    // Start a transaction
    let mut tx = pool.begin().await?;

    // Insert batches in transaction
    let users_batch1 = vec![
        (100i64, "Alice Transaction".to_string(), "alice_tx@test.com".to_string(), 28u8, Some(1995u16)),
        (101i64, "Bob Transaction".to_string(), "bob_tx@test.com".to_string(), 32u8, Some(1991u16)),
    ];

    let result1 = repo.insert_users_batch_with_transaction(users_batch1, &mut tx).await?;
    assert_eq!(result1.rows_affected(), 2);

    // Insert second batch in same transaction
    let users_batch2 = vec![(
        102i64, "Charlie Transaction".to_string(), "charlie_tx@test.com".to_string(), 25u8, Some(1998u16)
    )];

    let result2 = repo.insert_users_batch_with_transaction(users_batch2, &mut tx).await?;
    assert_eq!(result2.rows_affected(), 1);

    // Commit transaction
    tx.commit().await?;

    // Count final after commit
    let count_final = repo.count_users().await?;
    assert_eq!(count_final, count_initial + 3);

    Ok(())
}
```

## Best Practices by Database

### PostgreSQL Best Practices

1. **Use RETURNING**: Leverage PostgreSQL's RETURNING clause for efficient data retrieval
2. **Strong Types**: Take advantage of PostgreSQL's strong type system
3. **ON CONFLICT**: Use sophisticated conflict resolution with conditions

```rust
#[repo]
trait PostgreSQLBestPractices {
    // Efficient batch insert with immediate data retrieval
    #[dml("INSERT INTO users (name, email, age) VALUES ($1, $2, $3) RETURNING id, name, created_at")]
    async fn create_users_with_metadata(&self, rows: Vec<(String, String, i16)>) -> Result<Vec<(i64, String, chrono::DateTime<chrono::Utc>)>>;
}
```

### MySQL Best Practices

1. **AUTO_INCREMENT**: Use MySQL's auto-increment for ID generation
2. **ON DUPLICATE KEY**: Use for efficient upsert operations
3. **Type Mapping**: Be aware of MySQL's specific type mappings

```rust
#[repo]
trait MySQLBestPractices {
    // Let MySQL handle ID generation
    #[dml("INSERT INTO users (name, email, age) VALUES (?, ?, ?)")]
    async fn create_user_auto_id(&self, name: String, email: String, age: u8) -> Result<QueryResult>;
}
```

### SQLite Best Practices

1. **Flexible Schema**: Leverage SQLite's dynamic typing when needed
2. **UPSERT Options**: Use appropriate ON CONFLICT strategy
3. **RETURNING Support**: Use modern SQLite's RETURNING clause

```rust
#[repo]
trait SQLiteBestPractices {
    // Flexible parameter types
    #[dml("INSERT INTO users (name, email, age) VALUES (?, ?, ?) RETURNING id")]
    async fn create_user_flexible(&self, name: String, email: String, age: i64) -> Result<i64>;
}
```

## Performance Considerations

### Database-Specific Optimizations

1. **PostgreSQL**: Use batch inserts with RETURNING for maximum efficiency
2. **MySQL**: Leverage ON DUPLICATE KEY UPDATE for bulk upserts
3. **SQLite**: Use transactions for large batch operations

### Type System Performance

- **PostgreSQL**: Strong types provide better performance and safety
- **MySQL**: Use appropriate unsigned types for better space efficiency
- **SQLite**: Casting annotations help with type safety

## Migration Between Databases

SQLx-Data makes it easier to migrate between databases by abstracting differences:

```rust
// Generic trait that works across databases
#[repo]
trait DatabaseAgnosticRepo {
    // Use generic SQL that works everywhere
    #[dml("SELECT COUNT(*) FROM users WHERE age >= ?")]
    async fn count_adults(&self, min_age: i32) -> Result<i64>;

    // Basic insert without database-specific features
    #[dml("INSERT INTO users (name, email, age) VALUES (?, ?, ?)")]
    async fn create_user(&self, name: String, email: String, age: i32) -> Result<QueryResult>;
}
```

When you need database-specific features, you can create specialized traits:

```rust
// Database-specific optimizations
trait DatabaseSpecificOptimizations: DatabaseAgnosticRepo {
    // Implement with database-specific features
    async fn bulk_upsert_users(&self, users: Vec<(String, String, i32)>) -> Result<Vec<i64>>;
}

// PostgreSQL implementation
impl DatabaseSpecificOptimizations for PostgreSQLRepo {
    async fn bulk_upsert_users(&self, users: Vec<(String, String, i32)>) -> Result<Vec<i64>> {
        // Use PostgreSQL ON CONFLICT with RETURNING
        self.postgresql_upsert_with_returning(users).await
    }
}

// MySQL implementation
impl DatabaseSpecificOptimizations for MySQLRepo {
    async fn bulk_upsert_users(&self, users: Vec<(String, String, i32)>) -> Result<Vec<i64>> {
        // Use MySQL ON DUPLICATE KEY UPDATE then SELECT
        self.mysql_upsert_then_select(users).await
    }
}
```

## Next Steps

With database-specific features mastered, you can leverage the unique capabilities of each database while maintaining clean, type-safe code. In the next chapter, we'll explore error handling and debugging techniques to build robust applications.

Understanding these database-specific patterns allows you to choose the right tool for each task while maintaining the benefits of SQLx-Data's type safety and compile-time verification.