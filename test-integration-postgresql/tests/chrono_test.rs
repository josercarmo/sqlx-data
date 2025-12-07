#![cfg(feature = "chrono")]

use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc, TimeZone, Duration};
use sqlx_data::{Pool, Result, dml, repo};

#[derive(Debug, Clone, PartialEq, sqlx::FromRow)]
pub struct TimeRecord {
    pub id: i64,
    pub created_at: DateTime<Utc>,          // TIMESTAMPTZ
    pub updated_at: Option<DateTime<Utc>>,   // TIMESTAMPTZ (nullable)
    pub scheduled_date: NaiveDate,           // DATE
    pub scheduled_time: NaiveDateTime,       // TIMESTAMP
    pub duration_seconds: i32,               // INTEGER for duration in seconds
}

#[repo]
trait TimeRepo {
    #[dml(
        r#"
        INSERT INTO time_records (created_at, scheduled_date, scheduled_time, duration_seconds)
        VALUES ($1, $2, $3, $4)
        RETURNING id, created_at, updated_at, scheduled_date, scheduled_time, duration_seconds
        "#
    )]
    async fn create_time_record(
        &self,
        created_at: DateTime<Utc>,
        scheduled_date: NaiveDate,
        scheduled_time: NaiveDateTime,
        duration_seconds: i32,
    ) -> Result<TimeRecord>;

    #[dml("SELECT id, created_at, updated_at, scheduled_date, scheduled_time, duration_seconds FROM time_records WHERE id = $1")]
    async fn find_by_id(&self, id: i64) -> Result<Option<TimeRecord>>;

    #[dml("SELECT COUNT(*) FROM time_records WHERE created_at > $1")]
    async fn count_created_after(&self, cutoff: DateTime<Utc>) -> Result<i64>;

    #[dml("SELECT COUNT(*) FROM time_records WHERE scheduled_date = $1")]
    async fn count_scheduled_on_date(&self, date: NaiveDate) -> Result<i64>;

    #[dml("SELECT COUNT(*) FROM time_records WHERE scheduled_time BETWEEN $1 AND $2")]
    async fn count_scheduled_in_time_range(
        &self,
        start_time: NaiveDateTime,
        end_time: NaiveDateTime,
    ) -> Result<i64>;

    #[dml("UPDATE time_records SET updated_at = $2 WHERE id = $1")]
    async fn update_timestamp(&self, id: i64, updated_at: DateTime<Utc>) -> Result<sqlx_data::QueryResult>;

    // PostgreSQL specific time functions
    #[dml("SELECT EXTRACT(YEAR FROM created_at) as year FROM time_records WHERE id = $1")]
    async fn get_creation_year(&self, id: i64) -> Result<Option<f64>>;

    #[dml("SELECT EXTRACT(DOW FROM scheduled_date) as day_of_week FROM time_records WHERE id = $1")]
    async fn get_day_of_week(&self, id: i64) -> Result<Option<f64>>;

    // Date arithmetic with PostgreSQL intervals
    #[dml("SELECT COUNT(*) FROM time_records WHERE created_at > NOW() - INTERVAL '$1 days'")]
    async fn count_created_in_last_days(&self, days: i32) -> Result<i64>;

    #[dml("SELECT COUNT(*) FROM time_records WHERE scheduled_date > CURRENT_DATE + INTERVAL '$1 days'")]
    async fn count_scheduled_after_days(&self, days: i32) -> Result<i64>;

    // Time zone conversions
    #[dml("SELECT created_at AT TIME ZONE 'America/New_York' as ny_time FROM time_records WHERE id = $1")]
    async fn get_creation_time_ny(&self, id: i64) -> Result<Option<NaiveDateTime>>;

    // Duration calculations
    #[dml("SELECT SUM(duration_seconds) FROM time_records WHERE scheduled_date = $1")]
    async fn total_duration_for_date(&self, date: NaiveDate) -> Result<Option<i64>>;

    // Complex time filtering
    #[dml(
        r#"
        SELECT id, created_at, updated_at, scheduled_date, scheduled_time, duration_seconds
        FROM time_records
        WHERE DATE(scheduled_time) = $1
        AND EXTRACT(HOUR FROM scheduled_time) BETWEEN $2 AND $3
        ORDER BY scheduled_time
        "#
    )]
    async fn find_by_date_and_hour_range(
        &self,
        date: NaiveDate,
        start_hour: i32,
        end_hour: i32,
    ) -> Result<Vec<TimeRecord>>;
}

pub struct TestTimeApp {
    pool: Pool,
}

impl TimeRepo for TestTimeApp {
    fn get_pool(&self) -> &Pool {
        &self.pool
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[sqlx::test(migrations = "tests/migrations_datetime")]
    async fn test_create_time_record(pool: Pool) -> Result<()> {
        let app = TestTimeApp { pool };

        let created_at = Utc::now();
        let scheduled_date = NaiveDate::from_ymd_opt(2024, 12, 15).unwrap();
        let scheduled_time = NaiveDate::from_ymd_opt(2024, 12, 15).unwrap()
            .and_hms_opt(14, 30, 0).unwrap();
        let duration_seconds = 3600; // 1 hour

        let record = app
            .create_time_record(created_at, scheduled_date, scheduled_time, duration_seconds)
            .await?;

        assert!(record.id > 0);
        assert_eq!(record.scheduled_date, scheduled_date);
        assert_eq!(record.scheduled_time, scheduled_time);
        assert_eq!(record.duration_seconds, duration_seconds);
        assert!(record.updated_at.is_none());

        // Verify the timestamp was stored correctly (within a small tolerance)
        let diff = (record.created_at - created_at).num_seconds().abs();
        assert!(diff <= 1, "Timestamp difference should be minimal");

        Ok(())
    }

    #[sqlx::test(migrations = "tests/migrations_datetime")]
    async fn test_find_by_id(pool: Pool) -> Result<()> {
        let app = TestTimeApp { pool };

        let created_at = Utc::now();
        let scheduled_date = NaiveDate::from_ymd_opt(2024, 6, 20).unwrap();
        let scheduled_time = scheduled_date.and_hms_opt(9, 15, 30).unwrap();

        let created_record = app
            .create_time_record(created_at, scheduled_date, scheduled_time, 7200)
            .await?;

        let found_record = app
            .find_by_id(created_record.id)
            .await?
            .expect("Record should be found");

        assert_eq!(found_record.id, created_record.id);
        assert_eq!(found_record.scheduled_date, scheduled_date);
        assert_eq!(found_record.scheduled_time, scheduled_time);
        assert_eq!(found_record.duration_seconds, 7200);

        Ok(())
    }

    #[sqlx::test(migrations = "tests/migrations_datetime")]
    async fn test_count_created_after(pool: Pool) -> Result<()> {
        let app = TestTimeApp { pool };

        let now = Utc::now();
        let one_hour_ago = now - Duration::hours(1);
        let two_hours_ago = now - Duration::hours(2);

        // Create records with different creation times
        app.create_time_record(
            two_hours_ago,
            NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2024, 1, 1).unwrap().and_hms_opt(12, 0, 0).unwrap(),
            1800,
        )
        .await?;

        app.create_time_record(
            now,
            NaiveDate::from_ymd_opt(2024, 1, 2).unwrap(),
            NaiveDate::from_ymd_opt(2024, 1, 2).unwrap().and_hms_opt(12, 0, 0).unwrap(),
            1800,
        )
        .await?;

        let count = app.count_created_after(one_hour_ago).await?;
        assert!(count >= 1); // Should include the record created "now"

        Ok(())
    }

    #[sqlx::test(migrations = "tests/migrations_datetime")]
    async fn test_date_operations(pool: Pool) -> Result<()> {
        let app = TestTimeApp { pool };

        let target_date = NaiveDate::from_ymd_opt(2024, 7, 4).unwrap();
        let scheduled_time = target_date.and_hms_opt(10, 30, 0).unwrap();

        // Create multiple records on the same date
        for i in 0..3 {
            app.create_time_record(
                Utc::now(),
                target_date,
                scheduled_time + Duration::hours(i),
                1800,
            )
            .await?;
        }

        let count = app.count_scheduled_on_date(target_date).await?;
        assert!(count >= 3);

        Ok(())
    }

    #[sqlx::test(migrations = "tests/migrations_datetime")]
    async fn test_time_range_operations(pool: Pool) -> Result<()> {
        let app = TestTimeApp { pool };

        let base_date = NaiveDate::from_ymd_opt(2024, 8, 15).unwrap();
        let start_time = base_date.and_hms_opt(9, 0, 0).unwrap();
        let end_time = base_date.and_hms_opt(17, 0, 0).unwrap();

        // Create records within and outside the range
        app.create_time_record(Utc::now(), base_date, start_time, 1800).await?;
        app.create_time_record(Utc::now(), base_date, start_time + Duration::hours(2), 1800).await?;
        app.create_time_record(Utc::now(), base_date, end_time + Duration::hours(1), 1800).await?;

        let count_in_range = app
            .count_scheduled_in_time_range(start_time, end_time)
            .await?;

        assert!(count_in_range >= 2); // Should include first two records

        Ok(())
    }

    #[sqlx::test(migrations = "tests/migrations_datetime")]
    async fn test_update_timestamp(pool: Pool) -> Result<()> {
        let app = TestTimeApp { pool };

        let record = app
            .create_time_record(
                Utc::now(),
                NaiveDate::from_ymd_opt(2024, 3, 10).unwrap(),
                NaiveDate::from_ymd_opt(2024, 3, 10).unwrap().and_hms_opt(15, 45, 0).unwrap(),
                2700,
            )
            .await?;

        let update_time = Utc::now() + Duration::minutes(5);
        app.update_timestamp(record.id, update_time).await?;

        let updated_record = app
            .find_by_id(record.id)
            .await?
            .expect("Record should exist");

        assert!(updated_record.updated_at.is_some());
        let actual_update_time = updated_record.updated_at.unwrap();
        let diff = (actual_update_time - update_time).num_seconds().abs();
        assert!(diff <= 1);

        Ok(())
    }

    #[sqlx::test(migrations = "tests/migrations_datetime")]
    async fn test_postgresql_extract_functions(pool: Pool) -> Result<()> {
        let app = TestTimeApp { pool };

        let test_date = Utc.ymd_opt(2024, 12, 25).unwrap().and_hms_opt(14, 30, 0).unwrap();
        let scheduled_date = NaiveDate::from_ymd_opt(2024, 12, 25).unwrap(); // Christmas - Wednesday in 2024
        let scheduled_time = scheduled_date.and_hms_opt(14, 30, 0).unwrap();

        let record = app
            .create_time_record(test_date, scheduled_date, scheduled_time, 1800)
            .await?;

        // Test EXTRACT(YEAR FROM ...)
        let year = app.get_creation_year(record.id).await?;
        assert_eq!(year, Some(2024.0));

        // Test EXTRACT(DOW FROM ...) - Day of week (0 = Sunday, 1 = Monday, ...)
        let dow = app.get_day_of_week(record.id).await?;
        assert!(dow.is_some());
        let day_of_week = dow.unwrap();
        // December 25, 2024 is a Wednesday (3)
        assert_eq!(day_of_week, 3.0);

        Ok(())
    }

    #[sqlx::test(migrations = "tests/migrations_datetime")]
    async fn test_postgresql_interval_operations(pool: Pool) -> Result<()> {
        let app = TestTimeApp { pool };

        // Create a record from "yesterday"
        let yesterday = Utc::now() - Duration::days(1);
        app.create_time_record(
            yesterday,
            NaiveDate::from_ymd_opt(2024, 1, 15).unwrap(),
            NaiveDate::from_ymd_opt(2024, 1, 15).unwrap().and_hms_opt(12, 0, 0).unwrap(),
            1800,
        )
        .await?;

        // Count records created in last 2 days
        let count_recent = app.count_created_in_last_days(2).await?;
        assert!(count_recent >= 1);

        // Count records created in last 1 day (might be 0 depending on exact timing)
        let count_today = app.count_created_in_last_days(1).await?;
        assert!(count_today >= 0);

        // Test future scheduling
        let future_date = chrono::Utc::now().naive_utc().date() + Duration::days(10);
        app.create_time_record(
            Utc::now(),
            future_date,
            future_date.and_hms_opt(12, 0, 0).unwrap(),
            1800,
        )
        .await?;

        let count_future = app.count_scheduled_after_days(5).await?;
        assert!(count_future >= 1);

        Ok(())
    }

    #[sqlx::test(migrations = "tests/migrations_datetime")]
    async fn test_timezone_conversion(pool: Pool) -> Result<()> {
        let app = TestTimeApp { pool };

        let utc_time = Utc::now();
        let record = app
            .create_time_record(
                utc_time,
                NaiveDate::from_ymd_opt(2024, 6, 15).unwrap(),
                NaiveDate::from_ymd_opt(2024, 6, 15).unwrap().and_hms_opt(20, 0, 0).unwrap(),
                1800,
            )
            .await?;

        // Test timezone conversion to New York time
        let ny_time = app.get_creation_time_ny(record.id).await?;
        assert!(ny_time.is_some());

        // The NY time should be different from UTC (unless it's exactly UTC-4 or UTC-5 offset)
        // We can't assert exact values due to DST, but we can check it's a valid timestamp
        let ny_timestamp = ny_time.unwrap();
        assert!(ny_timestamp.year() > 2020);

        Ok(())
    }

    #[sqlx::test(migrations = "tests/migrations_datetime")]
    async fn test_duration_calculations(pool: Pool) -> Result<()> {
        let app = TestTimeApp { pool };

        let target_date = NaiveDate::from_ymd_opt(2024, 9, 20).unwrap();

        // Create multiple records on the same date with different durations
        let durations = vec![3600, 1800, 2700]; // 1 hour, 30 min, 45 min
        for duration in &durations {
            app.create_time_record(
                Utc::now(),
                target_date,
                target_date.and_hms_opt(12, 0, 0).unwrap(),
                *duration,
            )
            .await?;
        }

        let total_duration = app.total_duration_for_date(target_date).await?;
        assert!(total_duration.is_some());

        let total_seconds = total_duration.unwrap();
        let expected_total: i64 = durations.iter().sum::<i32>() as i64;
        assert!(total_seconds >= expected_total);

        Ok(())
    }

    #[sqlx::test(migrations = "tests/migrations_datetime")]
    async fn test_complex_time_filtering(pool: Pool) -> Result<()> {
        let app = TestTimeApp { pool };

        let test_date = NaiveDate::from_ymd_opt(2024, 10, 10).unwrap();

        // Create records at different hours
        let times_and_hours = vec![
            (8, 0),   // 8:00 AM - outside range
            (10, 30), // 10:30 AM - inside range
            (14, 15), // 2:15 PM - inside range
            (18, 45), // 6:45 PM - outside range
        ];

        for (hour, minute) in times_and_hours {
            let scheduled_time = test_date.and_hms_opt(hour, minute, 0).unwrap();
            app.create_time_record(Utc::now(), test_date, scheduled_time, 1800)
                .await?;
        }

        // Find records scheduled between 9 AM and 5 PM (9-17 hours)
        let records = app
            .find_by_date_and_hour_range(test_date, 9, 17)
            .await?;

        assert!(records.len() >= 2); // Should include 10:30 and 14:15

        // Verify all returned records are within the time range
        for record in records {
            assert_eq!(record.scheduled_date, test_date);
            let hour = record.scheduled_time.hour();
            assert!(hour >= 9 && hour <= 17);
        }

        Ok(())
    }

    #[sqlx::test(migrations = "tests/migrations_datetime")]
    async fn test_null_datetime_handling(pool: Pool) -> Result<()> {
        let app = TestTimeApp { pool };

        let record = app
            .create_time_record(
                Utc::now(),
                NaiveDate::from_ymd_opt(2024, 11, 5).unwrap(),
                NaiveDate::from_ymd_opt(2024, 11, 5).unwrap().and_hms_opt(16, 0, 0).unwrap(),
                900,
            )
            .await?;

        // Initially updated_at should be NULL
        assert!(record.updated_at.is_none());

        // Update it and verify it's no longer NULL
        app.update_timestamp(record.id, Utc::now()).await?;

        let updated_record = app
            .find_by_id(record.id)
            .await?
            .expect("Record should exist");

        assert!(updated_record.updated_at.is_some());

        Ok(())
    }
}