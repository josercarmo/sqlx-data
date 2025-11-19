//! Dynamic SQL building functionality.
//! Compiled only when a database backend feature is enabled.

#![cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]

pub mod builder;
pub mod count;
pub mod error;
pub mod validation;

// Re-exports for a clean public API
pub use builder::{BuiltSql, build_dynamic_sql};
pub use count::build_count_query_from_sql;
pub use error::{Result, SqlxError};
pub use validation::validate_fields;
