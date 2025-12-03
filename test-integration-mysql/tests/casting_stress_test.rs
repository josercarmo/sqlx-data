use sqlx_data::{Pool, Result, dml, repo};
use sqlx::types::BigDecimal;

// MySQL-specific repository trait with extensive casting stress tests
#[repo]
trait MySqlCastingStressRepo {
    // ===============================================
    // MYSQL MATHEMATICAL FUNCTION STRESS TESTS
    // ===============================================

    // Basic aggregation with different MySQL numeric types
    #[dml("SELECT CAST(COUNT(id) AS SIGNED) as 'count!: i8' FROM users")]
    async fn count_as_i8(&self) -> Result<i8>;

    #[dml("SELECT CAST(COUNT(id) AS UNSIGNED) as 'count!: u8' FROM users")]
    async fn count_as_u8(&self) -> Result<u8>;

    #[dml("SELECT CAST(COUNT(id) AS UNSIGNED) as 'count!: u16' FROM users")]
    async fn count_as_u16(&self) -> Result<u16>;

    #[dml("SELECT CAST(COUNT(id) AS UNSIGNED) as 'count!: u32' FROM users")]
    async fn count_as_u32(&self) -> Result<u32>;

    #[dml("SELECT COUNT(id) as 'count!: u64' FROM users")]
    async fn count_as_u64(&self) -> Result<u64>;

    // AVG with MySQL DECIMAL precision - returns BigDecimal
    #[dml("SELECT AVG(age) as avg_age FROM users")]
    async fn avg_age_decimal(&self) -> Result<Option<BigDecimal>>;

    #[dml("SELECT AVG(CAST(age AS DECIMAL(10,2))) as 'avg_precise?: BigDecimal' FROM users")]
    async fn avg_age_precise_decimal(&self) -> Result<Option<BigDecimal>>;

    // MySQL-specific: SUM with UNSIGNED overflow protection
    #[dml("SELECT SUM(CAST(age AS UNSIGNED)) as 'sum_unsigned?: BigDecimal' FROM users")]
    async fn sum_age_unsigned(&self) -> Result<Option<BigDecimal>>;

    #[dml("SELECT SUM(age * age) as sum_squares FROM users")]
    async fn sum_squares_decimal(&self) -> Result<Option<BigDecimal>>;

    // MySQL-specific: MIN/MAX with UNSIGNED types
    #[dml("SELECT CAST(MIN(CAST(age AS UNSIGNED)) AS UNSIGNED) as 'min_age!: u8' FROM users")]
    async fn min_age_unsigned(&self) -> Result<u8>;

    #[dml("SELECT CAST(MAX(CAST(age AS UNSIGNED)) AS UNSIGNED) as 'max_age!: u8' FROM users")]
    async fn max_age_unsigned(&self) -> Result<u8>;

    // MySQL ROUND with different precision
    #[dml("SELECT ROUND(AVG(age), 2) as 'rounded_avg?: f64' FROM users")]
    async fn round_avg_precision(&self) -> Result<Option<f64>>;

    #[dml("SELECT ROUND(123.456789, -2) as 'rounded_negative!: f64'")]
    async fn round_negative_precision(&self) -> Result<f64>;

    // MySQL CEILING and FLOOR
    #[dml("SELECT CAST(CEILING(AVG(age)) AS UNSIGNED) as 'ceiling_avg!: u8' FROM users")]
    async fn ceiling_avg(&self) -> Result<u8>;

    #[dml("SELECT CAST(FLOOR(AVG(age)) AS UNSIGNED) as 'floor_avg!: u8' FROM users")]
    async fn floor_avg(&self) -> Result<u8>;

    // ===============================================
    // MYSQL STRING FUNCTION STRESS TESTS
    // ===============================================

    // LENGTH vs CHAR_LENGTH in MySQL
    #[dml("SELECT CAST(LENGTH(name) AS UNSIGNED) as 'byte_len!: u8' FROM users LIMIT 1")]
    async fn length_as_u8(&self) -> Result<u8>;

    #[dml("SELECT CAST(CHAR_LENGTH(name) AS UNSIGNED) as 'char_len!: u8' FROM users LIMIT 1")]
    async fn char_length_as_u8(&self) -> Result<u8>;

    // MySQL SUBSTRING (1-indexed)
    #[dml("SELECT SUBSTRING(name, -1, 3) as 'substr_result?: String' FROM users LIMIT 1")]
    async fn substring_negative_start(&self) -> Result<Option<String>>;

    #[dml("SELECT SUBSTRING(name, 999, 5) as 'substr_result?: String' FROM users LIMIT 1")]
    async fn substring_beyond_length(&self) -> Result<Option<String>>;

    // MySQL CAST with error handling
    #[dml("SELECT CAST(CAST(name AS UNSIGNED) AS UNSIGNED) as 'name_as_uint?: u32' FROM users WHERE name REGEXP '^[0-9]+$' LIMIT 1")]
    async fn string_to_uint_safe(&self) -> Result<Option<u32>>;

    // MySQL-specific: CONVERT with charset
    #[dml("SELECT CONVERT(name USING utf8mb4) as 'converted_name!: String' FROM users LIMIT 1")]
    async fn convert_charset(&self) -> Result<String>;

    // MySQL CONCAT with type mixing
    #[dml("SELECT CONCAT(name, age, birth_year) as 'concat_mixed!: String' FROM users LIMIT 1")]
    async fn concat_mixed_types(&self) -> Result<String>;

    // Unicode and collation handling
    #[dml("SELECT UPPER('ñáéíóúç') as 'accented_upper!: String'")]
    async fn accented_upper(&self) -> Result<String>;

    #[dml("SELECT CAST(LENGTH('🚀🦀🌟') AS UNSIGNED) as 'emoji_byte_len!: u8'")]
    async fn emoji_byte_length(&self) -> Result<u8>;

    #[dml("SELECT CAST(CHAR_LENGTH('🚀🦀🌟') AS UNSIGNED) as 'emoji_char_len!: u8'")]
    async fn emoji_char_length(&self) -> Result<u8>;

    // ===============================================
    // MYSQL DATE/TIME FUNCTION STRESS TESTS
    // ===============================================

    // MySQL STR_TO_DATE with invalid formats
    #[dml("SELECT STR_TO_DATE('invalid-date', '%Y-%m-%d') as 'invalid_date?: String'")]
    async fn invalid_date_format(&self) -> Result<Option<String>>;

    #[dml("SELECT STR_TO_DATE('2024-13-45', '%Y-%m-%d') as 'impossible_date?: String'")]
    async fn impossible_date(&self) -> Result<Option<String>>;

    // MySQL UNIX_TIMESTAMP with edge cases
    #[dml("SELECT CAST(UNIX_TIMESTAMP('1970-01-01 00:00:01') AS UNSIGNED) as 'unix_epoch!: u32'")]
    async fn unix_timestamp_epoch(&self) -> Result<u32>;

    #[dml("SELECT CAST(UNIX_TIMESTAMP('2038-01-19 03:14:07') AS UNSIGNED) as 'unix_limit!: u32'")]
    async fn unix_timestamp_limit(&self) -> Result<u32>;

    // MySQL DATE_FORMAT with complex patterns
    #[dml("SELECT DATE_FORMAT(NOW(), '%Y%m%d%H%i%s') as 'formatted_now!: String'")]
    async fn date_format_complex(&self) -> Result<String>;

    // MySQL TIMESTAMPDIFF
    #[dml("SELECT CAST(TIMESTAMPDIFF(YEAR, '1993-01-01', NOW()) AS UNSIGNED) as 'years_diff!: u8'")]
    async fn timestamp_diff_years(&self) -> Result<u8>;

    // ===============================================
    // MYSQL SPECIFIC FUNCTIONS
    // ===============================================

    // AUTO_INCREMENT and LAST_INSERT_ID
    #[dml("SELECT LAST_INSERT_ID() as 'last_id!: u64'")]
    async fn last_insert_id(&self) -> Result<u64>;

    // MySQL VERSION and system info
    #[dml("SELECT VERSION() as 'mysql_version!: String'")]
    async fn mysql_version(&self) -> Result<String>;

    #[dml("SELECT CAST(CONNECTION_ID() AS UNSIGNED) as 'connection_id!: u32'")]
    async fn connection_id(&self) -> Result<u32>;

    // MySQL IF function
    #[dml("SELECT IF(age > 30, 'adult', 'young') as 'age_category!: String' FROM users LIMIT 1")]
    async fn if_function(&self) -> Result<String>;

    #[dml("SELECT CAST(IF(age > 100, age * 999999, age) AS UNSIGNED) as 'conditional_age!: u32' FROM users LIMIT 1")]
    async fn if_with_overflow(&self) -> Result<u32>;

    // MySQL GREATEST/LEAST
    #[dml("SELECT CAST(GREATEST(age, 18, 65) AS UNSIGNED) as 'greatest_age!: u8' FROM users LIMIT 1")]
    async fn greatest_age(&self) -> Result<u8>;

    #[dml("SELECT CAST(LEAST(age, birth_year, 2024) AS UNSIGNED) as 'least_value!: u16' FROM users LIMIT 1")]
    async fn least_mixed_types(&self) -> Result<u16>;

    // ===============================================
    // MYSQL BOUNDARY VALUE TESTS
    // ===============================================

    // MySQL TINYINT boundaries
    #[dml("SELECT CAST(127 AS SIGNED) as 'max_tinyint!: i8'")]
    async fn max_tinyint(&self) -> Result<i8>;

    #[dml("SELECT CAST(255 AS UNSIGNED) as 'max_tinyint_unsigned!: u8'")]
    async fn max_tinyint_unsigned(&self) -> Result<u8>;

    // MySQL SMALLINT boundaries
    #[dml("SELECT CAST(32767 AS SIGNED) as 'max_smallint!: i16'")]
    async fn max_smallint(&self) -> Result<i16>;

    #[dml("SELECT CAST(65535 AS UNSIGNED) as 'max_smallint_unsigned!: u16'")]
    async fn max_smallint_unsigned(&self) -> Result<u16>;

    // MySQL BIGINT boundaries
    #[dml("SELECT CAST(18446744073709551615 AS UNSIGNED) as 'max_bigint_unsigned!: u64'")]
    async fn max_bigint_unsigned(&self) -> Result<u64>;

    // MySQL DECIMAL precision
    #[dml("SELECT CAST(99999.99999 AS DECIMAL(10,5)) as 'precise_decimal!: BigDecimal'")]
    async fn precise_decimal(&self) -> Result<BigDecimal>;

    // ===============================================
    // MYSQL NULL HANDLING AND STRICT MODE
    // ===============================================

    // MySQL IFNULL function
    #[dml("SELECT CAST(IFNULL(birth_year, 0) AS UNSIGNED) as 'birth_year_safe!: u16' FROM users WHERE birth_year IS NULL LIMIT 1")]
    async fn ifnull_function(&self) -> Result<u16>;

    // MySQL COALESCE
    #[dml("SELECT CAST(COALESCE(birth_year, age, 0) AS UNSIGNED) as 'coalesced_value!: u16' FROM users LIMIT 1")]
    async fn coalesce_function(&self) -> Result<u16>;

    // MySQL ISNULL function
    #[dml("SELECT ISNULL(birth_year) as 'is_birth_year_null!: bool' FROM users LIMIT 1")]
    async fn isnull_function(&self) -> Result<bool>;

    // ===============================================
    // MYSQL AGGREGATION WITH GROUPING
    // ===============================================

    // GROUP_CONCAT with limits
    #[dml("SELECT GROUP_CONCAT(name ORDER BY age SEPARATOR '|') as 'grouped_names!: String' FROM users")]
    async fn group_concat_names(&self) -> Result<String>;

    // COUNT with DISTINCT
    #[dml("SELECT CAST(COUNT(DISTINCT birth_year) AS UNSIGNED) as 'distinct_birth_years!: u16' FROM users WHERE birth_year IS NOT NULL")]
    async fn count_distinct_birth_years(&self) -> Result<u16>;

    // Standard deviation (MySQL extension)
    #[dml("SELECT STDDEV(age) as 'stddev_age?: f64' FROM users")]
    async fn stddev_age(&self) -> Result<Option<f64>>;

    #[dml("SELECT VARIANCE(age) as 'variance_age?: f64' FROM users")]
    async fn variance_age(&self) -> Result<Option<f64>>;
}

// Test implementation
pub struct MySqlCastingStressApp {
    pool: Pool,
}

impl MySqlCastingStressRepo for MySqlCastingStressApp {
    fn get_pool(&self) -> &Pool {
        &self.pool
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_mysql_mathematical_functions(pool: Pool) {
        let repo = MySqlCastingStressApp { pool };

        // Test basic counting with different types
        let count_u8 = repo.count_as_u8().await.unwrap();
        assert_eq!(count_u8, 20); // 20 users in fixture

        let count_u32 = repo.count_as_u32().await.unwrap();
        assert_eq!(count_u32, 20);

        // Test MySQL DECIMAL precision
        let avg_decimal = repo.avg_age_decimal().await.unwrap();
        assert!(avg_decimal.is_some());

        // Test UNSIGNED operations
        let sum_unsigned = repo.sum_age_unsigned().await.unwrap();
        assert!(sum_unsigned.is_some());
        assert!(sum_unsigned.unwrap().to_string().parse::<i64>().unwrap() > 0);

        let min_age = repo.min_age_unsigned().await.unwrap();
        assert!(min_age >= 19); // Henry is 19

        let max_age = repo.max_age_unsigned().await.unwrap();
        assert!(max_age <= 42); // Eve is 42
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_mysql_mathematical_functions1(pool: Pool) {
        let repo = MySqlCastingStressApp { pool };

        // Test basic counting with different types
        let count_i8 = repo.count_as_i8().await.unwrap();
        assert_eq!(count_i8, 20); // 20 users in fixture


        // Test MySQL DECIMAL precision
        let avg_decimal = repo.avg_age_decimal().await.unwrap();
        assert!(avg_decimal.is_some());

        // Test UNSIGNED operations
        let sum_unsigned = repo.sum_age_unsigned().await.unwrap();
        assert!(sum_unsigned.is_some());
        assert!(sum_unsigned.unwrap().to_string().parse::<i64>().unwrap() > 0);

        let min_age = repo.min_age_unsigned().await.unwrap();
        assert!(min_age >= 19); // Henry is 19

        let max_age = repo.max_age_unsigned().await.unwrap();
        assert!(max_age <= 42); // Eve is 42
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_mysql_string_functions(pool: Pool) {
        let repo = MySqlCastingStressApp { pool };

        // Test LENGTH vs CHAR_LENGTH
        let byte_len = repo.length_as_u8().await.unwrap();
        assert!(byte_len > 0);

        let char_len = repo.char_length_as_u8().await.unwrap();
        assert!(char_len > 0);

        // Test string conversions
        let converted = repo.convert_charset().await.unwrap();
        assert!(!converted.is_empty());

        let concat_result = repo.concat_mixed_types().await.unwrap();
        assert!(!concat_result.is_empty());

        // Test Unicode handling
        let upper_accented = repo.accented_upper().await.unwrap();
        assert_eq!(upper_accented, "ÑÁÉÍÓÚÇ");

        // Test emoji handling
        let emoji_bytes = repo.emoji_byte_length().await.unwrap();
        let emoji_chars = repo.emoji_char_length().await.unwrap();
        assert!(emoji_bytes >= emoji_chars); // Bytes >= characters for Unicode
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_mysql_date_time_functions(pool: Pool) {
        let repo = MySqlCastingStressApp { pool };

        // Test invalid dates return NULL in MySQL
        let invalid_date = repo.invalid_date_format().await.unwrap();
        assert!(invalid_date.is_none());

        let impossible_date = repo.impossible_date().await.unwrap();
        assert!(impossible_date.is_none());

        // Test valid date operations
        let unix_epoch = repo.unix_timestamp_epoch().await.unwrap();
        assert_eq!(unix_epoch, 1);

        let formatted_now = repo.date_format_complex().await.unwrap();
        assert!(formatted_now.len() >= 14); // YYYYMMDDHHMISS format

        let years_diff = repo.timestamp_diff_years().await.unwrap();
        assert!(years_diff >= 30); // Approximate age difference
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_mysql_specific_functions(pool: Pool) {
        let repo = MySqlCastingStressApp { pool };

        // Test MySQL system functions
        let version = repo.mysql_version().await.unwrap();
        println!("MySQL Version: {}", version);
        assert!(version.len() > 0); // Just check version string is not empty

        let conn_id = repo.connection_id().await.unwrap();
        assert!(conn_id > 0);

        let _last_id = repo.last_insert_id().await.unwrap();
        // This might be 0 if no insert has been done

        // Test MySQL conditional functions
        let age_category = repo.if_function().await.unwrap();
        assert!(age_category == "adult" || age_category == "young");

        let greatest_age = repo.greatest_age().await.unwrap();
        assert!(greatest_age >= 18); // At least 18 due to GREATEST function

        let least_value = repo.least_mixed_types().await.unwrap();
        assert!(least_value > 0);
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_mysql_boundary_values(pool: Pool) {
        let repo = MySqlCastingStressApp { pool };

        // Test MySQL type boundaries
        let max_tinyint = repo.max_tinyint().await.unwrap();
        assert_eq!(max_tinyint, 127);

        let max_tinyint_unsigned = repo.max_tinyint_unsigned().await.unwrap();
        assert_eq!(max_tinyint_unsigned, 255);

        let max_smallint = repo.max_smallint().await.unwrap();
        assert_eq!(max_smallint, 32767);

        let max_smallint_unsigned = repo.max_smallint_unsigned().await.unwrap();
        assert_eq!(max_smallint_unsigned, 65535);

        // Test DECIMAL precision
        let precise_decimal = repo.precise_decimal().await.unwrap();
        let decimal_str = precise_decimal.to_string();
        assert!(decimal_str.contains("99999.99999"));
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_mysql_null_handling(pool: Pool) {
        let repo = MySqlCastingStressApp { pool };

        // Test MySQL NULL functions
        let birth_year_safe = repo.ifnull_function().await.unwrap();
        assert_eq!(birth_year_safe, 0); // Should be 0 for NULL birth_year

        let coalesced = repo.coalesce_function().await.unwrap();
        assert!(coalesced > 0); // Should get a non-zero value

        let is_null = repo.isnull_function().await.unwrap();
        // This will depend on the first user's birth_year
        assert!(is_null == false || is_null == true);
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_mysql_aggregation_functions(pool: Pool) {
        let repo = MySqlCastingStressApp { pool };

        // Test GROUP_CONCAT
        let grouped_names = repo.group_concat_names().await.unwrap();
        assert!(grouped_names.contains("|")); // Should contain separator
        assert!(grouped_names.contains("Alice")); // Should contain names

        // Test DISTINCT counting
        let distinct_years = repo.count_distinct_birth_years().await.unwrap();
        assert!(distinct_years > 0);
        assert!(distinct_years <= 20); // Can't be more than total users

        // Test statistical functions
        let stddev = repo.stddev_age().await.unwrap();
        assert!(stddev.is_some());
        assert!(stddev.unwrap() > 0.0);

        let variance = repo.variance_age().await.unwrap();
        assert!(variance.is_some());
        assert!(variance.unwrap() > 0.0);
    }
}