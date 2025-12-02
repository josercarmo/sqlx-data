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

        // Serial with page_size = 3, should fetch exactly 3 items (no +1)
        let params = SerialParams::new(1, 3);
        let page = repo.find_serial(params).await.unwrap();

        println!(
            "Serial: Page {}, Size {}, Total {}, Pages {}",
            page.page, page.size, page.total_items, page.total_pages
        );
        assert_eq!(page.size, 3);
        assert_eq!(page.total_items, 20); // 20 users from fixture
        assert_eq!(page.data.len(), 3); // Returns exactly 3 (no +1)
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_slice_with_limit_plus_one(pool: Pool) {
        let repo = TestUserRepo { pool: &pool };

        // Slice with page_size = 3, should fetch 4 items (3 + 1) to detect has_next
        let params = SliceParams::new(1, 3);
        let page = repo.find_slice(params).await.unwrap();

        println!(
            "Slice: Page {}, Size {}, HasNext {}",
            page.page, page.size, page.has_next
        );
        assert_eq!(page.size, 3);
        assert_eq!(page.data.len(), 3); // Returns 3 (the +1 is removed if has_next=true)
        assert!(page.has_next); // Should have next page (20 total, requested 3, 17 remaining)
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_slice_builder_disable_total_count(pool: Pool) {
        let repo = TestUserRepo { pool: &pool };

        // Builder slice should also use LIMIT+1
        let params = ParamsBuilder::new().slice().page(1, 3).done().build();

        let page = repo.find_slice_with_builder(params).await.unwrap();

        println!(
            "Slice Builder: Page {}, Size {}, HasNext {}, Total {:?}",
            page.page, page.size, page.has_next, page.total_items
        );
        assert_eq!(page.size, 3);
        assert_eq!(page.data.len(), 3);
        assert!(page.has_next);
        assert!(page.total_items.is_none()); // Slice doesn't count total by default
        assert!(!page.has_previous); // First page should not have previous
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_slice_no_next_page(pool: Pool) {
        let repo = TestUserRepo { pool: &pool };

        // Slice with page_size = 25, more than the 20 available items
        let params = SliceParams::new(1, 25);
        let page = repo.find_slice(params).await.unwrap();

        println!(
            "Slice Large: Page {}, Size {}, HasNext {}",
            page.page, page.size, page.has_next
        );
        assert_eq!(page.size, 25);
        assert_eq!(page.data.len(), 20); // Returns all 20 available items
        assert!(!page.has_next); // No next page
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_slice_has_previous_page(pool: Pool) {
        let repo = TestUserRepo { pool: &pool };

        // Test page 2 - should have has_previous = true
        let params = SliceParams::new(2, 5);
        let page = repo.find_slice(params).await.unwrap();

        println!(
            "Slice Page 2: Page {}, Size {}, HasNext {}, HasPrevious {}",
            page.page, page.size, page.has_next, page.has_previous
        );
        assert_eq!(page.page, 2);
        assert_eq!(page.size, 5);
        assert!(page.has_previous); // Page 2 should have previous
        assert!(page.has_next); // Should still have next (20 total, on page 2 with size 5)

        // Test page 4 - should have has_previous = true and has_next = false
        let params3 = SliceParams::new(4, 5);
        let page3 = repo.find_slice(params3).await.unwrap();

        println!(
            "Slice Page 4: Page {}, Size {}, HasNext {}, HasPrevious {}",
            page3.page, page3.size, page3.has_next, page3.has_previous
        );
        assert_eq!(page3.page, 4);
        assert_eq!(page3.size, 5);
        assert!(page3.has_previous); // Page 4 should have previous
        assert!(!page3.has_next); // Should not have next (20 items, 4 pages of 5)
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_slice_with_total_count_enabled_should_fail(pool: Pool) {
        let repo = TestUserRepo { pool: &pool };

        // Create SliceParams with disable_total_count = false
        // This should include total_items in the result, but currently it will fail
        // because the into_params() always forces disable_total_count: true
        let params = SliceParams::new(1, 5).with_disable_total_count(false); // User wants total count

        // Verify that our intention is set correctly
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
        // This test demonstrates the bug in into_params()
        let slice_params = SliceParams::new(1, 10).with_disable_total_count(false); // We want total count

        assert!(
            !slice_params.disable_total_count(),
            "SliceParams should have disable_total_count=false"
        );

        let params = slice_params.into_params();

        // This assertion will FAIL, proving the bug
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
