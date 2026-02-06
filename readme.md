<h1 align="center">SQLx-Data</h1>
<div align="center">
 <strong>
   🏗️ An advanced SQLx companion for type-safe query repositories
 </strong>
</div>

<br />

<div align="center">
  <p align="center">
    <img src="https://github.com/josercarmo/sqlx-data/raw/HEAD/resources/sqlxdata.jpg" alt="sqlx-data banner" width="50%">
  </p>
</div>

<br />

<div align="center">
  <!-- Crates.io -->
  <a href="https://crates.io/crates/sqlx-data">
    <img src="https://img.shields.io/crates/v/sqlx-data.svg?style=flat-square"
    alt="Crates.io version" /></a>
  <!-- Docs -->
  <a href="https://docs.rs/sqlx-data">
    <img src="https://img.shields.io/badge/docs-latest-blue.svg?style=flat-square" alt="docs.rs docs" /></a>
  <!-- Downloads -->
  <a href="https://crates.io/crates/sqlx-data">
    <img src="https://img.shields.io/crates/d/sqlx-data.svg?style=flat-square" alt="Download" />
  </a>
  <!-- License -->
  <a href="LICENSE">
    <img src="https://img.shields.io/badge/license-MIT-blue.svg?style=flat-square" alt="License" /></a>
</div>

<div align="center">
  <h4>
    <a href="#quick-start">
      Quick Start
    </a>
    <span> | </span>
    <a href="#features">
      Features
    </a>
    <span> | </span>
    <a href="https://docs.rs/sqlx-data">
      Docs
    </a>
    <span> | </span>
    <a href="#examples">
      Examples
    </a>
  </h4>
</div>

<br />

Zero-boilerplate Repository Pattern for modern Rust applications.

Automatic SQLx parameter binding and result parsing with trait-based repositories. Write SQL traits, get async implementations with sophisticated `pagination`, `streaming`, `batch operations`, and `more`. Seamlessly integrates with existing SQLx code — continue using SQLx queries normally, override generated methods, and reuse generated `_query` methods across different contexts.

```rust
#[repo]
trait UserRepo {
    #[dml("SELECT * FROM users WHERE id = ?")]
    async fn find_by_id(&self, id: i64) -> Result<User>;

    #[dml("SELECT * FROM users WHERE age >= ?")]
    async fn find_adults(&self, min_age: u8) -> Result<Vec<User>>;

    #[dml("SELECT * FROM users ORDER BY id")]
    async fn find_all_cursor(&self, params: FilterParams) -> Result<Cursor<User>>;

    #[dml("SELECT * FROM users WHERE age >= 18")]
    async fn stream_active(&self) -> Result<impl Stream<Item = Result<User>>>;
}
```

---

## Features

### 🎯 Core

- **Zero boilerplate** — Write traits, not implementations
- **Compile-time safety** — Always uses SQLx's compile-time macros (`query_as!`, `query!`, `query_scalar!`) ensuring type safety and SQL validation at build time
- **Smart type inference** — Returns `T`, `Option<T>`, `Vec<T>`, tuples, scalars and others
- **Multi-database** — PostgreSQL, MySQL, SQLite

### 🚀 Advanced

- **[Pagination & Dynamic Queries](#pagination--dynamic-queries)**
  Built-in Serial, Slice, and Cursor strategies. Automatically generates the correct `COUNT(*)` query for pagination metadata and handles complex dynamic filters via `ParamsBuilder`.

- **[Parameter Naming](#parameter-naming)**
  Use named parameters with `@parameter_name` syntax for cleaner, more readable queries. Parameters can appear in any order and be reused multiple times within the same query.

- **[Aliases](#aliases)**
  Keep your SQL DRY by defining reusable fragments (like common column sets or table joins) and injecting them using `{{mustache}}` syntax.

- **[Scopes](#scopes)**
  Rails-inspired query composability. Define reusable `WHERE` clauses or orderings that are **automatically injected** into every query in the repository. Supports Alias interpolation (e.g., `age > {{min_age}}`). Ideal for global patterns like 'SoftDeletable', 'Multi-tenancy', or 'Activable'.

- **[Batch Operations](#batch-operations)**
  Perform ultra-fast bulk inserts (up to 40x faster) by taking `Vec<T>` arguments, automatically generating efficient multi-value SQL statements.

- **[JSON Support](#json-support)**
  First-class JSON support. Use the `json` attribute to automatically serialize/deserialize complex Rust structs into database JSON columns without manual boilerplate.

- **[Unchecked Queries](#unchecked-queries)**
  An escape hatch for DDL, legacy queries, or dynamic SQL that cannot be verified at compile time. Use `unchecked` to bypass the macro's strict validation when necessary.

- **[Method Variants](#method-variants)**
  Automatically generate `_with_tx`, `_with_conn`, and `_with_pool` variants for every repository method, ensuring you never get stuck with the wrong executor type.

- **[Streaming](#streaming)**
  Return `impl Stream<Item = Result<T>>` to process large datasets efficiently row-by-row, keeping memory usage constant. Leverages SQLx's native [`fetch`](https://docs.rs/sqlx/latest/sqlx/query/struct.QueryAs.html#method.fetch) for zero-overhead streaming.

- **[Tracing](#tracing)**
  Zero-config observability powered by the [`tracing`](https://crates.io/crates/tracing) crate. Automatically instruments every query with [`#[instrument]`](https://docs.rs/tracing/latest/tracing/attr.instrument.html) spans, capturing execution time, arguments, and errors.

- **[Generics & Lifetimes](#generics--lifetimes)**
  Full support for Rust's generic type system, allowing repositories to handle generic executors, lifetimes, and complex trait bounds.

- **[Hover to inspect](#hover-to-inspect)**
  Hover over any repository method in your IDE to see the exact SQL query and generated implementation code.

## Feature Flags

SQLx-Data uses feature flags to enable database and type support. **You must specify both a database and typically `json`**:

### Database Features (choose one)
- `sqlite` — SQLite database support
- `mysql` — MySQL database support
- `postgres` — PostgreSQL database support

### Type Features
- `json` — JSON support with automatic serialization (recommended)
- `chrono` — Chrono date/time types
- `time` — Time crate support
- `uuid` — UUID type support
- `bigdecimal` — BigDecimal support
- `rust_decimal` — Rust Decimal support
- `ipnet` — IP network types
- `bit-vec` — Bit vector support
- `mac_address` — MAC address types

### Other Features
- `tracing` — Automatic query instrumentation
- `tls-native` — Native TLS support
- `tls-rustls` — Rustls TLS support

### Example Usage
```toml
# For SQLite with JSON
[dependencies]
sqlx-data = { version = "0.1.3", features = ["sqlite", "json"] }
sqlx = { version = "0.8", features = ["sqlite", "runtime-tokio", "macros", "migrate"] }

# For PostgreSQL with multiple types
[dependencies]
sqlx-data = { version = "0.1.3", features = ["postgres", "json", "chrono", "uuid"] }
sqlx = { version = "0.8", features = ["postgres", "runtime-tokio", "macros", "migrate"] }

# For MySQL with tracing
[dependencies]
sqlx-data = { version = "0.1.3", features = ["mysql", "json", "tracing"] }
sqlx = { version = "0.8", features = ["mysql", "runtime-tokio", "macros", "migrate"] }
```

---

## Quick Start

```toml
[dependencies]
sqlx-data = { version = "0.1.3", features = ["sqlite","json"] }
sqlx = { version = "0.8", features = ["sqlite", "runtime-tokio"] }
tokio = { version = "1", features = ["full"] }
```

```rust
use sqlx_data::{repo, dml, Pool, Result, QueryResult};

#[derive(sqlx::FromRow)]
struct User {
    id: i64,
    name: String,
    email: String,
}

#[repo]
trait UserRepo {
    #[dml("SELECT * FROM users WHERE id = ?")]
    async fn find_by_id(&self, id: i64) -> Result<User>;
    
    #[dml("SELECT * FROM users WHERE age >= ?")]
    async fn find_adults(&self, min_age: u8) -> Result<Vec<User>>;
    
    #[dml("INSERT INTO users (name, email) VALUES (?, ?)")]
    async fn create(&self, name: String, email: String) -> Result<QueryResult>;
}

struct App { pool: Pool }

impl UserRepo for App {
    fn get_pool(&self) -> &Pool { &self.pool }
}

#[tokio::main]
async fn main() -> Result<()> {
    let pool = Pool::connect("mysql://...").await?;
    let app = App { pool };
    
    let user = app.find_by_id(1).await?;
    println!("{}", user.name);
    
    Ok(())
}
```

---

## Under the Hood

When you define a repository trait with `#[repo]` and `#[dml]`, the macros generate additional `_query` methods that contain the actual SQL execution logic. Your original methods become default implementations that call these generated methods, giving you the flexibility to override them with custom logic.

```rust
// What you write:
#[repo]
trait UserRepo {
    #[dml("SELECT * FROM users WHERE id = ?")]
    async fn find_by_id(&self, id: i64) -> Result<User>;

    #[dml("SELECT * FROM users WHERE age >= ?")]
    async fn find_adults(&self, min_age: u8) -> Result<Vec<User>>;
}

// What gets generated (simplified):
trait UserRepo {
    // Generated _query methods with actual SQL execution
    async fn find_by_id_query(&self, id: i64) -> Result<User> {
        sqlx::query_as!(User, "SELECT * FROM users WHERE id = ?", id)
            .fetch_one(self.get_pool())
            .await
    }

    async fn find_adults_query(&self, min_age: u8) -> Result<Vec<User>> {
        sqlx::query_as!(User, "SELECT * FROM users WHERE age >= ?", min_age)
            .fetch_all(self.get_pool())
            .await
    }

    // Original methods become default implementations
    async fn find_by_id(&self, id: i64) -> Result<User> {
        self.find_by_id_query(id).await
    }

    async fn find_adults(&self, min_age: u8) -> Result<Vec<User>> {
        self.find_adults_query(min_age).await
    }

    // Must be implemented by user
    fn get_pool(&self) -> &Pool;
}
```

This design allows you to:
- **Use generated methods directly** by just implementing `get_pool()`
- **Override specific methods** with custom logic while still calling `_query` methods
- **Reuse `_query` methods** in different contexts or custom implementations

---

## Pagination & Dynamic Queries

Handle complex pagination scenarios with the fluent `ParamsBuilder` API.

### 1. Zero-Boilerplate Filters
Combine pagination, filtering, and sorting in a single object:

```rust
use sqlx_data::{Serial, IntoParams, ParamsBuilder, FilterValue};

#[repo]
trait UserRepo {
    // One argument handles everything: defaults + client overrides
    #[dml("SELECT * FROM users")]
    async fn find_users(&self, params: impl IntoParams) -> Result<Serial<User>>;
}

// Client usage:
let params = ParamsBuilder::new()
    .serial()
        .page(1, 20)      // Page 1, 20 items per page
        .done()
    .filter()
        .gt("age", 18)    // WHERE age > 18
        .like("name", "%Alice%") // AND name LIKE '%Alice%'
        .done()
    .sort()
        .desc("id")       // ORDER BY id DESC
        .asc("name")      // THEN BY name ASC
        .done()
    .build();

let result = repo.find_users(params).await?;
```

### 2. Cursor Pagination (Infinite Scroll)
Best for high-performance feeds. Supports `after`, `before` based on specific fields.

```rust
use sqlx_data::{Cursor, ParamsBuilder};

#[repo]
trait FeedRepo {
    // Automatically handles `before`/`after` cursors based on sorted fields
    #[dml("SELECT * FROM posts")]
    async fn user_feed(&self, params: impl IntoParams) -> Result<Cursor<Post>>;
}

// Initial Request:
let params = ParamsBuilder::new()
    .cursor()
        .limit(10)
        .after("id", 0) // Start from beginning
        .done()
    .sort()
        .desc("id")     // Critical: Cursor relies on stable sorting
        .done()
    .build();

let page = repo.user_feed(params).await?;

// Next Page:
if let Some(next_cursor) = page.next_cursor {
    let next_params = ParamsBuilder::new()
        .cursor()
            .limit(10)
            .next_cursor::<Post>(&next_cursor) // Type-safe continuation
            .done()
        .sort()
            .desc("id")
            .done()
        .build();
        
    let next_page = repo.user_feed(next_params).await?;
}
```

### 3. Dynamic Search
Built-in text search construction:

```rust
let params = ParamsBuilder::new()
    .slice()
        .page(1, 50)
        .done()
    .search()
        .query("alice")        // Search term
        .fields(["username", "email"]) // Columns to search
        .case_sensitive(false)
        .done()
    .build();

// Generates:
// WHERE ... AND (username LIKE '%alice%' OR email LIKE '%alice%')
```

---

## Parameter Naming

Clean, readable queries with named parameters using `@parameter_name` syntax. Parameters can be defined in any order and reused multiple times.

### Basic Named Parameters
```rust
#[repo]
trait UserRepo {
    // Single named parameter
    #[dml("SELECT * FROM users WHERE name = @name")]
    async fn find_by_name(&self, name: String) -> Result<Vec<User>>;

    // Multiple named parameters in any order
    #[dml("SELECT * FROM users WHERE age > @min_age AND name LIKE @pattern")]
    async fn find_by_age_and_name(&self, pattern: String, min_age: u8) -> Result<Vec<User>>;
}
```

### Parameter Reuse
Same parameter can be used multiple times in a single query(only for postgres and sqlite):
```rust
#[repo]
trait UserRepo {
    // @search_term used twice
    #[dml("SELECT * FROM users WHERE (name = @search_term OR email = @search_term) AND age > @min_age")]
    async fn search_user(&self, search_term: String, min_age: u8) -> Result<Vec<User>>;
}
```

### Mixed with Positional
Named parameters work alongside traditional positional parameters:
```rust
#[repo]
trait UserRepo {
    // Mix named and positional
    #[dml("SELECT * FROM users WHERE name = @name AND id > $1")]
    async fn find_recent_by_name(&self, min_id: i64, name: String) -> Result<Vec<User>>;
}
```

### Complex Queries
Named parameters shine in complex queries with many conditions:
```rust
#[repo]
trait UserRepo {
    #[dml("
        SELECT u.*, COUNT(o.id) as order_count
        FROM users u
        LEFT JOIN orders o ON u.id = o.user_id
        WHERE u.age BETWEEN @min_age AND @max_age
          AND u.created_at >= @start_date
          AND u.status = @status
        GROUP BY u.id
        HAVING COUNT(o.id) >= @min_orders
        ORDER BY u.created_at DESC
    ")]
    async fn find_active_customers(
        &self,
        min_age: u8,
        max_age: u8,
        start_date: chrono::NaiveDateTime,
        status: String,
        min_orders: i32
    ) -> Result<Vec<CustomerStats>>;
}
```

**Benefits:**
- 🔍 **Readable**: Parameter names explain their purpose in the query
- 🔄 **Flexible Order**: Function parameters can be in any order
- ♻️ **Reusable**: Same parameter used multiple times without repetition
- 🔗 **Compatible**: Works with existing positional parameter code
- 🛡️ **Type Safe**: Same compile-time verification as positional parameters

---

## Aliases

Reusable SQL fragments for DRY code:

```rust
```rust
#[repo]
#[alias(user_columns = "id, name, email, age")]
#[alias(user_table = "users")]
#[alias(active_filter = "WHERE age >= 18")]
trait UserRepo {
    #[dml("SELECT {{user_columns}} FROM {{user_table}} {{active_filter}}")]
    async fn find_adults(&self) -> Result<Vec<User>>;
    
    #[dml("SELECT COUNT(*) FROM {{user_table}} {{active_filter}}")]
    async fn count_adults(&self) -> Result<i64>;
}
```

---

## Scopes

Automatic query enhancement — define once, apply everywhere:

**Pro tip:** Perfect for Rails-like patterns such as multi-tenancy (`tenant_id = ?`), soft deletes (`archived_at IS NULL`), and active records (`status = 'active'`).

```rust
```rust
#[repo]
#[alias(min_age = "18")]
#[scope(adults = "age >= {{min_age}}")]
#[scope(named = "name IS NOT NULL")]
#[scope(recent_birth = "birth_year > 2000")]
#[scope(ordered = "age DESC", target = "order_by")]
trait UserRepo {
    // All scopes automatically applied
    #[dml("SELECT * FROM users")]
    async fn find_all(&self) -> Result<Vec<User>>;
    
    // Ignore specific scopes when needed
    #[scope_ignore(ordered)]
    #[dml("SELECT * FROM users ORDER BY name")]
    async fn find_alphabetical(&self) -> Result<Vec<User>>;
}
```

**Generated SQL:**
```sql
-- find_all() becomes:
SELECT * FROM users 
WHERE age >= 18 
  AND name IS NOT NULL 
  AND birth_year > 2000
ORDER BY age DESC

-- find_alphabetical() becomes:
SELECT * FROM users 
WHERE age >= 18 
  AND name IS NOT NULL 
  AND birth_year > 2000
ORDER BY name
```

---

## Batch Operations

Efficient bulk inserts:

```rust
#[repo]
trait UserRepo {
    #[dml("INSERT INTO users (name, email, age) VALUES")]
    async fn insert_batch(&self, rows: Vec<(String, String, u8)>) -> Result<QueryResult>;
}

// Usage
let users = vec![
    ("Alice".into(), "alice@example.com".into(), 30),
    ("Bob".into(), "bob@example.com".into(), 25),
    ("Charlie".into(), "charlie@example.com".into(), 35),
];

repo.insert_batch(users).await?;
```

**Generated Code:**
```rust
// The macro generates this optimized batch insert method:
async fn insert_batch_query(&self, rows: Vec<(String, String, u8)>) -> Result<QueryResult> {
    if rows.is_empty() {
        return Ok(QueryResult::default());
    }

    // Uses SQLx's efficient QueryBuilder with push_values
    let mut qb = sqlx::QueryBuilder::new("INSERT INTO users (name, email, age) ");
    qb.push_values(rows, |mut b, tuple| {
        b.push_bind(tuple.0)    // name
         .push_bind(tuple.1)    // email
         .push_bind(tuple.2);   // age
    });

    qb.build().execute(self.get_pool()).await
}
```

**Performance:** Inserts 1000 rows in ~50ms vs ~2000ms with individual inserts

---

### Streaming
Best for: Large datasets, memory efficiency

Uses SQLx's native [`fetch`](https://docs.rs/sqlx/latest/sqlx/query/struct.QueryAs.html#method.fetch) method for zero-overhead row-by-row processing, keeping memory usage constant regardless of result set size.

```rust
use futures::Stream;

#[repo]
trait UserRepo {
    // Return impl Stream for memory-efficient processing
    #[dml("SELECT * FROM users WHERE age >= 18")]
    fn stream_active(&self) -> impl Stream<Item = Result<User>> + Send;
}

// Usage
let mut stream = repo.stream_active();
while let Some(user) = stream.next().await {
    println!("Processing {}", user?.name);
}
```

---

### JSON Support
Automatic JSON serialization/deserialization:

```rust
use sqlx::types::Json;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct UserProfile {
    email: String,
    age: u32,
    department: String,
}

#[repo]
trait UserRepo {
    // Automatic JSON serialization for parameters using 'json' attribute
    #[dml("INSERT INTO json_users (name, profile_json) VALUES (?, ?)", json)]
    async fn save_profile(&self, name: String, profile: UserProfile) -> Result<QueryResult>;

    // Type-safe JSON retrieval
    #[dml("SELECT id, name, profile_json, preferences FROM json_users WHERE id = ?")]
    async fn find_raw_json(&self, id: i64) -> Result<Option<(i64, String, Json<JsonValue>, Option<JsonValue>)>>;
}
```

---

### Unchecked Queries
Bypass compile-time verification for complex queries or DDL:

```rust
#[repo]
trait AdminRepo {
    // Use 'unchecked' to skip SQL validation
    #[dml("CREATE TABLE IF NOT EXISTS logs (id INT)", unchecked)]
    async fn create_table(&self) -> Result<QueryResult>;

    #[dml("SELECT * FROM users WHERE id = " + "1", unchecked)]
    async fn dynamic_query(&self) -> Result<Vec<User>>;
}
```

---

### Binary Data (BLOBs)
Efficient handling of binary data:

```rust
use bytes::Bytes;

#[repo]
trait FileRepo {
    #[dml("INSERT INTO files (name, content_type, file_size, data, is_compressed) VALUES (?, ?, ?, ?, ?)")]
    async fn upload(
        &self, 
        name: String, 
        content_type: String, 
        file_size: u32, 
        data: bytes::Bytes,
        is_compressed: bool
    ) -> Result<QueryResult>;

    #[dml("SELECT data FROM files WHERE id = ?")]
    async fn download(&self, id: i64) -> Result<Vec<u8>>;
}
```

---

### Generics & Lifetimes
Full support for Rust's type system:

```rust
#[repo]
trait GenericRepo {
    // Support for lifetime parameters
    #[dml("SELECT * FROM users WHERE name = ?")]
    async fn find_by_name<'a>(&self, name: &'a str) -> Result<Option<User>>;

    // Support for custom executors (transactions, connections)
    #[dml("INSERT INTO logs (msg) VALUES (?)")]
    async fn log_with_executor<'e, E>(&self, executor: E, msg: &str) -> Result<QueryResult>
    where
        E: sqlx::Executor<'e, Database = sqlx::MySql>;
}
```

---

### Method Variants

Generate multiple executor variants automatically:

```rust
#[repo]
trait UserRepo {
    #[generate_versions(pool, tx, conn, exec)]
    #[dml("UPDATE users SET name = ? WHERE id = ?")]
    async fn update_name(&self, name: String, id: i64) -> Result<QueryResult>;
}

// Generates 5 methods:
// - update_name(&self, ...)                           // uses get_pool()
// - update_name_with_pool(&self, pool: &Pool, ...)    // explicit pool
// - update_name_with_tx(&self, tx: &mut Transaction, ...)
// - update_name_with_conn(&self, conn: &mut Connection, ...)
// - update_name_with_executor(&self, exec: impl Executor, ...)

// Usage
let mut tx = pool.begin().await?;
repo.update_name_with_tx(&mut tx, "Alice".into(), 1).await?;
repo.update_name_with_tx(&mut tx, "Bob".into(), 2).await?;
tx.commit().await?;
```

---

## Tracing

Built-in observability powered by the [`tracing`](https://crates.io/crates/tracing) library. Zero configuration required to get detailed logs automatically instrumented with [`#[instrument]`](https://docs.rs/tracing/latest/tracing/attr.instrument.html):

```rust
use tracing::instrument;

#[repo]
trait UserRepo {
    #[dml("SELECT * FROM users WHERE id = ?")]
    #[instrument(skip(self))]
    async fn find_by_id(&self, id: i64) -> Result<User>;
}

// Automatically logs:
// - Method entry/exit
// - Parameters (except skipped ones)
// - Execution time
// - Errors
```

---

## Hover to Inspect

See the generated SQL and implementation in your IDE:

![Hover to see generated code](https://github.com/josercarmo/sqlx-data/raw/HEAD/resources/hover_to_inspect.gif)

**Pro tip:** Copy the generated code to override methods or call `_query` methods from custom logic:

```rust
impl UserRepo for App {
    fn get_pool(&self) -> &Pool { &self.pool }

    // Override generated method with custom logic
    async fn find_by_id(&self, id: i64) -> Result<User> {
        // Add logging, caching, validation, etc.
        log::info!("Finding user with id: {}", id);
        self.find_by_id_query(id).await
    }

    // Use _query method in custom implementations
    async fn find_user_with_cache(&self, id: i64) -> Result<User> {
        if let Some(cached) = get_from_cache(id) {
            return Ok(cached);
        }
        let user = self.find_by_id_query(id).await?;
        cache_user(&user);
        Ok(user)
    }
}
```

---

### Complex Queries

```rust
#[repo]
trait UserRepo {
    #[dml("
        SELECT 
            u.id,
            u.name,
            COUNT(o.id) as order_count,
            SUM(o.total) as total_spent
        FROM users u
        LEFT JOIN orders o ON u.id = o.user_id
        WHERE u.age >= ?
        GROUP BY u.id, u.name
        HAVING COUNT(o.id) > ?
        ORDER BY total_spent DESC
    ")]
    async fn find_top_customers(
        &self, 
        min_age: u8, 
        min_orders: i32
    ) -> Result<Vec<CustomerStats>>;
}
```

### File-based Queries

```rust
#[repo]
trait UserRepo {
    #[dml(file = "queries/complex_user_report.sql")]
    async fn generate_report(&self) -> Result<Vec<ReportRow>>;
}
```



---

## Supported Return Types

| Return Type | Example | Fetch Strategy |
|------------|---------|----------------|
| `T` | `User` | `fetch_one` |
| `Option<T>` | `Option<User>` | `fetch_optional` |
| `Vec<T>` | `Vec<User>` | `fetch_all` |
| Scalar | `i64`, `String`, `bool` | `fetch_one` |
| Tuple | `(String, i64)` | `fetch_one` |
| `Vec<Tuple>` | `Vec<(String, i64)>` | `fetch_all` |
| `Serial<T>` | `Serial<User>` | Paginated |
| `Slice<T>` | `Slice<User>` | Paginated |
| `Cursor<T>` | `Cursor<User>` | Paginated |
| Database-specific | `MySqlQueryResult`, `PgQueryResult` | `execute` |

---

## Database Support

| Database | Placeholder | Example |
|----------|-------------|---------|
| **MySQL** | `?` | `WHERE id = ?` |
| **PostgreSQL** | `$1`, `$2` | `WHERE id = $1` |
| **SQLite** | `$1`, `$2` | `WHERE id = $1` |

---

## Performance

- **Compile-time overhead:** ~20-90µs per query (macro expansion)
- **Runtime overhead:** Zero — generates the same code you'd write manually
- **Batch inserts:** 40x faster than individual inserts (1000 rows: 50ms vs 2000ms)

---

## Examples

- [`hello-world`](examples/hello-world) — A minimal setup to get started.
- [`axum-crud-api`](examples/axum-crud-api) — A full-featured REST API using Axum, showcasing pagination, filtering, and best practices.

---

## Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](CONTRIBUTING.md).

---

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.

---

## Acknowledgments

Built on top of the excellent [SQLx](https://github.com/launchbadge/sqlx) library.
Powered by [syn](https://github.com/dtolnay/syn), [quote](https://github.com/dtolnay/quote), and [proc-macro2](https://github.com/dtolnay/proc-macro2) for macro expansion.
SQL parsing and validation leverage [sqlparser](https://github.com/sqlparser-rs/sqlparser-rs).

