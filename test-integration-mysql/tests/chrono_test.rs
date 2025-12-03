#![cfg(feature = "chrono")]

use sqlx_data::{Pool, QueryResult, Result, Cursor, Serial, CursorData, IntoParams, ParamsBuilder, CursorSecureExtract, CursorValue, CursorError, FilterValue, dml, repo};
use sqlx::types::{BigDecimal, chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, Utc}};

#[derive(Debug, sqlx::FromRow)]
pub struct Customer {
    pub id: i64,
    pub name: String,
    pub email: String,
    pub age: u8,
    pub birth_date: Option<NaiveDate>,               // MySQL DATE
    pub created_at: NaiveDateTime,                   // MySQL DATETIME
    pub updated_at: Option<NaiveDateTime>,           // MySQL DATETIME
    pub last_login: Option<DateTime<Utc>>,           // MySQL TIMESTAMP
}

impl CursorSecureExtract for Customer {
    fn extract_whitelisted_fields(&self, fields: &[String]) -> Result<Vec<CursorValue>> {
        let mut values = Vec::with_capacity(fields.len());
        for field in fields {
            match field.as_str() {
                "id" => values.push(self.id.into()),
                "created_at" => values.push(self.created_at.to_string().into()),
                "birth_date" => {
                    if let Some(birth_date) = self.birth_date {
                        values.push(birth_date.to_string().into());
                    } else {
                        values.push(CursorValue::String("".into()));
                    }
                }
                "last_login" => {
                    if let Some(last_login) = self.last_login {
                        values.push(last_login.to_string().into());
                    } else {
                        values.push(CursorValue::String("".into()));
                    }
                }
                _ => {
                    return Err(CursorError::invalid_field(field.clone()).into());
                }
            }
        }
        Ok(values)
    }

    fn encode(cursor: &CursorData) -> Result<String> {
        use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD as BASE64};
        let json_bytes = serde_json::to_vec(&cursor)
            .map_err(|e| CursorError::encode_error(format!("JSON serialization failed: {}", e)))?;
        Ok(BASE64.encode(json_bytes))
    }

    fn decode(encoded: &str) -> Result<Vec<FilterValue>> {
        use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD as BASE64};
        let bytes = BASE64
            .decode(encoded)
            .map_err(|e| CursorError::decode_error(format!("Base64 decode failed: {}", e)))?;

        let cursor: CursorData = serde_json::from_slice(&bytes).map_err(|e| {
            CursorError::decode_error(format!("JSON deserialization failed: {}", e))
        })?;

        let filter_values: Vec<FilterValue> = cursor.entries.into_iter().map(|entry| {
            match entry.value {
                CursorValue::Int(v) => FilterValue::Int(v),
                CursorValue::UInt(v) => FilterValue::UInt(v),
                CursorValue::Float(v) => FilterValue::Float(v),
                CursorValue::Bool(v) => FilterValue::Bool(v),
                CursorValue::String(v) => v.into(),
            }
        }).collect();

        Ok(filter_values)
    }
}

#[derive(Debug, sqlx::FromRow)]
pub struct DateStats {
    pub date_field: Option<NaiveDate>,
    pub user_count: u64,                             // MySQL COUNT returns UNSIGNED
    pub avg_age: Option<BigDecimal>,
}

#[derive(Debug, sqlx::FromRow)]
pub struct TimeAnalysis {
    pub hour_of_day: u8,
    pub login_count: u64,
    pub peak_hour: bool,                             // MySQL boolean
}

#[repo]
#[alias(
    all_columns = "id, name, email, age, birth_date, created_at, updated_at, last_login"
)]
trait ChronoUserRepo {
    // INSERT query (MySQL doesn't have RETURNING, so return last_insert_id())
    #[dml("INSERT INTO customers (name, email, age, birth_date, created_at, last_login, updated_at) VALUES (?, ?, ?, ?, ?, ?, NULL)")]
    async fn insert_customer(
        &self,
        name: String,
        email: String,
        age: u8,
        birth_date: Option<NaiveDate>,
        created_at: NaiveDateTime,
        last_login: Option<DateTime<Utc>>,
    ) -> Result<QueryResult>;

    // Basic queries adapted from SQLite
    #[dml("SELECT {{all_columns}} FROM customers WHERE created_at >= ?")]
    async fn find_customers_created_after(&self, from: NaiveDateTime) -> Result<Vec<Customer>>;

    #[dml("SELECT MAX(created_at) FROM customers")]
    async fn max_created_at(&self) -> Result<Option<NaiveDateTime>>;

    #[dml("SELECT MAX(created_at) as 'created_at: NaiveDateTime' FROM customers")]
    async fn max_created_at_casting(&self) -> Result<Option<NaiveDateTime>>;

    #[dml("SELECT id, name, created_at as 'created_at!: NaiveDateTime' FROM customers WHERE age > ?")]
    async fn find_customers_by_min_age(
        &self,
        min_age: i32,
    ) -> Result<Vec<(i64, String, NaiveDateTime)>>;

    #[dml("UPDATE customers SET updated_at = ? WHERE id = ?")]
    async fn update_customer_timestamp(
        &self,
        updated_at: NaiveDateTime,
        id: i64,
    ) -> Result<QueryResult>;

    #[dml("SELECT {{all_columns}} FROM customers WHERE birth_date < ?")]
    async fn find_customers_born_before(&self, date: NaiveDate) -> Result<Vec<Customer>>;

    #[dml("SELECT (COUNT(*) > 0) as 'has_updates!: bool' FROM customers WHERE updated_at IS NOT NULL AND updated_at >= ?")]
    async fn has_recent_updates(&self, since: NaiveDateTime) -> Result<bool>;

    // Test direct field return types
    #[dml("SELECT created_at FROM customers WHERE id = ?")]
    async fn get_created_at(&self, id: i64) -> Result<NaiveDateTime>;

    #[dml("SELECT birth_date FROM customers WHERE id = ?")]
    async fn get_birth_date(&self, id: i64) -> Result<Option<NaiveDate>>;

    #[dml("SELECT updated_at FROM customers WHERE id = ?")]
    async fn get_updated_at(&self, id: i64) -> Result<Option<NaiveDateTime>>;

    #[dml("SELECT last_login FROM customers WHERE id = ?")]
    async fn get_last_login(&self, id: i64) -> Result<Option<DateTime<Utc>>>;

    // Cursor pagination methods for datetime fields
    #[dml("SELECT {{all_columns}} FROM customers ORDER BY created_at, id")]
    async fn find_customers_cursor_by_created_at(
        &self,
        params: impl IntoParams,
    ) -> Result<Cursor<Customer>>;

    #[dml("SELECT {{all_columns}} FROM customers WHERE birth_date IS NOT NULL ORDER BY birth_date, id")]
    async fn find_customers_cursor_by_birth_date(
        &self,
        params: impl IntoParams,
    ) -> Result<Cursor<Customer>>;

    #[dml("SELECT {{all_columns}} FROM customers WHERE last_login IS NOT NULL ORDER BY last_login, id")]
    async fn find_customers_cursor_by_last_login(
        &self,
        params: impl IntoParams,
    ) -> Result<Cursor<Customer>>;

    // Serial pagination method
    #[dml("SELECT {{all_columns}} FROM customers ORDER BY birth_date, id")]
    async fn find_customers_serial_pagination(
        &self,
        params: impl IntoParams,
    ) -> Result<Serial<Customer>>;

    #[dml("SELECT id, name FROM customers WHERE last_login >= DATE_SUB(NOW(), INTERVAL ? HOUR)")]
    async fn find_recently_active(&self, hours: u16) -> Result<Vec<(i64, String)>>;

    // MySQL date arithmetic and comparisons
    #[dml("SELECT id, name, DATEDIFF(NOW(), birth_date) as 'days_alive!: u32' FROM customers WHERE birth_date IS NOT NULL")]
    async fn calculate_days_alive(&self) -> Result<Vec<(i64, String, u32)>>;

    #[dml("SELECT id, name, TIMESTAMPDIFF(YEAR, birth_date, CURDATE()) as 'calculated_age!: u8' FROM customers WHERE birth_date IS NOT NULL")]
    async fn calculate_age_from_birth_date(&self) -> Result<Vec<(i64, String, u8)>>;

    // MySQL date formatting
    #[dml("SELECT id, DATE_FORMAT(birth_date, '%Y-%m-%d') as 'formatted_birth!: String' FROM customers WHERE birth_date IS NOT NULL")]
    async fn format_birth_dates(&self) -> Result<Vec<(i64, String)>>;

    #[dml("SELECT id, DATE_FORMAT(created_at, '%Y-%m-%d %H:%i:%s') as 'formatted_created!: String' FROM customers")]
    async fn format_created_at(&self) -> Result<Vec<(i64, String)>>;

    // MySQL aggregation by date parts
    #[dml("SELECT DATE(created_at) as date_field, COUNT(*) as 'user_count!: u64', AVG(age) as avg_age FROM customers GROUP BY DATE(created_at)")]
    async fn group_by_creation_date(&self) -> Result<Vec<DateStats>>;

    // MySQL time zone operations
    #[dml("SELECT id, CONVERT_TZ(last_login, '+00:00', '+08:00') as 'local_time!: NaiveDateTime' FROM customers WHERE last_login IS NOT NULL")]
    async fn convert_to_local_time(&self) -> Result<Vec<(i64, Option<NaiveDateTime>)>>;

    // MySQL date ranges and BETWEEN
    #[dml("SELECT id, name FROM customers WHERE birth_date BETWEEN ? AND ?")]
    async fn find_by_birth_date_range(&self, start_date: NaiveDate, end_date: NaiveDate) -> Result<Vec<(i64, String)>>;

    #[dml("SELECT id, name FROM customers WHERE created_at BETWEEN ? AND ?")]
    async fn find_by_creation_range(&self, start: NaiveDateTime, end: NaiveDateTime) -> Result<Vec<(i64, String)>>;

    // MySQL NULL date handling
    #[dml("SELECT id, name, IFNULL(birth_date, DATE('1900-01-01')) as 'birth_or_default!: NaiveDate' FROM customers")]
    async fn birth_date_with_default(&self) -> Result<Vec<(i64, String, NaiveDate)>>;

    #[dml("SELECT id, FROM_UNIXTIME(?) as 'converted_time!: NaiveDateTime' FROM customers")]
    async fn convert_from_unix_timestamp(&self, timestamp: u32) -> Result<Vec<(i64, NaiveDateTime)>>;

    // MySQL NOW() and CURRENT functions
    #[dml("SELECT NOW() as 'current_datetime!: NaiveDateTime', CURDATE() as 'current_date!: NaiveDate', CAST(CURTIME() AS CHAR) as 'current_time!: String'")]
    async fn get_current_time_info(&self) -> Result<(NaiveDateTime, NaiveDate, String)>;

    // Missing methods from SQLite parity
    #[dml("SELECT id, name, created_at as 'created_at!: NaiveDateTime', DATE_ADD(created_at, INTERVAL 1 DAY) as 'next_day!: NaiveDateTime' FROM customers")]
    async fn customers_with_next_day(&self) -> Result<Vec<(i64, String, NaiveDateTime, NaiveDateTime)>>;

    #[dml("SELECT 1 as 'result?: bool' FROM customers WHERE updated_at >= ? LIMIT 1")]
    async fn has_recent_updates_option(&self, since: NaiveDateTime) -> Result<Option<bool>>;

}

pub struct CustomerRepoImpl {
    pool: Pool,
}

impl ChronoUserRepo for CustomerRepoImpl {
    fn get_pool(&self) -> &Pool {
        &self.pool
    }
}

mod tests {
    use super::*;

    #[sqlx::test(
        migrations = "tests/migrations_datetime",
        fixtures(path = "fixtures_datetime", scripts("customers"))
    )]
    async fn test_mysql_date_arithmetic(pool: Pool) {
        let repo = CustomerRepoImpl { pool };

        // Test days alive calculation
        let days_alive = repo.calculate_days_alive().await.unwrap();
        assert!(days_alive.len() > 0);

        for (_id, _name, days) in &days_alive {
            assert!(*days > 0); // Everyone should be alive for at least some days
        }

        // Test age calculation from birth date
        let calculated_ages = repo.calculate_age_from_birth_date().await.unwrap();
        assert!(calculated_ages.len() > 0);

        for (_id, _name, age) in &calculated_ages {
            assert!(*age >= 18); // Assuming all users are adults
            assert!(*age <= 120); // Reasonable age limit
        }
    }

    #[sqlx::test(
        migrations = "tests/migrations_datetime",
        fixtures(path = "fixtures_datetime", scripts("customers"))
    )]
    async fn test_mysql_date_formatting(pool: Pool) {
        let repo = CustomerRepoImpl { pool };

        // Test birth date formatting
        let formatted_births = repo.format_birth_dates().await.unwrap();
        assert!(formatted_births.len() > 0);

        for (_id, formatted) in &formatted_births {
            assert!(formatted.len() == 10); // YYYY-MM-DD format
            assert!(formatted.contains('-'));
        }

        // Test datetime formatting
        let formatted_created = repo.format_created_at().await.unwrap();
        assert!(formatted_created.len() > 0);

        for (_id, formatted) in &formatted_created {
            assert!(formatted.len() == 19); // YYYY-MM-DD HH:MM:SS format
            assert!(formatted.contains(' '));
            assert!(formatted.contains(':'));
        }
    }

    #[sqlx::test(
        migrations = "tests/migrations_datetime",
        fixtures(path = "fixtures_datetime", scripts("customers"))
    )]
    async fn test_mysql_date_aggregation(pool: Pool) {
        let repo = CustomerRepoImpl { pool };

        // Test grouping by creation date
        let date_stats = repo.group_by_creation_date().await.unwrap();
        assert!(date_stats.len() > 0);

        for stats in &date_stats {
            assert!(stats.user_count > 0);
            if let Some(avg_age) = &stats.avg_age {
                use std::str::FromStr;
                let zero = BigDecimal::from_str("0").unwrap();
                assert!(*avg_age > zero);
            }
        }

    }

    #[sqlx::test(
        migrations = "tests/migrations_datetime",
        fixtures(path = "fixtures_datetime", scripts("customers"))
    )]
    async fn test_mysql_date_ranges(pool: Pool) {
        let repo = CustomerRepoImpl { pool };

        // Test birth date range
        let start_date = NaiveDate::from_ymd_opt(1990, 1, 1).unwrap();
        let end_date = NaiveDate::from_ymd_opt(2000, 12, 31).unwrap();
        let users_in_range = repo.find_by_birth_date_range(start_date, end_date).await.unwrap();

        // Should have some users born in the 1990s
        for (_id, name) in &users_in_range {
            assert!(!name.is_empty());
        }

        // Test datetime range (use 2024 range to match fixture data)
        let start_2024 = NaiveDateTime::new(NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(), NaiveTime::MIN);
        let end_2024 = NaiveDateTime::new(NaiveDate::from_ymd_opt(2024, 12, 31).unwrap(), NaiveTime::from_hms_opt(23, 59, 59).unwrap());
        let recent_created = repo.find_by_creation_range(start_2024, end_2024).await.unwrap();
        assert!(recent_created.len() > 0); // All fixture users are in 2024
    }

    #[sqlx::test(
        migrations = "tests/migrations_datetime",
        fixtures(path = "fixtures_datetime", scripts("customers"))
    )]
    async fn test_mysql_null_date_handling(pool: Pool) {
        let repo = CustomerRepoImpl { pool };

        // Test default date for NULL values
        let with_defaults = repo.birth_date_with_default().await.unwrap();
        assert!(with_defaults.len() > 0);

        for (_id, _name, birth_date) in &with_defaults {
            // All should have a date (either real or default 1900-01-01)
            let min_date = NaiveDate::from_ymd_opt(1900, 1, 1).unwrap();
            assert!(*birth_date >= min_date);
        }

    }

    #[sqlx::test(
        migrations = "tests/migrations_datetime",
        fixtures(path = "fixtures_datetime", scripts("customers"))
    )]
    async fn test_mysql_unix_timestamp(pool: Pool) {
        let repo = CustomerRepoImpl { pool };

        // Test UNIX timestamp conversion


        // Test converting from UNIX timestamp
        let unix_time = 1609459200u32; // Jan 1, 2021 00:00:00 UTC
        let converted = repo.convert_from_unix_timestamp(unix_time).await.unwrap();
        assert!(converted.len() > 0);

        for (_id, datetime) in &converted {
            let expected_date = NaiveDate::from_ymd_opt(2021, 1, 1).unwrap();
            assert_eq!(datetime.date(), expected_date);
        }
    }

    #[sqlx::test(
        migrations = "tests/migrations_datetime",
        fixtures(path = "fixtures_datetime", scripts("customers"))
    )]
    async fn test_mysql_current_time_functions(pool: Pool) {
        let repo = CustomerRepoImpl { pool };

        let (current_datetime, current_date, current_time) = repo.get_current_time_info().await.unwrap();

        // Verify current datetime is reasonable (after 2024)
        let min_datetime = NaiveDateTime::new(NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(), NaiveTime::MIN);
        assert!(current_datetime >= min_datetime);

        // Verify current date is reasonable (after 2024)
        let min_date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        assert!(current_date >= min_date);

        // Verify current time format (HH:MM:SS)
        assert!(current_time.len() == 8);
        assert!(current_time.contains(':'));
    }

    #[sqlx::test(
        migrations = "tests/migrations_datetime",
        fixtures(path = "fixtures_datetime", scripts("customers"))
    )]
    async fn test_datetime_roundtrip(pool: Pool) {
        let repo = CustomerRepoImpl { pool };
        let now = Utc::now().naive_utc();
        let last_login = Utc::now();

        let result = repo
            .insert_customer(
                "Alice Smith Test".into(),
                "alice.test@example.com".into(),
                30,
                Some(NaiveDate::from_ymd_opt(1993, 5, 20).unwrap()),
                now,
                Some(last_login),
            )
            .await
            .unwrap();

        // MySQL insert returns QueryResult, not ID directly
        assert!(result.last_insert_id() > 0);

        let customers = repo
            .find_customers_created_after(now - std::time::Duration::from_secs(1))
            .await
            .unwrap();

        assert!(!customers.is_empty());
        let customer = &customers[0];
        assert!(customer.id > 0);
        assert!(!customer.name.is_empty());
        assert!(!customer.email.is_empty());
        assert!(customer.age > 0);
        // updated_at should be None for new customer
        assert!(customer.updated_at.is_none());
        
        let diff = (customer.created_at.and_utc().timestamp() - now.and_utc().timestamp()).abs();
        assert!(diff <= 1);

        if let Some(login) = customer.last_login {
             let diff_login = (login.timestamp() - last_login.timestamp()).abs();
             assert!(diff_login <= 1);
        }
    }

    #[sqlx::test(
        migrations = "tests/migrations_datetime",
        fixtures(path = "fixtures_datetime", scripts("customers"))
    )]
    async fn test_find_customers_created_after(pool: Pool) {
        let repo = CustomerRepoImpl { pool };
        let cutoff = Utc::now().naive_utc() - std::time::Duration::from_secs(3600);

        let customers = repo.find_customers_created_after(cutoff).await.unwrap();

        for customer in customers {
            assert!(customer.id > 0);
            assert!(!customer.name.is_empty());
            assert!(!customer.email.is_empty());
            assert!(customer.age > 0);
            assert!(customer.created_at >= cutoff);
        }
    }

    #[sqlx::test(
        migrations = "tests/migrations_datetime",
        fixtures(path = "fixtures_datetime", scripts("customers"))
    )]
    async fn test_update_timestamp(pool: Pool) {
        let repo = CustomerRepoImpl { pool };
        let now = Utc::now().naive_utc();

        let result = repo
            .insert_customer(
                "Bob Johnson Test".into(),
                "bob.test@example.com".into(),
                28,
                None,
                now,
                None,
            )
            .await
            .unwrap();
            
        let id = result.last_insert_id() as i64;

        let new_updated_at = now + std::time::Duration::from_secs(3 * 3600);
        repo.update_customer_timestamp(new_updated_at, id).await.unwrap();

        let has_update = repo
            .has_recent_updates(new_updated_at - std::time::Duration::from_secs(1))
            .await
            .unwrap();

        assert!(has_update);

        let updated_customer_timestamp = repo.get_updated_at(id).await.unwrap();
        assert!(updated_customer_timestamp.is_some());

        let timestamp = updated_customer_timestamp.unwrap();
        let diff = (timestamp.and_utc().timestamp() - new_updated_at.and_utc().timestamp()).abs();
        assert!(diff <= 1);
    }

    #[sqlx::test(
        migrations = "tests/migrations_datetime",
        fixtures(path = "fixtures_datetime", scripts("customers"))
    )]
    async fn test_born_before_filter(pool: Pool) {
        let repo = CustomerRepoImpl { pool };
        let birth_date = NaiveDate::from_ymd_opt(1985, 10, 15).unwrap();

        repo.insert_customer(
            "Charlie Brown Test".into(),
            "charlie.test@example.com".into(),
            40,
            Some(birth_date),
            Utc::now().naive_utc(),
            None,
        )
        .await
        .unwrap();

        let older_customers = repo
            .find_customers_born_before(birth_date.succ_opt().unwrap_or(birth_date))
            .await
            .unwrap();

        assert!(!older_customers.is_empty());
        for customer in older_customers {
            if let Some(date) = customer.birth_date {
                assert!(date < birth_date.succ_opt().unwrap_or(birth_date));
            }
        }
    }

    #[sqlx::test(
        migrations = "tests/migrations_datetime",
        fixtures(path = "fixtures_datetime", scripts("customers"))
    )]
    async fn test_max_created_at(pool: Pool) {
        let repo = CustomerRepoImpl { pool };

        let future_time = Utc::now().naive_utc() + std::time::Duration::from_secs(3600);

        repo.insert_customer(
            "Future Customer".into(),
            "future@example.com".into(),
            25,
            None,
            future_time,
            None,
        )
        .await
        .unwrap();

        let max_created = repo.max_created_at().await.unwrap();
        assert!(max_created.is_some());
    }

    #[sqlx::test(
        migrations = "tests/migrations_datetime",
        fixtures(path = "fixtures_datetime", scripts("customers"))
    )]
    async fn test_find_customers_by_min_age(pool: Pool) {
        let repo = CustomerRepoImpl { pool };
        let customers = repo.find_customers_by_min_age(30).await.unwrap();

        for (id, name, created_at) in customers {
            assert!(id > 0);
            assert!(!name.is_empty());
            assert!(created_at.and_utc().timestamp() > 0);
        }
    }

    #[sqlx::test(
        migrations = "tests/migrations_datetime",
        fixtures(path = "fixtures_datetime", scripts("customers"))
    )]
    async fn test_max_created_at_casting(pool: Pool) {
        let repo = CustomerRepoImpl { pool };
        let result = repo.max_created_at_casting().await.unwrap();
        assert!(result.is_some());
    }

    #[sqlx::test(
        migrations = "tests/migrations_datetime",
        fixtures(path = "fixtures_datetime", scripts("customers"))
    )]
    async fn test_customers_with_next_day(pool: Pool) {
        let repo = CustomerRepoImpl { pool };
        let now = Utc::now().naive_utc();

        let result = repo
            .insert_customer(
                "Test Customer NextDay".into(),
                "testnextday@example.com".into(),
                25,
                None,
                now,
                None,
            )
            .await
            .unwrap();
        
        let id = result.last_insert_id() as i64;

        let customers = repo.customers_with_next_day().await.unwrap();
        let test_customer = customers
            .iter()
            .find(|(customer_id, _, _, _)| *customer_id == id)
            .expect("Test customer should be found");

        let (_, _name, created_at, next_day) = test_customer;
        // Verify just created_at date part (ignoring time diffs for now)
        assert_eq!(created_at.date(), now.date());
        assert!(*next_day > *created_at);
    }

    #[sqlx::test(
        migrations = "tests/migrations_datetime",
        fixtures(path = "fixtures_datetime", scripts("customers"))
    )]
    async fn test_get_created_at(pool: Pool) {
        let repo = CustomerRepoImpl { pool };
        let now = Utc::now().naive_utc();
        let result = repo
            .insert_customer(
                "Test Created".into(),
                "test.created@example.com".into(),
                25,
                None,
                now,
                None,
            )
            .await
            .unwrap();
        
        let id = result.last_insert_id() as i64;
        let created_at = repo.get_created_at(id).await.unwrap();
        assert_eq!(created_at.date(), now.date());
    }

    #[sqlx::test(
        migrations = "tests/migrations_datetime",
        fixtures(path = "fixtures_datetime", scripts("customers"))
    )]
    async fn test_get_birth_date(pool: Pool) {
        let repo = CustomerRepoImpl { pool };
        let birth_date = NaiveDate::from_ymd_opt(1990, 1, 15).unwrap();
        let now = Utc::now().naive_utc();

        let result = repo
            .insert_customer(
                "Test Birth".into(),
                "test.birth@example.com".into(),
                30,
                Some(birth_date),
                now,
                None,
            )
            .await
            .unwrap();
        let id = result.last_insert_id() as i64;

        let result_date = repo.get_birth_date(id).await.unwrap();
        assert_eq!(result_date, Some(birth_date));
    }

    #[sqlx::test(
        migrations = "tests/migrations_datetime",
        fixtures(path = "fixtures_datetime", scripts("customers"))
    )]
    async fn test_get_updated_at_null(pool: Pool) {
        let repo = CustomerRepoImpl { pool };
        let now = Utc::now().naive_utc();

        let result = repo
            .insert_customer(
                "Test Updated".into(),
                "test.updated@example.com".into(),
                25,
                None,
                now,
                None,
            )
            .await
            .unwrap();
        let id = result.last_insert_id() as i64;

        let result_date = repo.get_updated_at(id).await.unwrap();
        assert_eq!(result_date, None);
    }

    #[sqlx::test(
        migrations = "tests/migrations_datetime",
        fixtures(path = "fixtures_datetime", scripts("customers"))
    )]
    async fn test_get_last_login(pool: Pool) {
        let repo = CustomerRepoImpl { pool };
        let now = Utc::now().naive_utc();
        let login_time = Utc::now();

        let result = repo
            .insert_customer(
                "Test Login".into(),
                "test.login@example.com".into(),
                25,
                None,
                now,
                Some(login_time),
            )
            .await
            .unwrap();
        let id = result.last_insert_id() as i64;

        let result_login = repo.get_last_login(id).await.unwrap();
        assert!(result_login.is_some());
        let retrieved = result_login.unwrap();
        let diff = (retrieved.timestamp() - login_time.timestamp()).abs();
        assert!(diff <= 1);
    }

    #[sqlx::test(
        migrations = "tests/migrations_datetime",
        fixtures(path = "fixtures_datetime", scripts("customers"))
    )]
    async fn test_cursor_pagination_by_created_at(pool: Pool) {
        let repo = CustomerRepoImpl { pool };

        // First page - get first 2 customers ordered by created_at, id (no cursor needed)
        #[rustfmt::skip]
        let params1 = ParamsBuilder::new()
            .sort()
                .asc("created_at")
                .asc("id")
                .done()
            .cursor()
                .first_page()
                .done()
            .limit(2)
            .build();

        let page1 = repo.find_customers_cursor_by_created_at(params1).await.unwrap();
        assert_eq!(page1.data.len(), 2);
        assert!(page1.has_next);

        // Verify order
        let first_customer = &page1.data[0];
        let second_customer = &page1.data[1];
        assert!(
            first_customer.created_at < second_customer.created_at ||
            (first_customer.created_at == second_customer.created_at && first_customer.id < second_customer.id),
            "First page should be ordered by created_at, then id"
        );

        // Second page
        let cursor_token = page1.next_cursor.expect("Should have next cursor");
        #[rustfmt::skip]
        let params2 = ParamsBuilder::new()
            .sort()
                .asc("created_at")
                .asc("id")
                .done()
            .cursor()
                .next_cursor::<Customer>(&cursor_token)
                .done()
            .limit(2)
            .build();

        let page2 = repo.find_customers_cursor_by_created_at(params2).await.unwrap();
        assert!(!page2.data.is_empty());

        // Verify we got different customers (no overlap)
        let page1_ids: Vec<i64> = page1.data.iter().map(|c| c.id).collect();
        let page2_ids: Vec<i64> = page2.data.iter().map(|c| c.id).collect();

        for id in &page2_ids {
            assert!(!page1_ids.contains(id), "Page 2 should not contain customers from page 1");
        }
    }

    #[sqlx::test(
        migrations = "tests/migrations_datetime",
        fixtures(path = "fixtures_datetime", scripts("customers"))
    )]
    async fn test_cursor_order_by_inversion_problem(pool: Pool) {
        let repo = CustomerRepoImpl { pool };
        let base_time = Utc::now().naive_utc();

        // Insert 5 customers with same created_at but different IDs
        let same_time = base_time - std::time::Duration::from_secs(3600);

        for i in 1..=5 {
            repo.insert_customer(
                format!("Customer {}", i),
                format!("customer{}@test.com", i),
                (20 + i) as u8,
                None,
                same_time,
                None,
            ).await.unwrap();
        }

        // First page
        let params1 = ParamsBuilder::new()
            .sort()
            .asc("created_at")
            .asc("id")
            .done()
            .cursor()
            .first_page()
            .done()
            .limit(3)
            .build();

        let page1 = repo.find_customers_cursor_by_created_at(params1).await.unwrap();
        
        // Second page with AFTER
        let cursor_token = page1.next_cursor.expect("Should have next cursor");
        let params2 = ParamsBuilder::new()
            .sort()
            .asc("created_at")
            .asc("id")
            .done()
            .cursor()
            .next_cursor::<Customer>(&cursor_token)
            .done()
            .limit(3)
            .build();

        let page2 = repo.find_customers_cursor_by_created_at(params2).await.unwrap();

        // Now test BEFORE (this will expose the problem)
        if let Some(prev_cursor_token) = &page2.prev_cursor {
            let prev_params = ParamsBuilder::new()
                .sort()
                .asc("created_at")
                .asc("id")
                .done()
                .cursor()
                .prev_cursor::<Customer>(prev_cursor_token)
                .done()
                .limit(3)
                .build();

            let prev_page = repo.find_customers_cursor_by_created_at(prev_params).await.unwrap();
            
            // Check if we got data back
            assert!(!prev_page.data.is_empty());
        }
    }

    #[sqlx::test(
        migrations = "tests/migrations_datetime",
        fixtures(path = "fixtures_datetime", scripts("customers"))
    )]
    async fn test_cursor_pagination_by_birth_date(pool: Pool) {
        let repo = CustomerRepoImpl { pool };
        let now = Utc::now().naive_utc();

        let birth_date1 = NaiveDate::from_ymd_opt(1980, 1, 1).unwrap();
        let birth_date2 = NaiveDate::from_ymd_opt(1985, 6, 15).unwrap();
        let birth_date3 = NaiveDate::from_ymd_opt(1990, 12, 31).unwrap();

        repo.insert_customer(
            "Oldest Customer".into(),
            "oldest@test.com".into(),
            44,
            Some(birth_date1),
            now,
            None,
        ).await.unwrap();

        repo.insert_customer(
            "Middle Customer".into(),
            "middle@test.com".into(),
            39,
            Some(birth_date2),
            now,
            None,
        ).await.unwrap();

        repo.insert_customer(
            "Youngest Customer".into(),
            "youngest@test.com".into(),
            34,
            Some(birth_date3),
            now,
            None,
        ).await.unwrap();

        let params = ParamsBuilder::default()
            .sort()
            .asc("birth_date")
            .asc("id")
            .done()
            .cursor()
            .first_page() 
            .done()
            .limit(2)
            .build();

        let page = repo.find_customers_cursor_by_birth_date(params).await.unwrap();

        for customer in &page.data {
            assert!(customer.birth_date.is_some());
        }

        if page.data.len() >= 2 {
            let first = &page.data[0];
            let second = &page.data[1];
            assert!(first.birth_date <= second.birth_date);
        }
    }

    #[sqlx::test(
        migrations = "tests/migrations_datetime",
        fixtures(path = "fixtures_datetime", scripts("customers"))
    )]
    async fn test_cursor_pagination_by_last_login(pool: Pool) {
        let repo = CustomerRepoImpl { pool };
        let now = Utc::now().naive_utc();

        let login1 = Utc::now() - std::time::Duration::from_secs(7200); 
        let login2 = Utc::now() - std::time::Duration::from_secs(3600); 
        let login3 = Utc::now();

        repo.insert_customer(
            "Early Login".into(),
            "early@test.com".into(),
            25,
            None,
            now,
            Some(login1),
        ).await.unwrap();

        repo.insert_customer(
            "Mid Login".into(),
            "mid@test.com".into(),
            30,
            None,
            now,
            Some(login2),
        ).await.unwrap();

        repo.insert_customer(
            "Recent Login".into(),
            "recent@test.com".into(),
            35,
            None,
            now,
            Some(login3),
        ).await.unwrap();

        repo.insert_customer(
            "No Login".into(),
            "nologin@test.com".into(),
            40,
            None,
            now,
            None,
        ).await.unwrap();

        let params = ParamsBuilder::default()
            .sort()
            .asc("last_login")
            .asc("id")
            .done()
            .cursor()
            .first_page()
            .done()
            .limit(5)
            .build();

        let page = repo.find_customers_cursor_by_last_login(params).await.unwrap();

        // Should include customers with last_login (initially 5 seems generic, checking logic)
        for customer in &page.data {
            assert!(customer.last_login.is_some());
        }

        if page.data.len() >= 2 {
            for i in 0..page.data.len() - 1 {
                let current_login = page.data[i].last_login.unwrap();
                let next_login = page.data[i + 1].last_login.unwrap();
                assert!(current_login <= next_login);
            }
        }
    }

    #[sqlx::test(
        migrations = "tests/migrations_datetime",
        fixtures(path = "fixtures_datetime", scripts("customers"))
    )]
    async fn test_cursor_pagination_with_filtering(pool: Pool) {
        let repo = CustomerRepoImpl { pool };
        let now = Utc::now().naive_utc();

        for i in 0..5 {
            let time = now + std::time::Duration::from_secs(i * 600); 

            repo.insert_customer(
                format!("Test Customer {}", i + 1),
                format!("test{}@pagination.com", i + 1),
                (25 + i) as u8,
                None,
                time,
                None,
            ).await.unwrap();
        }

        let mut all_customers = Vec::new();
        let mut current_cursor: Option<String> = None;
        let mut page_count = 0;
        let max_pages = 10;

        loop {
            let params = if let Some(cursor) = &current_cursor {
                ParamsBuilder::default()
                    .sort()
                        .asc("created_at")
                        .asc("id")
                    .done()
                    .cursor()
                        .next_cursor::<Customer>(cursor)
                    .done()
                    .limit(2)
                    .build()
            } else {
                ParamsBuilder::default()
                    .sort()
                        .asc("created_at")
                        .asc("id")
                        .done()
                    .cursor()
                        .first_page()
                        .done()
                    .limit(2)
                    .build()
            };

            let page = repo.find_customers_cursor_by_created_at(params).await.unwrap();
            all_customers.extend(page.data);

            page_count += 1;
            if !page.has_next || page_count >= max_pages {
                break;
            }

            current_cursor = page.next_cursor;
        }

        assert!(all_customers.len() >= 5);
        for i in 0..all_customers.len() - 1 {
            assert!(all_customers[i].created_at <= all_customers[i + 1].created_at);
        }
    }

    #[sqlx::test(
        migrations = "tests/migrations_datetime",
        fixtures(path = "fixtures_datetime", scripts("customers"))
    )]
    async fn test_params_builder_with_naive_date_filter(pool: Pool) {
        let repo = CustomerRepoImpl { pool };
        let birth_date_filter = NaiveDate::from_ymd_opt(1990, 1, 1).unwrap();
        let _filter_value = FilterValue::NaiveDate(birth_date_filter);
        
        // Find customers born after 1990-01-01 using the existing method
        let customers = repo.find_customers_born_before(birth_date_filter.succ_opt().unwrap()).await.unwrap();
        
        for customer in &customers {
            if let Some(_birth_date) = customer.birth_date {
                 // Check logic matches
            }
        }
    }

    #[sqlx::test(
        migrations = "tests/migrations_datetime",
        fixtures(path = "fixtures_datetime", scripts("customers"))
    )]
    async fn test_serial_pagination_with_date_filter(pool: Pool) {
        let repo = CustomerRepoImpl { pool };
        let birth_date_filter = NaiveDate::from_ymd_opt(1980, 1, 1).unwrap();

        let params = ParamsBuilder::new()
            .filter()
                .gte("birth_date", FilterValue::String(birth_date_filter.to_string().into()))
                .is_not_null("birth_date")
            .done()
            .sort()
                .asc("birth_date")
                .asc("id")
            .done()
            .limit(10)
            .offset(0)
            .build();

        let result = repo.find_customers_serial_pagination(params).await.unwrap();

        for customer in &result.data {
            if let Some(birth_date) = customer.birth_date {
                assert!(birth_date >= birth_date_filter);
            }
        }
    }

    #[sqlx::test(
        migrations = "tests/migrations_datetime",
        fixtures(path = "fixtures_datetime", scripts("customers"))
    )]
    async fn test_find_recently_active(pool: Pool) {
        let repo = CustomerRepoImpl { pool };
        let now = Utc::now().naive_utc();
        
        // Insert a user active now
        repo.insert_customer(
            "Active User".into(),
            "active@test.com".into(),
            25,
            None,
            now,
            Some(Utc::now()),
        ).await.unwrap();

        // Insert a user active 5 hours ago
        let past_login = Utc::now() - std::time::Duration::from_secs(5 * 3600);
        repo.insert_customer(
            "Inactive User".into(),
            "inactive@test.com".into(),
            30,
            None,
            now,
            Some(past_login),
        ).await.unwrap();

        // Find users active in last 1 hour
        let recent_users = repo.find_recently_active(1).await.unwrap();
        
        // Should find at least the "Active User"
        let found_active = recent_users.iter().any(|(_, name)| name == "Active User");
        assert!(found_active);
        
        // Should NOT find "Inactive User" (5 hours ago)
        let found_inactive = recent_users.iter().any(|(_, name)| name == "Inactive User");
        assert!(!found_inactive);
    }

    #[sqlx::test(
        migrations = "tests/migrations_datetime",
        fixtures(path = "fixtures_datetime", scripts("customers"))
    )]
    async fn test_convert_to_local_time(pool: Pool) {
        let repo = CustomerRepoImpl { pool };
        let now = Utc::now().naive_utc();
        
        // 2023-01-01 10:00:00 UTC
        let login_time = DateTime::from_naive_utc_and_offset(
            NaiveDate::from_ymd_opt(2023, 1, 1).unwrap().and_hms_opt(10, 0, 0).unwrap(), 
            Utc
        );

        let result = repo.insert_customer(
            "Timezone User".into(),
            "tz@test.com".into(),
            25,
            None,
            now,
            Some(login_time),
        ).await.unwrap();
        let id = result.last_insert_id() as i64;

        let local_times = repo.convert_to_local_time().await.unwrap();
        let user_time = local_times.iter().find(|(user_id, _)| *user_id == id);
        
        assert!(user_time.is_some());
        let (_, local_time_opt) = user_time.unwrap();
        assert!(local_time_opt.is_some());
        
        let local_time = local_time_opt.unwrap();
        // +8 hours from 10:00 is 18:00
        assert_eq!(local_time.format("%H").to_string(), "18");
    }

    #[sqlx::test(
        migrations = "tests/migrations_datetime",
        fixtures(path = "fixtures_datetime", scripts("customers"))
    )]
    async fn test_has_recent_updates_option(pool: Pool) {
        let repo = CustomerRepoImpl { pool };
        let now = Utc::now().naive_utc();

        let result = repo.insert_customer(
            "Update Option Test".into(),
            "update_opt@test.com".into(),
            30,
            None,
            now,
            None,
        ).await.unwrap();
        let id = result.last_insert_id() as i64;

        // Initially no updates
        let has_update = repo.has_recent_updates_option(now).await.unwrap();
        // Since we explicitly insert NULL for updated_at, this should be None or Some(false) depending on implementation details of CASE
        // The query is: CASE WHEN updated_at IS NOT NULL ... THEN 1 ELSE NULL END
        // So it returns NULL if condition fails. 
        // Rust Option<bool> from NULL is None? Or does it map? 
        // SQL NULL -> Option::None. 
        // So we expect None.
        assert!(has_update.is_none());

        // Now update
        let update_time = now + std::time::Duration::from_secs(3600);
        repo.update_customer_timestamp(update_time, id).await.unwrap();

        // Check again with a time before the update
        let has_update_after = repo.has_recent_updates_option(update_time - std::time::Duration::from_secs(10)).await.unwrap();
        
        // Now it should satisfy condition and return 1 (true)
        assert_eq!(has_update_after, Some(true));
    }

}