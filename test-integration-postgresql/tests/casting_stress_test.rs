use sqlx::postgres::types::{PgInterval, PgMoney, PgRange};
use sqlx_data::{DateTime, Decimal, Json, JsonValue, NaiveDate, NaiveDateTime, NaiveTime, Pool, Result, Uuid, dml, repo};

// PostgreSQL casting stress test repository
#[repo]
trait CastingStressRepo {
    // Integer type casting (PostgreSQL specific)
    #[dml("SELECT 127::SMALLINT as \"small_int!: i16\"")]
    async fn cast_to_smallint(&self) -> Result<i16>;

    #[dml("SELECT 2147483647::INTEGER as \"regular_int!: i32\"")]
    async fn cast_to_integer(&self) -> Result<i32>;

    #[dml("SELECT 9223372036854775807::BIGINT as \"big_int!: i64\"")]
    async fn cast_to_bigint(&self) -> Result<i64>;

    // Floating point casting
    #[dml("SELECT 3.14159::REAL as \"real_val!: f32\"")]
    async fn cast_to_real(&self) -> Result<f32>;

    #[dml("SELECT 3.141592653589793::DOUBLE PRECISION as \"double_val!: f64\"")]
    async fn cast_to_double(&self) -> Result<f64>;

    #[dml("SELECT 123.456::NUMERIC(10,3) as \"numeric_val!: Decimal\"")]
    async fn cast_to_numeric(&self) -> Result<Decimal>;

    // String type casting
    #[dml("SELECT 'Hello'::VARCHAR(50) as \"varchar_val!: String\"")]
    async fn cast_to_varchar(&self) -> Result<String>;

    #[dml("SELECT 'World'::TEXT as \"text_val!: String\"")]
    async fn cast_to_text(&self) -> Result<String>;

    #[dml("SELECT 'A'::CHAR(1) as \"char_val!: String\"")]
    async fn cast_to_char(&self) -> Result<String>;

    // Boolean casting
    #[dml("SELECT true::BOOLEAN as \"bool_val!: bool\"")]
    async fn cast_to_boolean(&self) -> Result<bool>;

    #[dml("SELECT 1::INTEGER::BOOLEAN as \"int_to_bool!: bool\"")]
    async fn cast_int_to_boolean(&self) -> Result<bool>;

    #[dml("SELECT 't'::CHAR::BOOLEAN as \"char_to_bool!: bool\"")]
    async fn cast_char_to_boolean(&self) -> Result<bool>;

    // Date/time casting
    #[dml("SELECT '2024-01-15'::DATE as \"date_val!: NaiveDate\"")]
    async fn cast_to_date(&self) -> Result<NaiveDate>;

    #[dml("SELECT '14:30:00'::TIME as \"time_val!: NaiveTime\"")]
    async fn cast_to_time(&self) -> Result<NaiveTime>;

    #[dml("SELECT '2024-01-15 14:30:00'::TIMESTAMP as \"timestamp_val!: NaiveDateTime\"")]
    async fn cast_to_timestamp(&self) -> Result<NaiveDateTime>;

    #[dml("SELECT '2024-01-15 14:30:00 UTC'::TIMESTAMPTZ as \"timestamptz_val!: DateTime\"")]
    async fn cast_to_timestamptz(&self) -> Result<DateTime>;

    // UUID casting (PostgreSQL specific)
    #[dml("SELECT '550e8400-e29b-41d4-a716-446655440000'::UUID as \"uuid_val!: Uuid\"")]
    async fn cast_to_uuid(&self) -> Result<Uuid>;

    // JSONB casting
    #[dml("SELECT '{\"key\": \"value\"}'::JSONB as \"json_val!: Json<JsonValue>\"")]
    async fn cast_to_jsonb(&self) -> Result<Json<JsonValue>>;

    // Array casting
    #[dml("SELECT ARRAY[1, 2, 3]::INTEGER[] as \"int_array!: Vec<i32>\"")]
    async fn cast_to_int_array(&self) -> Result<Vec<Vec<i32>>>;

    #[dml("SELECT ARRAY['a', 'b', 'c']::TEXT[] as \"text_array!: Vec<String>\"")]
    async fn cast_to_text_array(&self) -> Result<Vec<Vec<String>>>;

    // Complex casting chains
    #[dml("SELECT ('123'::TEXT)::INTEGER::BIGINT as \"chain_cast!: i64\"")]
    async fn complex_casting_chain(&self) -> Result<i64>;

    #[dml("SELECT (3.14::REAL)::DOUBLE PRECISION::NUMERIC(10,2) as \"float_chain!: Decimal\"")]
    async fn float_casting_chain(&self) -> Result<Decimal>;

    // Conditional casting with CASE
    #[dml(
        r#"
        SELECT CASE
            WHEN $1 > 0 THEN $1::TEXT
            ELSE 'negative'::TEXT
        END as "conditional_cast!: String"
        "#
    )]
    async fn conditional_casting(&self, value: i32) -> Result<String>;

    // Casting with functions
    #[dml("SELECT LENGTH('hello'::TEXT)::SMALLINT as \"func_cast!: i16\"")]
    async fn function_with_casting(&self) -> Result<i16>;

    #[dml("SELECT EXTRACT(YEAR FROM '2024-01-15'::DATE)::INTEGER as \"extract_cast!: i32\"")]
    async fn extract_with_casting(&self) -> Result<i32>;

    // Casting with aggregates
    #[dml("SELECT AVG(age)::NUMERIC(5,2) as \"avg_cast!: Decimal\" FROM users")]
    async fn aggregate_with_casting(&self) -> Result<Decimal>;

    #[dml("SELECT COUNT(*)::SMALLINT as \"count_cast!: i16\" FROM users WHERE age > $1")]
    async fn count_with_casting(&self, min_age: i16) -> Result<i16>;

    // NULL casting
    #[dml("SELECT NULL::INTEGER as \"null_int: i32\"")]
    async fn cast_null_to_int(&self) -> Result<Option<i32>>;

    #[dml("SELECT NULL::TEXT as \"null_text: String\"")]
    async fn cast_null_to_text(&self) -> Result<Option<String>>;

    // COALESCE with casting
    #[dml("SELECT COALESCE($1::INTEGER, 0::INTEGER)::SMALLINT as \"coalesce_cast!: i16\"")]
    async fn coalesce_with_casting(&self, value: Option<i32>) -> Result<i16>;

    // Array element casting
    #[dml("SELECT (ARRAY[1, 2, 3])[2]::SMALLINT as \"array_elem_cast!: i16\"")]
    async fn array_element_casting(&self) -> Result<i16>;

    // PostgreSQL specific: interval casting
    #[dml("SELECT '1 hour'::INTERVAL as \"interval_val!: PgInterval\"")]
    async fn cast_to_interval(&self) -> Result<PgInterval>;

    // Range type casting (PostgreSQL specific)
    #[dml("SELECT '[1,10)'::INT4RANGE as \"range_val!: PgRange<i32>\"")]
    async fn cast_to_int_range(&self) -> Result<PgRange<i32>>;

    // Money type casting (PostgreSQL specific)
    #[dml("SELECT '$123.45'::MONEY as \"money_val!: PgMoney\"")]
    async fn cast_to_money(&self) -> Result<PgMoney>;

    // Large object casting stress test
    #[dml("SELECT REPEAT('A', 1000)::TEXT as \"large_text!: String\"")]
    async fn large_text_casting(&self) -> Result<String>;

    // Multiple casts in one query
    #[dml(
        r#"
        SELECT
            123::SMALLINT as "small!: i16",
            456::BIGINT as "big!: i64",
            'test'::TEXT as "text!: String",
            true::BOOLEAN as "flag!: bool",
            3.14::REAL as "pi!: f32"
        "#
    )]
    async fn multiple_casts_in_query(&self) -> Result<(i16, i64, String, bool, f32)>;

    // Casting with table data
    #[dml("SELECT id::SMALLINT as \"id_small!: i16\", name::VARCHAR(100) as \"name_var!: String\" FROM users WHERE id = $1")]
    async fn cast_table_data(&self, id: i64) -> Result<Option<(i16, String)>>;

    // Extreme casting scenarios
    #[dml("SELECT ('9999999999999999999999999999.99'::NUMERIC)::TEXT as \"extreme_numeric!: String\"")]
    async fn extreme_numeric_casting(&self) -> Result<String>;

    #[dml("SELECT (EXTRACT(EPOCH FROM NOW()))::BIGINT as \"epoch_cast!: i64\"")]
    async fn epoch_casting(&self) -> Result<i64>;

    // PostgreSQL specific types in a single tuple
    #[dml(r#"
        SELECT
            '1 hour'::INTERVAL as "interval_val!: PgInterval",
            '[1,10)'::INT4RANGE as "range_val!: PgRange<i32>",
            '$123.45'::MONEY as "money_val!: PgMoney"
    "#)]
    async fn pg_types_tuple(&self) -> Result<(PgInterval, PgRange<i32>, PgMoney)>;

    // PostgreSQL specific types as input parameters
    #[dml(r#"
        SELECT
            $1::INTERVAL as "interval_val!: PgInterval",
            $2::INT4RANGE as "range_val!: PgRange<i32>",
            $3::MONEY as "money_val!: PgMoney"
    "#)]
    async fn pg_types_input(&self, interval: PgInterval, range: PgRange<i32>, money: PgMoney) -> Result<(PgInterval, PgRange<i32>, PgMoney)>;
}

pub struct CastingStressApp {
    pool: Pool,
}

impl CastingStressRepo for CastingStressApp {
    fn get_pool(&self) -> &Pool {
        &self.pool
    }
}

#[allow(clippy::unwrap_used)]
#[cfg(test)]
mod tests {
    use super::*;

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_integer_casting(pool: Pool) {
        let app = CastingStressApp { pool };

        let small = app.cast_to_smallint().await.unwrap();
        assert_eq!(small, 127);

        let regular = app.cast_to_integer().await.unwrap();
        assert_eq!(regular, 2147483647);

        let big = app.cast_to_bigint().await.unwrap();
        assert_eq!(big, 9223372036854775807);
    }

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_floating_point_casting(pool: Pool) {
        let app = CastingStressApp { pool };

        let real_val = app.cast_to_real().await.unwrap();
        assert!((real_val - std::f32::consts::PI).abs() < 0.00001);

        let double_val = app.cast_to_double().await.unwrap();
        assert!((double_val - std::f64::consts::PI).abs() < 0.000000000000001);

        let numeric_val = app.cast_to_numeric().await.unwrap();
        assert!(numeric_val.to_string().starts_with("123.456"));
    }

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_string_casting(pool: Pool) {
        let app = CastingStressApp { pool };

        let varchar_val = app.cast_to_varchar().await.unwrap();
        assert_eq!(varchar_val, "Hello");

        let text_val = app.cast_to_text().await.unwrap();
        assert_eq!(text_val, "World");

        let char_val = app.cast_to_char().await.unwrap();
        assert_eq!(char_val, "A");
    }

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_boolean_casting(pool: Pool) {
        let app = CastingStressApp { pool };

        let bool_val = app.cast_to_boolean().await.unwrap();
        assert!(bool_val);

        let int_to_bool = app.cast_int_to_boolean().await.unwrap();
        assert!(int_to_bool);

        let char_to_bool = app.cast_char_to_boolean().await.unwrap();
        assert!(char_to_bool);
    }

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_datetime_casting(pool: Pool) {
        let app = CastingStressApp { pool };

        let date_val = app.cast_to_date().await.unwrap();
        assert_eq!(date_val.to_string(), "2024-01-15");

        let time_val = app.cast_to_time().await.unwrap();
        assert!(time_val.to_string().starts_with("14:30:00"));

        let timestamp_val = app.cast_to_timestamp().await.unwrap();
        assert!(timestamp_val.to_string().starts_with("2024-01-15 14:30:00"));

        let timestamptz_val = app.cast_to_timestamptz().await.unwrap();
        assert!(timestamptz_val.to_string().starts_with("2024-01-15"));
    }

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_uuid_casting(pool: Pool) {
        let app = CastingStressApp { pool };

        let uuid_val = app.cast_to_uuid().await.unwrap();
        assert_eq!(uuid_val.to_string(), "550e8400-e29b-41d4-a716-446655440000");
    }

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_jsonb_casting(pool: Pool) {
        let app = CastingStressApp { pool };

        let json_val = app.cast_to_jsonb().await.unwrap();
        let json_str = json_val.to_string();
        assert!(json_str.contains("key"));
        assert!(json_str.contains("value"));

        // Parse to verify structure
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        assert_eq!(parsed["key"].as_str().unwrap(), "value");
    }

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_array_casting(pool: Pool) {
        let app = CastingStressApp { pool };

        let int_array = app.cast_to_int_array().await.unwrap();
        assert_eq!(int_array, vec![vec![1, 2, 3]]);

        let text_array = app.cast_to_text_array().await.unwrap();
        assert_eq!(text_array, vec![vec!["a".to_string(), "b".to_string(), "c".to_string()]]);
    }

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_complex_casting_chains(pool: Pool) {
        let app = CastingStressApp { pool };

        let chain_cast = app.complex_casting_chain().await.unwrap();
        assert_eq!(chain_cast, 123);

        let float_chain = app.float_casting_chain().await.unwrap();
        assert!(float_chain.to_string().starts_with("3.14"));
    }

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_conditional_casting(pool: Pool) {
        let app = CastingStressApp { pool };

        let positive_result = app.conditional_casting(42).await.unwrap();
        assert_eq!(positive_result, "42");

        let negative_result = app.conditional_casting(-5).await.unwrap();
        assert_eq!(negative_result, "negative");

        let zero_result = app.conditional_casting(0).await.unwrap();
        assert_eq!(zero_result, "negative");
    }

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_function_casting(pool: Pool) {
        let app = CastingStressApp { pool };

        let func_cast = app.function_with_casting().await.unwrap();
        assert_eq!(func_cast, 5); // LENGTH('hello') = 5

        let extract_cast = app.extract_with_casting().await.unwrap();
        assert_eq!(extract_cast, 2024);
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_aggregate_casting(pool: Pool) {
        let app = CastingStressApp { pool };

        let avg_cast = app.aggregate_with_casting().await.unwrap();
        // Average age should be reasonable (between 20-50)
        let avg_str = avg_cast.to_string();
        let avg_f64: f64 = avg_str.parse().unwrap();
        assert!(avg_f64 > 20.0);
        assert!(avg_f64 < 50.0);

        let count_cast = app.count_with_casting(25).await.unwrap();
        assert!(count_cast > 0);
        assert!(count_cast <= 20);
    }

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_null_casting(pool: Pool) {
        let app = CastingStressApp { pool };

        let null_int = app.cast_null_to_int().await.unwrap();
        assert!(null_int.is_none());

        let null_text = app.cast_null_to_text().await.unwrap();
        assert!(null_text.is_none());
    }

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_coalesce_casting(pool: Pool) {
        let app = CastingStressApp { pool };

        let with_value = app.coalesce_with_casting(Some(42)).await.unwrap();
        assert_eq!(with_value, 42);

        let with_null = app.coalesce_with_casting(None).await.unwrap();
        assert_eq!(with_null, 0);
    }

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_array_element_casting(pool: Pool) {
        let app = CastingStressApp { pool };

        let elem_cast = app.array_element_casting().await.unwrap();
        assert_eq!(elem_cast, 2); // Second element of [1, 2, 3]
    }

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_postgresql_specific_casting(pool: Pool) {
        let app = CastingStressApp { pool };

        // Test interval casting
        let interval_val = app.cast_to_interval().await.unwrap();
        assert_eq!(interval_val.microseconds, 3_600_000_000); // 1 hour = 3.6e9 microseconds

        // Test range casting
        let range_val = app.cast_to_int_range().await.unwrap();
        assert_eq!(range_val.start, std::ops::Bound::Included(1));
        assert_eq!(range_val.end, std::ops::Bound::Excluded(10));

        // Test money casting
        let money_val = app.cast_to_money().await.unwrap();
        assert_eq!(money_val.0, 12345); // $123.45 in cents
    }

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_large_text_casting(pool: Pool) {
        let app = CastingStressApp { pool };

        let large_text = app.large_text_casting().await.unwrap();
        assert_eq!(large_text.len(), 1000);
        assert!(large_text.chars().all(|c| c == 'A'));
    }

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_multiple_casts_in_query(pool: Pool) {
        let app = CastingStressApp { pool };

        let (small, big, text, flag, pi) = app.multiple_casts_in_query().await.unwrap();
        assert_eq!(small, 123);
        assert_eq!(big, 456);
        assert_eq!(text, "test");
        assert!(flag);
        assert!((pi - std::f32::consts::PI).abs() < 0.01);
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_cast_table_data(pool: Pool) {
        let app = CastingStressApp { pool };

        let result = app.cast_table_data(1).await.unwrap();
        assert!(result.is_some());

        let (id_small, name_var) = result.unwrap();
        assert_eq!(id_small, 1);
        assert_eq!(name_var, "Alice");

        // Test non-existent id
        let no_result = app.cast_table_data(999).await.unwrap();
        assert!(no_result.is_none());
    }

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_extreme_casting_scenarios(pool: Pool) {
        let app = CastingStressApp { pool };

        let extreme_numeric = app.extreme_numeric_casting().await.unwrap();
        assert!(extreme_numeric.contains("99999"));
        assert!(extreme_numeric.contains(".99"));

        let epoch_cast = app.epoch_casting().await.unwrap();
        // Should be a reasonable Unix timestamp (after 2020, before 2030)
        assert!(epoch_cast > 1_600_000_000); // After Sept 2020
        assert!(epoch_cast < 1_900_000_000); // Before March 2030
    }

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_pg_types_tuple(pool: Pool) {
        let app = CastingStressApp { pool };

        let (interval, range, money) = app.pg_types_tuple().await.unwrap();

        // 1 hour = 3_600_000_000 microseconds
        assert_eq!(interval.microseconds, 3_600_000_000);
        assert_eq!(interval.days, 0);
        assert_eq!(interval.months, 0);

        // [1,10) range
        assert_eq!(range.start, std::ops::Bound::Included(1));
        assert_eq!(range.end, std::ops::Bound::Excluded(10));

        // $123.45 = 12345 cents
        assert_eq!(money.0, 12345);
    }

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_pg_types_input(pool: Pool) {
        let app = CastingStressApp { pool };

        let input_interval = PgInterval { months: 1, days: 2, microseconds: 3_000_000 };
        let input_range = PgRange { start: std::ops::Bound::Included(5), end: std::ops::Bound::Excluded(15) };
        let input_money = PgMoney(9999);

        let (interval, range, money) = app.pg_types_input(input_interval, input_range, input_money).await.unwrap();

        assert_eq!(interval.months, 1);
        assert_eq!(interval.days, 2);
        assert_eq!(interval.microseconds, 3_000_000);

        assert_eq!(range.start, std::ops::Bound::Included(5));
        assert_eq!(range.end, std::ops::Bound::Excluded(15));

        assert_eq!(money.0, 9999);
    }
}