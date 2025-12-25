# Chapter 1: What is SQLx-Data?

SQLx-Data is an advanced SQLx companion that implements the Repository Pattern with compile-time type safety. It eliminates the boilerplate code typically associated with database operations while providing sophisticated features like pagination, streaming, and query composition.

## The Problem Space

Modern Rust database development faces several challenges:

### 1. Boilerplate Repetition
```rust
// Traditional approach - lots of repetition
impl UserRepository {
    async fn find_by_id(&self, id: i64) -> Result<User> {
        sqlx::query_as!(User, "SELECT * FROM users WHERE id = ?", id)
            .fetch_one(&self.pool)
            .await
    }

    async fn find_by_email(&self, email: &str) -> Result<Option<User>> {
        sqlx::query_as!(User, "SELECT * FROM users WHERE email = ?", email)
            .fetch_optional(&self.pool)
            .await
    }

    async fn find_all(&self) -> Result<Vec<User>> {
        sqlx::query_as!(User, "SELECT * FROM users")
            .fetch_all(&self.pool)
            .await
    }

    // ... dozens more methods with similar patterns
}
```

### 2. Parameter Binding Complexity
```rust
// Manual parameter binding becomes unwieldy
async fn complex_search(
    &self,
    name: Option<&str>,
    min_age: Option<u8>,
    department: Option<&str>,
    limit: u32,
    offset: u32
) -> Result<Vec<User>> {
    let mut query = "SELECT * FROM users WHERE 1=1".to_string();
    let mut params: Vec<Box<dyn std::any::Any>> = vec![];

    if let Some(n) = name {
        query.push_str(" AND name LIKE ?");
        params.push(Box::new(format!("%{}%", n)));
    }

    if let Some(age) = min_age {
        query.push_str(" AND age >= ?");
        params.push(Box::new(age));
    }

    // ... more complexity
}
```

### 3. Pagination Implementation
```rust
// Every pagination implementation is custom and error-prone
struct PaginationResult<T> {
    items: Vec<T>,
    total: u64,
    page: u32,
    per_page: u32,
    total_pages: u32,
    has_next: bool,
}

impl UserRepository {
    async fn paginated_users(&self, page: u32, per_page: u32) -> Result<PaginationResult<User>> {
        // Count query
        let total: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM users"
        ).fetch_one(&self.pool).await?;

        // Data query
        let offset = page * per_page;
        let users = sqlx::query_as!(
            User,
            "SELECT * FROM users LIMIT ? OFFSET ?",
            per_page,
            offset
        ).fetch_all(&self.pool).await?;

        // Manual calculation of pagination metadata
        let total_pages = (total.0 as u32 + per_page - 1) / per_page;
        let has_next = page + 1 < total_pages;

        Ok(PaginationResult {
            items: users,
            total: total.0 as u64,
            page,
            per_page,
            total_pages,
            has_next,
        })
    }
}
```

## The SQLx-Data Solution

SQLx-Data addresses these problems with a trait-based approach that generates implementations automatically:

### 1. Zero Boilerplate
```rust
#[repo]
trait UserRepo {
    #[dml("SELECT * FROM users WHERE id = ?")]
    async fn find_by_id(&self, id: i64) -> Result<User>;

    #[dml("SELECT * FROM users WHERE email = ?")]
    async fn find_by_email(&self, email: String) -> Result<Option<User>>;

    #[dml("SELECT * FROM users")]
    async fn find_all(&self) -> Result<Vec<User>>;
}
```

### 2. Automatic Parameter Binding
```rust
#[repo]
trait UserRepo {
    // Named parameters - cleaner and reusable
    #[dml("SELECT * FROM users WHERE (name LIKE @search OR email LIKE @search) AND age >= @min_age")]
    async fn search_users(&self, search: String, min_age: u8) -> Result<Vec<User>>;
}
```

### 3. Built-in Pagination
```rust
#[repo]
trait UserRepo {
    // Serial pagination with automatic count queries and metadata
    #[dml("SELECT * FROM users")]
    async fn paginated_users(&self, params: impl IntoParams) -> Result<Serial<User>>;

    // Cursor pagination for high-performance feeds
    #[dml("SELECT * FROM users ORDER BY id")]
    async fn cursor_users(&self, params: impl IntoParams) -> Result<Cursor<User>>;
}

// Usage
let params = ParamsBuilder::new()
    .serial()
        .page(1, 20)
        .done()
    .build();

let result = repo.paginated_users(params).await?;
// result.total_items, result.total_pages, result.has_next all computed automatically
```

## Core Philosophy

### 1. Build on Existing Foundations
SQLx-Data doesn't replace SQLx – it enhances it. Every generated query uses SQLx's proven macros (`query_as!`, `query!`, `query_scalar!`) ensuring:
- Compile-time SQL validation
- Type-safe result mapping
- Database-specific optimizations
- Zero runtime overhead

### 2. Trait-Based Design
The Repository Pattern through traits provides:
- **Clean abstraction boundaries** between business logic and data access
- **Easy testing** with mock implementations
- **Flexible execution contexts** (pools, transactions, connections)
- **Composition-friendly** interfaces

### 3. Progressive Enhancement
Start simple and add complexity as needed:

```rust
// Level 1: Basic CRUD
#[repo]
trait UserRepo {
    #[dml("SELECT * FROM users WHERE id = ?")]
    async fn find_by_id(&self, id: i64) -> Result<User>;
}

// Level 2: Add pagination
#[repo]
trait UserRepo {
    #[dml("SELECT * FROM users WHERE id = ?")]
    async fn find_by_id(&self, id: i64) -> Result<User>;

    #[dml("SELECT * FROM users")]
    async fn find_all(&self, params: impl IntoParams) -> Result<Serial<User>>;
}

// Level 3: Add composition with aliases and scopes
#[repo]
#[alias(active_filter = "WHERE deleted_at IS NULL")]
#[scope(tenant = "tenant_id = {{tenant_id}}")]
trait UserRepo {
    #[dml("SELECT * FROM users {{active_filter}} WHERE id = ?")]
    async fn find_by_id(&self, id: i64) -> Result<User>;

    #[dml("SELECT * FROM users {{active_filter}}")]
    async fn find_all(&self, params: impl IntoParams) -> Result<Serial<User>>;
}
```

## What SQLx-Data Is NOT

It's important to understand what SQLx-Data doesn't try to be:

### Not a Full ORM
- **No active record pattern** – entities are simple structs
- **No automatic relationship mapping** – relationships are explicit in SQL
- **No query builder DSL** – you write actual SQL

### Not a Database Abstraction Layer
- **Database-specific features** are embraced, not hidden
- **SQL dialects** are preserved and encouraged
- **Performance characteristics** of each database are respected

### Not a Migration Tool
- **Schema management** remains with dedicated tools
- **Database structure** is assumed to exist
- **DDL operations** are outside the scope (though supported via unchecked queries)

## When to Choose SQLx-Data

SQLx-Data is ideal when you want:

✅ **Compile-time safety** without runtime overhead
✅ **Clean repository interfaces** without implementation boilerplate
✅ **Advanced features** like pagination and streaming out-of-the-box
✅ **Full SQL control** with enhanced developer experience
✅ **Production-ready patterns** that scale from prototype to enterprise

Consider alternatives if you need:

❌ **Active record pattern** with rich entity models
❌ **Automatic relationship management** and eager loading
❌ **Query builder DSL** instead of raw SQL
❌ **Cross-database portability** at the expense of performance

## Next Steps

Now that you understand what SQLx-Data is and the problems it solves, let's get you set up with a working example. In the next chapter, we'll walk through the Quick Start guide to get you building type-safe repositories in minutes.