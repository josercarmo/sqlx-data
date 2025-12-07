#![cfg(feature = "time")]

use sqlx::types::time::{Date, OffsetDateTime, PrimitiveDateTime, Time, UtcOffset};
use sqlx_data::{Pool, QueryResult, Result, dml, repo};

#[derive(Debug, sqlx::FromRow)]
pub struct TimeRecord {
    pub id: i64,
    pub name: String,
    pub created_date: Date,
    pub created_time: Time,
    pub created_datetime: PrimitiveDateTime,
    pub created_offset: OffsetDateTime,
}

#[repo]
trait TimeRepo {
    #[dml(
        r#"
        INSERT INTO time_records (name, created_date, created_time, created_datetime, created_offset)
        VALUES ($1, $2, $3, $4, $5)
        "#
    )]
    async fn insert_time_record(
        &self,
        name: String,
        created_date: Date,
        created_time: Time,
        created_datetime: PrimitiveDateTime,
        created_offset: OffsetDateTime,
    ) -> Result<QueryResult>;

    #[dml("SELECT id, name, created_date, created_time, created_datetime, created_offset FROM time_records WHERE id = $1")]
    async fn find_by_id(&self, id: i64) -> Result<Option<TimeRecord>>;

    #[dml("SELECT id, name, created_date, created_time, created_datetime, created_offset FROM time_records WHERE created_date = $1")]
    async fn find_by_date(&self, date: Date) -> Result<Vec<TimeRecord>>;

    #[dml("SELECT id, name, created_date, created_time, created_datetime, created_offset FROM time_records WHERE created_time >= $1")]
    async fn find_by_time_after(&self, time: Time) -> Result<Vec<TimeRecord>>;

    #[dml("SELECT id, name, created_date, created_time, created_datetime, created_offset FROM time_records WHERE created_datetime BETWEEN $1 AND $2")]
    async fn find_by_datetime_range(
        &self,
        start: PrimitiveDateTime,
        end: PrimitiveDateTime,
    ) -> Result<Vec<TimeRecord>>;

    #[dml("SELECT id, name, created_date, created_time, created_datetime, created_offset FROM time_records WHERE created_offset > $1")]
    async fn find_after_offset(&self, offset_dt: OffsetDateTime) -> Result<Vec<TimeRecord>>;
}

pub struct TimeApp {
    pool: Pool,
}

impl TimeRepo for TimeApp {
    fn get_pool(&self) -> &Pool {
        &self.pool
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[sqlx::test(migrations = "tests/migrations_datetime")]
    async fn test_insert_and_retrieve_time_types(pool: Pool) {
        let app = TimeApp { pool };

        let test_date = Date::from_ordinal_date(2024, 75).unwrap(); // March 15
        let test_time = Time::from_hms(14, 30, 45).unwrap();
        let test_datetime = PrimitiveDateTime::new(test_date, test_time);
        let test_offset = test_datetime.assume_offset(UtcOffset::from_hms(-5, 0, 0).unwrap());

        let result = app
            .insert_time_record(
                "Test Record".to_string(),
                test_date,
                test_time,
                test_datetime,
                test_offset,
            )
            .await
            .unwrap();

        assert!(result.rows_affected() > 0);

        // PostgreSQL doesn't have last_insert_id like MySQL, query by name instead
        let records = app.find_by_date(test_date).await.unwrap();
        assert!(!records.is_empty());

        let record = &records[0];
        assert_eq!(record.name, "Test Record");
        assert_eq!(record.created_date, test_date);
        assert_eq!(record.created_time, test_time);
        assert_eq!(record.created_datetime, test_datetime);
    }

    #[sqlx::test(migrations = "tests/migrations_datetime")]
    async fn test_query_by_date(pool: Pool) {
        let app = TimeApp { pool };

        let test_date = Date::from_ordinal_date(2024, 153).unwrap(); // June 1
        let test_time = Time::from_hms(9, 0, 0).unwrap();
        let test_datetime = PrimitiveDateTime::new(test_date, test_time);
        let test_offset = test_datetime.assume_utc();

        app.insert_time_record(
            "Date Query Test".to_string(),
            test_date,
            test_time,
            test_datetime,
            test_offset,
        )
        .await
        .unwrap();

        let records = app.find_by_date(test_date).await.unwrap();
        assert!(!records.is_empty());
        assert!(records.iter().any(|r| r.name == "Date Query Test"));
    }

    #[sqlx::test(migrations = "tests/migrations_datetime")]
    async fn test_query_by_time_range(pool: Pool) {
        let app = TimeApp { pool };

        let morning_time = Time::from_hms(8, 0, 0).unwrap();
        let afternoon_time = Time::from_hms(14, 0, 0).unwrap();

        for (name, time_val) in [
            ("Morning Record", Time::from_hms(7, 30, 0).unwrap()),
            ("Mid Morning Record", Time::from_hms(10, 15, 0).unwrap()),
            ("Afternoon Record", Time::from_hms(16, 45, 0).unwrap()),
        ] {
            let date = Date::from_ordinal_date(2024, 183).unwrap(); // July 1
            let datetime = PrimitiveDateTime::new(date, time_val);
            let offset_dt = datetime.assume_utc();

            app.insert_time_record(name.to_string(), date, time_val, datetime, offset_dt)
                .await
                .unwrap();
        }

        let morning_records = app.find_by_time_after(morning_time).await.unwrap();
        let afternoon_records = app.find_by_time_after(afternoon_time).await.unwrap();

        assert!(morning_records.len() >= 2);
        assert!(afternoon_records.len() >= 1);

        for record in &afternoon_records {
            assert!(record.created_time >= afternoon_time);
        }
    }

    #[sqlx::test(migrations = "tests/migrations_datetime")]
    async fn test_datetime_range_queries(pool: Pool) {
        let app = TimeApp { pool };

        let base_date = Date::from_ordinal_date(2024, 228).unwrap(); // August 15
        let start_datetime = PrimitiveDateTime::new(base_date, Time::from_hms(10, 0, 0).unwrap());
        let end_datetime = PrimitiveDateTime::new(base_date, Time::from_hms(16, 0, 0).unwrap());

        for (name, hour) in [
            ("Before Range", 8),
            ("Start Range", 10),
            ("Mid Range", 13),
            ("End Range", 16),
            ("After Range", 18),
        ] {
            let time_val = Time::from_hms(hour, 0, 0).unwrap();
            let datetime = PrimitiveDateTime::new(base_date, time_val);
            let offset_dt = datetime.assume_utc();

            app.insert_time_record(name.to_string(), base_date, time_val, datetime, offset_dt)
                .await
                .unwrap();
        }

        let range_records = app
            .find_by_datetime_range(start_datetime, end_datetime)
            .await
            .unwrap();

        assert!(range_records.len() >= 3);

        for record in &range_records {
            assert!(record.created_datetime >= start_datetime);
            assert!(record.created_datetime <= end_datetime);
        }
    }

    #[sqlx::test(migrations = "tests/migrations_datetime")]
    async fn test_offset_datetime_queries(pool: Pool) {
        let app = TimeApp { pool };

        let utc_offset = UtcOffset::UTC;
        let eastern_offset = UtcOffset::from_hms(-5, 0, 0).unwrap();
        let pacific_offset = UtcOffset::from_hms(-8, 0, 0).unwrap();

        let base_date = Date::from_ordinal_date(2024, 245).unwrap(); // September 1
        let base_time = Time::from_hms(12, 0, 0).unwrap();
        let base_datetime = PrimitiveDateTime::new(base_date, base_time);

        for (name, offset) in [
            ("UTC Time", utc_offset),
            ("Eastern Time", eastern_offset),
            ("Pacific Time", pacific_offset),
        ] {
            let offset_dt = base_datetime.assume_offset(offset);
            let date = offset_dt.date();
            let time = offset_dt.time();

            app.insert_time_record(name.to_string(), date, time, base_datetime, offset_dt)
                .await
                .unwrap();
        }

        let cutoff = base_datetime.assume_offset(UtcOffset::from_hms(-6, 0, 0).unwrap());
        let after_cutoff = app.find_after_offset(cutoff).await.unwrap();

        assert!(!after_cutoff.is_empty());

        for record in &after_cutoff {
            assert!(record.created_offset > cutoff);
        }
    }

    #[sqlx::test(migrations = "tests/migrations_datetime")]
    async fn test_current_timestamp_functions(pool: Pool) {
        let app = TimeApp { pool };

        let now = OffsetDateTime::now_utc();
        let current_date = now.date();
        let current_time = now.time();
        let current_datetime = PrimitiveDateTime::new(current_date, current_time);

        let result = app
            .insert_time_record(
                "Current Time Test".to_string(),
                current_date,
                current_time,
                current_datetime,
                now,
            )
            .await
            .unwrap();

        assert!(result.rows_affected() > 0);

        let records = app.find_by_date(current_date).await.unwrap();
        assert!(!records.is_empty());
    }
}
