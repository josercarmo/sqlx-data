#![cfg(all(feature = "json"))]
use serde::{Deserialize, Serialize};
use sqlx_data::filters::{CursorSecureExtract, CursorValue, FilterValue};
use sqlx_data::pagination::Serial;
use sqlx_data::params::{IntoParams, SerialParams};
use sqlx_data::{Connection, CursorData, Pool, QueryResult, Result, Transaction};
use sqlx_data::{dml, repo};

#[derive(Debug, Serialize, Deserialize)]
struct Profile {
    age: u32,
    city: String,
    preferences: Preferences,
}

#[derive(Debug, Serialize, Deserialize)]
struct Preferences {
    theme: String,
    notifications: bool,
}

#[derive(Clone, PartialEq, Eq, Debug, sqlx::Type)]
#[sqlx(transparent)]
pub struct UserId(i64);

impl From<i64> for UserId {
    fn from(value: i64) -> Self {
        UserId(value)
    }
}

#[derive(Debug, Clone, PartialEq, sqlx::FromRow)]
pub struct User {
    pub id: UserId,
    pub name: String,
    pub profile_json: sqlx::types::Json<sqlx::types::JsonValue>,
    pub preferences: Option<sqlx::types::Json<sqlx::types::JsonValue>>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub struct JsonField {
    pub field_name: String,
    pub field_value: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub struct UserPreference {
    pub user_id: UserId,
    pub preference_key: String,
    pub preference_value: sqlx::types::JsonValue,
}

// New struct that exactly matches the json_users table structure with PostgreSQL JSONB
#[derive(Debug, Clone, PartialEq)]
pub struct JsonUsersRow {
    pub id: UserId,
    pub name: String,
    pub profile_json: sqlx::types::Json<sqlx::types::JsonValue>, // Use PostgreSQL JSONB
    pub preferences: Option<sqlx::types::Json<sqlx::types::JsonValue>>, // Optional JSONB column
}

impl CursorSecureExtract for User {
    fn extract_whitelisted_fields(&self, fields: &[String]) -> Result<Vec<CursorValue>> {
        let mut values = Vec::with_capacity(fields.len());
        for field in fields {
            match field.as_str() {
                "id" => values.push(self.id.0.into()),
                "name" => values.push(self.name.clone().into()),
                _ => {
                    return Err(sqlx::Error::Decode(
                        format!("Field '{}' not allowed for cursor pagination", field).into(),
                    ));
                }
            }
        }
        Ok(values)
    }

    fn encode(cursor: &CursorData) -> Result<String> {
        use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD as BASE64};
        let json_bytes = serde_json::to_vec(&cursor)
            .map_err(|e| sqlx::Error::Decode(format!("JSON serialization failed: {}", e).into()))?;
        Ok(BASE64.encode(json_bytes))
    }

    fn decode(encoded: &str) -> Result<Vec<FilterValue>> {
        use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD as BASE64};
        let bytes = BASE64
            .decode(encoded)
            .map_err(|e| sqlx::Error::Decode(format!("Base64 decode failed: {}", e).into()))?;

        let cursor: CursorData = serde_json::from_slice(&bytes).map_err(|e| {
            sqlx::Error::Decode(format!("JSON deserialization failed: {}", e).into())
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
trait JsonUserRepo {
    // Basic JSON operations with PostgreSQL JSONB
    #[dml("SELECT id, name, profile_json, preferences FROM json_users WHERE id = $1")]
    async fn find_user_by_id(&self, id: i64) -> Result<User>;

    #[dml("SELECT id, name, profile_json->>'email' as email FROM json_users")]
    async fn get_users_with_email(
        &self,
        params: impl IntoParams,
    ) -> Result<Serial<(UserId, String, Option<String>)>>;

    // JSON extraction and filtering with PostgreSQL operators
    #[dml("SELECT * FROM json_users WHERE (profile_json->>'age')::INTEGER > $1")]
    async fn find_users_older_than(
        &self,
        age: i64,
        params: impl IntoParams,
    ) -> Result<Serial<User>>;

    #[dml("SELECT * FROM json_users WHERE profile_json->>'department' = $1")]
    async fn find_users_by_department(&self, department: String) -> Result<Vec<User>>;

    // JSON modification with PostgreSQL jsonb_set
    #[dml(
        "UPDATE json_users SET profile_json = jsonb_set(profile_json, '{lastLogin}', $2::jsonb) WHERE id = $1"
    )]
    async fn update_last_login(&self, id: i64, timestamp: String) -> Result<QueryResult>;

    #[dml(
        "UPDATE json_users SET preferences = COALESCE(preferences, '{}'::jsonb) || jsonb_build_object($2, $3) WHERE id = $1"
    )]
    async fn set_user_preference(&self, id: i64, key: String, value: String)
    -> Result<QueryResult>;

    // JSON aggregation with PostgreSQL json_agg
    #[dml(
        "SELECT json_agg(json_build_object('id', id, 'name', name, 'email', profile_json->>'email'))::TEXT as \"users_json!: String\" FROM json_users"
    )]
    async fn get_users_as_json_array(&self) -> Result<String>;

    // PostgreSQL JSON validation
    #[dml("SELECT * FROM json_users WHERE jsonb_typeof(profile_json) = 'object'")]
    async fn find_users_with_valid_json(&self) -> Result<Vec<User>>;

    #[dml("SELECT * FROM json_users WHERE jsonb_typeof(profile_json->'age') = 'number'")]
    async fn find_users_with_numeric_age(&self) -> Result<Vec<User>>;

    // JSON array operations with PostgreSQL operators
    #[dml(
        "SELECT * FROM json_users WHERE jsonb_array_length(profile_json->'skills') > $1"
    )]
    async fn find_users_with_many_skills(&self, min_skills: i64) -> Result<Vec<User>>;

    // PostgreSQL JSONB contains operator
    #[dml("SELECT * FROM json_users WHERE profile_json->'skills' ? $1")]
    async fn find_users_with_skill(&self, skill: String) -> Result<Vec<User>>;

    #[dml("INSERT INTO json_users (name, profile_json, preferences) VALUES ($1, $2, $3)")]
    async fn create_user(
        &self,
        name: String,
        profile_json: sqlx::types::Json<sqlx::types::JsonValue>,
        preferences: Option<sqlx::types::Json<sqlx::types::JsonValue>>,
    ) -> Result<QueryResult>;

    #[dml(
        "INSERT INTO json_users (name, profile_json, preferences) VALUES ($1, $2, $3) RETURNING id"
    )]
    async fn create_user_returning_id(
        &self,
        name: String,
        profile_json: sqlx::types::Json<sqlx::types::JsonValue>,
        preferences: Option<sqlx::types::Json<sqlx::types::JsonValue>>,
    ) -> Result<i64>;

    // Connection/Transaction variations
    #[dml("SELECT * FROM json_users WHERE (profile_json->>'active')::BOOLEAN = true")]
    async fn find_active_users_with_conn(&self, conn: &mut Connection) -> Result<Vec<User>>;

    #[dml("DELETE FROM json_users WHERE (profile_json->>'toDelete')::BOOLEAN = true")]
    async fn delete_marked_users_with_tx(&self, tx: &mut Transaction<'_>) -> Result<QueryResult>;

    // Advanced JSON methods with PostgreSQL JSONB support
    #[dml("SELECT profile_json::TEXT FROM json_users WHERE id = $1")]
    async fn get_profile_json(&self, id: i64) -> Result<String>;

    #[dml("UPDATE json_users SET profile_json = jsonb_set(profile_json, '{age}', $2::jsonb) WHERE id = $1")]
    async fn update_profile_age(&self, id: i64, age: u32) -> Result<QueryResult>;

    // PostgreSQL JSONB type queries
    #[dml(
        "SELECT id, name, profile_json as \"profile_json!: sqlx::types::Json<sqlx::types::JsonValue>\", preferences as \"preferences: sqlx::types::Json<sqlx::types::JsonValue>\" FROM json_users WHERE id = $1"
    )]
    async fn get_json_user_row(&self, id: i64) -> Result<JsonUsersRow>;

    #[dml(
        "SELECT id, name, profile_json as \"profile_json: sqlx::types::Json<sqlx::types::JsonValue>\", preferences as \"preferences: sqlx::types::Json<sqlx::types::JsonValue>\" FROM json_users ORDER BY id"
    )]
    async fn get_all_json_user_rows(&self) -> Result<Vec<JsonUsersRow>>;

    // PostgreSQL-specific JSON path operations
    #[dml("SELECT * FROM json_users WHERE profile_json #> '{preferences,theme}' = $1::jsonb")]
    async fn find_users_by_theme(&self, theme: String) -> Result<Vec<User>>;

    // PostgreSQL JSONB containment operator
    #[dml("SELECT COUNT(*) FROM json_users WHERE profile_json @> $1::jsonb")]
    async fn count_users_with_profile_subset(&self, subset: sqlx::types::Json<sqlx::types::JsonValue>) -> Result<i64>;
}

pub struct JsonUserApp {
    pool: Pool,
}

impl JsonUserRepo for JsonUserApp {
    fn get_pool(&self) -> &Pool {
        &self.pool
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{json, Value};

    use super::*;

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("json_users"))
    )]
    async fn test_basic_json_extraction(pool: Pool) {
        let repo = JsonUserApp { pool };

        let user = repo.find_user_by_id(1).await.unwrap();
        assert_eq!(user.name, "Alice Johnson");
        let profile_str = user.profile_json.to_string();
        assert!(profile_str.contains("alice@example.com"));
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("json_users"))
    )]
    async fn test_json_extract_email(pool: Pool) {
        let repo = JsonUserApp { pool };

        let params = SerialParams::new(1, 10);
        let result = repo.get_users_with_email(params).await.unwrap();

        assert_eq!(result.data.len(), 4);
        let alice = &result.data[0];
        assert_eq!(alice.1, "Alice Johnson");
        assert_eq!(alice.2, Some("alice@example.com".to_string()));
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("json_users"))
    )]
    async fn test_json_filtering_by_age(pool: Pool) {
        let repo = JsonUserApp { pool };

        let params = SerialParams::new(1, 10);
        let result = repo.find_users_older_than(27, params).await.unwrap();

        // Should find Alice (30), Carol (35), and David (28)
        assert_eq!(result.data.len(), 3);
        assert_eq!(result.total_items, 3);
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("json_users"))
    )]
    async fn test_json_filtering_by_department(pool: Pool) {
        let repo = JsonUserApp { pool };

        let engineering_users = repo
            .find_users_by_department("Engineering".to_string())
            .await
            .unwrap();

        assert_eq!(engineering_users.len(), 2); // Alice and Carol
        assert!(engineering_users.iter().any(|u| u.name == "Alice Johnson"));
        assert!(engineering_users.iter().any(|u| u.name == "Carol Davis"));
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("json_users"))
    )]
    async fn test_postgresql_jsonb_modification(pool: Pool) {
        let repo = JsonUserApp { pool };

        let timestamp = "2024-01-15T10:30:00Z";
        let result = repo
            .update_last_login(1, format!("\"{}\"", timestamp))
            .await
            .unwrap();
        assert_eq!(result.rows_affected(), 1);

        let user = repo.find_user_by_id(1).await.unwrap();
        let profile_str = user.profile_json.to_string();
        assert!(profile_str.contains("lastLogin"));
        assert!(profile_str.contains(timestamp));
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("json_users"))
    )]
    async fn test_postgresql_jsonb_aggregation(pool: Pool) {
        let repo = JsonUserApp { pool };

        let json_array = repo.get_users_as_json_array().await.unwrap();
        assert!(json_array.starts_with('['));
        assert!(json_array.ends_with(']'));
        assert!(json_array.contains("Alice Johnson"));
        assert!(json_array.contains("alice@example.com"));
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("json_users"))
    )]
    async fn test_postgresql_json_validation(pool: Pool) {
        let repo = JsonUserApp { pool };

        let valid_users = repo.find_users_with_valid_json().await.unwrap();
        assert_eq!(valid_users.len(), 4); // All test users have valid JSONB

        let numeric_age_users = repo.find_users_with_numeric_age().await.unwrap();
        assert_eq!(numeric_age_users.len(), 4); // All have numeric age
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("json_users"))
    )]
    async fn test_postgresql_jsonb_array_operations(pool: Pool) {
        let repo = JsonUserApp { pool };

        let skilled_users = repo.find_users_with_many_skills(2).await.unwrap();
        // Alice has 3 skills, Carol has 3 skills
        assert_eq!(skilled_users.len(), 2);
        assert!(skilled_users.iter().any(|u| u.name == "Alice Johnson"));
        assert!(skilled_users.iter().any(|u| u.name == "Carol Davis"));

        // Test JSONB ? operator for skill existence
        let rust_users = repo.find_users_with_skill("Rust".to_string()).await.unwrap();
        assert!(rust_users.len() >= 1);
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("json_users"))
    )]
    async fn test_create_user_with_jsonb(pool: Pool) {
        let repo = JsonUserApp { pool };

        let new_profile = json!({
            "email": "eve@example.com",
            "age": 26,
            "department": "Design",
            "skills": ["UI/UX", "Figma"],
            "active": true
        });
        let new_preferences = Some(sqlx::types::Json(json!({
            "theme": "system",
            "notifications": true
        })));

        let result = repo
            .create_user(
                "Eve Taylor".to_string(),
                sqlx::types::Json(new_profile),
                new_preferences,
            )
            .await
            .unwrap();

        assert_eq!(result.rows_affected(), 1);

        // Verify user was created
        let user = repo.find_user_by_id(5).await.unwrap();
        assert_eq!(user.name, "Eve Taylor");
        let profile_str = user.profile_json.to_string();
        assert!(profile_str.contains("eve@example.com"));
        assert!(user.preferences.is_some());
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("json_users"))
    )]
    async fn test_postgresql_jsonb_with_connection(pool: Pool) {
        let repo = JsonUserApp { pool: pool.clone() };

        let mut conn = pool.acquire().await.unwrap();
        let active_users = repo.find_active_users_with_conn(&mut conn).await.unwrap();

        // Alice, Bob, and David are active (Carol is not)
        assert_eq!(active_users.len(), 3);
        assert!(active_users.iter().any(|u| u.name == "Alice Johnson"));
        assert!(active_users.iter().any(|u| u.name == "Bob Smith"));
        assert!(active_users.iter().any(|u| u.name == "David Wilson"));
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("json_users"))
    )]
    async fn test_postgresql_jsonb_with_transaction(pool: Pool) {
        let repo = JsonUserApp { pool: pool.clone() };

        let mut tx = pool.begin().await.unwrap();

        // This should delete Carol (she has toDelete: true)
        let result = repo.delete_marked_users_with_tx(&mut tx).await.unwrap();
        assert_eq!(result.rows_affected(), 1);

        tx.commit().await.unwrap();

        // Verify Carol was deleted
        let all_params = SerialParams::new(1, 10);
        let remaining = repo.get_users_with_email(all_params).await.unwrap();
        assert_eq!(remaining.data.len(), 3); // Alice, Bob, David remain
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("json_users"))
    )]
    async fn test_postgresql_specific_operators(pool: Pool) {
        let repo = JsonUserApp { pool };

        // Test JSONB containment operator @>
        let engineering_subset = sqlx::types::Json(json!({"department": "Engineering"}));
        let count = repo.count_users_with_profile_subset(engineering_subset).await.unwrap();
        assert_eq!(count, 2); // Alice and Carol

        // Test JSON path operator #>
        let dark_theme_users = repo.find_users_by_theme("\"dark\"".to_string()).await.unwrap();
        assert!(dark_theme_users.len() >= 1);
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("json_users"))
    )]
    async fn test_select_star_with_jsonb_values(pool: Pool) {
        let repo = JsonUserApp { pool };

        // Test SELECT * with exact table structure mapping
        let json_row = repo.get_json_user_row(1).await.unwrap(); // Alice

        // Verify the structure fields
        assert_eq!(json_row.id.0, 1);
        assert_eq!(json_row.name, "Alice Johnson");

        // Access JSONB fields as JsonValue
        println!("Profile JSONB: {:?}", json_row.profile_json);
        println!("Preferences JSONB: {:?}", json_row.preferences);

        // Verify JSONB content (profile_json should contain Alice's data)
        let profile_str = json_row.profile_json.to_string();
        assert!(profile_str.contains("alice@example.com"));
        assert!(profile_str.contains("Engineering"));

        // Verify preferences (Alice has preferences)
        assert!(json_row.preferences.is_some());
        let prefs_str = json_row.preferences.unwrap().to_string();
        assert!(prefs_str.contains("dark"));
        assert!(prefs_str.contains("notifications"));

        // Test SELECT * returning multiple rows
        let all_rows = repo.get_all_json_user_rows().await.unwrap();
        assert_eq!(all_rows.len(), 4); // Alice, Bob, Carol, David

        // Find Carol (who has null preferences)
        let carol = all_rows
            .iter()
            .find(|row| row.name == "Carol Davis")
            .unwrap();
        assert_eq!(carol.id.0, 3);
        assert!(carol.preferences.is_none());

        // Carol should have JSONB profile data
        let carol_profile = carol.profile_json.to_string();
        assert!(carol_profile.contains("carol@example.com"));
        assert!(carol_profile.contains("toDelete"));
    }
}