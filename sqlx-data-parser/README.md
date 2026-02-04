# sqlx-data-parser

SQL parsing and dynamic query building for [sqlx-data](https://crates.io/crates/sqlx-data). This crate handles AST manipulation, query transformation, pagination injection, and compile-time SQL validation.

## Features

- **SQL Parsing** - Parse and analyze SQL queries
- **Query Transformation** - Modify queries for pagination and filtering
- **AST Manipulation** - Work with SQL Abstract Syntax Trees
- **Validation** - Compile-time SQL validation and error reporting

## Usage

This crate is primarily used internally by sqlx-data macros for:

- Parsing SQL in `#[dml]` attributes
- Injecting pagination clauses
- Validating query syntax
- Transforming queries for different database engines

## Database Support

- PostgreSQL
- MySQL
- SQLite

For complete documentation, see the [sqlx-data documentation](https://docs.rs/sqlx-data).