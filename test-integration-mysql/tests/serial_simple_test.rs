use sqlx_data::{Pool, Result, Serial, SerialParams, dml, repo};

#[derive(Debug, sqlx::FromRow)]
pub struct User {
    pub id: i64,
    pub name: String,
    pub email: String,
    pub age: u8,
}

#[repo]
trait SimpleSerialRepo {
    #[dml("SELECT id, name, email, age FROM users ORDER BY id")]
    async fn find_all(&self, params: SerialParams) -> Result<Serial<User>>;

    #[dml("SELECT id, name, email, age FROM users WHERE age > ? ORDER BY name")]
    async fn find_by_min_age(&self, min_age: u8, params: SerialParams) -> Result<Serial<User>>;

    #[dml("SELECT COUNT(*) FROM users")]
    async fn count_all(&self) -> Result<i64>;
}

pub struct SimpleSerialApp {
    pool: Pool,
}

impl SimpleSerialRepo for SimpleSerialApp {
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
    async fn test_serial_basic_pagination(pool: Pool) {
        let app = SimpleSerialApp { pool };

        let params = SerialParams::new(1, 5);
        let page1 = app.find_all(params).await.unwrap();

        assert_eq!(page1.page, 1);
        assert_eq!(page1.size, 5);
        assert_eq!(page1.data.len(), 5);
        assert_eq!(page1.total_items, 20);
        assert_eq!(page1.total_pages, 4);

        let params = SerialParams::new(2, 5);
        let page2 = app.find_all(params).await.unwrap();

        assert_eq!(page2.page, 2);
        assert_eq!(page2.data.len(), 5);

        let first_page_ids: Vec<i64> = page1.data.iter().map(|u| u.id).collect();
        let second_page_ids: Vec<i64> = page2.data.iter().map(|u| u.id).collect();

        for id in second_page_ids {
            assert!(!first_page_ids.contains(&id));
        }
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_serial_with_filter(pool: Pool) {
        let app = SimpleSerialApp { pool };

        let params = SerialParams::new(1, 10);
        let page = app.find_by_min_age(25, params).await.unwrap();

        assert_eq!(page.page, 1);
        assert_eq!(page.size, 10);

        for user in &page.data {
            assert!(user.age > 25);
        }
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_serial_last_page(pool: Pool) {
        let app = SimpleSerialApp { pool };

        let params = SerialParams::new(4, 5);
        let last_page = app.find_all(params).await.unwrap();

        assert_eq!(last_page.page, 4);
        assert_eq!(last_page.size, 5);
        assert_eq!(last_page.data.len(), 5);
        assert_eq!(last_page.total_pages, 4);
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_serial_beyond_available_pages(pool: Pool) {
        let app = SimpleSerialApp { pool };

        let params = SerialParams::new(10, 5);
        let empty_page = app.find_all(params).await.unwrap();

        assert_eq!(empty_page.page, 10);
        assert_eq!(empty_page.size, 5);
        assert_eq!(empty_page.data.len(), 0);
        assert_eq!(empty_page.total_items, 20);
        assert_eq!(empty_page.total_pages, 4);
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_count_consistency(pool: Pool) {
        let app = SimpleSerialApp { pool };

        let count = app.count_all().await.unwrap();

        let params = SerialParams::new(1, 100);
        let page = app.find_all(params).await.unwrap();

        assert_eq!(count, page.total_items as i64);
        assert_eq!(count, 20);
    }
}