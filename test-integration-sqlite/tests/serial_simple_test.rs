use sqlx_data::{IntoParams, ParamsBuilder, Pool, Result, Serial, SerialParams};
use sqlx_data::{dml, repository};

#[derive(Debug, sqlx::FromRow)]
pub struct User {
    #[allow(dead_code)]
    id: i64,
    #[allow(dead_code)]
    name: String,
}

#[repository]
trait UserRepo {
    #[dml("SELECT id, name FROM users")]
    async fn find_all_serial(&self, params: SerialParams) -> Result<Serial<User>>;

    #[dml("SELECT id, name FROM users")]
    async fn find_all_builder(&self, params: impl IntoParams) -> Result<Serial<User>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_serial_params_direct(pool: Pool) {
        let repo = TestUserRepo { pool: &pool };

        let params = SerialParams::new(1, 5);
        let page = repo.find_all_serial(params).await.unwrap();

        assert_eq!(page.page, 1);
        assert_eq!(page.size, 5);
        assert_eq!(page.total_items, 20); // 20 users from fixture
        assert_eq!(page.total_pages, 4); // 20/5 = 4 pages
        println!(
            "Serial params direct works! Page: {}, Size: {}, Total: {}, Pages: {}",
            page.page, page.size, page.total_items, page.total_pages
        );
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_serial_params_with_builder(pool: Pool) {
        let repo = TestUserRepo { pool: &pool };

        let params = ParamsBuilder::new().serial().page(1, 5).done().build();

        let page = repo.find_all_builder(params).await.unwrap();

        assert_eq!(page.page, 1);
        assert_eq!(page.size, 5);
        assert_eq!(page.total_items, 20); // 20 users from fixture
        assert_eq!(page.total_pages, 4); // 20/5 = 4 pages
        println!(
            "Serial params with builder works! Page: {}, Size: {}, Total: {}, Pages: {}",
            page.page, page.size, page.total_items, page.total_pages
        );
    }
}

struct TestUserRepo<'a> {
    pool: &'a Pool,
}

impl UserRepo for TestUserRepo<'_> {
    fn get_pool(&self) -> &Pool {
        self.pool
    }
}
