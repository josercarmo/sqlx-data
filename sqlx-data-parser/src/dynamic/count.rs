//! Count query building functionality.
//! Compiled only when a database backend feature is enabled.

#![cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]

use crate::constants::cache::{COUNT_SQL_CACHE, SQL_PARSER_CACHE};
use crate::core::{ParserError, generate_count_query, parse_sql};
use std::sync::Arc;

pub type Result<T> = ::std::result::Result<T, ParserError>;

/// Build a count query from SQL using cache for performance
pub fn build_count_query_from_sql(sql: &str) -> Result<Arc<String>> {
    let key = SQL_PARSER_CACHE.fingerprint(sql);

    // First check if we already have the count SQL cached
    if let Some(cached_count_sql) = COUNT_SQL_CACHE.get(key) {
        return Ok(Arc::clone(&cached_count_sql));
    }

    // Check AST cache or parse SQL
    let statement = if let Some(cached_ast) = SQL_PARSER_CACHE.get(key) {
        cached_ast
    } else {
        // Parse SQL and cache AST
        let statement_option = parse_sql(sql)?;
        statement_option.ok_or(ParserError::parse_sql("No valid SQL statement found"))?
    };

    // Generate count SQL
    let count_sql = generate_count_query(&statement, None);

    // Cache the count SQL for future use
    let arc_count_sql = COUNT_SQL_CACHE.insert(key, count_sql);
    Ok(Arc::clone(&arc_count_sql))
}
