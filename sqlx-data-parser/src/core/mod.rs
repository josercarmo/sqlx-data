//! Core SQL parsing functionality - always available, no database features required

pub mod error;
pub mod parser;
pub mod parser_count;
pub mod parser_insert;
pub mod util;

// Re-exports for clean API
pub use error::ParserError;
pub use parser::{SqlStatementType, infer_columns_from_stmt, parse_sql};
pub use parser_count::generate_count_query;
pub use parser_insert::{
    extract_insert_base_from_statement, extract_on_conflict_clause_from_statement,
    extract_returning_clause_from_statement, extract_values_clause_from_statement,
    has_complex_sql_functions_in_values, infer_insert_columns_from_stmt,
};
pub use util::placeholder_prefix as PLACEHOLDER;
