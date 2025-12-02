use sqlx_data::{FilterValue, ParamsBuilder, build_dynamic_sql};

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cursor_only_single_field() {
        let sql = "SELECT * FROM users";
        let params = ParamsBuilder::new()
            .sort()
            .asc("id")
            .done()
            .cursor()
            .after(FilterValue::Int(100))
            .done()
            .limit(10)
            .build();

        let built = build_dynamic_sql(sql, &params, vec![]).unwrap();
        let result_sql = built.sql.as_ref();

        println!("Single field cursor: {}", result_sql);

        // Should be: SELECT * FROM users WHERE id > $1 ORDER BY id ASC LIMIT $2
        assert!(result_sql.contains("WHERE id > $1"));
        assert!(!result_sql.contains("("));
        assert!(!result_sql.contains(")"));
        assert!(result_sql.contains("ORDER BY id ASC"));
        assert!(result_sql.contains("LIMIT 11")); // limit + 1 for cursor pagination
        assert_eq!(built.bind_values.len(), 1);
    }

    #[tokio::test]
    async fn test_cursor_only_multi_field() {
        let sql = "SELECT * FROM users";
        let params = ParamsBuilder::new()
            .sort()
            .desc("created_at")
            .asc("id")
            .done()
            .cursor()
            .after(FilterValue::String("2024-01-01".into()))
            .and_field(FilterValue::Int(50))
            .done()
            .limit(5)
            .build();

        let built = build_dynamic_sql(sql, &params, vec![]).unwrap();
        let result_sql = built.sql.as_ref();

        println!("Multi field cursor: {}", result_sql);

        // Should have OR-based cursor condition but no extra parentheses around the whole thing
        assert!(result_sql.contains("WHERE"));
        assert!(result_sql.contains("created_at < $1"));
        assert!(result_sql.contains("OR"));
        assert!(result_sql.contains("created_at = $1 AND id > $2"));
        assert!(result_sql.contains("ORDER BY created_at DESC, id ASC"));
        assert!(result_sql.contains("LIMIT 6"));
        assert_eq!(built.bind_values.len(), 2);

        // The OR condition should not have extra outer parentheses
        assert!(!result_sql.contains("(created_at < $1 OR (created_at = $1 AND id > $2))"));
    }

    #[tokio::test]
    async fn test_cursor_before_direction() {
        let sql = "SELECT * FROM products";
        let params = ParamsBuilder::new()
            .sort()
                .asc("price")
                .done()
            .cursor()
                .before(FilterValue::Float(99.99))
                .done()
            .limit(20)
            .build();

        let built = build_dynamic_sql(sql, &params, vec![]).unwrap();
        let result_sql = built.sql.as_ref();

        println!("Before cursor: {}", result_sql);

        // Should be: SELECT * FROM products WHERE price < $1 ORDER BY price DESC LIMIT $2
        assert!(result_sql.contains("WHERE price < $1"));
        assert!(result_sql.contains("ORDER BY price DESC")); // Inverted for BEFORE
        assert!(result_sql.contains("LIMIT 21"));
        assert!(!result_sql.contains("("));
        assert!(!result_sql.contains(")"));
        assert_eq!(built.bind_values.len(), 1);
    }

    #[tokio::test]
    async fn test_cursor_before_multi_field() {
        let sql = "SELECT * FROM orders";
        let params = ParamsBuilder::new()
            .sort()
            .desc("order_date")
            .desc("id")
            .done()
            .cursor()
            .before(FilterValue::String("2024-06-01".into()))
            .and_field(FilterValue::Int(1000))
            .done()
            .limit(15)
            .build();

        let built = build_dynamic_sql(sql, &params, vec![]).unwrap();
        let result_sql = built.sql.as_ref();

        println!("Before multi field cursor: {}", result_sql);

        // Before with DESC should become After with ASC (inverted)
        assert!(result_sql.contains("WHERE"));
        assert!(result_sql.contains("order_date > $1"));
        assert!(result_sql.contains("OR"));
        assert!(result_sql.contains("order_date = $1 AND id > $2"));
        assert!(result_sql.contains("ORDER BY order_date ASC, id ASC")); // Inverted
        assert!(result_sql.contains("LIMIT 16")); // limit + 1 for cursor pagination
        assert_eq!(built.bind_values.len(), 2);
    }

    #[tokio::test]
    async fn test_cursor_with_existing_where_no_extra_conditions() {
        let sql = "SELECT * FROM users WHERE active = true";
        let params = ParamsBuilder::new()
            .sort()
            .asc("name")
            .done()
            .cursor()
            .after(FilterValue::String("John".into()))
            .done()
            .limit(25)
            .build();

        let built = build_dynamic_sql(sql, &params, vec![]).unwrap();
        let result_sql = built.sql.as_ref();

        println!("Cursor with existing WHERE: {}", result_sql);

        // Should combine existing WHERE with cursor condition using AND
        assert!(result_sql.contains("WHERE active = true AND name > $1"));
        assert!(result_sql.contains("ORDER BY name ASC"));
        assert!(result_sql.contains("LIMIT 26")); // limit + 1 for cursor pagination
        assert_eq!(built.bind_values.len(), 1);
    }

    #[tokio::test]
    async fn test_cursor_complex_existing_where() {
        let sql = "SELECT * FROM posts WHERE (status = 'published' OR status = 'featured') AND category_id = 5";
        let params = ParamsBuilder::new()
            .sort()
                .desc("published_at")
                .asc("id")
                .done()
            .cursor()
                .after(FilterValue::String("2024-01-15T10:00:00Z".into()))
                .and_field(FilterValue::Int(42))
                .done()
            .limit(30)
            .build();

        let built = build_dynamic_sql(sql, &params, vec![]).unwrap();
        let result_sql = built.sql.as_ref();

        println!("Complex existing WHERE with cursor: {}", result_sql);

        // Should preserve original WHERE structure and add cursor condition
        assert!(result_sql.contains("(status = 'published' OR status = 'featured')"));
        assert!(result_sql.contains("category_id = 5"));
        assert!(result_sql.contains("published_at < $1"));
        assert!(result_sql.contains("published_at = $1 AND id > $2"));
        assert!(result_sql.contains("ORDER BY published_at DESC, id ASC"));
        assert!(result_sql.contains("LIMIT 31")); // limit + 1 for cursor pagination
        assert_eq!(built.bind_values.len(), 2);

        // The cursor condition should be properly grouped
        assert!(result_sql.contains("(published_at < $1 OR (published_at = $1 AND id > $2))"));
    }

    #[tokio::test]
    async fn test_cursor_three_fields() {
        let sql = "SELECT * FROM events";
        let params = ParamsBuilder::new()
            .sort()
                .desc("priority")
                .asc("created_at")
                .asc("id")
                .done()
            .cursor()
                .after(FilterValue::Int(5))
                .and_field(FilterValue::String("2024-03-01T00:00:00Z".into()))
                .and_field(FilterValue::Int(123))
                .done()
            .limit(10)
            .build();

        let built = build_dynamic_sql(sql, &params, vec![]).unwrap();
        let result_sql = built.sql.as_ref();

        println!("Three field cursor: {}", result_sql);

        // Should create nested OR conditions for three fields
        assert!(result_sql.contains("priority < $1"));
        assert!(result_sql.contains("priority = $1"));
        assert!(result_sql.contains("created_at > $2"));
        assert!(result_sql.contains("created_at = $2"));
        assert!(result_sql.contains("id > $3"));
        assert!(result_sql.contains("ORDER BY priority DESC, created_at ASC, id ASC"));
        assert!(result_sql.contains("LIMIT 11")); // limit + 1 for cursor pagination
        assert_eq!(built.bind_values.len(), 3);

        // Should have proper nesting
        assert!(result_sql.contains("OR"));
        assert!(result_sql.contains("AND"));
    }

    #[tokio::test]
    async fn test_cursor_edge_case_single_desc() {
        let sql = "SELECT * FROM logs";
        let params = ParamsBuilder::new()
            .sort()
            .desc("timestamp")
            .done()
            .cursor()
            .after(FilterValue::String("2024-12-01T12:00:00Z".into()))
            .done()
            .limit(100)
            .build();

        let built = build_dynamic_sql(sql, &params, vec![]).unwrap();
        let result_sql = built.sql.as_ref();

        println!("Single DESC cursor: {}", result_sql);

        // After + DESC = Less than
        assert!(result_sql.contains("WHERE timestamp < $1"));
        assert!(result_sql.contains("ORDER BY timestamp DESC"));
        assert!(result_sql.contains("LIMIT 101")); // limit + 1 for cursor pagination
        assert!(!result_sql.contains("("));
        assert!(!result_sql.contains(")"));
        assert_eq!(built.bind_values.len(), 1);
    }

    #[tokio::test]
    async fn test_cursor_nulls_handling() {
        let sql = "SELECT * FROM comments";
        let params = ParamsBuilder::new()
            .sort()
            .asc("rating")
            .nulls_first()
            .done()
            .cursor()
            .after(FilterValue::Int(3))
            .done()
            .limit(50)
            .build();

        let built = build_dynamic_sql(sql, &params, vec![]).unwrap();
        let result_sql = built.sql.as_ref();

        println!("Cursor with nulls handling: {}", result_sql);

        assert!(result_sql.contains("WHERE rating > $1"));
        assert!(result_sql.contains("ORDER BY rating ASC NULLS FIRST"));
        assert!(result_sql.contains("LIMIT 51")); // limit + 1 for cursor pagination
        assert_eq!(built.bind_values.len(), 1);
    }
}
