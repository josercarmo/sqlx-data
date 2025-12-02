// #![cfg(all(feature = "time", not(feature = "chrono")))]

// use sqlx_data::{Pool, QueryResult, Result, dml, repo};

// // Import time types from sqlx when time feature is enabled
// use sqlx::types::time::{OffsetDateTime, PrimitiveDateTime, Date};

// #[derive(Debug, sqlx::FromRow)]
// pub struct Customer {
//     pub id: i64,
//     pub name: String,
//     pub email: String,
//     pub age: i32,
//     pub birth_date: Option<Date>,
//     pub created_at: PrimitiveDateTime,
//     pub updated_at: Option<PrimitiveDateTime>,
//     pub last_login: Option<OffsetDateTime>,
// }

// #[repo]
// #[alias(
//     all_columns = "id, name, email, age as 'age: i32', birth_date as 'birth_date: Date', created_at as 'created_at: PrimitiveDateTime', updated_at as 'updated_at: PrimitiveDateTime', last_login as 'last_login: OffsetDateTime'"
// )]
// trait CustomerRepo {
//     #[dml(
//         "INSERT INTO customers (name, email, age, birth_date, created_at, last_login)
//          VALUES ($1, $2, $3, $4, $5, $6)
//          RETURNING id"
//     )]
//     async fn insert_customer(
//         &self,
//         name: String,
//         email: String,
//         age: i32,
//         birth_date: Option<Date>,
//         created_at: PrimitiveDateTime,
//         last_login: Option<OffsetDateTime>,
//     ) -> Result<i64>;

//     #[dml("SELECT {{all_columns}} FROM customers WHERE created_at >= $1")]
//     async fn find_customers_created_after(&self, from: PrimitiveDateTime) -> Result<Vec<Customer>>;

//     #[dml("SELECT MAX(created_at) FROM customers")]
//     async fn max_created_at(&self) -> Result<Option<PrimitiveDateTime>>;

//     #[dml("SELECT MAX(created_at) as 'created_at: PrimitiveDateTime' FROM customers")]
//     async fn max_created_at_casting(&self) -> Result<Option<PrimitiveDateTime>>;

//     #[dml("SELECT id, name, created_at as 'created_at!: PrimitiveDateTime' FROM customers WHERE age > $1")]
//     async fn find_customers_by_min_age(
//         &self,
//         min_age: i32,
//     ) -> Result<Vec<(i64, String, PrimitiveDateTime)>>;

//     #[dml(
//         "UPDATE customers
//          SET updated_at = $2
//          WHERE id = $1"
//     )]
//     async fn update_customer_timestamp(
//         &self,
//         id: i64,
//         updated_at: PrimitiveDateTime,
//     ) -> Result<QueryResult>;

//     #[dml(
//         "SELECT {{all_columns}}
//          FROM customers
//          WHERE birth_date < $1"
//     )]
//     async fn find_customers_born_before(&self, date: Date) -> Result<Vec<Customer>>;

//     #[dml(
//         "SELECT
//             id,
//             name,
//             created_at as 'created_at!: PrimitiveDateTime',
//             datetime(created_at, '+1 day') as 'next_day!: PrimitiveDateTime'
//          FROM customers"
//     )]
//     async fn customers_with_next_day(
//         &self,
//     ) -> Result<Vec<(i64, String, PrimitiveDateTime, PrimitiveDateTime)>>;

//     #[dml(
//         "SELECT COUNT(*) > 0 as 'has_updates: bool'
//          FROM customers
//          WHERE updated_at IS NOT NULL
//            AND updated_at >= $1"
//     )]
//     async fn has_recent_updates(&self, since: PrimitiveDateTime) -> Result<bool>;

//     // SQLite-specific example
//     #[dml(
//         "SELECT id, name
//          FROM customers
//          WHERE datetime(created_at) < datetime('now', '-7 days')"
//     )]
//     async fn find_inactive_customers_sqlite(&self) -> Result<Vec<(i64, String)>>;

//     // Intentionally broken to test compile-time error
//     #[dml("SELECT created_at FROM customers")]
//     async fn broken_created_at(&self) -> Result<i64>;
// }

// pub struct CustomerTimeRepoImpl {
//     pool: Pool,
// }

// impl CustomerRepo for CustomerTimeRepoImpl {
//     fn get_pool(&self) -> &Pool {
//         &self.pool
//     }
// }

// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[sqlx::test(
//         migrations = "tests/migrations_datetime",
//         fixtures(path = "fixtures", scripts("customers"))
//     )]
//     async fn test_datetime_roundtrip(pool: Pool) -> Result<()> {
//         let repo = CustomerTimeRepoImpl { pool };
//         let now = OffsetDateTime::now_utc().to_offset(time::UtcOffset::UTC);
//         let primitive_now = PrimitiveDateTime::new(now.date(), now.time());
//         let scheduled = OffsetDateTime::now_utc();

//         let inserted_id = repo
//             .insert_event(
//                 "Alice Customer Test".into(),
//                 "Test event for Alice".into(),
//                 1,
//                 Some(Date::from_calendar_date(2023, time::Month::May, 20).unwrap()),
//                 primitive_now,
//                 Some(scheduled),
//             )
//             .await?;

//         assert!(inserted_id > 0);

//         let customers = repo
//             .find_customers_created_after(primitive_now - std::time::Duration::from_secs(1))
//             .await?;

//         assert!(!customers.is_empty());
//         let event = &customers[0];
//         assert_eq!(event.created_at.date(), primitive_now.date());
//         assert_eq!(
//             event.scheduled_at.unwrap().unix_timestamp(),
//             scheduled.unix_timestamp()
//         );

//         Ok(())
//     }

//     #[sqlx::test(
//         migrations = "tests/migrations_datetime",
//         fixtures(path = "fixtures", scripts("customers"))
//     )]
//     async fn test_find_customers_created_after(pool: Pool) -> Result<()> {
//         let repo = CustomerTimeRepoImpl { pool };
//         let cutoff = OffsetDateTime::now_utc().to_offset(time::UtcOffset::UTC);
//         let primitive_cutoff = PrimitiveDateTime::new(cutoff.date(), cutoff.time())
//             - std::time::Duration::from_secs(3600);

//         let customers = repo.find_customers_created_after(primitive_cutoff).await?;

//         for event in customers {
//             assert!(event.created_at >= primitive_cutoff);
//         }

//         Ok(())
//     }

//     #[sqlx::test(
//         migrations = "tests/migrations_datetime",
//         fixtures(path = "fixtures", scripts("customers"))
//     )]
//     async fn test_update_timestamp(pool: Pool) -> Result<()> {
//         let repo = CustomerTimeRepoImpl { pool };
//         let now = OffsetDateTime::now_utc().to_offset(time::UtcOffset::UTC);
//         let primitive_now = PrimitiveDateTime::new(now.date(), now.time());

//         let id = repo
//             .insert_event(
//                 "Bob Customer Test".into(),
//                 "Test event for Bob".into(),
//                 2,
//                 None,
//                 primitive_now,
//                 None,
//             )
//             .await?;

//         let new_updated_at = primitive_now + std::time::Duration::from_secs(3 * 3600);
//         repo.update_customer_timestamp(id, new_updated_at).await?;

//         let has_update = repo
//             .has_recent_updates(new_updated_at - std::time::Duration::from_secs(1))
//             .await?;

//         assert!(has_update);

//         Ok(())
//     }

//     #[sqlx::test(
//         migrations = "tests/migrations_datetime",
//         fixtures(path = "fixtures", scripts("customers"))
//     )]
//     async fn test_customers_before_date_filter(pool: Pool) -> Result<()> {
//         let repo = CustomerTimeRepoImpl { pool };
//         let event_date = Date::from_calendar_date(2023, time::Month::October, 15).unwrap();
//         let now = OffsetDateTime::now_utc().to_offset(time::UtcOffset::UTC);
//         let primitive_now = PrimitiveDateTime::new(now.date(), now.time());

//         repo.insert_event(
//             "Charlie Customer Test".into(),
//             "Test event for Charlie".into(),
//             3,
//             Some(event_date),
//             primitive_now,
//             None,
//         )
//         .await?;

//         let next_day = event_date.next_day().unwrap_or(event_date);
//         let older_customers = repo.find_customers_before_date(next_day).await?;

//         assert!(!older_customers.is_empty());
//         for event in older_customers {
//             if let Some(date) = event.event_date {
//                 assert!(date < next_day);
//             }
//         }

//         Ok(())
//     }

//     #[sqlx::test(
//         migrations = "tests/migrations_datetime",
//         fixtures(path = "fixtures", scripts("customers"))
//     )]
//     async fn test_max_created_at(pool: Pool) -> Result<()> {
//         let repo = CustomerTimeRepoImpl { pool };

//         // Insert an event with a future timestamp
//         let future_time = OffsetDateTime::now_utc().to_offset(time::UtcOffset::UTC);
//         let primitive_future = PrimitiveDateTime::new(future_time.date(), future_time.time())
//             + std::time::Duration::from_secs(3600);

//         repo.insert_event(
//             "Future Customer".into(),
//             "Customer in the future".into(),
//             1,
//             None,
//             primitive_future,
//             None,
//         )
//         .await?;

//         let max_created = repo.max_created_at().await?;

//         assert!(max_created.is_some());
//         let max_time = max_created.unwrap();

//         // The max created_at should be close to our future_time
//         let diff = (max_time.assume_utc().unix_timestamp()
//             - primitive_future.assume_utc().unix_timestamp())
//         .abs();
//         assert!(diff <= 1); // Allow 1 second difference

//         Ok(())
//     }

//     #[sqlx::test(
//         migrations = "tests/migrations_datetime",
//         fixtures(path = "fixtures", scripts("customers"))
//     )]
//     async fn test_find_customers_by_min_priority(pool: Pool) -> Result<()> {
//         let repo = CustomerTimeRepoImpl { pool };

//         // Find customers with priority > 2
//         let customers = repo.find_customers_by_min_priority(2).await?;

//         // Should return customers with priority > 2
//         for (id, name, created_at) in customers {
//             assert!(id > 0);
//             assert!(!name.is_empty());
//             assert!(created_at.assume_utc().unix_timestamp() > 0);
//         }

//         Ok(())
//     }

//     #[sqlx::test(
//         migrations = "tests/migrations_datetime",
//         fixtures(path = "fixtures", scripts("customers"))
//     )]
//     async fn test_max_created_at_casting(pool: Pool) -> Result<()> {
//         let repo = CustomerTimeRepoImpl { pool };

//         let result = repo.max_created_at_casting().await?;
//         assert!(result.is_some());

//         Ok(())
//     }

//     #[sqlx::test(
//         migrations = "tests/migrations_datetime",
//         fixtures(path = "fixtures", scripts("customers"))
//     )]
//     async fn test_customers_with_next_day(pool: Pool) -> Result<()> {
//         let repo = CustomerTimeRepoImpl { pool };
//         let now = OffsetDateTime::now_utc().to_offset(time::UtcOffset::UTC);
//         let primitive_now = PrimitiveDateTime::new(now.date(), now.time());

//         // Insert a test event
//         let id = repo
//             .insert_event(
//                 "Test Customer".into(),
//                 "Test description".into(),
//                 1,
//                 None,
//                 primitive_now,
//                 None,
//             )
//             .await?;

//         // Get customers with next day calculation
//         let customers = repo.customers_with_next_day().await?;

//         // Find our test event
//         let test_event = customers
//             .iter()
//             .find(|(event_id, _, _, _)| *event_id == id)
//             .expect("Test event should be found");

//         let (_, name, created_at, next_day) = test_event;

//         // Verify the data
//         assert_eq!(name, "Test Customer");
//         assert_eq!(created_at.date(), primitive_now.date());

//         // Verify next_day is exactly 1 day after created_at
//         let expected_next_day = *created_at + std::time::Duration::from_secs(24 * 60 * 60);
//         assert_eq!(*next_day, expected_next_day);

//         Ok(())
//     }
// }
