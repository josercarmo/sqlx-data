//! Error types for dynamic SQL functionality.
//! Compiled only when a database backend feature is enabled.

#![cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]

use crate::core::ParserError;

// Re-export sqlx::Error as SqlxError for convenience
pub use sqlx_data_params::SqlxError;

// Result type using SqlxError
pub type Result<T> = ::std::result::Result<T, SqlxError>;

impl From<ParserError> for SqlxError {
    fn from(err: ParserError) -> Self {
        match err {
            ParserError::ParseSql(msg) => SqlxError::protocol(msg),
            ParserError::Validation(msg) => SqlxError::protocol(msg),
            ParserError::InvalidArgument(msg) => SqlxError::protocol(msg),
        }
    }
}
