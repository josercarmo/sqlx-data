use sqlx_data::{DateTime, Pool, Result, Uuid, dml, repo};

// PostgreSQL type aliases and transparent newtypes
#[derive(Clone, PartialEq, Eq, Debug, sqlx::Type)]
#[sqlx(transparent)]
pub struct UserId(i64);

impl From<i64> for UserId {
    fn from(value: i64) -> Self {
        UserId(value)
    }
}

#[derive(Clone, PartialEq, Eq, Debug, sqlx::Type)]
#[sqlx(transparent)]
pub struct UserName(String);

impl From<String> for UserName {
    fn from(value: String) -> Self {
        UserName(value)
    }
}

impl From<&str> for UserName {
    fn from(value: &str) -> Self {
        UserName(value.to_string())
    }
}

// PostgreSQL-specific strong typed struct
#[derive(Debug, Clone, PartialEq)]
pub struct User {
    pub id: UserId,
    pub name: UserName,
    pub email: String,
    pub age: i16, // PostgreSQL SMALLINT
    pub birth_year: Option<i16>,
}

#[repo]
trait AliasRepo {
    // Basic queries with type aliases
    #[dml("SELECT id as \"id!: UserId\", name, email, age, birth_year FROM users WHERE id = $1")]
    async fn find_by_typed_id(&self, id: i64) -> Result<User>;

    #[dml(
        "SELECT id as \"id!: UserId\", name as \"name!: UserName\", email, age, birth_year FROM users WHERE name = $1"
    )]
    async fn find_by_typed_name(&self, name: String) -> Result<Option<User>>;

    // Multiple type aliases in same query
    #[dml(
        "SELECT id as \"id!: UserId\", name as \"name!: UserName\" FROM users WHERE id = $1 AND name = $2"
    )]
    async fn find_id_name_by_typed_params(
        &self,
        id: i64,
        name: String,
    ) -> Result<Option<(UserId, UserName)>>;

    // Type aliases with aggregation
    #[dml("SELECT COUNT(*)::BIGINT as user_count FROM users WHERE id > $1")]
    async fn count_users_after_typed_id(&self, id: i64) -> Result<Option<i64>>;

    // PostgreSQL-specific: UUID aliases (if available)
    #[dml("SELECT gen_random_uuid() as random_uuid")]
    async fn generate_uuid(&self) -> Result<Option<Uuid>>;

    // Complex type coercion with PostgreSQL casting
    #[dml(
        "SELECT id::TEXT as \"id_str!: String\", name::TEXT as \"name_str!: String\" FROM users WHERE id = $1"
    )]
    async fn get_user_as_strings(&self, id: i64) -> Result<(String, String)>;

    // PostgreSQL ENUM-like behavior with type aliases
    #[dml(
        "SELECT CASE WHEN age < 25 THEN 'young'::TEXT WHEN age < 35 THEN 'middle'::TEXT ELSE 'senior'::TEXT END as \"category!: String\" FROM users WHERE id = $1"
    )]
    async fn get_age_category(&self, id: i64) -> Result<String>;

    // Multiple return types with aliases
    #[dml("SELECT id as \"id!: UserId\", name as \"name!: UserName\", age FROM users ORDER BY id")]
    async fn get_all_typed_users(&self) -> Result<Vec<(UserId, UserName, i16)>>;

    // Nullable type aliases
    #[dml("SELECT birth_year FROM users WHERE id = $1")]
    async fn get_optional_birth_year(&self, id: i64) -> Result<Option<i16>>;

    // Complex expression with type aliases
    #[dml(
        "SELECT (id * 2) as \"doubled_id!: i64\", UPPER(name::TEXT) as \"upper_name!: String\" FROM users WHERE id = $1"
    )]
    async fn get_computed_values(&self, id: i64) -> Result<(i64, String)>;

    // PostgreSQL-specific: JSON path with type alias
    #[dml(
        "SELECT jsonb_build_object('id', id, 'name', name) as \"user_json!: sqlx::types::Json<sqlx::types::JsonValue>\" FROM users WHERE id = $1"
    )]
    async fn get_user_as_json(
        &self,
        id: i64,
    ) -> Result<sqlx::types::Json<sqlx::types::JsonValue>>;

    // Window functions with type aliases
    #[dml(
        "SELECT id as \"id!: UserId\", name as \"name!: UserName\", ROW_NUMBER() OVER (ORDER BY age) as \"row_num!: i64\" FROM users"
    )]
    async fn get_users_with_row_numbers(&self) -> Result<Vec<(UserId, UserName, i64)>>;

    // PostgreSQL interval types
    #[dml("SELECT NOW() - INTERVAL '1 day' as \"yesterday!: DateTime\"")]
    async fn get_yesterday(&self) -> Result<DateTime>;

    //Does not work yet, maybe in future versions of sqlx 0.9
    // Type aliases in subqueries
    // #[dml(
    //     "SELECT id as \"id!: UserId\" FROM users WHERE age > (SELECT AVG(age)::SMALLINT FROM users) ORDER BY id"
    // )]
    // async fn get_above_average_users(&self) -> Result<Vec<UserId>>;
}

pub struct AliasApp {
    pool: Pool,
}

impl AliasRepo for AliasApp {
    fn get_pool(&self) -> &Pool {
        &self.pool
    }   
}

#[allow(clippy::unwrap_used)]
#[cfg(test)]
mod tests {

    use super::*;

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_basic_type_aliases(pool: Pool) {
        let app = AliasApp { pool };

        let user_id = UserId(1);
        let user = app.find_by_typed_id(user_id.0).await.unwrap();

        assert_eq!(user.id, UserId(1));
        assert_eq!(user.name, UserName("Alice".into()));
        assert_eq!(user.email, "alice@example.com");
        assert_eq!(user.age, 30);
        assert_eq!(user.birth_year, Some(1993));
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_find_by_typed_name(pool: Pool) {
        let app = AliasApp { pool };

        let user_name = UserName("Alice".into());
        let user = app.find_by_typed_name(user_name.0).await.unwrap();

        assert!(user.is_some());
        let user = user.unwrap();
        assert_eq!(user.id, UserId(1));
        assert_eq!(user.name, UserName("Alice".into()));

        // Test non-existent user
        let non_existent = UserName("NonExistent".into());
        let result = app.find_by_typed_name(non_existent.0).await.unwrap();
        assert!(result.is_none());
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_multiple_typed_parameters(pool: Pool) {
        let app = AliasApp { pool };

        let user_id = UserId(1);
        let user_name = UserName("Alice".into());
        let result = app
            .find_id_name_by_typed_params(user_id.0, user_name.0)
            .await
            .unwrap();

        assert!(result.is_some());
        let (id, name) = result.unwrap();
        assert_eq!(id, UserId(1));
        assert_eq!(name, UserName("Alice".into()));

        // Test mismatched parameters
        let wrong_name = UserName("Bob".into());
        let no_match = app
            .find_id_name_by_typed_params(user_id.0, wrong_name.0)
            .await
            .unwrap();
        assert!(no_match.is_none());
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_aggregation_with_type_aliases(pool: Pool) {
        let app = AliasApp { pool };

        let user_id = UserId(5);
        let count = app.count_users_after_typed_id(user_id.0).await.unwrap();

        // Should count users with id > 5 (users 6-20 = 15 users)
        assert_eq!(count, Some(15));

        let user_id_high = UserId(100);
        let count_high = app.count_users_after_typed_id(user_id_high.0).await.unwrap();
        assert_eq!(count_high, Some(0));
    }

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_postgresql_uuid_generation(pool: Pool) {
        let app = AliasApp { pool };

        let uuid1 = app.generate_uuid().await.unwrap().unwrap();
        let uuid2 = app.generate_uuid().await.unwrap().unwrap();

        // UUIDs should be different
        assert_ne!(uuid1, uuid2);

        // Should be valid UUID format
        assert_eq!(uuid1.to_string().len(), 36);
        assert!(uuid1.to_string().contains('-'));
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_type_coercion_to_strings(pool: Pool) {
        let app = AliasApp { pool };

        let user_id = UserId(1);
        let (id_str, name_str) = app.get_user_as_strings(user_id.0).await.unwrap();

        assert_eq!(id_str, "1");
        assert_eq!(name_str, "Alice");
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_age_category_computation(pool: Pool) {
        let app = AliasApp { pool };

        // Test different age categories (based on fixture data)
        // User 8: Henry, age 19 -> "young"
        let category_young = app.get_age_category(8).await.unwrap();
        assert_eq!(category_young, "young");

        // User 1: Alice, age 30 -> "middle"
        let category_middle = app.get_age_category(1).await.unwrap();
        assert_eq!(category_middle, "middle");

        // User 5: Eve, age 42 -> "senior"
        let category_senior = app.get_age_category(5).await.unwrap();
        assert_eq!(category_senior, "senior");
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_multiple_return_types(pool: Pool) {
        let app = AliasApp { pool };

        let users = app.get_all_typed_users().await.unwrap();
        assert_eq!(users.len(), 20);

        // Verify first user (Alice)
        assert_eq!(users[0].0, UserId(1));
        assert_eq!(users[0].1, UserName("Alice".into()));
        assert_eq!(users[0].2, 30);

        // Verify ordering by id
        for i in 1..users.len() {
            assert!(users[i - 1].0.0 <= users[i].0.0);
        }
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_nullable_type_aliases(pool: Pool) {
        let app = AliasApp { pool };

        // User with birth_year
        let user_id_with_birth = UserId(1); // Alice has birth_year 1993
        let birth_year = app
            .get_optional_birth_year(user_id_with_birth.0)
            .await
            .unwrap();
        assert_eq!(birth_year, Some(1993));

        // User without birth_year
        let user_id_without_birth = UserId(3); // Charlie has NULL birth_year
        let no_birth_year = app
            .get_optional_birth_year(user_id_without_birth.0)
            .await
            .unwrap();
        assert_eq!(no_birth_year, None);
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_computed_values_with_aliases(pool: Pool) {
        let app = AliasApp { pool };

        let user_id = UserId(5);
        let (doubled_id, upper_name) = app.get_computed_values(user_id.0).await.unwrap();

        assert_eq!(doubled_id, 10); // 5 * 2
        assert!(
            upper_name
                .chars()
                .all(|c| c.is_uppercase() || !c.is_alphabetic())
        );
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_json_type_alias(pool: Pool) {
        let app = AliasApp { pool };

        let user_id = UserId(1);
        let user_json = app.get_user_as_json(user_id.0).await.unwrap();

        let json_str = user_json.to_string();
        assert!(json_str.contains("\"id\":1"));
        assert!(json_str.contains("\"name\":\"Alice\""));

        // Parse to verify it's valid JSON
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        assert_eq!(parsed["id"].as_i64().unwrap(), 1);
        assert_eq!(parsed["name"].as_str().unwrap(), "Alice");
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("users"))
    )]
    async fn test_window_functions_with_aliases(pool: Pool) {
        let app = AliasApp { pool };

        let users_with_rows = app.get_users_with_row_numbers().await.unwrap();
        assert_eq!(users_with_rows.len(), 20);

        // Verify row numbers are sequential
        for (i, (_, _, row_num)) in users_with_rows.iter().enumerate() {
            assert_eq!(*row_num, (i + 1) as i64);
        }

        // Should be ordered by age (due to ROW_NUMBER() OVER (ORDER BY age))
        for i in 1..users_with_rows.len() {
            // Row numbers should be sequential
            assert_eq!(users_with_rows[i].2, users_with_rows[i - 1].2 + 1);
        }
    }

    #[cfg(feature = "chrono")]
    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_postgresql_interval_types(pool: Pool) {
        let app = AliasApp { pool };

        let yesterday = app.get_yesterday().await.unwrap();
        let now = sqlx::types::chrono::Utc::now();

        // Should be approximately 24 hours ago
        let diff = now.signed_duration_since(yesterday);
        assert!(diff.num_hours() >= 23 && diff.num_hours() <= 25);
    }

    // Note: test_subquery_with_type_aliases is commented out because
    // get_above_average_users() doesn't work yet with SQLx 0.8
    // See comment in AliasRepo trait
}
