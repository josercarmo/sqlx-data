# Chapter 8: Streaming and Batch Operations

SQLx-Data provides powerful streaming capabilities for memory-efficient processing of large datasets and batch operations for high-performance data insertion. These features are essential for building applications that handle large amounts of data efficiently.

## Streaming Results

Streaming allows you to process query results one row at a time without loading the entire result set into memory. This is crucial for handling large datasets or when memory is constrained.

### Basic Streaming

```rust
use futures::{Stream, StreamExt};
use sqlx_data::{repo, dml, Result};

#[derive(Debug, sqlx::FromRow)]
pub struct User {
    pub id: i64,
    pub name: String,
    pub age: u8,
}

#[repo]
trait StreamRepo {
    // Stream all users (note: fn not async fn)
    #[dml("SELECT id, name, age FROM users")]
    fn stream_users(&self) -> impl Stream<Item = Result<User>>;

    // Stream with filtering
    #[dml("SELECT id, name, age FROM users WHERE age >= ?")]
    fn stream_users_by_age(&self, min_age: u8) -> impl Stream<Item = Result<User>>;
}
```

**Important**: Streaming methods use `fn` (not `async fn`) and return `impl Stream<Item = Result<T>>`.

### Processing Streams

```rust
async fn process_all_users(repo: &impl StreamRepo) -> Result<()> {
    let mut stream = repo.stream_users();
    let mut count = 0;

    while let Some(user_result) = stream.next().await {
        let user = user_result?;
        println!("Processing user: {} ({})", user.name, user.age);

        // Process user without loading all into memory
        process_user(user).await;
        count += 1;
    }

    println!("Processed {} users", count);
    Ok(())
}

async fn process_user(user: User) {
    // Your business logic here
    println!("User {} is {} years old", user.name, user.age);
}
```

### Streaming Different Data Types

#### Scalar Values
```rust
#[repo]
trait ScalarStreamRepo {
    // Stream names only
    #[dml("SELECT name FROM users ORDER BY name")]
    fn stream_names(&self) -> impl Stream<Item = Result<String>>;

    // Stream ages with filtering
    #[dml("SELECT age FROM users WHERE age >= ?")]
    fn stream_ages(&self, min_age: u8) -> impl Stream<Item = Result<u8>>;

    // Stream counts
    #[dml("SELECT COUNT(*) FROM users")]
    fn stream_count(&self) -> impl Stream<Item = Result<i64>>;
}
```

#### Tuples
```rust
#[repo]
trait TupleStreamRepo {
    // Stream tuples
    #[dml("SELECT name, age FROM users WHERE age BETWEEN ? AND ?")]
    fn stream_user_info(&self, min_age: u8, max_age: u8) -> impl Stream<Item = Result<(String, u8)>>;

    // Complex tuples
    #[dml("SELECT id, name, email, age FROM users")]
    fn stream_full_info(&self) -> impl Stream<Item = Result<(i64, String, String, u8)>>;
}
```

### Advanced Stream Processing

#### Filtering and Transforming
```rust
use futures::{StreamExt, TryStreamExt};

async fn find_specific_users(repo: &impl StreamRepo) -> Result<Vec<String>> {
    let names: Vec<String> = repo.stream_users()
        .try_filter(|user| futures::future::ready(user.age >= 30)) // Filter adults
        .map_ok(|user| user.name) // Transform to names
        .try_collect() // Collect results
        .await?;

    Ok(names)
}
```

#### Chunked Processing
```rust
async fn process_in_chunks(repo: &impl StreamRepo) -> Result<()> {
    let mut stream = repo.stream_users();
    let mut chunk = Vec::new();
    const CHUNK_SIZE: usize = 100;

    while let Some(user_result) = stream.next().await {
        let user = user_result?;
        chunk.push(user);

        if chunk.len() >= CHUNK_SIZE {
            process_chunk(chunk).await;
            chunk = Vec::new(); // Reset chunk
        }
    }

    // Process remaining items
    if !chunk.is_empty() {
        process_chunk(chunk).await;
    }

    Ok(())
}

async fn process_chunk(users: Vec<User>) {
    println!("Processing chunk of {} users", users.len());
    // Batch process users
}
```

#### Early Termination
```rust
async fn find_first_adult(repo: &impl StreamRepo) -> Result<Option<User>> {
    let mut stream = repo.stream_users();

    while let Some(user_result) = stream.next().await {
        let user = user_result?;
        if user.age >= 18 {
            return Ok(Some(user)); // Found first adult, stop streaming
        }
    }

    Ok(None) // No adults found
}
```

## Batch Operations

Batch operations allow you to insert multiple records efficiently in a single database operation, dramatically improving performance for bulk data insertion.

### Basic Batch Insertion

```rust
#[derive(Debug, sqlx::FromRow)]
pub struct User {
    pub id: i64,
    pub name: String,
    pub email: String,
    pub age: u8,
    pub birth_year: Option<u16>,
}

#[repo]
trait BatchRepo {
    // Batch insert with auto-generated IDs
    #[dml("INSERT INTO users (name, email, age, birth_year) VALUES (?, ?, ?, ?)")]
    async fn insert_users(&self, rows: Vec<(String, String, u8, Option<u16>)>) -> Result<QueryResult>;

    // Batch insert with explicit IDs
    #[dml("INSERT INTO users (id, name, email, age, birth_year) VALUES (?, ?, ?, ?, ?)")]
    async fn insert_users_with_id(&self, rows: Vec<(i64, String, String, u8, Option<u16>)>) -> Result<QueryResult>;
}
```

### Using Batch Operations

```rust
async fn create_multiple_users(repo: &impl BatchRepo) -> Result<()> {
    let batch_data = vec![
        ("Alice Smith".to_string(), "alice@example.com".to_string(), 30, Some(1993)),
        ("Bob Johnson".to_string(), "bob@example.com".to_string(), 25, Some(1998)),
        ("Carol Davis".to_string(), "carol@example.com".to_string(), 35, None),
    ];

    let result = repo.insert_users(batch_data).await?;

    println!("Inserted {} users", result.rows_affected());
    println!("First inserted ID: {}", result.last_insert_id());

    Ok(())
}
```

### Database-Specific Batch Features

#### MySQL - ON DUPLICATE KEY UPDATE
```rust
#[repo]
trait MySQLBatchRepo {
    // Upsert with MySQL syntax
    #[dml("INSERT INTO users (name, email, age) VALUES (?, ?, ?) ON DUPLICATE KEY UPDATE age = VALUES(age)")]
    async fn upsert_users(&self, rows: Vec<(String, String, u8)>) -> Result<QueryResult>;
}
```

#### PostgreSQL - RETURNING with Batch
```rust
#[repo]
trait PostgresBatchRepo {
    // Batch insert with RETURNING (PostgreSQL only)
    #[dml("INSERT INTO users (name, email, age) VALUES ($1, $2, $3) RETURNING id, name")]
    async fn insert_users_returning(&self, rows: Vec<(String, String, u8)>) -> Result<Vec<(i64, String)>>;
}
```

#### SQLite - INSERT OR REPLACE
```rust
#[repo]
trait SQLiteBatchRepo {
    // SQLite upsert syntax
    #[dml("INSERT OR REPLACE INTO users (id, name, email, age) VALUES (?, ?, ?, ?)")]
    async fn replace_users(&self, rows: Vec<(i64, String, String, u8)>) -> Result<QueryResult>;

    // SQLite with ON CONFLICT
    #[dml("INSERT INTO users (name, email, age) VALUES (?, ?, ?) ON CONFLICT(email) DO UPDATE SET age = excluded.age")]
    async fn upsert_by_email(&self, rows: Vec<(String, String, u8)>) -> Result<QueryResult>;
}
```

### Working with Aliases in Batch Operations

```rust
#[repo]
#[alias(values = "(?, ?, ?, ?, ?)")] // Reusable values pattern
#[alias(user_columns = "id, name, email, age, birth_year")]
trait AliasedBatchRepo {
    #[dml("INSERT INTO users ({{user_columns}}) VALUES {{values}}")]
    async fn insert_users_with_alias(&self, rows: Vec<(i64, String, String, u8, Option<u16>)>) -> Result<QueryResult>;
}
```

### Large Batch Processing

For very large datasets, consider chunking:

```rust
async fn insert_large_dataset(repo: &impl BatchRepo, all_users: Vec<(String, String, u8, Option<u16>)>) -> Result<()> {
    const BATCH_SIZE: usize = 1000; // Adjust based on database limits
    let mut total_inserted = 0;

    for chunk in all_users.chunks(BATCH_SIZE) {
        let result = repo.insert_users(chunk.to_vec()).await?;
        total_inserted += result.rows_affected();
        println!("Inserted batch: {} users (total: {})", result.rows_affected(), total_inserted);
    }

    println!("Total users inserted: {}", total_inserted);
    Ok(())
}
```

## Advanced Patterns

### Streaming with Batch Processing

Combine streaming and batching for memory-efficient bulk operations:

```rust
#[repo]
trait StreamBatchRepo {
    // Stream source data
    #[dml("SELECT name, email, age FROM legacy_users")]
    fn stream_legacy_users(&self) -> impl Stream<Item = Result<(String, String, u8)>>;

    // Batch insert target
    #[dml("INSERT INTO users (name, email, age, created_at) VALUES (?, ?, ?, NOW())")]
    async fn insert_migrated_users(&self, rows: Vec<(String, String, u8)>) -> Result<QueryResult>;
}

async fn migrate_users(repo: &impl StreamBatchRepo) -> Result<()> {
    let mut stream = repo.stream_legacy_users();
    let mut batch = Vec::new();
    const BATCH_SIZE: usize = 500;

    while let Some(user_result) = stream.next().await {
        let user = user_result?;
        batch.push(user);

        if batch.len() >= BATCH_SIZE {
            repo.insert_migrated_users(batch).await?;
            batch = Vec::new();
            println!("Migrated batch of {} users", BATCH_SIZE);
        }
    }

    // Insert remaining users
    if !batch.is_empty() {
        repo.insert_migrated_users(batch).await?;
        println!("Migrated final batch of {} users", batch.len());
    }

    Ok(())
}
```

### Error Handling in Streams

```rust
async fn robust_stream_processing(repo: &impl StreamRepo) -> Result<()> {
    let mut stream = repo.stream_users();
    let mut success_count = 0;
    let mut error_count = 0;

    while let Some(user_result) = stream.next().await {
        match user_result {
            Ok(user) => {
                match process_user(user).await {
                    Ok(_) => success_count += 1,
                    Err(e) => {
                        eprintln!("Failed to process user: {}", e);
                        error_count += 1;
                    }
                }
            }
            Err(e) => {
                eprintln!("Failed to fetch user: {}", e);
                error_count += 1;
            }
        }
    }

    println!("Processing complete: {} success, {} errors", success_count, error_count);
    Ok(())
}
```

## Performance Considerations

### Streaming
- **Memory Efficiency**: Streams process one row at a time, using constant memory
- **Network Efficiency**: Rows are fetched as needed, reducing initial latency
- **Processing Speed**: Allows concurrent processing while fetching more data
- **Connection Usage**: Keeps database connection active during streaming

### Batching
- **Insertion Speed**: Dramatically faster than individual inserts (10-100x improvement)
- **Transaction Overhead**: Reduces transaction cost per record
- **Network Efficiency**: Fewer round trips to database
- **Memory Usage**: Requires holding entire batch in memory

### Best Practices

1. **Batch Size**: Start with 100-1000 records per batch, adjust based on performance
2. **Memory Management**: Monitor memory usage with large batches
3. **Error Handling**: Plan for partial batch failures
4. **Transaction Boundaries**: Consider transaction scope for batch operations
5. **Index Impact**: Batch inserts may temporarily impact index performance
6. **Connection Pooling**: Streaming ties up connections longer

## Use Cases

### When to Use Streaming
- Large result sets (>10,000 rows)
- Memory-constrained environments
- Real-time data processing
- ETL operations
- Report generation from large datasets

### When to Use Batch Operations
- Bulk data imports
- Data migration
- High-volume data ingestion
- Periodic batch jobs
- Initializing databases with large datasets

## Next Steps

With streaming and batch operations mastered, you can efficiently handle large-scale data processing scenarios. In the next chapter, we'll explore database-specific features that leverage the unique capabilities of SQLite, MySQL, and PostgreSQL.

Understanding these patterns prepares you for building production-grade applications that can handle millions of records while maintaining optimal performance and resource usage.