use super::error::ParserError;
use sqlparser::ast::Statement;

pub type Result<T> = ::std::result::Result<T, ParserError>;

/// Extract INSERT base part from parsed statement (without VALUES clause)
/// Returns "INSERT INTO table (col1, col2, col3)" without the VALUES part
pub fn extract_insert_base_from_statement(stmt: &Statement) -> Result<String> {
    let Statement::Insert(insert) = stmt else {
        return Err(ParserError::error(
            "Expected INSERT statement for multi-insert operation",
        ))?;
    };

    // Create a clean INSERT with only the essential parts
    let mut clean_insert = insert.clone();
    clean_insert.source = None;
    clean_insert.on = None; // Remove ON CONFLICT clause (will be added back later)
    clean_insert.returning = None; // Remove RETURNING clause

    // Convert back to SQL string - this gives us "INSERT INTO table (cols)"
    Ok(clean_insert.to_string())
}

/// Extract RETURNING clause from INSERT statement
/// Returns Some("RETURNING id, name") or None if no RETURNING clause exists
pub fn extract_returning_clause_from_statement(stmt: &Statement) -> Option<String> {
    let Statement::Insert(insert) = stmt else {
        return None;
    };

    insert.returning.as_ref().map(|returning| {
        format!(
            " RETURNING {}",
            returning
                .iter()
                .map(|item| item.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        )
    })
}

/// Extract VALUES pattern from INSERT statement
/// Example: "INSERT INTO users (...) VALUES (?, UPPER(?), LOWER(?))" -> Some("(?, UPPER(?), LOWER(?))")
pub fn extract_values_clause_from_statement(stmt: &Statement) -> Option<String> {
    let Statement::Insert(insert) = stmt else {
        return None;
    };

    let Some(source) = &insert.source else {
        return None;
    };

    let sqlparser::ast::SetExpr::Values(values) = source.body.as_ref() else {
        return None;
    };

    // Get the first row pattern
    let first_row = values.rows.first()?;

    Some(format!(
        "({})",
        first_row
            .iter()
            .map(|expr| expr.to_string())
            .collect::<Vec<_>>()
            .join(", ")
    ))
}

/// Extract column names from INSERT statement (not RETURNING columns)
/// Returns the column names from the INSERT INTO table (col1, col2, ...) part
pub fn infer_insert_columns_from_stmt(stmt: &Statement) -> Result<Vec<String>> {
    match stmt {
        Statement::Insert(insert) => Ok(insert
            .columns
            .iter()
            .map(|ident| ident.value.clone())
            .collect()),
        _ => Err(ParserError::error("Not an INSERT statement"))?,
    }
}

/// Extract ON CONFLICT clause from INSERT statement
/// Returns Some(" ON CONFLICT(id) DO UPDATE SET ...") or None if no ON CONFLICT clause exists
pub fn extract_on_conflict_clause_from_statement(stmt: &Statement) -> Option<String> {
    let Statement::Insert(insert) = stmt else {
        return None;
    };

    insert
        .on
        .as_ref()
        .map(|on_conflict| on_conflict.to_string())
}

pub fn has_complex_sql_functions_in_values(stmt: &Statement) -> bool {
    let Statement::Insert(insert) = stmt else {
        return false;
    };

    let Some(source) = &insert.source else {
        return false;
    };

    let sqlparser::ast::SetExpr::Values(values) = source.body.as_ref() else {
        return false;
    };

    values
        .rows
        .iter()
        .any(|row| row.iter().any(|expr| !is_simple_placeholder(expr)))
}

fn is_simple_placeholder(expr: &sqlparser::ast::Expr) -> bool {
    matches!(
        expr,
        sqlparser::ast::Expr::Value(sqlparser::ast::ValueWithSpan {
            value: sqlparser::ast::Value::Placeholder(_),
            ..
        })
    )
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::parse_sql;

    /// Helper function for tests to parse SQL and unwrap everything
    fn parse_sql_for_tests(sql: &str) -> Arc<Statement> {
        parse_sql(sql).unwrap().unwrap()
    }

    /// Returns placeholder string based on database dialect feature
    /// PostgreSQL uses $1, $2, etc. Others use ?
    #[cfg(feature = "postgres")]
    fn placeholders(count: usize) -> String {
        (1..=count).map(|i| format!("${}", i)).collect::<Vec<_>>().join(", ")
    }

    #[cfg(not(feature = "postgres"))]
    fn placeholders(count: usize) -> String {
        vec!["?"; count].join(", ")
    }

    #[test]
    fn test_infer_insert_columns() {
        let sql = format!(
            "INSERT INTO users (name, email, age, birth_year) VALUES ({}) RETURNING id",
            placeholders(4)
        );
        let stmt = parse_sql_for_tests(&sql);

        let columns = infer_insert_columns_from_stmt(stmt.as_ref()).unwrap();
        assert_eq!(columns, vec!["name", "email", "age", "birth_year"]);
    }

    #[test]
    fn test_infer_insert_columns_vs_returning_columns() {
        let sql = format!(
            "INSERT INTO users (name, email, age, birth_year) VALUES ({}) RETURNING id",
            placeholders(4)
        );
        let stmt = parse_sql_for_tests(&sql);

        // INSERT columns should be different from RETURNING columns
        let insert_columns = infer_insert_columns_from_stmt(stmt.as_ref()).unwrap();
        let returning_columns = crate::infer_columns_from_stmt(stmt.as_ref())
            .unwrap()
            .columns;

        assert_eq!(insert_columns, vec!["name", "email", "age", "birth_year"]);
        assert_eq!(returning_columns, vec!["id"]);
        assert_ne!(insert_columns, returning_columns);
    }

    #[test]
    fn test_infer_insert_columns_non_insert_should_error() {
        let sql = "SELECT name FROM users";
        let stmt = parse_sql_for_tests(sql);

        let err = infer_insert_columns_from_stmt(stmt.as_ref()).unwrap_err();
        assert!(err.to_string().contains("Not an INSERT statement"));
    }

    #[test]
    fn test_extract_insert_base_from_statement() {
        let sql = format!(
            "INSERT INTO users (id, name, email, age, birth_year) VALUES ({}) RETURNING id",
            placeholders(5)
        );
        let stmt = parse_sql_for_tests(&sql);

        let base = extract_insert_base_from_statement(stmt.as_ref()).unwrap();

        // Should extract only the INSERT part without VALUES and RETURNING
        assert!(base.starts_with("INSERT INTO users"));
        assert!(base.contains("(id, name, email, age, birth_year)"));
        assert!(!base.contains("VALUES"));
        assert!(!base.contains("RETURNING"));

        // Test with ON CONFLICT
        let sql_conflict = format!(
            "INSERT INTO users (id, name, email) VALUES ({}) ON CONFLICT(id) DO UPDATE SET name = EXCLUDED.name RETURNING id",
            placeholders(3)
        );
        let stmt = parse_sql_for_tests(&sql_conflict);

        let base = extract_insert_base_from_statement(stmt.as_ref()).unwrap();
        assert!(base.starts_with("INSERT INTO users"));
        assert!(base.contains("(id, name, email)"));
        assert!(!base.contains("VALUES"));
        assert!(!base.contains("ON CONFLICT"));
        assert!(!base.contains("RETURNING"));
    }

    #[test]
    fn test_extract_insert_base_non_insert_should_error() {
        let sql = "SELECT name FROM users";
        let stmt = parse_sql_for_tests(sql);

        let err = extract_insert_base_from_statement(stmt.as_ref()).unwrap_err();
        assert!(err.to_string().contains("Expected INSERT statement"));
    }

    #[test]
    fn test_extract_returning_clause_from_statement() {
        // Test with RETURNING clause
        let sql_with_returning = format!(
            "INSERT INTO users (name, email) VALUES ({}) RETURNING id",
            placeholders(2)
        );
        let stmt = parse_sql_for_tests(&sql_with_returning);

        let returning = extract_returning_clause_from_statement(stmt.as_ref());
        assert_eq!(returning, Some(" RETURNING id".to_string()));

        // Test with multiple RETURNING columns
        let sql_multiple = format!(
            "INSERT INTO users (name, email) VALUES ({}) RETURNING id, name, created_at",
            placeholders(2)
        );
        let stmt = parse_sql_for_tests(&sql_multiple);

        let returning = extract_returning_clause_from_statement(stmt.as_ref());
        assert_eq!(
            returning,
            Some(" RETURNING id, name, created_at".to_string())
        );

        // Test without RETURNING clause
        let sql_no_returning = format!(
            "INSERT INTO users (name, email) VALUES ({})",
            placeholders(2)
        );
        let stmt = parse_sql_for_tests(&sql_no_returning);

        let returning = extract_returning_clause_from_statement(stmt.as_ref());
        assert_eq!(returning, None);

        // Test non-INSERT statement
        let sql_select = "SELECT name FROM users";
        let stmt = parse_sql_for_tests(sql_select);

        let returning = extract_returning_clause_from_statement(stmt.as_ref());
        assert_eq!(returning, None);
    }

    #[test]
    fn test_extract_values_clause_from_statement() {
        // Test with simple VALUES pattern
        let sql_simple = format!(
            "INSERT INTO users (name, email) VALUES ({})",
            placeholders(2)
        );
        let stmt = parse_sql_for_tests(&sql_simple);

        let values = extract_values_clause_from_statement(stmt.as_ref());
        assert_eq!(values, Some(format!("({})", placeholders(2))));

        // Test with SQL functions in VALUES - these tests are dialect-specific
        // Skip complex function tests when postgres feature is enabled as syntax differs
        #[cfg(not(feature = "postgres"))]
        {
            let sql_functions = "INSERT INTO users (id, name, email, age, birth_year) VALUES (?, UPPER(?), LOWER(?), ? + 1, ?)";
            let stmt = parse_sql_for_tests(sql_functions);

            let values = extract_values_clause_from_statement(stmt.as_ref());
            assert_eq!(
                values,
                Some("(?, UPPER(?), LOWER(?), ? + 1, ?)".to_string())
            );

            // Test with more complex functions
            let sql_complex = "INSERT INTO users (id, name, status) VALUES (?, COALESCE(?, 'default'), CONCAT(?, '_suffix'))";
            let stmt = parse_sql_for_tests(sql_complex);

            let values = extract_values_clause_from_statement(stmt.as_ref());
            assert_eq!(
                values,
                Some("(?, COALESCE(?, 'default'), CONCAT(?, '_suffix'))".to_string())
            );

            // Test with literals and expressions
            let sql_literals =
                "INSERT INTO users (id, name, active, created_at) VALUES (?, ?, true, NOW())";
            let stmt = parse_sql_for_tests(sql_literals);

            let values = extract_values_clause_from_statement(stmt.as_ref());
            assert_eq!(values, Some("(?, ?, true, NOW())".to_string()));
        }

        // Test non-INSERT statement should return None
        let sql_select = "SELECT name FROM users";
        let stmt = parse_sql_for_tests(sql_select);

        let values = extract_values_clause_from_statement(stmt.as_ref());
        assert_eq!(values, None);

        // Test INSERT without explicit VALUES (like INSERT ... SELECT)
        let sql_select_insert = "INSERT INTO users (name) SELECT name FROM other_table";
        let stmt = parse_sql_for_tests(sql_select_insert);

        let values = extract_values_clause_from_statement(stmt.as_ref());
        assert_eq!(values, None);

        // Test with RETURNING clause
        let sql_returning = format!(
            "INSERT INTO users (name, email, age, birth_year) VALUES ({}) RETURNING id",
            placeholders(4)
        );
        let stmt = parse_sql_for_tests(&sql_returning);

        let values = extract_values_clause_from_statement(stmt.as_ref());
        assert_eq!(values, Some(format!("({})", placeholders(4))));
    }

    #[test]
    fn test_has_complex_sql_functions_in_values() {
        // Test with simple placeholders (should return false)
        let sql_simple = format!(
            "INSERT INTO users (name, email) VALUES ({})",
            placeholders(2)
        );
        let stmt = parse_sql_for_tests(&sql_simple);

        assert!(!has_complex_sql_functions_in_values(stmt.as_ref()));

        // Dialect-specific tests for complex functions
        #[cfg(not(feature = "postgres"))]
        {
            // Test with SQL functions (should return true)
            let sql_functions =
                "INSERT INTO users (id, name, email, age) VALUES (?, UPPER(?), LOWER(?), ? + 1)";
            let stmt = parse_sql_for_tests(sql_functions);

            assert!(has_complex_sql_functions_in_values(stmt.as_ref()));

            // Test with COALESCE function
            let sql_coalesce = "INSERT INTO users (id, name) VALUES (?, COALESCE(?, 'default'))";
            let stmt = parse_sql_for_tests(sql_coalesce);

            assert!(has_complex_sql_functions_in_values(stmt.as_ref()));
        }

        // Test non-INSERT statement (should return false)
        let sql_select = "SELECT name FROM users";
        let stmt = parse_sql_for_tests(sql_select);

        assert!(!has_complex_sql_functions_in_values(stmt.as_ref()));
    }

    #[test]
    fn test_extract_on_conflict_clause() {
        // Test with ON CONFLICT clause
        let sql_upsert = format!(
            "INSERT INTO users (id, name, email) VALUES ({}) ON CONFLICT(id) DO UPDATE SET name = EXCLUDED.name",
            placeholders(3)
        );
        let stmt = parse_sql_for_tests(&sql_upsert);

        let on_conflict = extract_on_conflict_clause_from_statement(stmt.as_ref());
        assert_eq!(
            on_conflict,
            Some(" ON CONFLICT(id) DO UPDATE SET name = EXCLUDED.name".to_string())
        );

        // Test without ON CONFLICT clause
        let sql_simple = format!(
            "INSERT INTO users (id, name, email) VALUES ({})",
            placeholders(3)
        );
        let stmt = parse_sql_for_tests(&sql_simple);

        let on_conflict = extract_on_conflict_clause_from_statement(stmt.as_ref());
        assert_eq!(on_conflict, None);
    }
}
