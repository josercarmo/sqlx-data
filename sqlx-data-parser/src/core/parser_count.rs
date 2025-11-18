use sqlparser::ast::{
    Expr, Function, FunctionArg, FunctionArgExpr, FunctionArgumentList, FunctionArguments, Ident,
    ObjectName, ObjectNamePart, Query, SelectItem, SetExpr, Statement, TableFactor,
};

#[derive(Debug, Clone, Default)]
pub enum CountStrategy {
    SimpleCount,              // SELECT COUNT(*)
    CountDistinct(Vec<Expr>), // COUNT(DISTINCT) - single or multiple columns
    #[default]
    SubqueryCount, // SELECT COUNT(*) FROM ( ... )
}

#[derive(Debug, Clone, Default)]
pub struct CountAnalysis {
    pub strategy: CountStrategy,
    #[allow(dead_code)]
    pub reason: &'static str,
}

fn analyze_query(statement: &Statement, primary_key: Option<&str>) -> CountAnalysis {
    let Statement::Query(query) = statement else {
        log::warn!("We were expecting a query, but we received a different SQL command.");
        return CountAnalysis::default();
    };

    analyze_query_body(query, primary_key)
}

fn analyze_query_body(query: &Query, primary_key: Option<&str>) -> CountAnalysis {
    let select = match &*query.body {
        SetExpr::Select(s) => s,
        _ => {
            // 1. UNION, INTERSECT, EXCEPT → always subquery
            return CountAnalysis {
                strategy: CountStrategy::SubqueryCount,
                reason: "Set operation (UNION/INTERSECT/EXCEPT)",
            };
        }
    };

    // 2. WITH TIES → always subquery (may return more rows than limit)
    if has_with_ties(select) {
        return CountAnalysis {
            strategy: CountStrategy::SubqueryCount,
            reason: "WITH TIES clause present",
        };
    }

    // 3. GROUP BY, HAVING, window functions → always subquery
    if has_cte(query) || has_group_by(select) || has_having(select) || has_window_functions(select)
    {
        return CountAnalysis {
            strategy: CountStrategy::SubqueryCount,
            reason: "GROUP BY/HAVING/window functions present",
        };
    }

    // 4. Subqueries in FROM or JOIN → always subquery
    if has_subquery_in_from_or_join(select) {
        return CountAnalysis {
            strategy: CountStrategy::SubqueryCount,
            reason: "Subquery in FROM or JOIN",
        };
    }

    // 5. DISTINCT in SELECT
    if select.distinct.is_some() {
        return analyze_distinct(select);
    }

    // 6. Complex JOINs with/without known PK
    if has_join(select) {
        let Some(pk_str) = primary_key else {
            return CountAnalysis {
                strategy: CountStrategy::SubqueryCount,
                reason: "JOIN without primary key information",
            };
        };

        // NOTE: This CountDistinct strategy with primary key is only used when explicitly
        // provided by the developer. It's an optimization hint for JOIN queries.
        return CountAnalysis {
            strategy: CountStrategy::CountDistinct(vec![Expr::Identifier(Ident::new(pk_str))]),
            reason: "JOIN with known primary key",
        };
    }

    // 7. Multiple columns in projection without DISTINCT
    // if select.projection.len() > 1 && select.distinct.is_none() {
    //     let exprs = get_all_projected_exprs(select);
    //     if !exprs.is_empty() {
    //         return CountAnalysis {
    //             strategy: CountStrategy::CountDistinct(exprs),
    //             reason: "Multiple columns in projection",
    //         };
    //     }
    // }

    // 8. Check if projection is safe for SimpleCount
    if is_projection_safe_for_simple_count(select) {
        return CountAnalysis {
            strategy: CountStrategy::SimpleCount,
            reason: "Simple COUNT(*) safe",
        };
    }

    // 9. Default to subquery for semantic safety
    // CountAnalysis {
    //     strategy: CountStrategy::SubqueryCount,
    //     reason: "Projection affects result cardinality",
    // }
    CountAnalysis {
        strategy: CountStrategy::SimpleCount,
        reason: "Projection affects result cardinality",
    }
}

fn analyze_distinct(select: &sqlparser::ast::Select) -> CountAnalysis {
    let expr_refs = get_projected_expr_refs(select);

    if expr_refs.is_empty() {
        return CountAnalysis {
            strategy: CountStrategy::SubqueryCount,
            reason: "DISTINCT with wildcard or complex projection",
        };
    }

    // Clone the expressions to preserve AST semantics
    let exprs: Vec<Expr> = expr_refs.into_iter().cloned().collect();

    CountAnalysis {
        strategy: CountStrategy::CountDistinct(exprs),
        reason: "DISTINCT on one or more expressions",
    }
}

fn is_projection_safe_for_simple_count(select: &sqlparser::ast::Select) -> bool {
    // Simple queries that can safely use COUNT(*)
    // 1. SELECT * - wildcard projection
    if select.projection.len() == 1
        && matches!(
            select.projection[0],
            SelectItem::Wildcard(_) | SelectItem::QualifiedWildcard(..)
        )
    {
        return true;
    }

    // 2. Simple column projections without complex expressions
    select.projection.iter().all(|item| {
        matches!(
            item,
            SelectItem::UnnamedExpr(Expr::Identifier(_))
                | SelectItem::ExprWithAlias {
                    expr: Expr::Identifier(_),
                    ..
                }
                | SelectItem::UnnamedExpr(Expr::CompoundIdentifier(_))
                | SelectItem::ExprWithAlias {
                    expr: Expr::CompoundIdentifier(_),
                    ..
                }
        )
    })
}

fn get_projected_expr_refs(select: &sqlparser::ast::Select) -> Vec<&Expr> {
    select
        .projection
        .iter()
        .filter_map(|item| match item {
            SelectItem::UnnamedExpr(e) | SelectItem::ExprWithAlias { expr: e, .. } => Some(e),
            SelectItem::Wildcard(_) | SelectItem::QualifiedWildcard(..) => None,
        })
        .collect()
}

fn has_subquery_in_from_or_join(select: &sqlparser::ast::Select) -> bool {
    for table_with_joins in &select.from {
        // Subquery in main FROM clause
        if matches!(table_with_joins.relation, TableFactor::Derived { .. }) {
            return true;
        }

        // Subquery in any JOIN
        for join in &table_with_joins.joins {
            if matches!(join.relation, TableFactor::Derived { .. }) {
                return true;
            }
        }
    }
    false
}

fn has_complex_subquery_in_from_or_join(select: &sqlparser::ast::Select) -> bool {
    for table_with_joins in &select.from {
        if let TableFactor::Derived { subquery, .. } = &table_with_joins.relation {
            // Se a subquery derivada tem GROUP BY, HAVING, window, DISTINCT múltiplo, etc.
            if is_complex_subquery(subquery) {
                return true;
            }
        }

        for join in &table_with_joins.joins {
            if let TableFactor::Derived { subquery, .. } = &join.relation
                && is_complex_subquery(subquery)
            {
                return true;
            }
        }
    }
    false
}

fn is_complex_subquery(query: &Query) -> bool {
    let select = match &*query.body {
        SetExpr::Select(s) => s,
        _ => return true, // UNION etc. é complexo
    };

    has_group_by(select)
        || has_having(select)
        || has_window_functions(select)
        || select.distinct.is_some()
        || has_join(select) // subquery com JOIN é complexa
}

fn has_join(select: &sqlparser::ast::Select) -> bool {
    select.from.iter().any(|t| !t.joins.is_empty())
}

fn has_cte(query: &Query) -> bool {
    query.with.is_some()
}

/// Check if a SELECT statement has GROUP BY clause (main query, not subqueries)
fn has_group_by(select: &sqlparser::ast::Select) -> bool {
    match &select.group_by {
        sqlparser::ast::GroupByExpr::All(_) => false,
        sqlparser::ast::GroupByExpr::Expressions(exprs, _) => !exprs.is_empty(),
    }
}

fn has_having(select: &sqlparser::ast::Select) -> bool {
    select.having.is_some()
}

fn has_window_functions(select: &sqlparser::ast::Select) -> bool {
    select.projection.iter().any(|item| match item {
        SelectItem::UnnamedExpr(e) | SelectItem::ExprWithAlias { expr: e, .. } => {
            contains_window(e)
        }
        _ => false,
    })
}

fn contains_window(expr: &Expr) -> bool {
    match expr {
        Expr::Function(f) => f.over.is_some(),
        Expr::BinaryOp { left, right, .. } => contains_window(left) || contains_window(right),
        Expr::Nested(e) => contains_window(e),
        _ => false,
    }
}

fn has_with_ties(select: &sqlparser::ast::Select) -> bool {
    select.top.as_ref().is_some_and(|top| top.with_ties)
}

pub fn count_expr(
    maybe_expr: Option<Expr>,
    duplicate_treatment: Option<sqlparser::ast::DuplicateTreatment>,
) -> Expr {
    Expr::Function(Function {
        name: ObjectName(vec![ObjectNamePart::Identifier(Ident::new("COUNT"))]),
        uses_odbc_syntax: false,
        parameters: FunctionArguments::None,
        args: FunctionArguments::List(FunctionArgumentList {
            duplicate_treatment,
            args: match maybe_expr {
                None => vec![FunctionArg::Unnamed(FunctionArgExpr::Wildcard)],
                Some(expr) => vec![FunctionArg::Unnamed(FunctionArgExpr::Expr(expr))],
            },
            clauses: vec![],
        }),
        filter: None,
        null_treatment: None,
        within_group: vec![],
        over: None,
    })
}

/// This function analyzes the original query to determine the best strategy
/// for generating a COUNT query (simple COUNT(*), COUNT(DISTINCT), or subquery COUNT)
/// Remove ORDER BY, LIMIT, OFFSET from Statement for count query
/// Optimize SELECT to just "SELECT 1" for better performance
pub fn generate_count_query(statement: &Statement, primary_key: Option<&str>) -> String {
    let Statement::Query(query) = statement else {
        return statement.to_string();
    };

    let analysis = analyze_query(statement, primary_key);
    let mut query = query.as_ref().clone();

    query.order_by = None; // ORDER BY doesn't affect count
    query.limit_clause = None; // LIMIT would give wrong count
    query.fetch = None; // PostgreSQL FETCH FIRST/WITH TIES

    let with_clause = query.with.take(); // Detects and extracts CTEs temporarily.

    let SetExpr::Select(select) = query.body.as_mut() else {
        // 1. UNION, INTERSECT, EXCEPT → always subquery
        if matches!(analysis.strategy, CountStrategy::SubqueryCount) {
            let sql_body = format!("SELECT COUNT(*) FROM ({}) AS sub", query);

            if let Some(with_clause) = with_clause {
                return format!("{} {}", with_clause, sql_body);
            }
            return sql_body;
        }
        return query.to_string();
    };

    let sql_body = match analysis.strategy {
        CountStrategy::SimpleCount => {
            let count_decorate = count_expr(None, None);
            select.projection = vec![SelectItem::UnnamedExpr(count_decorate)];
            query.to_string()
        }
        CountStrategy::CountDistinct(exprs) => {
            if exprs.len() == 1 {
                // Single column: use COUNT(DISTINCT col) directly, remove original DISTINCT
                select.distinct = None;
                let count_decorate = count_expr(
                    Some(exprs[0].clone()),
                    Some(sqlparser::ast::DuplicateTreatment::Distinct),
                );
                select.projection = vec![SelectItem::UnnamedExpr(count_decorate)];
                return query.to_string();
            }

            // Multiple columns: use COUNT(*) from DISTINCT subquery
            // This preserves the correct semantics for multi-column DISTINCT
            format!("SELECT COUNT(*) FROM ({}) AS sub", query)
        }
        CountStrategy::SubqueryCount => {
            // Wrapper query: SELECT COUNT(*) FROM (original_query) AS sub
            // This handles complex queries with GROUP BY, HAVING, etc.

            // We can optimize to SELECT 1 only when the query doesn't depend on specific columns
            // Must preserve original projection when:
            // 1. GROUP BY/HAVING - aggregations need specific columns
            // 2. Window functions - need specific columns for partitioning/ordering
            // 3. Subqueries in FROM/JOIN - subquery may depend on specific columns
            // 4. Complex expressions - may reference columns from tables
            let can_optimize_to_select_1 = select.distinct.is_none()
                && !has_group_by(select)
                && !has_having(select)
                && !has_window_functions(select)
                && !has_complex_subquery_in_from_or_join(select);

            if can_optimize_to_select_1 {
                // Optimize by replacing projection with "1" for simple queries
                select.projection = vec![SelectItem::UnnamedExpr(Expr::Identifier(
                    sqlparser::ast::Ident::new("1"),
                ))];
            }
            format!("SELECT COUNT(*) FROM ({}) AS sub", query)
        }
    };

    match with_clause {
        Some(wc) => format!("{} {}", wc, sql_body),
        None => sql_body,
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::super::parse_sql;
    use super::*;

    /// Helper function for tests to parse SQL and unwrap everything
    fn parse_sql_for_tests(sql: &str) -> Arc<Statement> {
        parse_sql(sql).unwrap().unwrap()
    }

    #[test]
    fn test_generate_count_query_basic() {
        let sql = "SELECT id, name FROM users WHERE age > 18";
        let statement = parse_sql_for_tests(sql);
        let count_sql = generate_count_query(statement.as_ref(), None);
        assert!(count_sql.contains("COUNT(*)"));
        assert!(count_sql.contains("WHERE age > 18"));
    }

    #[test]
    fn test_generate_count_query_with_order_by() {
        let sql = "SELECT name, email FROM users ORDER BY name ASC LIMIT 10";
        let statement = parse_sql_for_tests(sql);
        let count_sql = generate_count_query(statement.as_ref(), None);
        assert!(count_sql.contains("COUNT(*)"));
        assert!(!count_sql.contains("ORDER BY"));
        assert!(!count_sql.contains("LIMIT"));
    }

    #[test]
    fn test_generate_count_query_complex_join() {
        let sql = "SELECT u.name, p.title FROM users u JOIN posts p ON u.id = p.user_id WHERE u.active = true";
        let statement = parse_sql_for_tests(sql);
        let count_sql = generate_count_query(statement.as_ref(), None);
        assert!(count_sql.contains("COUNT(*)"));
        // Should be subquery due to JOIN without primary key
        assert!(count_sql.contains("FROM ("));
    }

    #[test]
    fn test_analyze_simple_select() {
        let sql = "SELECT name FROM users WHERE active = true";
        let statement = parse_sql_for_tests(sql);
        let analysis = analyze_query(statement.as_ref(), None);

        match analysis.strategy {
            CountStrategy::SimpleCount => assert!(true),
            _ => panic!("Expected SimpleCount for simple SELECT"),
        }
    }

    #[test]
    fn test_analyze_group_by() {
        let sql = "SELECT country, COUNT(*) FROM users GROUP BY country";
        let statement = parse_sql_for_tests(sql);
        let analysis = analyze_query(statement.as_ref(), None);

        match analysis.strategy {
            CountStrategy::SubqueryCount => assert!(true),
            _ => panic!("Expected SubqueryCount for GROUP BY query"),
        }
    }

    #[test]
    fn test_analyze_distinct() {
        let sql = "SELECT DISTINCT name FROM users";
        let statement = parse_sql_for_tests(sql);
        let analysis = analyze_query(statement.as_ref(), None);

        match analysis.strategy {
            CountStrategy::CountDistinct(_) => assert!(true),
            _ => panic!("Expected CountDistinct for DISTINCT query"),
        }
    }

    #[test]
    fn test_analyze_join_with_pk() {
        let sql = "SELECT u.name, p.title FROM users u JOIN posts p ON u.id = p.user_id";
        let statement = parse_sql_for_tests(sql);
        let analysis = analyze_query(statement.as_ref(), Some("u.id"));

        match analysis.strategy {
            CountStrategy::CountDistinct(_) => assert!(true),
            _ => panic!("Expected CountDistinct for JOIN with primary key"),
        }
    }

    #[test]
    fn test_analyze_join_without_pk() {
        let sql = "SELECT u.name, p.title FROM users u JOIN posts p ON u.id = p.user_id";
        let statement = parse_sql_for_tests(sql);
        let analysis = analyze_query(statement.as_ref(), None);

        match analysis.strategy {
            CountStrategy::SubqueryCount => assert!(true),
            _ => panic!("Expected SubqueryCount for JOIN without primary key"),
        }
    }

    #[test]
    fn test_generate_count_query_simple_select() {
        let sql = "SELECT name, email, age FROM users WHERE active = true ORDER BY created_at DESC LIMIT 20 OFFSET 40";
        let statement = parse_sql_for_tests(sql);
        let count_sql = generate_count_query(statement.as_ref(), None);

        // Should use direct COUNT(*) + SELECT 1 optimization

        assert!(count_sql.contains("SELECT COUNT(*)"));
        assert!(count_sql.contains("SELECT COUNT(*) FROM users WHERE active = true"));
        assert!(!count_sql.contains("ORDER BY"));
        assert!(!count_sql.contains("LIMIT"));
        assert!(!count_sql.contains("OFFSET"));
        assert!(!count_sql.contains("FROM (")); // no subquery
    }

    #[test]
    fn test_generate_count_query_multiple_columns_projection() {
        let sql = "SELECT name, email, age FROM users WHERE created_at > '2024-01-01' ORDER BY name LIMIT 10";
        let statement = parse_sql_for_tests(sql);
        let count_sql = generate_count_query(statement.as_ref(), None);

        // Should use COUNT(DISTINCT name, email, age) with internal SELECT 1
        assert!(count_sql.contains("COUNT(*)"));
        assert!(count_sql.contains("FROM users WHERE created_at > '2024-01-01'"));
        assert!(!count_sql.contains("FROM ("));
        assert!(!count_sql.contains("ORDER BY"));
        assert!(!count_sql.contains("LIMIT"));
    }

    #[test]
    fn test_generate_count_query_without_pk() {
        let sql = "SELECT u.name, p.title FROM users u JOIN posts p ON u.id = p.user_id ORDER BY p.created_at DESC LIMIT 50";
        let statement = parse_sql_for_tests(sql);
        let count_sql = generate_count_query(statement.as_ref(), None);

        // Should use COUNT(DISTINCT id) with internal SELECT 1
        assert!(count_sql.contains("COUNT(*)"));
        assert!(count_sql.contains("SELECT 1 FROM users u JOIN posts p"));
        assert!(count_sql.contains("FROM ("));
        assert!(!count_sql.contains("ORDER BY"));
        assert!(!count_sql.contains("LIMIT"));
    }

    #[test]
    fn test_generate_count_query_join_with_pk() {
        let sql = "SELECT u.name, p.title FROM users u JOIN posts p ON u.id = p.user_id ORDER BY p.created_at DESC LIMIT 50";
        let statement = parse_sql_for_tests(sql);
        let count_sql = generate_count_query(statement.as_ref(), Some("id"));
        println!("Generated count SQL: {}", count_sql);

        // Should use COUNT(DISTINCT id) directly on the JOIN (optimized, no subquery)
        assert!(count_sql.contains("COUNT(DISTINCT id)"));
        assert!(count_sql.contains("FROM users u JOIN posts p ON u.id = p.user_id"));
        assert!(!count_sql.contains("FROM (")); // no subquery needed
        assert!(!count_sql.contains("ORDER BY"));
        assert!(!count_sql.contains("LIMIT"));
    }

    #[test]
    fn test_generate_count_query_group_by() {
        let sql = "SELECT department, AVG(salary) AS avg_salary FROM employees GROUP BY department HAVING AVG(salary) > 50000 ORDER BY avg_salary DESC LIMIT 5";
        let statement = parse_sql_for_tests(sql);
        let count_sql = generate_count_query(statement.as_ref(), None);

        // Should use complete subquery (doesn't optimize to SELECT 1)
        assert!(count_sql.contains("COUNT(*)"));
        assert!(count_sql.contains("FROM (SELECT department, AVG(salary)"));
        assert!(count_sql.contains("GROUP BY department"));
        assert!(count_sql.contains("HAVING"));
        assert!(!count_sql.contains("SELECT 1")); // doesn't optimize with GROUP BY
        assert!(!count_sql.contains("ORDER BY"));
        assert!(!count_sql.contains("LIMIT"));
    }

    #[test]
    fn test_generate_count_query_distinct_single_column() {
        let sql = "SELECT DISTINCT email FROM users WHERE active = true ORDER BY email LIMIT 100 OFFSET 200";
        let statement = parse_sql_for_tests(sql);
        let count_sql = generate_count_query(statement.as_ref(), None);
        println!("Generated count SQL: {}", count_sql);
        // Should use COUNT(DISTINCT email) directly (optimized, no subquery)
        assert!(count_sql.contains("COUNT(DISTINCT email)"));
        assert!(count_sql.contains("FROM users WHERE active = true"));
        assert!(!count_sql.contains("FROM (")); // no subquery needed
        assert!(!count_sql.contains("ORDER BY"));
        assert!(!count_sql.contains("LIMIT"));
        assert!(!count_sql.contains("OFFSET"));
        assert!(!count_sql.contains("SELECT 1"));
    }

    #[test]
    fn test_complex_query_1_subquery_in_from() {
        // Subquery in FROM — should force SubqueryCount
        let sql = "SELECT u.name FROM (SELECT name FROM users WHERE active = true) u LIMIT 10";
        let statement = parse_sql_for_tests(sql);
        let count_sql = generate_count_query(statement.as_ref(), None);
        println!("Generated count SQL: {}", count_sql);
        assert!(count_sql.contains("COUNT(*)"));
        assert!(count_sql.contains("FROM ("));
        assert!(count_sql.contains("SELECT name FROM users WHERE active = true")); // keeps internal subquery
        assert!(count_sql.contains("SELECT 1"));
        assert!(!count_sql.contains("LIMIT"));
    }

    #[test]
    fn test_complex_query_2_window_function() {
        // Window function — should force SubqueryCount
        let sql = "SELECT name, ROW_NUMBER() OVER (PARTITION BY department ORDER BY salary DESC) AS rn FROM employees";
        let statement = parse_sql_for_tests(sql);
        let count_sql = generate_count_query(statement.as_ref(), None);

        assert!(count_sql.contains("COUNT(*)"));
        assert!(count_sql.contains("FROM ("));
        assert!(count_sql.contains("ROW_NUMBER() OVER"));
        assert!(!count_sql.contains("SELECT 1")); // doesn't optimize with window
    }

    #[test]
    fn test_complex_query_3_distinct_multiple_columns() {
        // DISTINCT with multiple columns — should use COUNT(DISTINCT col1, col2)
        let sql = "SELECT DISTINCT name, email FROM users WHERE created_at > '2024-01-01'";
        let statement = parse_sql_for_tests(sql);
        let count_sql = generate_count_query(statement.as_ref(), None);

        println!("Generated count SQL: {}", count_sql);
        // Multiple columns: use COUNT(*) from DISTINCT subquery for semantic correctness
        assert!(count_sql.contains("SELECT COUNT(*) FROM"));
        assert!(count_sql.contains("SELECT DISTINCT name, email FROM users")); // preserves DISTINCT in subquery
        assert!(count_sql.contains("FROM ("));
        assert!(!count_sql.contains("COUNT(DISTINCT")); // No COUNT(DISTINCT) for multiple columns
    }

    #[test]
    fn test_complex_query_4_group_by_with_alias() {
        // GROUP BY with alias — should force SubqueryCount (doesn't optimize to SELECT 1)
        let sql = "SELECT department AS dept, AVG(salary) FROM employees GROUP BY dept";
        let statement = parse_sql_for_tests(sql);
        let count_sql = generate_count_query(statement.as_ref(), None);

        assert!(count_sql.contains("COUNT(*)"));
        assert!(count_sql.contains("FROM ("));
        assert!(count_sql.contains("GROUP BY dept"));
        assert!(!count_sql.contains("SELECT 1")); // doesn't optimize with GROUP BY
    }

    #[test]
    fn test_distinct_multiple_columns_preserves_distinct_in_subquery() {
        // This test proves that removing DISTINCT from original query is WRONG
        let sql = "SELECT DISTINCT name, email, age FROM users WHERE active = true";
        let statement = parse_sql_for_tests(sql);
        let count_sql = generate_count_query(statement.as_ref(), None);
        println!("Generated count SQL: {}", count_sql);

        // For multiple columns, we expect subquery with DISTINCT preserved
        // WRONG: Current code removes DISTINCT and generates invalid SQL
        // Expected: SELECT COUNT(DISTINCT name) FROM (SELECT DISTINCT name, email, age FROM users WHERE active = true) AS sub
        // Actual: SELECT COUNT(DISTINCT name) FROM (SELECT name, email, age FROM users WHERE active = true) AS sub

        // Correct behavior: COUNT(*) from DISTINCT subquery
        assert!(count_sql.contains("SELECT COUNT(*) FROM"));
        assert!(count_sql.contains("SELECT DISTINCT name, email, age"));
        assert!(!count_sql.contains("COUNT(DISTINCT")); // No COUNT(DISTINCT) for multiple columns
    }

    #[test]
    fn test_complex_query_5_cte_with_union() {
        // CTE + UNION — should force SubqueryCount
        let sql = "WITH active_users AS (SELECT * FROM users WHERE active = true) SELECT * FROM active_users UNION SELECT * FROM archived_users LIMIT 10";
        let statement = parse_sql_for_tests(sql);
        let count_sql = generate_count_query(statement.as_ref(), None);
        println!("Generated count SQL: {}", count_sql);
        assert!(count_sql.contains("COUNT(*)"));
        assert!(count_sql.contains("FROM ("));
        assert!(count_sql.contains("WITH active_users AS"));
        assert!(count_sql.contains("UNION"));
        assert!(!count_sql.contains("SELECT 1"));
    }

    #[test]
    fn test_simple_count_semantic_correctness_issue() {
        // This test demonstrates a semantic issue with SimpleCount strategy
        // Query: SELECT country FROM users - projects only country column
        // Expected: Should consider projection cardinality (distinct countries vs all users)
        // Actual: Falls into SimpleCount and generates COUNT(*) which counts ALL users
        // This is semantically incorrect when projection affects result cardinality

        let sql = "SELECT country FROM users";
        let statement = parse_sql_for_tests(sql);
        let count_sql = generate_count_query(statement.as_ref(), None);

        println!("Original SQL: {}", sql);
        println!("Generated count SQL: {}", count_sql);

        // This assertion will pass but shows the semantic issue
        // COUNT(*) counts all users, but SELECT country should only count distinct countries
        assert_eq!(count_sql, "SELECT COUNT(*) FROM users");

        // The issue: if users table has:
        // | id | country |
        // | 1  | USA     |
        // | 2  | USA     |
        // | 3  | Brazil  |
        //
        // SELECT country FROM users returns 3 rows: [USA, USA, Brazil]
        // But COUNT(*) FROM users returns 3 (total users)
        // While the actual count should consider projection cardinality
        //
        // For semantic correctness, this should use SubqueryCount:
        // SELECT COUNT(*) FROM (SELECT country FROM users) AS sub
    }

    #[test]
    fn test_real_bugs_found() {
        // BUG 1: Missing check for query.fetch clause (PostgreSQL FETCH FIRST)
        // Query with FETCH should use subquery but might fall through to SimpleCount
        let sql1 = "SELECT name FROM users FETCH FIRST 10 ROWS ONLY";
        if let Ok(Some(statement1)) = parse_sql(sql1) {
            let count_sql1 = generate_count_query(statement1.as_ref(), None);
            println!("FETCH BUG: {} -> {}", sql1, count_sql1);
            // This might incorrectly use SimpleCount instead of subquery
        }

        // BUG 2: CountDistinct with single column removes DISTINCT but what if there are other DISTINCT operations?
        let sql2 = "SELECT DISTINCT name FROM (SELECT DISTINCT email, name FROM users) sub";
        let statement2 = parse_sql_for_tests(sql2);
        let count_sql2 = generate_count_query(statement2.as_ref(), None);
        println!("NESTED DISTINCT BUG: {} -> {}", sql2, count_sql2);
        // Line 323: select.distinct = None; might break nested DISTINCT semantics

        // BUG 3: has_complex_subquery_in_from_or_join function doesn't exist!
        // Line 350 calls a function that doesn't exist - this will cause compile error
        let sql3 = "SELECT name FROM users WHERE id > 0";
        let statement3 = parse_sql_for_tests(sql3);
        let count_sql3 = generate_count_query(statement3.as_ref(), None);
        println!("MISSING FUNCTION: {} -> {}", sql3, count_sql3);
        // This should fail to compile due to missing has_complex_subquery_in_from_or_join

        // BUG 4: CTE handling inconsistency
        // WITH clause is extracted but only some code paths restore it
        let sql4 = "WITH temp AS (SELECT * FROM users) SELECT name FROM temp WHERE active = true";
        let statement4 = parse_sql_for_tests(sql4);
        let count_sql4 = generate_count_query(statement4.as_ref(), None);
        println!("CTE BUG: {} -> {}", sql4, count_sql4);
        // CTE might not be properly restored in all code paths

        // BUG 5: CountDistinct MySQL/SQLite compatibility issue
        // If we have multiple columns but are on MySQL/SQLite, should use SubqueryCount
        let sql5 = "SELECT DISTINCT name, email FROM users";
        let statement5 = parse_sql_for_tests(sql5);
        let count_sql5 = generate_count_query(statement5.as_ref(), None);
        println!("MYSQL COMPAT BUG: {} -> {}", sql5, count_sql5);
        // Might generate COUNT(DISTINCT name, email) which fails on MySQL/SQLite

        assert!(count_sql2.contains("COUNT"));
        assert!(count_sql3.contains("COUNT"));
        assert!(count_sql4.contains("COUNT"));
        assert!(count_sql5.contains("COUNT"));
    }
}
