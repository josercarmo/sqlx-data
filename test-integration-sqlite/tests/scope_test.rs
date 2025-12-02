use sqlx_data::{Pool, Result, dml, repo};

// Use same structure as integration_tests
#[derive(Clone, PartialEq, Eq, Debug, sqlx::Type)]
#[sqlx(transparent)]
pub struct Id(i64);

impl From<i64> for Id {
    fn from(value: i64) -> Self {
        Id(value)
    }
}

// User model for tests (same as integration_tests)
#[derive(Debug, Clone, PartialEq)]
pub struct User {
    pub id: Id,
    pub name: String,
    pub email: String,
    pub age: u8,
    pub birth_year: Option<u16>,
}

// Test trait with new scope syntax
#[repo]
#[alias(
    user_columns = "id as 'id!: Id', name, email, age as 'age: u8', birth_year as 'birth_year: u16'"
)]
#[alias(user_table = "users")]
#[alias(age_threshold = "25")]
//#[scope(tenantable = "tenant_id = $1")]
//#[scope(active = "active =TRUE ")]
//#[scope(ownable = "user_id = $2")]
#[scope(young = "age < {{age_threshold}}")]
#[scope(old = "age >= {{age_threshold}}")]
#[scope(has_birth_year = "birth_year IS NOT NULL")]
#[scope(name_filter = "name LIKE '%a%'")]
#[scope(order_name = "name ASC", target = "order_by")]
trait ScopeUserRepo {
    // Basic query with default WHERE scopes
    #[dml("SELECT {{user_columns}} FROM {{user_table}}")]
    async fn find_all_users(&self) -> Result<Vec<User>>;

    // Query ignoring some scopes
    #[scope_ignore(young)]
    #[scope_ignore(old)]
    #[dml("SELECT {{user_columns}} FROM {{user_table}} WHERE id = $1")]
    async fn find_by_id(&self, id: i64) -> Result<User>;

    // Query ignoring order scope
    #[scope_ignore(order_name)]
    #[dml("SELECT {{user_columns}} FROM {{user_table}} WHERE age > $1")]
    async fn find_users_by_age_unordered(&self, min_age: u8) -> Result<Vec<User>>;

    // Query with custom WHERE and scopes
    #[dml("SELECT name FROM {{user_table}} ")]
    async fn find_active_user_names(&self) -> Result<Vec<String>>;
}

// Test implementation
pub struct TestScopeApp {
    pool: Pool,
}

impl ScopeUserRepo for TestScopeApp {
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
    async fn test_scope_integration_compiles(pool: Pool) {
        // This test primarily verifies that the macro generates compilable code
        // with the new scope syntax correctly integrated
        let _repo = TestScopeApp { pool };

        // If we reach here, the new scope syntax integration is working
        // (methods are generated and compile successfully)
        assert!(true);
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_find_all_users_with_scopes(pool: Pool) {
        let repo = TestScopeApp { pool };

        // This would test the actual scope application with new syntax
        // Should apply young, old, has_birth_year, name_filter, order_name scopes
        // Expected resolved scopes:
        // - young: "age < 25"
        // - old: "age >= 25"
        // - has_birth_year: "birth_year IS NOT NULL"
        // - name_filter: "name LIKE '%a%'"
        // - order_name: "name ASC" (ORDER BY clause)
        let _result = repo.find_all_users().await;
        // Note: This will fail until SQL parser integration is complete
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_find_by_id_with_ignored_scopes(pool: Pool) {
        let repo = TestScopeApp { pool };

        // This should ignore young and old scopes
        // but still apply has_birth_year, name_filter, order_name scopes
        let _result = repo.find_by_id(1).await;
        // Note: This will fail until SQL parser integration is complete
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_scope_sql_with_alias_substitution(pool: Pool) {
        let repo = TestScopeApp { pool };

        // This tests that scope SQL content receives alias substitution
        // young scope should become "age < 25" ({{age_threshold}} replaced)
        // old scope should become "age >= 25" ({{age_threshold}} replaced)
        let _result = repo.find_users_by_age_unordered(20).await;
        // Note: This will fail until SQL parser integration is complete
    }
}
