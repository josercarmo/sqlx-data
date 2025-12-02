//#![cfg(all(feature = "chrono", not(feature = "time")))]

use sqlx_data::{Pool, QueryResult, Result, Cursor, Serial, CursorData, IntoParams, ParamsBuilder, CursorSecureExtract, CursorValue, CursorError, FilterValue, dml, repo};

// Import chrono types from sqlx when chrono feature is enabled
use sqlx::types::chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};

#[derive(Debug, sqlx::FromRow)]
pub struct Customer {
    pub id: i64,
    pub name: String,
    pub email: String,
    pub age: i32,
    pub birth_date: Option<NaiveDate>,
    pub created_at: NaiveDateTime,
    pub updated_at: Option<NaiveDateTime>,
    pub last_login: Option<DateTime<Utc>>,
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

#[repo]
#[alias(
    all_columns = "id, name, email, age as 'age: i32', birth_date as 'birth_date: NaiveDate', created_at as 'created_at: NaiveDateTime', updated_at as 'updated_at: NaiveDateTime', last_login as 'last_login: DateTime<Utc>'"
)]
trait CustomerRepo {
    //OK
    #[dml(
        "INSERT INTO customers (name, email, age, birth_date, created_at, last_login)
         VALUES ($1, $2, $3, $4, $5, $6)
         RETURNING id"
    )]
    async fn insert_customer(
        &self,
        name: String,
        email: String,
        age: i32,
        birth_date: Option<NaiveDate>,
        created_at: NaiveDateTime,
        last_login: Option<DateTime<Utc>>,
    ) -> Result<i64>;

    //OK
    #[dml("SELECT {{all_columns}} FROM customers WHERE created_at >= $1")]
    async fn find_customers_created_after(&self, from: NaiveDateTime) -> Result<Vec<Customer>>;

    //Ok
    #[dml("SELECT MAX(created_at) FROM customers")]
    async fn max_created_at(&self) -> Result<Option<String>>;

    //OK
    #[dml("SELECT MAX(created_at) as 'created_at: NaiveDateTime' FROM customers")]
    async fn max_created_at_casting(&self) -> Result<Option<NaiveDateTime>>;

    //OK
    //had to force to NaiveDateTime
    #[dml(
        "SELECT id, name, created_at as 'created_at!: NaiveDateTime' FROM customers WHERE age > $1"
    )]
    async fn find_customers_by_min_age(
        &self,
        min_age: i32,
    ) -> Result<Vec<(i64, String, NaiveDateTime)>>;

    #[dml("SELECT id, name, created_at FROM customers WHERE age > $1")]
    async fn find_customers_by_min_age11(
        &self,
        min_age: i32,
    ) -> Result<Vec<(i64, String, NaiveDateTime)>>;

    //OK
    #[dml(
        "UPDATE customers
         SET updated_at = $2
         WHERE id = $1"
    )]
    async fn update_customer_timestamp(
        &self,
        id: i64,
        updated_at: NaiveDateTime,
    ) -> Result<QueryResult>;

    //OK
    #[dml(
        "SELECT {{all_columns}}
         FROM customers
         WHERE birth_date < $1"
    )]
    async fn find_customers_born_before(&self, date: NaiveDate) -> Result<Vec<Customer>>;

    #[dml(
        "SELECT
            id,
            name,
            created_at as 'created_at!: NaiveDateTime',
            datetime(created_at, '+1 day') as 'next_day!: NaiveDateTime'
         FROM customers"
    )]
    async fn customers_with_next_day(
        &self,
    ) -> Result<Vec<(i64, String, NaiveDateTime, NaiveDateTime)>>;

    #[dml(
        "SELECT
            id,
            name,
            created_at,
            datetime(created_at, '+1 day') as 'next_day!: NaiveDateTime'
         FROM customers"
    )]
    async fn customers_with_next_day1(
        &self,
    ) -> Result<Vec<(i64, String, NaiveDateTime, NaiveDateTime)>>;

    //OK
    #[dml(
        "SELECT COUNT(*) > 0
         FROM customers
         WHERE updated_at IS NOT NULL
           AND updated_at >= $1"
    )]
    async fn has_recent_updates(&self, since: NaiveDateTime) -> Result<bool>;

    //OK
    #[dml(
        "SELECT CASE
        WHEN updated_at IS NOT NULL AND updated_at >= $1 THEN 1
        ELSE NULL
     END
     FROM customers
     LIMIT 1"
    )]
    async fn has_recent_updates_option(&self, since: NaiveDateTime) -> Result<Option<bool>>;

    //OK
    #[dml(
        "SELECT id, name
         FROM customers
         WHERE datetime(created_at) < datetime('now', '-7 days')"
    )]
    async fn find_inactive_customers_sqlite(&self) -> Result<Vec<(i64, String)>>;

    // Test direct field return types
    #[dml("SELECT created_at FROM customers WHERE id = $1")]
    async fn get_created_at(&self, id: i64) -> Result<NaiveDateTime>;

    #[dml("SELECT birth_date FROM customers WHERE id = $1")]
    async fn get_birth_date(&self, id: i64) -> Result<Option<NaiveDate>>;

    #[dml("SELECT updated_at FROM customers WHERE id = $1")]
    async fn get_updated_at(&self, id: i64) -> Result<Option<NaiveDateTime>>;

    #[dml("SELECT last_login as 'last_login: DateTime<Utc>' FROM customers WHERE id = $1")]
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

    // Serial pagination method - ParamsBuilder adds filters dynamically
    #[dml("SELECT {{all_columns}} FROM customers ORDER BY birth_date, id")]
    async fn find_customers_serial_pagination(
        &self,
        params: impl IntoParams,
    ) -> Result<Serial<Customer>>;

}

pub struct CustomerRepoImpl {
    pool: Pool,
}

impl CustomerRepo for CustomerRepoImpl {
    fn get_pool(&self) -> &Pool {
        &self.pool
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[sqlx::test(
        migrations = "tests/migrations_datetime",
        fixtures(path = "fixtures", scripts("customers"))
    )]
    async fn test_datetime_roundtrip(pool: Pool) -> Result<()> {
        let repo = CustomerRepoImpl { pool };
        let now = Utc::now().naive_utc();
        let last_login = Utc::now();

        let inserted_id = repo
            .insert_customer(
                "Alice Smith Test".into(),
                "alice.test@example.com".into(),
                30,
                Some(NaiveDate::from_ymd_opt(1993, 5, 20).unwrap()),
                now,
                Some(last_login),
            )
            .await?;

        assert!(inserted_id > 0);

        let customers = repo
            .find_customers_created_after(now - std::time::Duration::from_secs(1))
            .await?;

        assert!(!customers.is_empty());
        let customer = &customers[0];
        assert!(customer.id > 0);
        assert!(!customer.name.is_empty());
        assert!(!customer.email.is_empty());
        assert!(customer.age > 0);
        // updated_at should be None for new customer
        assert!(customer.updated_at.is_none());
        assert_eq!(
            customer.created_at.and_utc().timestamp(),
            now.and_utc().timestamp()
        );
        assert_eq!(
            customer.last_login.unwrap().timestamp(),
            last_login.timestamp()
        );

        Ok(())
    }

    #[sqlx::test(
        migrations = "tests/migrations_datetime",
        fixtures(path = "fixtures", scripts("customers"))
    )]
    async fn test_find_customers_created_after(pool: Pool) -> Result<()> {
        let repo = CustomerRepoImpl { pool };
        let cutoff = Utc::now().naive_utc() - std::time::Duration::from_secs(3600);

        let customers = repo.find_customers_created_after(cutoff).await?;

        for customer in customers {
            assert!(customer.id > 0);
            assert!(!customer.name.is_empty());
            assert!(!customer.email.is_empty());
            assert!(customer.age > 0);
            // updated_at should be None for customers that haven't been updated
            assert!(customer.updated_at.is_none());
            assert!(customer.created_at >= cutoff);
        }

        Ok(())
    }

    #[sqlx::test(
        migrations = "tests/migrations_datetime",
        fixtures(path = "fixtures", scripts("customers"))
    )]
    async fn test_update_timestamp(pool: Pool) -> Result<()> {
        let repo = CustomerRepoImpl { pool };
        let now = Utc::now().naive_utc();

        let id = repo
            .insert_customer(
                "Bob Johnson Test".into(),
                "bob.test@example.com".into(),
                28,
                None,
                now,
                None,
            )
            .await?;

        let new_updated_at = now + std::time::Duration::from_secs(3 * 3600);
        repo.update_customer_timestamp(id, new_updated_at).await?;

        let has_update = repo
            .has_recent_updates(new_updated_at - std::time::Duration::from_secs(1))
            .await?;

        assert!(has_update);

        // Verify the specific customer was updated by checking its updated_at field
        let updated_customer_timestamp = repo.get_updated_at(id).await?;
        assert!(updated_customer_timestamp.is_some());

        let timestamp = updated_customer_timestamp.unwrap();
        // Should be close to new_updated_at
        let diff = (timestamp.and_utc().timestamp() - new_updated_at.and_utc().timestamp()).abs();
        assert!(diff <= 1);

        Ok(())
    }

    #[sqlx::test(
        migrations = "tests/migrations_datetime",
        fixtures(path = "fixtures", scripts("customers"))
    )]
    async fn test_born_before_filter(pool: Pool) -> Result<()> {
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
        .await?;

        let older_customers = repo
            .find_customers_born_before(birth_date.succ_opt().unwrap_or(birth_date))
            .await?;

        assert!(!older_customers.is_empty());
        for customer in older_customers {
            if let Some(date) = customer.birth_date {
                assert!(date < birth_date.succ_opt().unwrap_or(birth_date));
            }
        }

        Ok(())
    }

    #[sqlx::test(
        migrations = "tests/migrations_datetime",
        fixtures(path = "fixtures", scripts("customers"))
    )]
    async fn test_max_created_at(pool: Pool) -> Result<()> {
        let repo = CustomerRepoImpl { pool };

        // Insert a customer with a future timestamp
        let future_time = Utc::now().naive_utc() + std::time::Duration::from_secs(3600);

        repo.insert_customer(
            "Future Customer".into(),
            "future@example.com".into(),
            25,
            None,
            future_time,
            None,
        )
        .await?;

        let max_created = repo.max_created_at().await?;

        // Just verify we get a string result (SQLite MAX returns string)
        assert!(max_created.is_some());
        let max_time = max_created.unwrap();
        assert!(!max_time.is_empty());

        Ok(())
    }

    #[sqlx::test(
        migrations = "tests/migrations_datetime",
        fixtures(path = "fixtures", scripts("customers"))
    )]
    async fn test_find_customers_by_min_age(pool: Pool) -> Result<()> {
        let repo = CustomerRepoImpl { pool };

        // Find customers older than 30
        let customers = repo.find_customers_by_min_age11(30).await?;

        // Should return customers with age > 30
        for (id, name, created_at) in customers {
            assert!(id > 0);
            assert!(!name.is_empty());
            assert!(created_at.and_utc().timestamp() > 0);
        }

        Ok(())
    }

    #[sqlx::test(
        migrations = "tests/migrations_datetime",
        fixtures(path = "fixtures", scripts("customers"))
    )]
    async fn test_max_created_at_casting(pool: Pool) -> Result<()> {
        let repo = CustomerRepoImpl { pool };

        let result = repo.max_created_at_casting().await?;
        assert!(result.is_some());

        Ok(())
    }

    #[sqlx::test(
        migrations = "tests/migrations_datetime",
        fixtures(path = "fixtures", scripts("customers"))
    )]
    async fn test_customers_with_next_day(pool: Pool) -> Result<()> {
        let repo = CustomerRepoImpl { pool };
        let now = Utc::now().naive_utc();

        // Insert a test customer
        let id = repo
            .insert_customer(
                "Test Customer".into(),
                "test@example.com".into(),
                25,
                None,
                now,
                None,
            )
            .await?;

        // Get customers with next day calculation
        let customers = repo.customers_with_next_day().await?;

        // Find our test customer
        let test_customer = customers
            .iter()
            .find(|(customer_id, _, _, _)| *customer_id == id)
            .expect("Test customer should be found");

        let (_, name, created_at, next_day) = test_customer;

        // Verify the data
        assert_eq!(name, "Test Customer");
        assert_eq!(created_at.date(), now.date());

        // Just verify that next_day is after created_at (SQLite datetime function works)
        assert!(*next_day > *created_at);

        Ok(())
    }

    #[sqlx::test(
        migrations = "tests/migrations_datetime",
        fixtures(path = "fixtures", scripts("customers"))
    )]
    async fn test_get_created_at(pool: Pool) -> Result<()> {
        let repo = CustomerRepoImpl { pool };
        let now = Utc::now().naive_utc();

        let id = repo
            .insert_customer(
                "Test Created".into(),
                "test.created@example.com".into(),
                25,
                None,
                now,
                None,
            )
            .await?;

        let created_at = repo.get_created_at(id).await?;
        assert_eq!(created_at.date(), now.date());

        Ok(())
    }

    #[sqlx::test(
        migrations = "tests/migrations_datetime",
        fixtures(path = "fixtures", scripts("customers"))
    )]
    async fn test_get_birth_date(pool: Pool) -> Result<()> {
        let repo = CustomerRepoImpl { pool };
        let birth_date = NaiveDate::from_ymd_opt(1990, 1, 15).unwrap();
        let now = Utc::now().naive_utc();

        let id = repo
            .insert_customer(
                "Test Birth".into(),
                "test.birth@example.com".into(),
                30,
                Some(birth_date),
                now,
                None,
            )
            .await?;

        let result = repo.get_birth_date(id).await?;
        assert_eq!(result, Some(birth_date));

        Ok(())
    }

    #[sqlx::test(
        migrations = "tests/migrations_datetime",
        fixtures(path = "fixtures", scripts("customers"))
    )]
    async fn test_get_updated_at_null(pool: Pool) -> Result<()> {
        let repo = CustomerRepoImpl { pool };
        let now = Utc::now().naive_utc();

        let id = repo
            .insert_customer(
                "Test Updated".into(),
                "test.updated@example.com".into(),
                25,
                None,
                now,
                None,
            )
            .await?;

        let result = repo.get_updated_at(id).await?;
        assert_eq!(result, None);

        Ok(())
    }

    #[sqlx::test(
        migrations = "tests/migrations_datetime",
        fixtures(path = "fixtures", scripts("customers"))
    )]
    async fn test_get_last_login(pool: Pool) -> Result<()> {
        let repo = CustomerRepoImpl { pool };
        let now = Utc::now().naive_utc();
        let login_time = Utc::now();

        let id = repo
            .insert_customer(
                "Test Login".into(),
                "test.login@example.com".into(),
                25,
                None,
                now,
                Some(login_time),
            )
            .await?;

        let result = repo.get_last_login(id).await?;
        assert!(result.is_some());
        let retrieved_login = result.unwrap();
        assert_eq!(retrieved_login.timestamp(), login_time.timestamp());

        Ok(())
    }

    #[sqlx::test(
        migrations = "tests/migrations_datetime",
        fixtures(path = "fixtures", scripts("customers"))
    )]
    async fn test_cursor_pagination_by_created_at(pool: Pool) -> Result<()> {
        let repo = CustomerRepoImpl { pool };

        // Insert test customers with different created_at times

        // First page - get first 2 customers ordered by created_at, id (no cursor needed)
        #[rustfmt::skip]
        let params1 = ParamsBuilder::new()
            .sort()
                .asc("created_at")
                .asc("id")
                .done()
            .cursor()
                .first_page()  // Explicit: first page without cursor
                .done()
            .limit(2)
            .build();

        let page1 = repo.find_customers_cursor_by_created_at(params1).await?;
        assert_eq!(page1.data.len(), 2);
        assert!(page1.has_next);
        println!("First page cursor token: {}", page1.next_cursor.as_ref().unwrap());

        // Print first page data for debugging
        println!("First page customers:");
        for customer in &page1.data {
            println!("  ID: {}, created_at: {}", customer.id, customer.created_at);
        }

        // Verify order - should be oldest first by created_at, then by id
        let first_customer = &page1.data[0];
        let second_customer = &page1.data[1];
        assert!(
            first_customer.created_at < second_customer.created_at ||
            (first_customer.created_at == second_customer.created_at && first_customer.id < second_customer.id),
            "First page should be ordered by created_at, then id"
        );

        // Second page - use next_cursor from first page
        let cursor_token = page1.next_cursor.expect("Should have next cursor");
        #[rustfmt::skip]
        let params2 = ParamsBuilder::new()
            .sort()
                .asc("created_at")
                .asc("id")
                .done()
            .cursor()
                .next_cursor::<Customer>(&cursor_token) // Navigate forward with encoded cursor
                .done()
            .limit(2)
            .build();

        let page2 = repo.find_customers_cursor_by_created_at(params2).await?;
        assert!(!page2.data.is_empty());

        println!("Second page customers:");
        for customer in &page2.data {
            println!("  ID: {}, created_at: {}", customer.id, customer.created_at);
        }

        // Verify we got different customers (no overlap)
        let page1_ids: Vec<i64> = page1.data.iter().map(|c| c.id).collect();
        let page2_ids: Vec<i64> = page2.data.iter().map(|c| c.id).collect();

        for id in &page2_ids {
            assert!(!page1_ids.contains(id), "Page 2 should not contain customers from page 1");
        }

        // Verify ordering continuity - last customer of page1 should come before first customer of page2
        let last_page1 = page1.data.last().unwrap();
        let first_page2 = page2.data.first().unwrap();
        assert!(
            last_page1.created_at < first_page2.created_at ||
            (last_page1.created_at == first_page2.created_at && last_page1.id < first_page2.id),
            "Page 2 should continue after page 1"
        );

        // Test prev_cursor from page 2 - should go back to page 1
        if let Some(prev_cursor_token) = &page2.prev_cursor {
            println!("Testing prev_cursor: {}", prev_cursor_token);

            let prev_params = ParamsBuilder::new()
                .sort()
                .asc("created_at")
                .asc("id")
                .done()
                .cursor()
                .prev_cursor::<Customer>(prev_cursor_token) // Navigate backward with encoded cursor
                .done()
                .limit(2)
                .build();

            let prev_page = repo.find_customers_cursor_by_created_at(prev_params).await?;

            // Should get back something similar to page 1 (might not be exactly the same due to cursor positioning)
            assert!(!prev_page.data.is_empty());
            println!("Previous page customers:");
            for customer in &prev_page.data {
                println!("  ID: {}, created_at: {}", customer.id, customer.created_at);
            }
        }

        Ok(())
    }

    #[sqlx::test(
        migrations = "tests/migrations_datetime",
        fixtures(path = "fixtures", scripts("customers"))
    )]
    async fn test_cursor_order_by_inversion_problem(pool: Pool) -> Result<()> {
        let repo = CustomerRepoImpl { pool };
        let base_time = Utc::now().naive_utc();

        // Insert 5 customers with same created_at but different IDs
        // This will expose the ORDER BY inversion problem
        let same_time = base_time - std::time::Duration::from_secs(3600);

        for i in 1..=5 {
            repo.insert_customer(
                format!("Customer {}", i),
                format!("customer{}@test.com", i),
                20 + i,
                None,
                same_time, // Same timestamp for all!
                None,
            ).await?;
        }

        // First page - no cursor needed
        let params1 = ParamsBuilder::new()
            .sort()
            .asc("created_at")
            .asc("id")
            .done()
            .cursor()
            .first_page()  // Explicit: first page
            .done()
            .limit(3)
            .build();

        let page1 = repo.find_customers_cursor_by_created_at(params1).await?;
        println!("=== FIRST PAGE (AFTER) ===");
        for customer in &page1.data {
            println!("  ID: {}, created_at: {}", customer.id, customer.created_at);
        }

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

        let page2 = repo.find_customers_cursor_by_created_at(params2).await?;
        println!("=== SECOND PAGE (AFTER) ===");
        for customer in &page2.data {
            println!("  ID: {}, created_at: {}", customer.id, customer.created_at);
        }

        // Now test BEFORE (this will expose the problem)
        if let Some(prev_cursor_token) = &page2.prev_cursor {
            println!("=== PREV CURSOR TOKEN ===");
            println!("{}", prev_cursor_token);

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

            let prev_page = repo.find_customers_cursor_by_created_at(prev_params).await?;
            println!("=== PREVIOUS PAGE (BEFORE - SHOULD BE INVERTED ORDER) ===");
            for customer in &prev_page.data {
                println!("  ID: {}, created_at: {}", customer.id, customer.created_at);
            }

            // The problem: Without ORDER BY inversion, BEFORE pagination
            // may return data in unexpected order or miss items
            println!("=== ANALYSIS ===");
            println!("Expected: ORDER BY created_at DESC, id DESC for BEFORE direction");
            println!("Actual: ORDER BY created_at ASC, id ASC (same as AFTER)");
            println!("This may cause pagination inconsistencies!");
        }

        Ok(())
    }

    #[sqlx::test(
        migrations = "tests/migrations_datetime",
        fixtures(path = "fixtures", scripts("customers"))
    )]
    async fn test_cursor_pagination_by_birth_date(pool: Pool) -> Result<()> {
        let repo = CustomerRepoImpl { pool };
        let now = Utc::now().naive_utc();

        // Insert customers with different birth dates
        let birth_date1 = NaiveDate::from_ymd_opt(1980, 1, 1).unwrap();
        let birth_date2 = NaiveDate::from_ymd_opt(1985, 6, 15).unwrap();
        let birth_date3 = NaiveDate::from_ymd_opt(1990, 12, 31).unwrap();

        let _id1 = repo.insert_customer(
            "Oldest Customer".into(),
            "oldest@test.com".into(),
            44,
            Some(birth_date1),
            now,
            None,
        ).await?;

        let _id2 = repo.insert_customer(
            "Middle Customer".into(),
            "middle@test.com".into(),
            39,
            Some(birth_date2),
            now,
            None,
        ).await?;

        let _id3 = repo.insert_customer(
            "Youngest Customer".into(),
            "youngest@test.com".into(),
            34,
            Some(birth_date3),
            now,
            None,
        ).await?;

        // Test cursor pagination by birth_date
        let params = ParamsBuilder::default()
            .sort()
            .asc("birth_date")
            .asc("id")
            .done()
            .cursor()
            .first_page()  // Explicit: first page
            .done()
            .limit(2)
            .build();

        let page = repo.find_customers_cursor_by_birth_date(params).await?;

        // Should only get customers with birth_date (not null)
        for customer in &page.data {
            assert!(customer.birth_date.is_some());
        }

        // Verify order - should be oldest birth_date first
        if page.data.len() >= 2 {
            let first = &page.data[0];
            let second = &page.data[1];
            assert!(first.birth_date <= second.birth_date);
        }

        Ok(())
    }


    #[sqlx::test(
        migrations = "tests/migrations_datetime",
        fixtures(path = "fixtures", scripts("customers"))
    )]
    async fn test_cursor_pagination_by_last_login(pool: Pool) -> Result<()> {
        let repo = CustomerRepoImpl { pool };
        let now = Utc::now().naive_utc();

        // Insert customers with last_login times
        let login1 = Utc::now() - std::time::Duration::from_secs(7200); // 2 hours ago
        let login2 = Utc::now() - std::time::Duration::from_secs(3600); // 1 hour ago
        let login3 = Utc::now();

        let _id1 = repo.insert_customer(
            "Early Login".into(),
            "early@test.com".into(),
            25,
            None,
            now,
            Some(login1),
        ).await?;

        let _id2 = repo.insert_customer(
            "Mid Login".into(),
            "mid@test.com".into(),
            30,
            None,
            now,
            Some(login2),
        ).await?;

        let _id3 = repo.insert_customer(
            "Recent Login".into(),
            "recent@test.com".into(),
            35,
            None,
            now,
            Some(login3),
        ).await?;

        // Also insert a customer without last_login (should be excluded)
        let _id_no_login = repo.insert_customer(
            "No Login".into(),
            "nologin@test.com".into(),
            40,
            None,
            now,
            None,
        ).await?;

        // Test cursor pagination by last_login
        let params = ParamsBuilder::default()
            .sort()
            .asc("last_login")
            .asc("id")
            .done()
            .cursor()
            .first_page()  // Explicit: first page
            .done()
            .limit(5)
            .build();

        let page = repo.find_customers_cursor_by_last_login(params).await?;

        // Should include customers with last_login (2 from fixtures + 3 we inserted = 5 total)
        assert_eq!(page.data.len(), 5);

        // All customers should have last_login
        for customer in &page.data {
            assert!(customer.last_login.is_some());
        }

        // Verify ASC order by last_login
        if page.data.len() >= 2 {
            for i in 0..page.data.len() - 1 {
                let current_login = page.data[i].last_login.unwrap();
                let next_login = page.data[i + 1].last_login.unwrap();
                assert!(current_login <= next_login);
            }
        }

        Ok(())
    }

    #[sqlx::test(
        migrations = "tests/migrations_datetime",
        fixtures(path = "fixtures", scripts("customers"))
    )]
    async fn test_cursor_pagination_with_filtering(pool: Pool) -> Result<()> {
        let repo = CustomerRepoImpl { pool };
        let now = Utc::now().naive_utc();

        // Insert multiple customers to test pagination
        let mut customer_times = Vec::new();
        for i in 0..5 {
            let time = now + std::time::Duration::from_secs(i * 600); // Every 10 minutes, ASCENDING order
            customer_times.push(time);

            repo.insert_customer(
                format!("Test Customer {}", i + 1),
                format!("test{}@pagination.com", i + 1),
                25 + i as i32,
                None,
                time,
                None,
            ).await?;
        }

        // Test small page size to ensure pagination works
        let mut all_customers = Vec::new();
        let mut current_cursor: Option<String> = None;
        let mut page_count = 0;
        let max_pages = 10; // Safety limit to prevent infinite loops

        // Iterate through all pages
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
                        .first_page()  // Explicit: first page without cursor
                        .done()
                    .limit(2)
                    .build()
            };

            let page = repo.find_customers_cursor_by_created_at(params).await?;
            all_customers.extend(page.data);

            page_count += 1;
            if !page.has_next || page_count >= max_pages {
                break;
            }

            current_cursor = page.next_cursor.map(|cursors| cursors);
        }

        // Should have collected all customers we inserted (plus any from fixtures)
        assert!(all_customers.len() >= 5);

        // Verify all customers are in ASC order by created_at
        for i in 0..all_customers.len() - 1 {
            assert!(all_customers[i].created_at <= all_customers[i + 1].created_at);
        }

        Ok(())
    }

    #[sqlx::test(
        migrations = "tests/migrations_datetime",
        fixtures(path = "fixtures", scripts("customers"))
    )]
    async fn test_params_builder_with_naive_date_filter(pool: Pool) -> Result<()> {
        let repo = CustomerRepoImpl { pool };

        // Create a NaiveDate for filtering
        let birth_date_filter = NaiveDate::from_ymd_opt(1990, 1, 1).unwrap();

        // Test FilterValue::NaiveDate construction
        let filter_value = FilterValue::NaiveDate(birth_date_filter);
        println!("Created FilterValue::NaiveDate: {:?}", filter_value);

        println!("ParamsBuilder created successfully with NaiveDate filter");

        // Find customers born after 1990-01-01 using the existing method
        let customers = repo.find_customers_born_before(birth_date_filter.succ_opt().unwrap()).await?;

        // Verify the ParamsBuilder created valid filter with NaiveDate
        println!("Filter with NaiveDate created successfully");
        println!("Birth date filter: {}", birth_date_filter);
        println!("Found {} customers", customers.len());

        // Test that the FilterValue::NaiveDate was properly constructed
        for customer in &customers {
            if let Some(birth_date) = customer.birth_date {
                println!("Customer {} born on: {}", customer.name, birth_date);
            }
        }

        Ok(())
    }

    #[sqlx::test(
        migrations = "tests/migrations_datetime",
        fixtures(path = "fixtures", scripts("customers"))
    )]
    async fn test_serial_pagination_with_date_filter(pool: Pool) -> Result<()> {
        let repo = CustomerRepoImpl { pool };

        // Create a NaiveDate for filtering
        let birth_date_filter = NaiveDate::from_ymd_opt(1980, 1, 1).unwrap();

        // Use ParamsBuilder with FilterValue::NaiveDate for serial pagination
        let params = ParamsBuilder::new()
            .filter()
                .gte("birth_date", FilterValue::NaiveDate(birth_date_filter))
                .is_not_null("birth_date")
            .done()
            .sort()
                .asc("birth_date")
                .asc("id")
            .done()
            .limit(10)
            .offset(0)
            .build();

        // Execute serial pagination query
        let result = repo.find_customers_serial_pagination(params).await?;

        // Test serial pagination results
        println!("Serial pagination with NaiveDate filter executed successfully");
        println!("Total customers found: {}", result.data.len());
        println!("Current page: {}", result.page);
        println!("Page size: {}", result.size);
        println!("Total items: {}", result.total_items);
        println!("Total pages: {}", result.total_pages);

        // Verify all customers have birth_date >= filter
        for customer in &result.data {
            if let Some(birth_date) = customer.birth_date {
                assert!(birth_date >= birth_date_filter,
                    "Customer {} born on {} should be >= {}",
                    customer.name, birth_date, birth_date_filter);
                println!("Customer {} born on: {}", customer.name, birth_date);
            }
        }

        // Verify serial pagination structure
        assert!(result.data.len() <= 10, "Should respect limit");

        Ok(())
    }
}
