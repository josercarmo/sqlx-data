#![cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]

use super::error::{Result, SqlxError};
use sqlparser::ast::*;
use sqlx_data_params::{FilterOperator, FilterValue};
use std::sync::Arc;

fn invalid_argument(msg: impl std::fmt::Display) -> SqlxError {
    SqlxError::InvalidArgument(msg.to_string())
}

/// Result of building dynamic SQL - contains SQL string and bind values to reconstruct Arguments
#[derive(Debug)]
pub struct BuiltSql {
    pub sql: Arc<String>,
    pub bind_values: Vec<FilterValue>,
}

pub fn build_dynamic_sql(
    base_sql: &str,
    params: &sqlx_data_params::Params,
    initial_binds: Vec<FilterValue>,
) -> Result<BuiltSql> {
    let mut bind_values: Vec<FilterValue> = Vec::new();

    // Add initial binds
    bind_values.extend(initial_binds);

    let statement_opt = crate::parse_sql(base_sql)?;

    let statement_arc = statement_opt.ok_or(SqlxError::protocol(format!(
        "Failed to parse SQL: `{}`",
        base_sql
    )))?;

    if params.is_empty() {
        return Ok(BuiltSql {
            sql: Arc::new(statement_arc.to_string()),
            bind_values,
        });
    }

    let mut statement = (*statement_arc).clone();

    // Handle both SELECT and INSERT statements
    match &mut statement {
        Statement::Query(query) => {
            process_query(query, params, &mut bind_values)?;
            Ok(BuiltSql {
                sql: Arc::new(statement.to_string()),
                bind_values,
            })
        }
        _ => Err(SqlxError::protocol(
            "Only SELECT and INSERT statements are supported",
        )),
    }
}

fn process_query(
    query: &mut Box<Query>,
    params: &sqlx_data_params::Params,
    bind_values: &mut Vec<FilterValue>,
) -> Result<()> {
    let mut next_placeholder = make_generator(bind_values.len() as u8);
    let mut bind = |v: &FilterValue| {
        bind_values.push(v.clone());
        Ok(value_expr(Value::Placeholder(next_placeholder())))
    };

    // Process the SetExpr recursively - could be SELECT, UNION, subquery, etc.
    process_set_expr(query.body.as_mut(), params, &mut bind)?;

    // ----------------------------------------------------------
    // SORT
    // ----------------------------------------------------------
    if let Some(sort_by) = &params.sort_by
        && !sort_by.sorts().is_empty()
    {
        let mut order_exprs = Vec::with_capacity(sort_by.sorts().len());

        // Check if we need to invert ORDER BY for BEFORE cursor pagination
        let should_invert_order =
            if let Some(sqlx_data_params::Pagination::Cursor(cursor)) = &params.pagination {
                matches!(
                    cursor.direction,
                    Some(sqlx_data_params::CursorDirection::Before)
                )
            } else {
                false
            };

        for s in sort_by.sorts() {
            let final_direction = if should_invert_order {
                !s.is_asc() // Invert: ASC becomes DESC, DESC becomes ASC
            } else {
                s.is_asc()
            };

            order_exprs.push(OrderByExpr {
                expr: id(&s.field),
                options: OrderByOptions {
                    asc: Some(final_direction),
                    nulls_first: s.nulls_as_bool(),
                },
                with_fill: None,
            });
        }

        query.order_by = Some(OrderBy {
            kind: OrderByKind::Expressions(order_exprs),
            interpolate: None,
        });
    }

    // ----------------------------------------------------------
    // LIMIT / OFFSET
    // ----------------------------------------------------------
    let limit_expr = match &params.limit {
        Some(limit) => {
            let actual_limit = limit.0 + params.limit_plus_one();
            let value = Value::Number(actual_limit.to_string(), false);
            Some(value_expr(value))
        }
        None => None,
    };

    let offset_expr = match &params.offset {
        Some(offset) => Some(Offset {
            value: value_expr(Value::Number(offset.0.to_string(), false)),
            rows: OffsetRows::None,
        }),
        None => None,
    };

    if limit_expr.is_some() || offset_expr.is_some() {
        query.limit_clause = Some(LimitClause::LimitOffset {
            limit: limit_expr,
            offset: offset_expr,
            limit_by: vec![],
        });
    }

    Ok(())
}

/// Process any SetExpr recursively - SELECT, UNION, subquery, etc.
fn process_set_expr<F>(
    set_expr: &mut SetExpr,
    params: &sqlx_data_params::Params,
    bind: &mut F,
) -> Result<()>
where
    F: FnMut(&FilterValue) -> Result<Expr>,
{
    match set_expr {
        SetExpr::Select(select) => process_select(select.as_mut(), params, bind),
        SetExpr::SetOperation { left, right, .. } => {
            // Recursively process both sides of UNION, INTERSECT, EXCEPT
            process_set_expr(left.as_mut(), params, bind)?;
            process_set_expr(right.as_mut(), params, bind)
        }
        SetExpr::Query(query) => {
            // Recursively process subquery
            process_set_expr(query.body.as_mut(), params, bind)
        }
        SetExpr::Values(_) => {
            // VALUES clause - nothing to process for filters
            Ok(())
        }
        SetExpr::Table(_) => {
            // Table reference - nothing to process for filters
            Ok(())
        }
        _ => {
            // Insert, Update, Delete, Merge - not applicable for filter processing
            Ok(())
        }
    }
}

/// Process SELECT statement with filters, search, and cursor pagination
fn process_select<F>(
    select: &mut sqlparser::ast::Select,
    params: &sqlx_data_params::Params,
    bind: &mut F,
) -> Result<()>
where
    F: FnMut(&FilterValue) -> Result<Expr>,
{
    // ----------------------------------------------------------
    // WHERE - separate vectors by type for intelligent grouping
    // ----------------------------------------------------------
    let filter_count = params.filters.as_ref().map_or(0, |f| f.filters.len());
    let search_count = if params.search.is_some() { 1 } else { 0 };
    let cursor_count = if matches!(
        params.pagination,
        Some(sqlx_data_params::Pagination::Cursor(_))
    ) {
        1
    } else {
        0
    };

    let mut where_filters = Vec::with_capacity(filter_count); // FilterParams
    let mut where_search = Vec::with_capacity(search_count); // SearchParams (0 or 1)
    let mut where_cursor = Vec::with_capacity(cursor_count); // CursorParams (0 or 1)

    // ----------------------------------------------------------
    // WHERE — filters (applied first)
    // ----------------------------------------------------------
    if let Some(filter_params) = &params.filters {
        for filter in &filter_params.filters {
            match filter.operator {
                FilterOperator::Eq => where_filters.push(eq(&filter.field, bind(&filter.value)?)),
                FilterOperator::Ne => where_filters.push(ne(&filter.field, bind(&filter.value)?)),
                FilterOperator::Gt => where_filters.push(gt(&filter.field, bind(&filter.value)?)),
                FilterOperator::Gte => where_filters.push(gte(&filter.field, bind(&filter.value)?)),
                FilterOperator::Lt => where_filters.push(lt(&filter.field, bind(&filter.value)?)),
                FilterOperator::Lte => where_filters.push(lte(&filter.field, bind(&filter.value)?)),
                FilterOperator::Like => {
                    where_filters.push(like(&filter.field, bind(&filter.value)?, filter.not))
                }
                FilterOperator::UnsafeLike => {
                    where_filters.push(unsafe_like(&filter.field, bind(&filter.value)?, filter.not))
                }
                FilterOperator::ILike => {
                    // Case-insensitive LIKE - database specific implementation
                    if cfg!(feature = "postgres") {
                        // PostgreSQL native ILIKE support
                        where_filters.push(ilike(&filter.field, bind(&filter.value)?, filter.not));
                    } else {
                        // For SQLite/MySQL: simulate ILIKE using LOWER() on both sides
                        let lower_col = lower_expr(id(&filter.field));

                        // Convert pattern to lowercase if it's a string (avoid clone for other types)
                        let lower_pattern_value = match &filter.value {
                            FilterValue::String(s) => FilterValue::String(s.to_lowercase().into()),
                            other => other.clone(),
                        };

                        where_filters.push(like_expr(lower_col, bind(&lower_pattern_value)?));
                    }
                }
                FilterOperator::Between => {
                    if let FilterValue::Array(arr) = &filter.value {
                        if arr.len() != 2 {
                            return Err(invalid_argument(format!(
                                "Between operator requires exactly 2 values, got {}",
                                arr.len()
                            )));
                        }
                        let low_ph = bind(&arr[0])?;
                        let high_ph = bind(&arr[1])?;
                        where_filters.push(between(&filter.field, low_ph, high_ph, filter.not));
                    } else {
                        return Err(invalid_argument(format!(
                            "Between operator requires an array of values, got {:?}",
                            filter.value
                        )));
                    }
                }
                FilterOperator::IsNull => where_filters.push(is_null(&filter.field)),
                FilterOperator::IsNotNull => where_filters.push(is_not_null(&filter.field)),
                FilterOperator::In => {
                    if let FilterValue::Array(arr) = &filter.value {
                        if arr.is_empty() {
                            return Err(invalid_argument(
                                "In operator requires at least 1 value, got empty array",
                            ));
                        }
                        let mut placeholders = Vec::with_capacity(arr.len());
                        for value in arr {
                            placeholders.push(bind(value)?);
                        }

                        where_filters.push(in_list(&filter.field, placeholders, filter.not));
                    } else {
                        return Err(invalid_argument(format!(
                            "In operator requires an array of values, got {:?}",
                            filter.value
                        )));
                    }
                }
                FilterOperator::NotIn => {
                    if let FilterValue::Array(arr) = &filter.value {
                        if arr.is_empty() {
                            return Err(invalid_argument(
                                "NotIn operator requires at least 1 value, got empty array",
                            ));
                        }
                        let mut placeholders = Vec::with_capacity(arr.len());
                        for value in arr {
                            placeholders.push(bind(value)?);
                        }

                        where_filters.push(in_list(&filter.field, placeholders, filter.not));
                    } else {
                        return Err(invalid_argument(format!(
                            "NotIn operator requires an array of values, got {:?}",
                            filter.value
                        )));
                    }
                }
                FilterOperator::Contains => {
                    // Contains should add wildcards: user provides "abc", we bind "%abc%"
                    let pattern_expr = match &filter.value {
                        FilterValue::String(s) => {
                            let mut wrapped = String::with_capacity(s.len() + 2);
                            wrapped.push('%');
                            wrapped.push_str(s);
                            wrapped.push('%');
                            bind(&FilterValue::String(wrapped.into()))?
                        }
                        _ => {
                            return Err(invalid_argument(
                                "Contains operator only supports string values",
                            ));
                        }
                    };

                    where_filters.push(like(&filter.field, pattern_expr, filter.not));
                }
            }
        }
    } // End of filters block

    // ----------------------------------------------------------
    // WHERE — search OR dynamic
    // ----------------------------------------------------------
    if let Some(search) = &params.search
        && !search.fields.is_empty()
        && !search.query.is_empty()
    {
        let pattern_string = if search.exact_match {
            search.query.clone()
        } else {
            // Only allocate once for the wrapped pattern
            format!("%{}%", search.query)
        };

        let mut ors = Vec::with_capacity(search.fields.len());

        for field in &search.fields {
            let pattern_value = FilterValue::String(pattern_string.clone().into());
            let ph = bind(&pattern_value)?;

            let expr = if search.case_sensitive {
                like(field, ph, false) // Search is never negated
            } else {
                let lower_col = lower_expr(id(field));
                let lower_pattern = lower_expr(ph);
                like_expr(lower_col, lower_pattern)
            };
            ors.push(expr);
        }

        if !ors.is_empty() {
            let combined = ors
                .into_iter()
                .reduce(or)
                .ok_or(invalid_argument("No search expressions found"))?;

            where_search.push(combined);
        }
    }

    // ----------------------------------------------------------
    // WHERE — cursor pagination (applied LAST to ensure proper ordering)
    // ----------------------------------------------------------
    if let Some(sqlx_data_params::Pagination::Cursor(cursor)) = &params.pagination
        && !cursor.is_empty()
    {
        // Check if cursor has decode error
        if cursor.has_error() {
            return Err(invalid_argument(format!(
                "Cursor decode failed: {}",
                cursor.error().unwrap_or("unknown error")
            )));
        }
        let sort_by = params.sort_by.as_ref().ok_or_else(|| {
            invalid_argument("Cursor pagination requires ORDER BY clause to be defined in sort_by")
        })?;

        let sort_fields: Vec<&str> = sort_by.sorts().iter().map(|s| s.field.as_str()).collect();

        if sort_fields.len() != cursor.len() {
            return Err(invalid_argument(format!(
                "Number of cursor values ({}) must match number of sort fields ({})",
                cursor.len(),
                sort_fields.len()
            )));
        }

        let cursor_direction = cursor
            .direction
            .as_ref()
            .ok_or_else(|| invalid_argument("Cursor pagination is missing direction"))?;

        let cursor_expr = if cursor.len() == 1 {
            let field = sort_fields[0];
            let value = bind(&cursor.values()[0])?;
            let sort = &sort_by.sorts()[0];

            let operator = match (cursor_direction, sort.is_asc()) {
                (sqlx_data_params::CursorDirection::After, true) => BinaryOperator::Gt, // ASC After  -> >
                (sqlx_data_params::CursorDirection::After, false) => BinaryOperator::Lt, // DESC After -> <
                (sqlx_data_params::CursorDirection::Before, true) => BinaryOperator::Lt, // ASC Before -> <
                (sqlx_data_params::CursorDirection::Before, false) => BinaryOperator::Gt, // DESC Before -> >
            };

            bin(operator, id(field), value)
        } else {
            build_or_based_cursor_condition(&sort_by.sorts(), cursor, cursor_direction, bind)?
        };

        where_cursor.push(cursor_expr);
    }

    // ----------------------------------------------------------
    // WHERE final combination with intelligent grouping
    // ----------------------------------------------------------
    let filters_expr = where_filters.into_iter().reduce(and);

    let dynamic_where = [
        filters_expr,
        where_search.into_iter().next(),
        where_cursor.into_iter().next(),
    ]
    .into_iter()
    .flatten()
    .reduce(|acc, expr| grouped_and(acc, expr))
    .map(|expr| {
        if select.selection.is_some() && contains_or(&expr) {
            Expr::Nested(Box::new(expr))
        } else {
            expr
        }
    });

    if let Some(combined) = dynamic_where {
        select.selection = match select.selection.take() {
            Some(existing) => Some(and(existing, combined)),
            None => Some(combined),
        };
    }

    Ok(())
}

fn build_or_based_cursor_condition<F>(
    sorts: &[sqlx_data_params::Sort],
    cursor_params: &sqlx_data_params::CursorParams,
    cursor_direction: &sqlx_data_params::CursorDirection,
    bind: &mut F,
) -> Result<Expr>
where
    F: FnMut(&FilterValue) -> Result<Expr>,
{
    build_cursor_condition_recursive(sorts, cursor_params, cursor_direction, 0, bind)
}

fn build_cursor_condition_recursive<F>(
    sorts: &[sqlx_data_params::Sort],
    cursor_params: &sqlx_data_params::CursorParams,
    cursor_direction: &sqlx_data_params::CursorDirection,
    index: usize,
    bind: &mut F,
) -> Result<Expr>
where
    F: FnMut(&FilterValue) -> Result<Expr>,
{
    if index >= sorts.len() {
        return Err(invalid_argument("Index out of bounds in cursor condition"));
    }

    let sort = &sorts[index];
    let cursor_value = &cursor_params.values()[index];
    let value = bind(cursor_value)?;

    let operator = match (cursor_direction, sort.is_asc()) {
        (sqlx_data_params::CursorDirection::After, true) => BinaryOperator::Gt, // ASC After  -> >
        (sqlx_data_params::CursorDirection::After, false) => BinaryOperator::Lt, // DESC After -> <
        (sqlx_data_params::CursorDirection::Before, true) => BinaryOperator::Lt, // ASC Before -> <
        (sqlx_data_params::CursorDirection::Before, false) => BinaryOperator::Gt, // DESC Before -> >
    };

    if index == sorts.len() - 1 {
        // Last field - simple comparison
        Ok(bin(operator, id(&sort.field), value))
    } else {
        // Recursive: field OP value OR (field = value AND rest)
        let rest_condition = build_cursor_condition_recursive(
            sorts,
            cursor_params,
            cursor_direction,
            index + 1,
            bind,
        )?;

        // Only wrap in parentheses if not the last field
        let wrapped_rest = if index + 1 == sorts.len() - 1 {
            rest_condition
        } else {
            Expr::Nested(Box::new(rest_condition))
        };

        // For MySQL, we need to bind the same value twice since it doesn't support parameter reuse
        #[cfg(feature = "mysql")]
        let eq_value = bind(cursor_value)?; // Bind the same value again for MySQL
        #[cfg(not(feature = "mysql"))]
        let eq_value = value.clone(); // Reuse the same placeholder for PostgreSQL/SQLite

        Ok(or(
            bin(operator, id(&sort.field), value),
            Expr::Nested(Box::new(and(
                bin(BinaryOperator::Eq, id(&sort.field), eq_value),
                wrapped_rest,
            ))),
        ))
    }
}

/// Build count SQL from raw SQL string - delegates to count module
pub fn build_count_query_from_sql(sql: &str) -> Result<Arc<String>> {
    super::count::build_count_query_from_sql(sql).map_err(|e| e.into())
}

fn make_generator(start: u8) -> impl FnMut() -> String {
    #[allow(unused_variables)]
    #[allow(unused_mut)]
    let mut n = start + 1;
    move || {
        #[cfg(feature = "mysql")]
        {
            crate::PLACEHOLDER().to_string()
        }
        #[cfg(any(feature = "postgres", feature = "sqlite"))]
        {
            let s = format!("{}{}", crate::PLACEHOLDER(), n);
            n += 1;
            s
        }
        #[cfg(not(any(feature = "sqlite", feature = "postgres", feature = "mysql")))]
        {
            let s = format!("{}{}", crate::PLACEHOLDER(), n);
            n += 1;
            s
        }
    }
}

#[inline]
fn id(name: &str) -> Expr {
    Expr::Identifier(Ident::new(name))
}

#[inline]
fn bin(op: BinaryOperator, left: Expr, right: Expr) -> Expr {
    Expr::BinaryOp {
        left: Box::new(left),
        op,
        right: Box::new(right),
    }
}

#[inline]
fn eq(col: &str, rhs: Expr) -> Expr {
    bin(BinaryOperator::Eq, id(col), rhs)
}

#[inline]
fn ne(col: &str, rhs: Expr) -> Expr {
    bin(BinaryOperator::NotEq, id(col), rhs)
}

#[inline]
fn gt(col: &str, rhs: Expr) -> Expr {
    bin(BinaryOperator::Gt, id(col), rhs)
}

#[inline]
fn lt(col: &str, rhs: Expr) -> Expr {
    bin(BinaryOperator::Lt, id(col), rhs)
}

#[inline]
fn gte(col: &str, rhs: Expr) -> Expr {
    bin(BinaryOperator::GtEq, id(col), rhs)
}

#[inline]
fn lte(col: &str, rhs: Expr) -> Expr {
    bin(BinaryOperator::LtEq, id(col), rhs)
}

#[inline]
fn like(col: &str, rhs: Expr, negated: bool) -> Expr {
    let escape_char = if cfg!(feature = "mysql") {
        None  // MySQL doesn't need ESCAPE clause
    } else {
        Some(Value::SingleQuotedString("\\".to_string())) // PostgreSQL/SQLite needs ESCAPE
    };
    Expr::Like {
        negated,
        expr: Box::new(id(col)),
        pattern: Box::new(rhs),
        escape_char,  // Safe: escapes % and _ automatically
        any: false,
    }
}

#[inline]
fn unsafe_like(col: &str, rhs: Expr, negated: bool) -> Expr {
    Expr::Like {
        negated,
        expr: Box::new(id(col)),
        pattern: Box::new(rhs),
        escape_char: None, // Unsafe: allows intentional wildcards
        any: false,
    }
}

#[inline]
fn ilike(col: &str, rhs: Expr, negated: bool) -> Expr {
    Expr::ILike {
        negated,
        expr: Box::new(id(col)),
        pattern: Box::new(rhs),
        escape_char: Some(Value::SingleQuotedString("\\".to_string())),
        any: false,
    }
}

#[inline]
fn in_list(col: &str, values: Vec<Expr>, negated: bool) -> Expr {
    Expr::InList {
        expr: Box::new(id(col)),
        list: values,
        negated,
    }
}

#[inline]
fn between(col: &str, low: Expr, high: Expr, negated: bool) -> Expr {
    Expr::Between {
        expr: Box::new(id(col)),
        negated,
        low: Box::new(low),
        high: Box::new(high),
    }
}

#[inline]
fn is_null(col: &str) -> Expr {
    Expr::IsNull(Box::new(id(col)))
}

#[inline]
fn is_not_null(col: &str) -> Expr {
    Expr::IsNotNull(Box::new(id(col)))
}

#[inline]
fn and(left: Expr, right: Expr) -> Expr {
    bin(BinaryOperator::And, left, right)
}

#[inline]
fn or(left: Expr, right: Expr) -> Expr {
    bin(BinaryOperator::Or, left, right)
}

// Helper function to wrap expressions that contain OR operations in parentheses
// when they need to be combined with AND to ensure correct precedence
#[inline]
fn grouped_and(left: Expr, right: Expr) -> Expr {
    let left_has_or = contains_or(&left);
    let right_has_or = contains_or(&right);

    let left_grouped = if left_has_or {
        Expr::Nested(Box::new(left))
    } else {
        left
    };

    let right_grouped = if right_has_or {
        Expr::Nested(Box::new(right))
    } else {
        right
    };

    bin(BinaryOperator::And, left_grouped, right_grouped)
}

// Check if an expression contains OR operations that need grouping
fn contains_or(expr: &Expr) -> bool {
    match expr {
        Expr::BinaryOp {
            op: BinaryOperator::Or,
            ..
        } => true,
        Expr::BinaryOp { left, right, .. } => contains_or(left) || contains_or(right),
        _ => false,
    }
}

#[inline]
fn like_expr(col_expr: Expr, pattern_expr: Expr) -> Expr {
    let escape_char = if cfg!(feature = "mysql") {
        None  // MySQL doesn't need ESCAPE clause
    } else {
        Some(Value::SingleQuotedString("\\".to_string())) // PostgreSQL/SQLite needs ESCAPE
    };
    Expr::Like {
        negated: false,
        expr: Box::new(col_expr),
        pattern: Box::new(pattern_expr),
        escape_char,
        any: false,
    }
}

// Helper to create LOWER() function expressions efficiently
#[inline]
fn lower_expr(expr: Expr) -> Expr {
    Expr::Function(Function {
        name: ObjectName(vec![ObjectNamePart::Identifier(Ident::new("LOWER"))]),
        args: FunctionArguments::List(FunctionArgumentList {
            duplicate_treatment: None,
            args: vec![FunctionArg::Unnamed(FunctionArgExpr::Expr(expr))],
            clauses: vec![],
        }),
        parameters: FunctionArguments::None,
        over: None,
        filter: None,
        null_treatment: None,
        within_group: vec![],
        uses_odbc_syntax: false,
    })
}

#[inline]
fn value_expr(value: Value) -> Expr {
    Expr::Value(sqlparser::ast::ValueWithSpan {
        value,
        span: sqlparser::tokenizer::Span::empty(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx_data_params::{FilterValue, ParamsBuilder};

    #[tokio::test]
    async fn test_initial_params_order() {
        // Test with initial parameters from function signature
        let initial_binds = vec![
            FilterValue::String("john".into()), // $1
            FilterValue::Int(25),               // $2
        ];

        let params = sqlx_data_params::ParamsBuilder::new()
            .filter()
            .eq("active", sqlx_data_params::FilterValue::Bool(true)) // Should be $3
            .done()
            .build();

        let built = build_dynamic_sql(
            "SELECT * FROM users WHERE name = $1 AND age = $2",
            &params,
            initial_binds,
        )
        .unwrap();
        let sql = built.sql.as_ref();
        // Check SQL
        assert!(sql.contains("$1")); // Original param
        assert!(sql.contains("$2")); // Original param
        assert!(sql.contains("$3")); // New filter param

        // Check that we have the right number of arguments
        assert_eq!(built.bind_values.len(), 3);
    }

    #[tokio::test]
    async fn test_simple_select_no_params() {
        let sql = "SELECT * FROM users";
        let params = ParamsBuilder::new().build();

        let built = build_dynamic_sql(sql, &params, vec![]).unwrap();
        let result_sql = built.sql.as_ref();
        assert_eq!(result_sql, "SELECT * FROM users");
        assert_eq!(built.bind_values.len(), 0);
    }

    #[test]
    fn test_add_where_filter() {
        let sql = "SELECT * FROM users";
        let params = ParamsBuilder::new()
            .filter()
            .gt("age", FilterValue::Int(25))
            .done()
            .build();

        let built = build_dynamic_sql(sql, &params, vec![]).unwrap();
        let result_sql = built.sql.as_ref();

        // Test SQL generation without database execution
        assert!(result_sql.contains("WHERE"));
        assert!(result_sql.contains("age >"));
        assert_eq!(built.bind_values.len(), 1);

        // Verify the exact SQL structure
        assert_eq!(result_sql, "SELECT * FROM users WHERE age > $1");
    }

    #[test]
    fn test_add_where_to_existing_where() {
        let sql = "SELECT * FROM users WHERE active = true";
        let params = ParamsBuilder::new()
            .filter()
            .gt("age", FilterValue::Int(25))
            .done()
            .build();

        let built = build_dynamic_sql(sql, &params, vec![]).unwrap();
        let result_sql = built.sql.as_ref();

        // Test SQL generation for existing WHERE clause
        assert!(result_sql.contains("WHERE"));
        assert!(result_sql.contains("active = true"));
        assert!(result_sql.contains("age >"));
        assert!(result_sql.contains("AND"));
        assert_eq!(built.bind_values.len(), 1);

        // Verify the exact SQL structure with AND concatenation
        assert_eq!(
            result_sql,
            "SELECT * FROM users WHERE active = true AND age > $1"
        );
    }

    #[test]
    fn test_multiple_filters() {
        let sql = "SELECT * FROM users";
        let params = ParamsBuilder::new()
            .filter()
            .gt("age", FilterValue::Int(25))
            .like("name", "%john%")
            .done()
            .build();

        let built = build_dynamic_sql(sql, &params, vec![]).unwrap();
        let result_sql = built.sql.as_ref();

        // Test SQL generation with multiple filters
        assert!(result_sql.contains("WHERE"));
        assert!(result_sql.contains("age >"));
        assert!(result_sql.contains("name LIKE"));
        assert!(result_sql.contains("AND"));
        assert_eq!(built.bind_values.len(), 2);

        // Verify the exact SQL structure with multiple filters (includes ESCAPE clause for LIKE)
        assert_eq!(
            result_sql,
            "SELECT * FROM users WHERE age > $1 AND name LIKE $2 ESCAPE '\\'"
        );
    }

    #[tokio::test]
    async fn test_between_filter() {
        let sql = "SELECT * FROM users";
        let params = ParamsBuilder::new()
            .filter()
            .between("age", FilterValue::Int(20), FilterValue::Int(30))
            .done()
            .build();

        let built = build_dynamic_sql(sql, &params, vec![]).unwrap();
        let result_sql = built.sql.as_ref();

        assert!(result_sql.contains("WHERE"));
        assert!(result_sql.contains("age BETWEEN"));
        assert_eq!(built.bind_values.len(), 2);
        // Cannot directly test argument values with SQLx Arguments
        // assert_eq!(args[0], FilterValue::Int(20));
        // assert_eq!(args[1], FilterValue::Int(30));
    }

    #[tokio::test]
    async fn test_in_filter() {
        let sql = "SELECT * FROM users";
        let params = ParamsBuilder::new()
            .filter()
            .in_values(
                "status",
                vec![
                    FilterValue::String("active".into()),
                    FilterValue::String("pending".into()),
                ],
            )
            .done()
            .build();

        let built = build_dynamic_sql(sql, &params, vec![]).unwrap();
        let result_sql = built.sql.as_ref();

        println!("Generated SQL: {}", result_sql);
        assert!(result_sql.contains("WHERE"));
        assert!(result_sql.contains("status IN"));
        assert_eq!(built.bind_values.len(), 2);
        // Cannot directly test argument values with SQLx Arguments
        // assert_eq!(args[0], FilterValue::String("active".to_string()));
        // assert_eq!(args[1], FilterValue::String("pending".to_string()));
    }

    #[tokio::test]
    async fn test_search_functionality() {
        let sql = "SELECT * FROM users";
        let params = ParamsBuilder::new()
            .search()
            .query("john")
            .fields(["name", "email"])
            .case_sensitive(true)
            .done()
            .build();

        let built = build_dynamic_sql(sql, &params, vec![]).unwrap();
        let result_sql = built.sql.as_ref();

        assert!(result_sql.contains("WHERE"));
        assert!(result_sql.contains("name LIKE"));
        assert!(result_sql.contains("email LIKE"));
        assert!(result_sql.contains("OR"));
        assert_eq!(built.bind_values.len(), 2);
        // Cannot directly test argument values with SQLx Arguments
        // assert_eq!(args[0], FilterValue::String("%john%".to_string()));
        // assert_eq!(args[1], FilterValue::String("%john%".to_string()));
    }

    #[tokio::test]
    async fn test_case_insensitive_search() {
        let sql = "SELECT * FROM users";
        let params = ParamsBuilder::new()
            .search()
            .query("JOHN")
            .fields(["name"])
            .case_sensitive(false)
            .done()
            .build();

        let built = build_dynamic_sql(sql, &params, vec![]).unwrap();
        let result_sql = built.sql.as_ref();

        assert!(result_sql.contains("WHERE"));
        assert!(result_sql.contains("LOWER(name) LIKE LOWER"));
        assert_eq!(built.bind_values.len(), 1);
        // Cannot directly test argument values with SQLx Arguments
        // assert_eq!(args[0], FilterValue::String("%JOHN%".to_string()));
    }

    #[tokio::test]
    async fn test_sorting() {
        let sql = "SELECT * FROM users";
        let params = ParamsBuilder::new()
            .sort()
            .asc("name")
            .desc("age")
            .done()
            .build();

        let built = build_dynamic_sql(sql, &params, vec![]).unwrap();
        let result_sql = built.sql.as_ref();
        println!("{}", result_sql);
        assert!(result_sql.contains("ORDER BY"));
        assert!(result_sql.contains("name ASC"));
        assert!(result_sql.contains("age DESC"));
        assert_eq!(built.bind_values.len(), 0);
    }

    #[tokio::test]
    async fn test_nulls_ordering() {
        let base_sql = "SELECT * FROM users";

        // Test NULLS FIRST
        let params = ParamsBuilder::new()
            .sort()
            .desc("name")
            .nulls_first()
            .done()
            .build();

        let built = build_dynamic_sql(base_sql, &params, vec![]).unwrap();
        let sql = built.sql.as_ref();
        assert!(sql.contains("name DESC NULLS FIRST"));

        // Test NULLS LAST
        let params = ParamsBuilder::new()
            .sort()
            .asc("age")
            .nulls_last()
            .done()
            .build();

        let built = build_dynamic_sql(base_sql, &params, vec![]).unwrap();
        let sql = built.sql.as_ref();
        assert!(sql.contains("age ASC NULLS LAST"));

        // Test Default (no nulls clause) - demonstrates when no nulls ordering is set, no NULLS clause appears
        let params = ParamsBuilder::new().sort().desc("status").done().build();

        let built = build_dynamic_sql(base_sql, &params, vec![]).unwrap();
        let sql = built.sql.as_ref();
        assert!(sql.contains("status DESC"));
        assert!(!sql.contains("status DESC NULLS"));
    }

    #[test]
    fn test_pagination() {
        let sql = "SELECT * FROM users";
        let params = ParamsBuilder::new().serial().page(2, 10).done().build();

        let built = build_dynamic_sql(sql, &params, vec![]).unwrap();
        let result_sql = built.sql.as_ref();

        // Test SQL generation for pagination
        assert!(result_sql.contains("LIMIT"));
        assert!(result_sql.contains("OFFSET"));

        // LIMIT/OFFSET are embedded as literals, not as bind placeholders
        assert_eq!(built.bind_values.len(), 0);

        // Verify the SQL structure for page 2 with size 10
        // Page 2 with size 10 = LIMIT 10 OFFSET 10
        assert!(result_sql.contains("LIMIT 10"));
        assert!(result_sql.contains("OFFSET 10"));
    }

    #[tokio::test]
    async fn test_vec_ids_filter() {
        // Simulates: fn get_produtos(pool: &PgPool, ids: Vec<i32>) -> Result<Vec<Produto>, sqlx::Error>
        let sql = "SELECT * FROM produtos";
        let ids = vec![1, 5, 10, 25];

        // Converting Vec<i32> to Vec<FilterValue>
        let filter_values: Vec<FilterValue> =
            ids.into_iter().map(|id| FilterValue::from(id)).collect();

        let params = ParamsBuilder::new()
            .filter()
            .in_values("id", filter_values)
            .done()
            .build();

        let built = build_dynamic_sql(sql, &params, vec![]).unwrap();
        let result_sql = built.sql.as_ref();

        assert!(result_sql.contains("WHERE"));
        assert!(result_sql.contains("id IN"));
        assert_eq!(built.bind_values.len(), 4); // 4 IDs

        // Verify SQL has correct placeholders
        assert!(result_sql.contains("($1, $2, $3, $4)"));
    }

    #[tokio::test]
    async fn test_invalid_between_arrays() {
        use sqlx_data_params::{Filter, FilterParams, Params};

        let sql = "SELECT * FROM users";

        // Test empty array - create Params manually because builder doesn't allow empty arrays
        let params = Params {
            pagination: None,
            filters: Some(FilterParams {
                filters: vec![Filter::new(
                    "age",
                    FilterOperator::Between,
                    FilterValue::Array(vec![]),
                )],
            }),
            search: None,
            sort_by: None,
            limit: None,
            offset: None,
        };

        let result = build_dynamic_sql(sql, &params, vec![]);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Between operator requires exactly 2 values, got 0")
        );

        // Test single value array
        let params = Params {
            filters: Some(FilterParams {
                filters: vec![Filter::new(
                    "age",
                    FilterOperator::Between,
                    FilterValue::Array(vec![FilterValue::Int(20)]),
                )],
            }),
            search: None,
            pagination: None,
            sort_by: None,
            limit: None,
            offset: None,
        };

        let result = build_dynamic_sql(sql, &params, vec![]);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Between operator requires exactly 2 values, got 1")
        );

        // Test too many values
        let params = Params {
            filters: Some(FilterParams {
                filters: vec![Filter::new(
                    "age",
                    FilterOperator::Between,
                    FilterValue::Array(vec![
                        FilterValue::Int(20),
                        FilterValue::Int(30),
                        FilterValue::Int(40),
                    ]),
                )],
            }),
            search: None,
            sort_by: None,
            limit: None,
            offset: None,
            pagination: None,
        };

        let result = build_dynamic_sql(sql, &params, vec![]);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Between operator requires exactly 2 values, got 3")
        );

        // Test non-array value for Between
        let params = Params {
            filters: Some(FilterParams {
                filters: vec![Filter::new(
                    "age",
                    FilterOperator::Between,
                    FilterValue::Int(25),
                )],
            }),
            search: None,
            sort_by: None,
            limit: None,
            offset: None,
            pagination: None,
        };

        let result = build_dynamic_sql(sql, &params, vec![]);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Between operator requires an array of values")
        );
    }

    #[tokio::test]
    async fn test_invalid_in_arrays() {
        use sqlx_data_params::{Filter, FilterParams, Params};

        let sql = "SELECT * FROM users";

        // Test empty array for In
        let params = Params {
            pagination: None,
            filters: Some(FilterParams {
                filters: vec![Filter::new(
                    "status",
                    FilterOperator::In,
                    FilterValue::Array(vec![]),
                )],
            }),
            search: None,
            sort_by: None,
            limit: None,
            offset: None,
        };

        let result = build_dynamic_sql(sql, &params, vec![]);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("In operator requires at least 1 value, got empty array")
        );

        // Test non-array value for In
        let params = Params {
            pagination: None,
            filters: Some(FilterParams {
                filters: vec![Filter::new(
                    "status",
                    FilterOperator::In,
                    FilterValue::String("active".into()),
                )],
            }),
            search: None,
            sort_by: None,
            limit: None,
            offset: None,
        };

        let result = build_dynamic_sql(sql, &params, vec![]);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("In operator requires an array of values")
        );

        // Test empty array for NotIn - create manually because builder doesn't allow empty arrays
        let params = Params {
            pagination: None,
            filters: Some(FilterParams {
                filters: vec![Filter::new(
                    "status",
                    FilterOperator::NotIn,
                    FilterValue::Array(vec![]),
                )],
            }),
            search: None,
            sort_by: None,
            limit: None,
            offset: None,
        };

        let result = build_dynamic_sql(sql, &params, vec![]);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("NotIn operator requires at least 1 value, got empty array")
        );

        // Test non-array value for NotIn
        let params = Params {
            pagination: None,
            filters: Some(FilterParams {
                filters: vec![Filter::new(
                    "status",
                    FilterOperator::NotIn,
                    FilterValue::String("inactive".into()),
                )],
            }),
            search: None,
            sort_by: None,
            limit: None,
            offset: None,
        };

        let result = build_dynamic_sql(sql, &params, vec![]);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("NotIn operator requires an array of values")
        );
    }

    #[tokio::test]
    async fn test_pagination_underflow_protection() {
        use sqlx_data_params::{Pagination, Params, SerialParams};

        let sql = "SELECT * FROM users";

        // Test manual construction with page 0 (potential underflow)
        let serial_params = SerialParams::new(0, 10);
        let params = Params {
            pagination: Some(Pagination::Serial(serial_params)),
            filters: None,
            search: None,
            sort_by: None,
            limit: Some(sqlx_data_params::LimitParam(10)),
            offset: Some(sqlx_data_params::OffsetParam(0)),
        };

        let built = build_dynamic_sql(sql, &params, vec![]).unwrap();
        let result_sql = built.sql.as_ref();

        // Should generate LIMIT 10 OFFSET 0 (not OFFSET 4294967295)
        assert!(result_sql.contains("LIMIT"));
        assert!(result_sql.contains("OFFSET"));
        // LIMIT/OFFSET are embedded as literals, not as bind placeholders
        assert_eq!(built.bind_values.len(), 0);

        println!("Generated SQL with page 0: {}", result_sql);
    }

    //TODO Replace this tests with FilterParams to FilterBuilder
    #[tokio::test]
    async fn test_like_vs_ilike_difference() {
        use sqlx_data_params::{Filter, FilterParams, Params};

        let sql = "SELECT * FROM users";

        // Test case-sensitive LIKE
        let params = Params {
            pagination: None,
            filters: Some(FilterParams {
                filters: vec![Filter::new(
                    "name",
                    FilterOperator::Like,
                    FilterValue::String("John%".into()),
                )],
            }),
            search: None,
            sort_by: None,
            limit: None,
            offset: None,
        };

        let like_built = build_dynamic_sql(sql, &params, vec![]).unwrap();
        let like_sql = like_built.sql.as_ref();
        println!("LIKE SQL: {}", like_sql);

        // Test case-insensitive ILIKE
        let params = Params {
            pagination: None,
            filters: Some(FilterParams {
                filters: vec![Filter::new(
                    "name",
                    FilterOperator::ILike,
                    FilterValue::String("John%".into()),
                )],
            }),
            search: None,
            sort_by: None,
            limit: None,
            offset: None,
        };

        let ilike_built = build_dynamic_sql(sql, &params, vec![]).unwrap();
        let ilike_sql = ilike_built.sql.as_ref();
        println!("ILIKE SQL: {}", ilike_sql);

        // LIKE should be case-sensitive (just name LIKE $1)
        assert!(like_sql.contains("name LIKE"));
        assert!(!like_sql.contains("LOWER"));
        assert_eq!(like_built.bind_values.len(), 1);

        // ILIKE behavior is database-specific
        #[cfg(feature = "postgres")]
        {
            // PostgreSQL uses native ILIKE
            assert!(ilike_sql.contains("ILIKE"));
        }
        #[cfg(not(feature = "postgres"))]
        {
            // SQLite/MySQL: simulate ILIKE using LOWER() on both sides
            assert!(ilike_sql.contains("LOWER(name) LIKE"));
            assert!(!ilike_sql.contains("LOWER($")); // Pattern is lowercased before bind, not in SQL
        }
        assert_eq!(ilike_built.bind_values.len(), 1);
    }

    #[tokio::test]
    async fn test_like_ilike_with_builder() {
        let sql = "SELECT * FROM users";

        // Test using builder for LIKE
        let like_params = ParamsBuilder::new()
            .filter()
            .like("name", "John%")
            .done()
            .build();

        let like_built = build_dynamic_sql(sql, &like_params, vec![]).unwrap();
        let like_sql = like_built.sql.as_ref();

        // Test using builder for ILIKE
        let ilike_params = ParamsBuilder::new()
            .filter()
            .ilike("name", "John%")
            .done()
            .build();

        let ilike_built = build_dynamic_sql(sql, &ilike_params, vec![]).unwrap();
        let ilike_sql = ilike_built.sql.as_ref();

        // Verify the difference
        assert!(like_sql.contains("name LIKE"));
        assert!(!like_sql.contains("LOWER(name)"));

        // ILIKE behavior is database-specific
        #[cfg(feature = "postgres")]
        {
            // PostgreSQL uses native ILIKE
            assert!(ilike_sql.contains("ILIKE"));
        }
        #[cfg(not(feature = "postgres"))]
        {
            // SQLite/MySQL: simulate ILIKE using LOWER()
            assert!(ilike_sql.contains("LOWER(name) LIKE"));
        }

        println!("Builder LIKE SQL: {}", like_sql);
        println!("Builder ILIKE SQL: {}", ilike_sql);
    }

    #[tokio::test]
    async fn test_complex_query() {
        let sql = "SELECT id, name, age FROM users WHERE active = true";
        let params = ParamsBuilder::new()
            .filter()
            .between("age", FilterValue::Int(20), FilterValue::Int(30))
            .like("department", "%engineering%")
            .done()
            .search()
            .query("john")
            .fields(["name", "email"])
            .case_sensitive(true)
            .done()
            .sort()
            .desc("age")
            .asc("name")
            .done()
            .serial()
            .page(1, 5)
            .done()
            .build();

        let built = build_dynamic_sql(sql, &params, vec![]).unwrap();
        let result_sql = built.sql.as_ref();

        println!("{}", result_sql);

        // Should have WHERE with existing condition AND new conditions
        assert!(result_sql.contains("WHERE"));
        assert!(result_sql.contains("active = true"));
        assert!(result_sql.contains("age BETWEEN"));
        assert!(result_sql.contains("department LIKE"));
        assert!(result_sql.contains("name LIKE"));
        assert!(result_sql.contains("email LIKE"));
        assert!(result_sql.contains("AND"));
        assert!(result_sql.contains("OR"));

        // Should have ORDER BY
        assert!(result_sql.contains("ORDER BY"));
        assert!(result_sql.contains("age DESC"));
        assert!(result_sql.contains("name ASC"));

        // Should have LIMIT and OFFSET
        assert!(result_sql.contains("LIMIT"));

        // Check binds: 2 for between + 1 for like + 2 for search = 5
        // Note: LIMIT/OFFSET are embedded as literals, not as bind placeholders
        assert_eq!(built.bind_values.len(), 5);
    }

    #[tokio::test]
    async fn test_existing_order_by_replaced() {
        let sql = "SELECT * FROM users ORDER BY created_at DESC";
        let params = ParamsBuilder::new().sort().asc("name").done().build();

        let built = build_dynamic_sql(sql, &params, vec![]).unwrap();
        let result_sql = built.sql.as_ref();

        // The new ORDER BY should replace the existing one
        assert!(result_sql.contains("ORDER BY"));
        assert!(result_sql.contains("name ASC"));
        assert_eq!(built.bind_values.len(), 0);
    }

    #[tokio::test]
    async fn test_existing_limit_replaced() {
        let sql = "SELECT * FROM users LIMIT 100";
        let params = ParamsBuilder::new().serial().page(1, 5).done().build();

        let built = build_dynamic_sql(sql, &params, vec![]).unwrap();
        let result_sql = built.sql.as_ref();

        // Should override existing LIMIT with new pagination
        assert!(result_sql.contains("LIMIT"));
        // LIMIT/OFFSET are embedded as literals, not as bind placeholders
        assert_eq!(built.bind_values.len(), 0);
        // Verify the new LIMIT value is in the SQL
        assert!(result_sql.contains("LIMIT 5"));
    }

    #[tokio::test]
    async fn test_null_and_not_null_filters() {
        let sql = "SELECT * FROM users";
        let params = ParamsBuilder::new()
            .filter()
            .is_null("deleted_at")
            .is_not_null("email")
            .done()
            .build();

        let built = build_dynamic_sql(sql, &params, vec![]).unwrap();
        let result_sql = built.sql.as_ref();

        assert!(result_sql.contains("WHERE"));
        assert!(result_sql.contains("deleted_at IS NULL"));
        assert!(result_sql.contains("email IS NOT NULL"));
        assert!(result_sql.contains("AND"));
        assert_eq!(built.bind_values.len(), 0); // NULL checks don't need binds
    }

    #[tokio::test]
    async fn test_contains_filter() {
        let sql = "SELECT * FROM users";
        let params = ParamsBuilder::new()
            .filter()
            .contains("tags", FilterValue::String("admin".into()))
            .done()
            .build();

        let built = build_dynamic_sql(sql, &params, vec![]).unwrap();
        let result_sql = built.sql.as_ref();

        assert!(result_sql.contains("WHERE"));
        assert!(result_sql.contains("tags LIKE"));
        assert_eq!(built.bind_values.len(), 1);
        // Cannot directly test argument values with SQLx Arguments
        // assert_eq!(args[0], FilterValue::String("%admin%".to_string()));
    }

    #[tokio::test]
    async fn test_safe_vs_unsafe_like() {
        let sql = "SELECT * FROM files";

        // Test safe LIKE (with escaping)
        let safe_params = ParamsBuilder::new()
            .filter()
            .like("filename", "test_file%.txt")
            .done()
            .build();

        // Test unsafe LIKE (allows wildcards)
        let unsafe_params = ParamsBuilder::new()
            .filter()
            .like_pattern("pattern", "user_%")
            .done()
            .build();

        let safe_built = build_dynamic_sql(sql, &safe_params, vec![]).unwrap();
        let safe_sql = safe_built.sql.as_ref();
        let unsafe_built = build_dynamic_sql(sql, &unsafe_params, vec![]).unwrap();
        let unsafe_sql = unsafe_built.sql.as_ref();

        println!("Safe LIKE:   {}", safe_sql);
        println!("Unsafe LIKE: {}", unsafe_sql);

        // Safe LIKE should have ESCAPE clause
        assert!(safe_sql.contains("ESCAPE"));
        // Unsafe LIKE should NOT have ESCAPE clause
        assert!(!unsafe_sql.contains("ESCAPE"));

        assert!(safe_sql.contains("filename LIKE"));
        assert!(unsafe_sql.contains("pattern LIKE"));
    }

    #[tokio::test]
    async fn test_negated_filters() {
        let sql = "SELECT * FROM users";

        // Test negated filters with fluent API
        #[rustfmt::skip]
        let params = ParamsBuilder::new()
            .filter()
                .like("name", "John")
                .not() // NOT LIKE
                .like_pattern("pattern", "user_%")
                .not() // NOT LIKE (unsafe)
                .r#in("age", vec![FilterValue::Int(25), FilterValue::Int(30)])
                .not() // NOT IN
                .between("score", FilterValue::Int(80), FilterValue::Int(100))
                .not() // NOT BETWEEN
                .done()
            .build();

        let built = build_dynamic_sql(sql, &params, vec![]).unwrap();
        let result_sql = built.sql.as_ref();

        println!("Negated filters: {}", result_sql);

        // Verify all negations are present
        assert!(result_sql.contains("name NOT LIKE"));
        assert!(result_sql.contains("pattern NOT LIKE"));
        assert!(result_sql.contains("age NOT IN"));
        assert!(result_sql.contains("score NOT BETWEEN"));

        // Safe LIKE should still have ESCAPE even when negated
        assert!(result_sql.contains("ESCAPE"));
    }

    #[tokio::test]
    async fn test_where_precedence_with_parentheses() {
        // Test proper behavior when user explicitly uses parentheses
        let sql = "SELECT * FROM users WHERE (status = 'active' OR (role = 'admin' AND department = 'IT'))";
        let params = ParamsBuilder::new()
            .filter()
            .eq("age", FilterValue::Int(25))
            .done()
            .build();

        let built = build_dynamic_sql(sql, &params, vec![]).unwrap();
        let result_sql = built.sql.as_ref();

        println!("Original: {}", sql);
        println!("Result:   {}", result_sql);

        // With explicit parentheses, the result should correctly preserve grouping:
        // WHERE (status = 'active' OR (role = 'admin' AND department = 'IT')) AND age = ?
        assert!(result_sql.contains("WHERE"));
        assert!(result_sql.contains("age"));
        assert!(result_sql.contains("(status"));
    }

    #[tokio::test]
    async fn test_extremely_complex_query_with_all_features() {
        // A very complex query with CTE, subqueries, functions, group by, having, order by, limit
        let complex_sql = r#"
            WITH user_stats AS (
                SELECT
                    id,
                    name,
                    age,
                    COALESCE(email, 'unknown@domain.com') as email,
                    COUNT(*) OVER (PARTITION BY age) as age_group_count,
                    ROW_NUMBER() OVER (ORDER BY age DESC, name ASC) as user_rank
                FROM users
                WHERE age >= 18
                    AND UPPER(name) != 'ADMIN'
                    AND created_at > DATE('2020-01-01')
            ),
            department_summary AS (
                SELECT
                    department,
                    AVG(salary) as avg_salary,
                    COUNT(*) as employee_count
                FROM users u
                WHERE u.status IN ('active', 'pending')
                GROUP BY department
                HAVING COUNT(*) > 5
            )
            SELECT
                us.id,
                us.name,
                us.age,
                us.email,
                us.age_group_count,
                us.user_rank,
                ds.avg_salary,
                ds.employee_count,
                CASE
                    WHEN us.age >= 65 THEN 'Senior'
                    WHEN us.age >= 40 THEN 'Mid-Career'
                    WHEN us.age >= 25 THEN 'Young Professional'
                    ELSE 'Entry Level'
                END as career_stage,
                (SELECT COUNT(*) FROM users u2 WHERE u2.manager_id = us.id) as direct_reports
            FROM user_stats us
            LEFT JOIN department_summary ds ON us.department = ds.department
            WHERE us.user_rank <= 100
                AND LENGTH(us.name) > 2
                AND us.email LIKE '%@company.com'
            GROUP BY us.id, us.name, us.age, us.email, us.age_group_count, us.user_rank, ds.avg_salary, ds.employee_count
            HAVING AVG(us.age_group_count) > 1
            ORDER BY us.user_rank ASC, ds.avg_salary DESC NULLS LAST
            LIMIT 50 OFFSET 0
        "#;

        #[rustfmt::skip]
        let params = ParamsBuilder::new()
            .filter()
                .between("us.age", FilterValue::Int(25), FilterValue::Int(55)) // Add age filter
                .like("us.name", "John")
                .not() // NOT LIKE to exclude Johns
                .like_pattern("us.email", "%@%.com") // Email filter with wildcard
                .r#in(
                    "ds.department",
                    vec![
                        FilterValue::String("Engineering".into()),
                        FilterValue::String("Marketing".into()),
                        FilterValue::String("Sales".into()),
                    ]) // Department filter
                .gte("ds.avg_salary", FilterValue::Float(50000.0)) // Minimum salary filter
                .is_not_null("us.email") // Email cannot be null
                .contains("us.department", FilterValue::String("Tech".into())) // Department contains "Tech"
                .done()
            .search()
                .query("senior developer manager") // Search across multiple fields
                .fields(["us.name", "us.title", "ds.department"])
                .case_sensitive(false)
                .done()
            .sort()
                .desc("ds.avg_salary")
                .nulls_first() // Override existing ORDER BY
                .asc("us.age")
                .desc("us.user_rank")
                .nulls_last()
                .done()
            .serial()
                .page(3, 25) // Override existing LIMIT/OFFSET
                .done()
            .build();

        let built = build_dynamic_sql(complex_sql, &params, vec![]).unwrap();
        let result_sql = built.sql.as_ref();

        println!("\n=== ORIGINAL COMPLEX SQL ===");
        println!("{}", complex_sql);
        println!("\n=== MODIFIED SQL WITH DYNAMIC PARAMS ===");
        println!("{}", result_sql);
        println!("\n=== ARGUMENT COUNT ===");
        println!("Total arguments bound: {}", built.bind_values.len());

        // Basic SQL structure verifications
        assert!(result_sql.contains("WITH user_stats AS")); // CTE preserved
        assert!(result_sql.contains("department_summary AS")); // Second CTE preserved
        assert!(result_sql.contains("COALESCE")); // Function preserved
        assert!(result_sql.contains("COUNT(*) OVER")); // Window function preserved
        assert!(result_sql.contains("ROW_NUMBER() OVER")); // Window function preserved
        assert!(result_sql.contains("LEFT JOIN")); // JOIN preserved
        assert!(result_sql.contains("CASE WHEN")); // CASE expression preserved
        assert!(result_sql.contains("SELECT COUNT(*) FROM users u2")); // Subquery preserved

        // Added filters verifications
        assert!(result_sql.contains("us.age BETWEEN")); // Between filter
        assert!(result_sql.contains("us.name NOT LIKE")); // Negated LIKE
        assert!(result_sql.contains("us.email LIKE")); // Unsafe LIKE
        assert!(result_sql.contains("ds.department IN")); // IN filter
        assert!(result_sql.contains("ds.avg_salary >=")); // GTE filter
        assert!(result_sql.contains("us.email IS NOT NULL")); // IS NOT NULL
        assert!(result_sql.contains("us.department LIKE")); // Contains filter

        // Search verifications (OR conditions)
        assert!(result_sql.contains("LOWER(us.name) LIKE")); // Case insensitive search
        assert!(result_sql.contains("LOWER(us.title) LIKE")); // Multiple field search
        assert!(result_sql.contains("LOWER(ds.department) LIKE")); // Search across joined table

        // GROUP BY preserved verifications
        assert!(result_sql.contains("GROUP BY us.id, us.name")); // Original GROUP BY preserved

        // HAVING preserved verifications
        assert!(result_sql.contains("HAVING")); // Original HAVING preserved

        // ORDER BY replacement verifications
        assert!(result_sql.contains("ORDER BY"));
        assert!(result_sql.contains("ds.avg_salary DESC NULLS FIRST")); // New ordering
        assert!(result_sql.contains("us.age ASC")); // New ordering
        assert!(result_sql.contains("us.user_rank DESC NULLS LAST")); // New ordering
        // The original ORDER BY should have been replaced
        assert!(!result_sql.contains("us.user_rank ASC, ds.avg_salary DESC NULLS LAST"));

        // LIMIT/OFFSET replacement verifications
        assert!(result_sql.contains("LIMIT"));
        assert!(result_sql.contains("OFFSET"));
        // The original LIMIT (50) should have been replaced by the new one (25)
        // The original OFFSET (0) should have been replaced by the calculated one for page 3

        // Verify we have the correct binds
        // 2 (between) + 1 (like negated) + 1 (unsafe like) + 3 (in) + 1 (gte) + 1 (contains) + 3 (search) = 12
        // Note: LIMIT/OFFSET are embedded as literals, not as bind placeholders
        assert_eq!(built.bind_values.len(), 12);

        // Verify that WHERE conditions are combined correctly with AND
        let where_count = result_sql.matches(" AND ").count();
        assert!(where_count >= 8); // Should have many AND conditions due to multiple filters

        // Verify that search fields are combined with OR
        let search_or_count = result_sql.matches("LOWER(us.name) LIKE").count()
            + result_sql.matches("LOWER(us.title) LIKE").count()
            + result_sql.matches("LOWER(ds.department) LIKE").count();
        assert_eq!(search_or_count, 3); // Should have 3 search fields
    }

    #[tokio::test]
    async fn test_cursor_pagination_simple() {
        let sql = "SELECT * FROM users";

        // Test simple cursor using ParamsBuilder - needs ORDER BY for cursor
        let params = ParamsBuilder::new()
            .sort()
            .asc("id")
            .done()
            .cursor()
            .after(FilterValue::Int(123))
            .done()
            .limit(20)
            .build();

        let built = build_dynamic_sql(sql, &params, vec![]).unwrap();
        let result_sql = built.sql.as_ref();

        println!("Simple cursor SQL: {}", result_sql);

        // Should generate: SELECT * FROM users WHERE id > $1 LIMIT 20
        assert!(result_sql.contains("WHERE"));
        assert!(result_sql.contains("id >"));
        assert!(result_sql.contains("LIMIT"));
        // LIMIT is embedded as literal, cursor value is a bind
        assert_eq!(built.bind_values.len(), 1); // cursor value only
    }

    #[tokio::test]
    async fn test_cursor_with_existing_where_and_filters() {
        // SQL already has WHERE - let's test if cursor + filters combine well
        let sql = "SELECT * FROM users WHERE department = 'engineering' AND active = true";

        #[rustfmt::skip]
        let params = ParamsBuilder::new()
            .cursor()
                .after(FilterValue::String("2024-01-01T00:00:00Z".into()))
                .and_field(FilterValue::Int(100))
                .done()
            .limit(25)
            .filter()
                .eq("status", FilterValue::String("verified".into()))
                .gte("login_count", FilterValue::Int(5))
                .done()
            .search()
                .query("senior")
                .fields(["title", "bio"])
                .case_sensitive(false)
                .done()
            .sort()
                .desc("created_at")
                .desc("id")
                .done()
            .build();

        let built = build_dynamic_sql(sql, &params, vec![]).unwrap();
        let result_sql = built.sql.as_ref();

        println!("Complex cursor + existing WHERE SQL: {}", result_sql);

        // Should combine: existing WHERE + cursor + filters + search
        // Expected: WHERE (original) AND (cursor) AND (filters) AND (search)
        assert!(result_sql.contains("WHERE"));
        assert!(result_sql.contains("department = 'engineering'")); // Original WHERE
        assert!(result_sql.contains("active = true")); // Original WHERE
        assert!(result_sql.contains("(created_at < $5 OR (created_at = $5 AND id < $6))")); // OR-based cursor condition
        assert!(result_sql.contains("status =")); // New filter
        assert!(result_sql.contains("login_count >=")); // New filter
        assert!(result_sql.contains("(LOWER(title) LIKE")); // Search properly grouped
        assert!(result_sql.contains("OR LOWER(bio) LIKE")); // Search OR condition
        assert!(result_sql.contains("ORDER BY")); // Sort
        assert!(result_sql.contains("created_at DESC")); // Sort
        assert!(result_sql.contains("id DESC")); // Sort
        assert!(result_sql.contains("LIMIT")); // Pagination

        // All conditions should be connected with AND
        let and_count = result_sql.matches(" AND ").count();
        assert!(and_count >= 5); // Many AND connections

        // Search fields should be connected with OR
        assert!(result_sql.contains(" OR "));

        // Should have: 2 cursor (OR-based, reusing one bind) + 2 filters + 2 search = 6 args
        // Note: LIMIT is embedded as literal, not as bind placeholder
        assert_eq!(built.bind_values.len(), 6);

        println!("Total arguments bound: {}", built.bind_values.len());
        println!("AND count: {}", and_count);
    }

    #[test]
    fn test_build_count_sql_from_sql_with_cache() {
        let sql = "SELECT id, name, email FROM users WHERE active = true ORDER BY created_at DESC";

        // First call - should miss cache
        let result1 = build_count_query_from_sql(sql).unwrap();

        // Second call - should hit cache
        let result2 = build_count_query_from_sql(sql).unwrap();

        // Both should produce the same count query
        assert_eq!(result1, result2);
        assert!(result1.contains("SELECT COUNT(*)"));
        assert!(result1.contains("FROM users"));
        assert!(result1.contains("WHERE active = true"));
        assert!(!result1.contains("ORDER BY")); // Count queries don't need ORDER BY

        // Test with a more complex query
        let complex_sql = "SELECT u.id, u.name, p.title FROM users u LEFT JOIN posts p ON u.id = p.user_id WHERE u.active = true AND p.published = true GROUP BY u.id";
        let complex_count = build_count_query_from_sql(complex_sql).unwrap();

        println!("Simple count query: {}", result1);
        println!("Complex count query: {}", complex_count);

        assert!(complex_count.contains("SELECT COUNT(*)"));
        assert!(complex_count.contains("FROM ("));
        assert!(complex_count.contains(") AS sub"));
    }
}
