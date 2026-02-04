# sqlx-data-macros

Procedural macros for [sqlx-data](https://crates.io/crates/sqlx-data). This crate provides the core `#[repo]` and `#[dml]` derive macros that enable automatic SQL generation and repository pattern implementation.

## Macros

- `#[repo]` - Transform traits into repository implementations
- `#[dml]` - Generate SQL query methods with compile-time validation

## Usage

This crate is typically used through the main `sqlx-data` crate:

```rust
use sqlx_data::{repo, dml, Result};

#[repo]
trait UserRepo {
    #[dml("SELECT * FROM users WHERE id = ?")]
    async fn find_by_id(&self, id: i64) -> Result<User>;
}
```

## Features

- Compile-time SQL validation
- Automatic parameter binding
- Type-safe query generation
- Repository pattern implementation

For complete documentation, see the [sqlx-data documentation](https://docs.rs/sqlx-data).