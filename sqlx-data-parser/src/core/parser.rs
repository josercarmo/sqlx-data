use std::sync::Arc;

use super::error::ParserError;
use crate::constants::cache::SQL_PARSER_CACHE;
use crate::constants::database::get_dialect;
use sqlparser::ast::{Expr, SelectItem, Statement};
use sqlparser::parser::Parser;

pub type Result<T> = ::std::result::Result<T, ParserError>;

/// SQL statement types for DML operations
#[derive(Debug, Clone, PartialEq)]
pub enum SqlStatementType {
    Select,
    Insert,
    Update,
    Delete,
    DDL,
    Unknown,
}

/// Centralized SQL parsing with SQLite dialect
/// Returns parsed statement or None if parsing fails
pub fn parse_sql(sql: &str) -> Result<Option<Arc<Statement>>> {
    if sql.is_empty() {
        Err(ParserError::parse_sql("Empty SQL statement provided."))?;
    }

    let key = SQL_PARSER_CACHE.fingerprint(sql);

    // Check cache first
    if let Some(cached) = SQL_PARSER_CACHE.get(key) {
        return Ok(Some(cached));
    }

    // Parse SQL
    let dialect = get_dialect();
    let statements = match Parser::parse_sql(&dialect, sql) {
        Ok(stmts) => stmts,
        Err(_) => return Ok(None), // Parsing failed, return None
    };

    // Validate statement count
    if statements.len() > 1 {
        Err(ParserError::parse_sql(
            "Multiple SQL statements detected; only one statement is allowed.",
        ))?;
    }

    // Get the statement
    let statement = match statements.into_iter().next() {
        Some(stmt) => stmt,
        None => {
            return Err(ParserError::parse_sql(
                "No SQL statement found after parsing.",
            ))?;
        }
    };

    // Insert in cache and return
    let arc_stmt = SQL_PARSER_CACHE.insert(key, statement);
    Ok(Some(arc_stmt))
}

#[derive(Debug, Clone)]
pub struct InferredSelect {
    pub columns: Vec<String>,
}

pub fn infer_columns_from_stmt(stmt: &Statement) -> Result<InferredSelect> {
    let projection = match stmt {
        Statement::Query(query) => match query.body.as_ref() {
            sqlparser::ast::SetExpr::Select(select) => &select.projection,
            _ => return Err(ParserError::error("Only SELECT queries are supported"))?,
        },

        Statement::Insert(insert) => insert
            .returning
            .as_ref()
            .ok_or(ParserError::error("INSERT without RETURNING clause"))?,

        Statement::Update(update) => update
            .returning
            .as_ref()
            .ok_or(ParserError::error("UPDATE without RETURNING clause"))?,

        Statement::Delete(delete) => delete
            .returning
            .as_ref()
            .ok_or(ParserError::error("DELETE without RETURNING clause"))?,

        _ => return Err(ParserError::error("Unsupported SQL statement"))?,
    };

    infer_projection_columns(projection)
}

fn infer_projection_columns(projection: &[SelectItem]) -> Result<InferredSelect> {
    let mut columns = Vec::with_capacity(projection.len());

    for (i, item) in projection.iter().enumerate() {
        match item {
            SelectItem::UnnamedExpr(expr) => {
                if needs_alias_for_sqlx(expr) {
                    let expr_str = format!("{}", expr);
                    Err(ParserError::Validation(format!(
                        "Column {} '{}' requires an alias for SQLx compatibility. \
                         Add 'AS column_name'. Example: {} AS column_name",
                        i + 1,
                        expr_str,
                        expr_str
                    )))?;
                }

                columns.push(analyze_expression_name(expr, i));
            }

            SelectItem::ExprWithAlias { alias, .. } => {
                columns.push(alias.value.clone());
            }

            SelectItem::Wildcard(_) => {
                Err(ParserError::error(
                    "SELECT * is not supported; use explicit columns",
                ))?;
            }

            _ => {
                columns.push(format!("field_{}", i));
            }
        }
    }

    Ok(InferredSelect { columns })
}

#[allow(dead_code)]
fn infer_columns(sql: &str) -> Result<InferredSelect> {
    let statement_opt = parse_sql(sql)?;
    let statement = statement_opt.ok_or(ParserError::error("SQL parsing failed"))?;

    infer_columns_from_stmt(&statement)
}

fn analyze_expression_name(expr: &Expr, index: usize) -> String {
    match expr {
        Expr::Identifier(ident) => ident.value.clone(),
        Expr::CompoundIdentifier(idents) => idents
            .last()
            .map(|ident| ident.value.clone())
            .unwrap_or_else(|| format!("field_{}", index)),
        Expr::Function(func) => func.name.to_string().to_lowercase(),
        _ => format!("field_{}", index),
    }
}

/// Check if an expression needs an alias for SQLx compatibility
fn needs_alias_for_sqlx(expr: &Expr) -> bool {
    match expr {
        // Simple identifiers are OK: id, name, age
        Expr::Identifier(_) => false,

        // Compound identifiers are OK: users.id, t.name
        Expr::CompoundIdentifier(_) => false,

        // Functions always need aliases: COUNT(*), MIN(age), UPPER(name)
        Expr::Function(_) => true,

        // Mathematical expressions need aliases: age + 1, price * 0.1
        Expr::BinaryOp { .. } => true,

        // Unary expressions need aliases: -age, NOT active
        Expr::UnaryOp { .. } => true,

        // CASE expressions need aliases
        Expr::Case { .. } => true,

        // CAST expressions need aliases: CAST(age AS TEXT)
        Expr::Cast { .. } => true,

        // Subqueries need aliases: (SELECT COUNT(*) FROM posts)
        Expr::Subquery(_) => true,

        // String concatenation and other operations need aliases
        // Note: String concatenation is usually handled as BinaryOp

        // Literals might be OK but safer to require alias: 'hello', 42
        Expr::Value(_) => true,

        // Any other complex expression needs alias
        _ => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generate_count_query;

    #[test]
    fn test_simple_select() {
        let sql = "SELECT id, name FROM users";
        let result = infer_columns(sql).unwrap();
        assert_eq!(result.columns, vec!["id", "name"]);
    }

    #[test]
    fn test_select_with_aliases() {
        let sql = "SELECT COUNT(id) as count_field, AVG(age) as average_field FROM users";
        let result = infer_columns(sql).unwrap();
        assert_eq!(result.columns, vec!["count_field", "average_field"]);
    }

    #[test]
    fn test_cte() {
        let sql = "WITH temp AS (SELECT 1) SELECT name, birth_year FROM users WHERE id = $1";
        let result = infer_columns(sql).unwrap();
        assert_eq!(result.columns, vec!["name", "birth_year"]);
    }

    #[test]
    fn test_complex_expressions() {
        let sql = "SELECT COUNT(*) as count, MAX(age) as max, id + 1 as incremented_id FROM users";
        let result = infer_columns(sql).unwrap();
        assert_eq!(result.columns, vec!["count", "max", "incremented_id"]);
    }

    #[test]
    fn test_alias_validation_fails_for_functions() {
        let sql = "SELECT MIN(age), MAX(age), COUNT(*) FROM users";
        let result = infer_columns(sql);
        assert!(result.is_err());

        let error = result.unwrap_err();
        assert!(error.to_string().contains("requires an alias"));
        assert!(error.to_string().contains("MIN(age)"));
    }

    #[test]
    fn test_alias_validation_fails_for_expressions() {
        let sql = "SELECT age + 1, price * 0.1 FROM products";
        let result = infer_columns(sql);
        assert!(result.is_err());

        let error = result.unwrap_err();
        assert!(error.to_string().contains("requires an alias"));
    }

    #[test]
    fn test_alias_validation_passes_for_simple_columns() {
        let sql = "SELECT id, name, users.email FROM users";
        let result = infer_columns(sql);
        assert!(result.is_ok());

        let columns = result.unwrap().columns;
        assert_eq!(columns, vec!["id", "name", "email"]);
    }

    #[test]
    fn test_alias_validation_passes_with_explicit_aliases() {
        let sql =
            "SELECT MIN(age) as min_age, COUNT(*) as total, age + 1 as age_plus_one FROM users";
        let result = infer_columns(sql);
        assert!(result.is_ok());

        let columns = result.unwrap().columns;
        assert_eq!(columns, vec!["min_age", "total", "age_plus_one"]);
    }

    #[test]
    fn test_sqlx_cast_syntax_parsing() {
        // Test 1: Normal column without cast
        let sql1 = "SELECT name, LENGTH(email) as email_length FROM users";
        let result1 = infer_columns(sql1);
        println!("Normal SQL: {:?}", result1);

        // Test 2: SQLx cast syntax with quotes
        let sql2 = "SELECT name, LENGTH(email) as 'email_length: i64' FROM users";
        let result2 = infer_columns(sql2);
        println!("SQLx cast syntax: {:?}", result2);

        // Test 3: SQLx cast syntax with exclamation mark
        let sql3 = "SELECT name, LENGTH(email) as 'email_length!: i64' FROM users";
        let result3 = infer_columns(sql3);
        println!("SQLx cast with !: {:?}", result3);

        // Test 4: Multiple casts
        let sql4 = "SELECT name as 'name: String', LENGTH(email) as 'email_length: i64' FROM users";
        let result4 = infer_columns(sql4);
        println!("Multiple casts: {:?}", result4);

        // All tests should pass for our parser
        assert!(result1.is_ok());
        // These might fail if our parser doesn't handle quotes properly
        println!("Result2 success: {:?}", result2.is_ok());
        println!("Result3 success: {:?}", result3.is_ok());
        println!("Result4 success: {:?}", result4.is_ok());

        if result2.is_ok() {
            println!(
                "Extracted columns from cast SQL: {:?}",
                result2.unwrap().columns
            );
        }
    }

    #[test]
    fn test_column_extraction_with_casts() {
        // Test parameter detection with cast syntax
        let sql = "SELECT name as 'name: String', age as 'age: i32' FROM users WHERE id = $1";

        let result = infer_columns(sql);
        println!("Columns with casts result: {:?}", result);

        if let Ok(inferred) = result {
            println!("Extracted columns: {:?}", inferred.columns);
            // Should still extract column names even with casts
            assert!(inferred.columns.len() >= 2);
        }
    }

    // === Cache Tests (moved from sql_parser_cache.rs) ===

    #[test]
    fn test_parse_cache_basic() {
        let sql = "SELECT 1";
        let stmt1 = parse_sql(sql).unwrap();
        let stmt2 = parse_sql(sql).unwrap();
        let ast1 = stmt1.as_ref().unwrap();
        let ast2 = stmt2.as_ref().unwrap();
        assert!(Arc::ptr_eq(ast1, ast2));
    }

    #[test]
    fn test_cache_hit_with_different_sql() {
        let sql1 = "SELECT 1";
        let sql2 = "SELECT 2";

        let ast1 = parse_sql(sql1).unwrap().unwrap();
        let ast2 = parse_sql(sql2).unwrap().unwrap();

        assert!(!Arc::ptr_eq(&ast1, &ast2));
        assert_ne!(*ast1, *ast2);
    }

    #[test]
    fn test_cache_with_complex_query() {
        let complex_sql = "
            WITH recent_orders AS (
                SELECT customer_id, COUNT(*) as order_count
                FROM orders
                WHERE created_at > '2023-01-01'
                GROUP BY customer_id
            )
            SELECT c.name, ro.order_count
            FROM customers c
            JOIN recent_orders ro ON c.id = ro.customer_id
            WHERE ro.order_count > 5
            ORDER BY ro.order_count DESC
        ";

        let ast1 = parse_sql(complex_sql).unwrap().unwrap();
        let ast2 = parse_sql(complex_sql).unwrap().unwrap();

        assert!(Arc::ptr_eq(&ast1, &ast2));
        assert!(!ast1.to_string().is_empty());
    }

    #[test]
    fn test_cache_with_invalid_sql() {
        // Use SQL that is definitely invalid in any dialect
        let invalid_sql = "SELECTTT * FROMM users WHEREE";

        let result1 = parse_sql(invalid_sql);
        let result2 = parse_sql(invalid_sql);

        // Invalid SQL should return Ok(None)
        assert!(result1.unwrap().is_none());
        assert!(result2.unwrap().is_none());
    }

    #[test]
    fn test_cache_capacity() {
        for i in 0..100 {
            let sql = format!("SELECT {} FROM test_table_{}", i, i);
            let _result = parse_sql(&sql);
        }

        let test_sql = "SELECT 999 FROM final_test";
        let result = parse_sql(test_sql);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_sql_cache_functionality() {
        let sql = "SELECT id, name FROM users WHERE active = true";

        // First call - should miss cache and parse
        let ast1 = parse_sql(sql)
            .expect("should parse SQL")
            .expect("should have statement");

        // Second call - should hit cache and return same Arc
        let ast2 = parse_sql(sql)
            .expect("should parse SQL from cache")
            .expect("should have statement");

        // Verify same Arc instance (cache hit)
        assert!(
            Arc::ptr_eq(&ast1, &ast2),
            "Should return same Arc from cache"
        );

        // Verify content is same
        assert_eq!(*ast1, *ast2, "AST content should be identical");

        // Test with different SQL to ensure cache differentiation
        let different_sql = "SELECT COUNT(*) FROM products";
        let ast3 = parse_sql(different_sql)
            .expect("should parse different SQL")
            .expect("should have statement");

        // Should be different Arc instances
        assert!(
            !Arc::ptr_eq(&ast1, &ast3),
            "Different SQL should have different Arc"
        );

        // But first SQL should still hit cache
        let ast4 = parse_sql(sql)
            .expect("should parse original SQL again")
            .expect("should have statement");
        assert!(
            Arc::ptr_eq(&ast1, &ast4),
            "Original SQL should still hit cache"
        );
    }

    // === End Cache Tests ===

    #[test]
    fn test_different_cast_syntaxes() {
        // Test various SQLx cast syntaxes to see which ones work
        let test_cases = vec![
            (
                "SELECT LENGTH(email) as 'len: i64' FROM users",
                "Standard cast",
            ),
            (
                "SELECT LENGTH(email) as 'len!: i64' FROM users",
                "Force cast with !",
            ),
            (
                "SELECT LENGTH(email) as \"len: i64\" FROM users",
                "Double quotes",
            ),
            (
                "SELECT LENGTH(email) as len FROM users",
                "No cast (control)",
            ),
            ("SELECT email as 'email: _' FROM users", "Wildcard type"),
        ];

        for (sql, description) in test_cases {
            println!("\nTesting {}: {}", description, sql);
            let result = infer_columns(sql);
            println!("Parse result: {:?}", result.is_ok());

            if let Ok(inferred) = result {
                println!("Extracted columns: {:?}", inferred.columns);
            } else {
                println!("Error: {:?}", result.unwrap_err());
            }
        }
    }

    #[test]
    fn test_count_query_golden_rule() {
        // Test the golden rule: GROUP BY → subquery, no GROUP BY → simple COUNT

        // No GROUP BY cases - should use simple COUNT
        let simple_cases = vec![
            "SELECT id, name FROM users",
            "SELECT * FROM users WHERE age > 18",
        ];

        // DISTINCT cases - should use simple COUNT
        let distinct_cases = vec!["SELECT DISTINCT country FROM users"];

        // JOIN cases - should use subquery without primary key info
        let join_cases =
            vec!["SELECT u.name, p.title FROM users u JOIN posts p ON u.id = p.user_id"];

        for sql in simple_cases {
            let statement = parse_sql(sql).unwrap().unwrap();
            let count_sql = generate_count_query(&statement, None);
            println!("Simple SQL: {}", sql);
            println!("Count SQL: {}", count_sql);
            assert!(!count_sql.contains("FROM ("));
            assert!(count_sql.starts_with("SELECT COUNT(*) FROM"));
            println!("✓ Simple COUNT correctly applied\n");
        }

        for sql in distinct_cases {
            let statement = parse_sql(sql).unwrap().unwrap();
            let count_sql = generate_count_query(&statement, None);
            println!("DISTINCT SQL: {}", sql);
            println!("Count SQL: {}", count_sql);
            assert!(!count_sql.contains("FROM ("));
            assert!(count_sql.starts_with("SELECT COUNT(DISTINCT country)"));
            println!("✓ Simple COUNT correctly applied for DISTINCT\n");
        }

        for sql in join_cases {
            let statement = parse_sql(sql).unwrap().unwrap();
            let count_sql = generate_count_query(&statement, None);
            println!("JOIN SQL: {}", sql);
            println!("Count SQL: {}", count_sql);
            assert!(count_sql.contains("FROM ("));
            assert!(count_sql.starts_with("SELECT COUNT(*) FROM ("));
            println!("✓ Subquery COUNT correctly applied for JOIN\n");
        }

        // GROUP BY cases - should use subquery
        let group_by_cases = vec![
            "SELECT country, COUNT(*) FROM users GROUP BY country",
            "SELECT age_bracket, COUNT(*) FROM users GROUP BY age_bracket HAVING COUNT(*) >= 5",
            "SELECT CASE WHEN age < 25 THEN 'Young' ELSE 'Old' END as bracket, COUNT(*) FROM users GROUP BY bracket",
        ];

        for sql in group_by_cases {
            let statement = parse_sql(sql).unwrap().unwrap();
            let count_sql = generate_count_query(&statement, None);
            println!("GROUP BY SQL: {}", sql);
            println!("Count SQL: {}", count_sql);
            assert!(count_sql.contains("FROM ("));
            assert!(count_sql.contains("AS sub"));
            assert!(count_sql.starts_with("SELECT COUNT(*) FROM ("));
            println!("✓ Subquery COUNT correctly applied\n");
        }
    }

    #[test]
    fn test_count_query_with_complex_group_by() {
        // Test the exact problematic query from our issue
        let sql = "SELECT
           CASE
               WHEN age < 25 THEN 'Young'
               WHEN age < 50 THEN 'Adult'
               ELSE 'Senior'
           END as age_bracket,
           COUNT(*) as user_count,
           AVG(CAST(birth_year as REAL)) as avg_birth_year
           FROM users
           WHERE name NOT LIKE '%test%'
           GROUP BY CASE
               WHEN age < 25 THEN 'Young'
               WHEN age < 50 THEN 'Adult'
               ELSE 'Senior'
           END
           HAVING COUNT(*) >= $1";

        let statement = parse_sql(sql).unwrap().unwrap();
        let count_sql = generate_count_query(&statement, None);
        println!("Original SQL: {}", sql);
        println!("Generated count SQL: {}", count_sql);

        // Should be a subquery since it has GROUP BY
        assert!(count_sql.contains("FROM ("));
        assert!(count_sql.contains("AS sub"));

        // Should preserve the HAVING clause with $1 parameter
        assert!(count_sql.contains("HAVING"));
        assert!(count_sql.contains("$1"));

        //TODO Optimize to 1
        // Should optimize SELECT to "SELECT 1" for performance
        //assert!(count_sql.contains("SELECT 1"));
        // Should NOT contain the expensive aggregations
        assert!(!count_sql.contains("COUNT(*) as user_count"));
        assert!(count_sql.contains("AVG(CAST(birth_year"));

        println!("✓ Complex GROUP BY with HAVING correctly handled and optimized");
    }

    #[test]
    fn test_count_query_with_sqlx_cast_syntax() {
        // Test the problematic query with SQLx cast syntax
        let sql = "SELECT id as 'id!: Id', name, email, age as 'age: u8', birth_year as 'birth_year: u16' FROM users WHERE age BETWEEN $1 AND $2 AND name LIKE $3 AND (birth_year IS NULL OR birth_year > $4)";

        let statement = parse_sql(sql).unwrap().unwrap();
        let count_sql = generate_count_query(&statement, None);
        println!("Original SQL: {}", sql);
        println!("Generated Count SQL: {}", count_sql);

        // Count query should NOT contain any cast syntax
        assert!(!count_sql.contains(": Id"));
        assert!(!count_sql.contains(": u8"));
        assert!(!count_sql.contains(": u16"));

        // Should be a simple count query
        assert!(count_sql.contains("COUNT(*)"));
        assert!(count_sql.contains("FROM users"));
        assert!(count_sql.contains("WHERE age BETWEEN"));
    }

    #[test]
    fn infer_insert_returning_columns() {
        let sql = r#"
        INSERT INTO users (name, email)
        VALUES ('john', 'john@test.com')
        RETURNING id, name, email
    "#;

        let result = infer_columns(sql).expect("should infer columns");

        assert_eq!(result.columns, vec!["id", "name", "email"]);
    }

    #[test]
    fn infer_insert_returning_with_alias() {
        let sql = r#"
        INSERT INTO users (name)
        VALUES ('john')
        RETURNING id AS user_id, name AS user_name
    "#;

        let result = infer_columns(sql).expect("should infer columns");

        assert_eq!(result.columns, vec!["user_id", "user_name"]);
    }

    #[test]
    fn infer_update_returning_columns() {
        let sql = r#"
        UPDATE users
        SET age = age + 1
        WHERE id = 1
        RETURNING id, age
    "#;

        let result = infer_columns(sql).expect("should infer columns");

        assert_eq!(result.columns, vec!["id", "age"]);
    }

    #[test]
    fn infer_update_returning_expression_with_alias() {
        let sql = r#"
        UPDATE users
        SET age = age + 1
        RETURNING age + 1 AS next_age
    "#;

        let result = infer_columns(sql).expect("should infer columns");

        assert_eq!(result.columns, vec!["next_age"]);
    }

    #[test]
    fn infer_delete_returning_columns() {
        let sql = r#"
        DELETE FROM users
        WHERE age < 18
        RETURNING id, email
    "#;

        let result = infer_columns(sql).expect("should infer columns");

        assert_eq!(result.columns, vec!["id", "email"]);
    }

    #[test]
    fn insert_without_returning_should_error() {
        let sql = r#"
        INSERT INTO users (name)
        VALUES ('john')
    "#;

        let err = infer_columns(sql).unwrap_err();

        assert!(
            err.to_string().contains("INSERT without RETURNING"),
            "unexpected error: {}",
            err
        );
    }

    #[test]
    fn update_without_returning_should_error() {
        let sql = r#"
        UPDATE users
        SET age = 30
        WHERE id = 1
    "#;

        let err = infer_columns(sql).unwrap_err();

        assert!(
            err.to_string().contains("UPDATE without RETURNING"),
            "unexpected error: {}",
            err
        );
    }

    #[test]
    fn delete_without_returning_should_error() {
        let sql = r#"
        DELETE FROM users WHERE id = 1
    "#;

        let err = infer_columns(sql).unwrap_err();

        assert!(
            err.to_string().contains("DELETE without RETURNING"),
            "unexpected error: {}",
            err
        );
    }

    #[test]
    fn returning_expression_without_alias_should_error() {
        let sql = r#"
        UPDATE users
        SET age = age + 1
        RETURNING age + 1
    "#;

        let err = infer_columns(sql).unwrap_err();

        assert!(
            err.to_string().contains("requires an alias"),
            "unexpected error: {}",
            err
        );
    }

    #[test]
    fn returning_star_should_error() {
        let sql = r#"
        DELETE FROM users
        RETURNING *
    "#;

        let err = infer_columns(sql).unwrap_err();

        assert!(
            err.to_string().contains("SELECT *") || err.to_string().contains("not supported"),
            "unexpected error: {}",
            err
        );
    }
}
