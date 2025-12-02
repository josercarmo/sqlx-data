use sqlx_data::{Pool, Result, dml, repo};

// Repository trait with extensive casting stress tests
#[repo]
trait CastingStressRepo {
    // ===============================================
    // MATHEMATICAL FUNCTION STRESS TESTS
    // ===============================================

    // Basic aggregation with different numeric types
    #[dml("SELECT COUNT(id) as 'count!: i8' FROM users")]
    async fn count_as_i8(&self) -> Result<i8>;

    #[dml("SELECT COUNT(id) as 'count!: i16' FROM users")]
    async fn count_as_i16(&self) -> Result<i16>;

    #[dml("SELECT COUNT(id) as 'count!: i32' FROM users")]
    async fn count_as_i32(&self) -> Result<i32>;

    #[dml("SELECT COUNT(id) as 'count!: i64' FROM users")]
    async fn count_as_i64(&self) -> Result<i64>;

    #[dml("SELECT COUNT(id) as 'count!: u8' FROM users")]
    async fn count_as_u8(&self) -> Result<u8>;

    #[dml("SELECT COUNT(id) as 'count!: u16' FROM users")]
    async fn count_as_u16(&self) -> Result<u16>;

    #[dml("SELECT COUNT(id) as 'count!: u32' FROM users")]
    async fn count_as_u32(&self) -> Result<u32>;

    #[dml("SELECT COUNT(id) as 'count!: u64' FROM users")]
    async fn count_as_u64(&self) -> Result<u64>;

    // AVG with different float types - can cause precision issues
    #[dml("SELECT AVG(age) as 'avg_age: f32' FROM users")]
    async fn avg_age_f32(&self) -> Result<Option<f32>>;

    #[dml("SELECT AVG(age) as 'avg_age: f64' FROM users")]
    async fn avg_age_f64(&self) -> Result<Option<f64>>;

    #[dml("SELECT AVG(age) as 'avg_age!: f32' FROM users WHERE age IS NOT NULL")]
    async fn avg_age_non_null_f32(&self) -> Result<f32>;

    // SUM operations with potential overflow
    #[dml("SELECT SUM(age * age) as 'sum_squares!: u8' FROM users")]
    async fn sum_squares_u8_overflow(&self) -> Result<u8>;

    #[dml("SELECT SUM(age * 1000000) as 'big_sum!: i32' FROM users")]
    async fn sum_with_overflow(&self) -> Result<i32>;

    #[dml(
        "SELECT SUM(CASE WHEN age > 100 THEN 99999999999 ELSE age END) as 'conditional_sum!: i64' FROM users"
    )]
    async fn conditional_sum_large(&self) -> Result<i64>;

    // MIN/MAX with casting edge cases
    #[dml("SELECT MIN(age - 300) as 'min_negative!: u8' FROM users")]
    async fn min_negative_to_unsigned(&self) -> Result<u8>;

    #[dml("SELECT MAX(age * 999) as 'max_overflow!: u8' FROM users")]
    async fn max_overflow_u8(&self) -> Result<u8>;

    // ABS function with edge cases
    #[dml("SELECT ABS(age - 200) as 'abs_result!: u8' FROM users LIMIT 1")]
    async fn abs_underflow_u8(&self) -> Result<u8>;

    #[dml("SELECT ABS(-9223372036854775808) as 'abs_min_i64!: i64'")]
    async fn abs_min_i64(&self) -> Result<i64>;

    // ROUND with different precision
    #[dml("SELECT ROUND(AVG(age), -5) as 'rounded_avg!: f32' FROM users")]
    async fn round_negative_precision(&self) -> Result<f32>;

    // ===============================================
    // STRING FUNCTION STRESS TESTS
    // ===============================================

    // LENGTH with casting to different numeric types
    #[dml("SELECT LENGTH(name) as 'name_len!: i8' FROM users LIMIT 1")]
    async fn length_as_i8(&self) -> Result<i8>;

    #[dml(
        "SELECT LENGTH('xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx') as 'long_len!: u8'"
    )]
    async fn length_overflow_u8(&self) -> Result<u8>;

    // SUBSTR with invalid positions
    #[dml("SELECT SUBSTR(name, -10, 5) as 'substr_result: String' FROM users LIMIT 1")]
    async fn substr_negative_start(&self) -> Result<Option<String>>;

    #[dml("SELECT SUBSTR(name, 999, 5) as 'substr_result: String' FROM users LIMIT 1")]
    async fn substr_beyond_length(&self) -> Result<Option<String>>;

    // String to number conversion attempts
    #[dml("SELECT CAST(name AS INTEGER) as 'name_as_int!: i32' FROM users LIMIT 1")]
    async fn string_to_int_cast(&self) -> Result<i32>;

    #[dml("SELECT CAST('not_a_number' AS REAL) as 'invalid_number!: f64'")]
    async fn invalid_string_to_float(&self) -> Result<f64>;

    #[dml("SELECT CAST('999999999999999999999' AS INTEGER) as 'huge_number!: i32'")]
    async fn huge_string_to_int(&self) -> Result<i32>;

    // Unicode and special character handling
    #[dml("SELECT LENGTH('🚀🦀🌟') as 'emoji_len!: u8'")]
    async fn emoji_length(&self) -> Result<u8>;

    #[dml("SELECT UPPER('ñáéíóúç') as 'accented_upper: String'")]
    async fn accented_upper(&self) -> Result<Option<String>>;

    // ===============================================
    // DATE/TIME FUNCTION STRESS TESTS
    // ===============================================

    // Invalid date formats
    #[dml("SELECT DATE('invalid-date') as 'invalid_date!: String'")]
    async fn invalid_date_format(&self) -> Result<String>;

    #[dml("SELECT DATE('2024-13-45') as 'impossible_date!: String'")]
    async fn impossible_date(&self) -> Result<String>;

    // JULIANDAY with extreme dates
    #[dml("SELECT JULIANDAY('1-01-01') as 'ancient_date!: f64'")]
    async fn ancient_julian_date(&self) -> Result<f64>;

    #[dml("SELECT JULIANDAY('9999-12-31') as 'future_date!: f64'")]
    async fn future_julian_date(&self) -> Result<f64>;

    // STRFTIME with invalid format
    #[dml("SELECT STRFTIME('%invalid%', 'now') as 'bad_format: String'")]
    async fn invalid_strftime_format(&self) -> Result<Option<String>>;

    // Date arithmetic overflow
    #[dml("SELECT DATE('2024-01-01', '+999999999 days') as 'overflow_date: String'")]
    async fn date_arithmetic_overflow(&self) -> Result<Option<String>>;

    // Casting JULIANDAY to integer types
    #[dml("SELECT CAST(JULIANDAY('now') AS INTEGER) as 'julian_as_int!: u32'")]
    async fn julian_as_u32(&self) -> Result<u32>;

    // ===============================================
    // COMPLEX EXPRESSIONS AND NESTED FUNCTIONS
    // ===============================================

    // Nested functions with type mismatches
    #[dml("SELECT ROUND(LENGTH(UPPER(name)), 2) as 'nested_result!: u8' FROM users LIMIT 1")]
    async fn nested_functions_cast(&self) -> Result<u8>;

    #[dml("SELECT ABS(LENGTH(name) - AVG(age)) as 'complex_calc!: f32' FROM users LIMIT 1")]
    async fn complex_abs_calc(&self) -> Result<f32>;

    // CASE WHEN with incompatible types
    #[dml(
        r#"
        SELECT CASE
            WHEN age > 50 THEN 'old'
            WHEN age > 30 THEN 42.5
            ELSE true
        END as 'mixed_case!: String'
        FROM users LIMIT 1
    "#
    )]
    async fn case_mixed_types(&self) -> Result<String>;

    #[dml(
        r#"
        SELECT CASE
            WHEN LENGTH(name) > 10 THEN 99999999999
            ELSE age
        END as 'case_overflow!: u8'
        FROM users LIMIT 1
    "#
    )]
    async fn case_with_overflow(&self) -> Result<u8>;

    // Window functions with problematic casting
    #[dml("SELECT ROW_NUMBER() OVER (ORDER BY name) as 'row_num!: i8' FROM users")]
    async fn row_number_as_i8(&self) -> Result<Vec<i8>>;

    #[dml("SELECT DENSE_RANK() OVER (ORDER BY age DESC) as 'dense_rank!: u8' FROM users")]
    async fn dense_rank_as_u8(&self) -> Result<Vec<u8>>;

    // CTEs with type conflicts
    #[dml(
        r#"
        WITH calc AS (
            SELECT age * 999999999 as big_calc FROM users
        )
        SELECT big_calc as 'result!: i16' FROM calc LIMIT 1
    "#
    )]
    async fn cte_overflow_cast(&self) -> Result<i16>;

    // ===============================================
    // JSON AND BLOB STRESS TESTS
    // ===============================================

    // Invalid JSON extractions
    #[dml("SELECT JSON_EXTRACT('not json', '$.field') as 'json_value!: String'")]
    async fn invalid_json_extract(&self) -> Result<String>;

    #[dml("SELECT JSON_EXTRACT('{\"num\": \"not_a_number\"}', '$.num') as 'json_num!: i32'")]
    async fn json_invalid_number(&self) -> Result<i32>;

    // BLOB operations with casting
    #[dml("SELECT LENGTH(RANDOMBLOB(1000000)) as 'blob_len!: u16'")]
    async fn huge_blob_length(&self) -> Result<u16>;

    // ===============================================
    // BOUNDARY VALUE TESTS
    // ===============================================

    // Maximum values for each type
    #[dml("SELECT 127 as 'max_i8!: i8'")]
    async fn max_i8(&self) -> Result<i8>;

    #[dml("SELECT 128 as 'overflow_i8!: i8'")]
    async fn overflow_i8(&self) -> Result<i8>;

    #[dml("SELECT 32767 as 'max_i16!: i16'")]
    async fn max_i16(&self) -> Result<i16>;

    #[dml("SELECT 32768 as 'overflow_i16!: i16'")]
    async fn overflow_i16(&self) -> Result<i16>;

    #[dml("SELECT 255 as 'max_u8!: u8'")]
    async fn max_u8(&self) -> Result<u8>;

    #[dml("SELECT 256 as 'overflow_u8!: u8'")]
    async fn overflow_u8(&self) -> Result<u8>;

    #[dml("SELECT -1 as 'negative_u8!: u8'")]
    async fn negative_to_unsigned(&self) -> Result<u8>;

    // Float precision edge cases
    #[dml("SELECT 3.4028235e38 as 'max_f32!: f32'")]
    async fn max_f32(&self) -> Result<f32>;

    #[dml("SELECT 3.4028236e38 as 'overflow_f32!: f32'")]
    async fn overflow_f32(&self) -> Result<f32>;

    #[dml("SELECT 1.0/0.0 as 'infinity!: f64'")]
    async fn division_by_zero(&self) -> Result<f64>;

    #[dml("SELECT CAST('inf' AS REAL) as 'nan!: f64'")]
    async fn cast_invalid_float(&self) -> Result<f64>;

    // ===============================================
    // NULL HANDLING EDGE CASES
    // ===============================================

    // Non-null assertions on NULL values
    #[dml("SELECT NULL as 'null_as_string!: String'")]
    async fn null_as_non_null_string(&self) -> Result<String>;

    #[dml("SELECT NULL as 'null_as_int!: i32'")]
    async fn null_as_non_null_int(&self) -> Result<i32>;

    // Operations with NULL that might produce unexpected results
    #[dml("SELECT NULL + 5 as 'null_math!: i32'")]
    async fn null_arithmetic(&self) -> Result<i32>;

    #[dml("SELECT LENGTH(NULL) as 'null_length!: i32'")]
    async fn null_length(&self) -> Result<i32>;
}

// Test implementation
pub struct CastingStressApp {
    pool: Pool,
}

impl CastingStressRepo for CastingStressApp {
    fn get_pool(&self) -> &Pool {
        &self.pool
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_mathematical_functions(pool: Pool) {
        let repo = CastingStressApp { pool };

        // These should work fine
        let count_i32 = repo.count_as_i32().await;
        println!("Count as i32: {:?}", count_i32);

        let avg_f64 = repo.avg_age_f64().await;
        println!("Avg age as f64: {:?}", avg_f64);

        // This might overflow depending on data
        let count_u8 = repo.count_as_u8().await;
        println!("Count as u8: {:?}", count_u8);

        // Test some that are designed to fail/cause issues
        let sum_overflow = repo.sum_squares_u8_overflow().await;
        println!("Sum squares (u8 overflow test): {:?}", sum_overflow);

        let negative_unsigned = repo.min_negative_to_unsigned().await;
        println!("Negative to unsigned test: {:?}", negative_unsigned);
    }

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_string_function_stress(pool: Pool) {
        let repo = CastingStressApp { pool };

        let length_i8 = repo.length_as_i8().await;
        println!("Name length as i8: {:?}", length_i8);

        let substr_negative = repo.substr_negative_start().await;
        println!("SUBSTR with negative start: {:?}", substr_negative);

        // This should fail - converting name to integer
        let string_to_int = repo.string_to_int_cast().await;
        println!("String to int cast: {:?}", string_to_int);

        let emoji_len = repo.emoji_length().await;
        println!("Emoji length: {:?}", emoji_len);
    }

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_boundary_values(pool: Pool) {
        let repo = CastingStressApp { pool };

        // Test maximum values
        let max_i8 = repo.max_i8().await;
        println!("Max i8 (127): {:?}", max_i8);

        // This should cause overflow
        let overflow_i8 = repo.overflow_i8().await;
        println!("Overflow i8 (128): {:?}", overflow_i8);

        let max_u8 = repo.max_u8().await;
        println!("Max u8 (255): {:?}", max_u8);

        let overflow_u8 = repo.overflow_u8().await;
        println!("Overflow u8 (256): {:?}", overflow_u8);

        // Negative to unsigned
        let negative_unsigned = repo.negative_to_unsigned().await;
        println!("Negative to unsigned: {:?}", negative_unsigned);
    }

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_date_time_stress(pool: Pool) {
        let repo = CastingStressApp { pool };

        let invalid_date = repo.invalid_date_format().await;
        println!("Invalid date format: {:?}", invalid_date);

        let impossible_date = repo.impossible_date().await;
        println!("Impossible date: {:?}", impossible_date);

        let julian_as_u32 = repo.julian_as_u32().await;
        println!("Julian day as u32: {:?}", julian_as_u32);
    }

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_complex_expressions(pool: Pool) {
        let repo = CastingStressApp { pool };

        let nested_result = repo.nested_functions_cast().await;
        println!("Nested functions result: {:?}", nested_result);

        let case_mixed = repo.case_mixed_types().await;
        println!("CASE with mixed types: {:?}", case_mixed);

        let cte_overflow = repo.cte_overflow_cast().await;
        println!("CTE overflow test: {:?}", cte_overflow);
    }

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_null_handling_stress(pool: Pool) {
        let repo = CastingStressApp { pool };

        // These should fail because we're asserting non-null on NULL values
        let null_string = repo.null_as_non_null_string().await;
        println!("NULL as non-null String: {:?}", null_string);

        let null_int = repo.null_as_non_null_int().await;
        println!("NULL as non-null i32: {:?}", null_int);

        let null_math = repo.null_arithmetic().await;
        println!("NULL arithmetic: {:?}", null_math);
    }

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_float_edge_cases(pool: Pool) {
        let repo = CastingStressApp { pool };

        let max_f32 = repo.max_f32().await;
        println!("Max f32: {:?}", max_f32);

        let overflow_f32 = repo.overflow_f32().await;
        println!("Overflow f32: {:?}", overflow_f32);

        let division_zero = repo.division_by_zero().await;
        println!("Division by zero: {:?}", division_zero);

        let cast_invalid = repo.cast_invalid_float().await;
        println!("Cast invalid float: {:?}", cast_invalid);
    }

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_json_and_blob_stress(pool: Pool) {
        let repo = CastingStressApp { pool };

        let invalid_json = repo.invalid_json_extract().await;
        println!("Invalid JSON extract: {:?}", invalid_json);

        let json_invalid_num = repo.json_invalid_number().await;
        println!("JSON invalid number: {:?}", json_invalid_num);

        let blob_len = repo.huge_blob_length().await;
        println!("Huge blob length: {:?}", blob_len);
    }
}
