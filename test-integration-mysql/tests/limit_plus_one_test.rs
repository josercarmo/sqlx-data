use sqlx_data::{IntoParams, ParamsBuilder, Pool, Serial, SerialParams, Slice, SliceParams};
use sqlx_data::{dml, repo};

#[derive(Debug, sqlx::FromRow)]
pub struct User {
    #[allow(dead_code)]
    pub id: i64,
    #[allow(dead_code)]
    pub name: String,
}

#[repo]
trait UserRepo {
    #[dml("SELECT id, name FROM users ORDER BY id")]
    async fn find_serial(&self, params: SerialParams) -> Result<Serial<User>, sqlx::Error>;

    #[dml("SELECT id, name FROM users ORDER BY id")]
    async fn find_slice(&self, params: SliceParams) -> Result<Slice<User>, sqlx::Error>;

    #[dml("SELECT id, name FROM users ORDER BY id")]
    async fn find_slice_with_builder(
        &self,
        params: impl IntoParams,
    ) -> Result<Slice<User>, sqlx::Error>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_serial_no_limit_plus_one(pool: Pool) {
        let repo = TestUserRepo { pool: &pool };

        let params = SerialParams::new(1, 3);
        let page = repo.find_serial(params).await.unwrap();

        assert_eq!(page.size, 3);
        assert_eq!(page.total_items, 20);
        assert_eq!(page.data.len(), 3);
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_slice_with_limit_plus_one(pool: Pool) {
        let repo = TestUserRepo { pool: &pool };

        let params = SliceParams::new(1, 3);
        let page = repo.find_slice(params).await.unwrap();

        assert_eq!(page.size, 3);
        assert_eq!(page.data.len(), 3);
        assert!(page.has_next);
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_slice_builder_disable_total_count(pool: Pool) {
        let repo = TestUserRepo { pool: &pool };

        let params = ParamsBuilder::new().slice().page(1, 3).done().build();

        let page = repo.find_slice_with_builder(params).await.unwrap();

        assert_eq!(page.size, 3);
        assert_eq!(page.data.len(), 3);
        assert!(page.has_next);
        assert!(page.total_items.is_none());
        assert!(!page.has_previous);
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_slice_no_next_page(pool: Pool) {
        let repo = TestUserRepo { pool: &pool };

        let params = SliceParams::new(1, 25);
        let page = repo.find_slice(params).await.unwrap();

        assert_eq!(page.size, 25);
        assert_eq!(page.data.len(), 20);
        assert!(!page.has_next);
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_slice_has_previous_page(pool: Pool) {
        let repo = TestUserRepo { pool: &pool };

        let params = SliceParams::new(2, 5);
        let page = repo.find_slice(params).await.unwrap();

        assert_eq!(page.page, 2);
        assert_eq!(page.size, 5);
        assert!(page.has_previous);
        assert!(page.has_next);

        let params3 = SliceParams::new(4, 5);
        let page3 = repo.find_slice(params3).await.unwrap();

        assert_eq!(page3.page, 4);
        assert_eq!(page3.size, 5);
        assert!(page3.has_previous);
        assert!(!page3.has_next);
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_slice_with_total_count_enabled_should_fail(pool: Pool) {
        let repo = TestUserRepo { pool: &pool };

        let params = SliceParams::new(1, 5).with_disable_total_count(false);

        assert!(
            !params.disable_total_count(),
            "disable_total_count should be false"
        );

        let page = repo.find_slice(params).await.unwrap();

        assert!(
            page.total_items.is_some(),
            "total_items should be Some when disable_total_count=false, but got None. \
            This proves the bug: into_params() ignores SliceParams.disable_total_count and always forces true"
        );

        if let Some(total) = page.total_items {
            assert_eq!(total, 20, "Should have 20 total items");
        }
    }

    #[test]
    fn test_slice_params_configuration_bug() {
        let slice_params = SliceParams::new(1, 10).with_disable_total_count(false);

        assert!(
            !slice_params.disable_total_count(),
            "SliceParams should have disable_total_count=false"
        );

        let params = slice_params.into_params();

        assert!(
            !params.is_disable_total_count(),
            "BUG: into_params() should respect SliceParams.disable_total_count=false, \
            but it always forces disable_total_count=true on line 75 of slice.rs"
        );
    }
}

struct TestUserRepo<'a> {
    pool: &'a Pool,
}

impl<'a> UserRepo for TestUserRepo<'a> {
    fn get_pool(&self) -> &Pool {
        self.pool
    }
}