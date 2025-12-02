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
    pub profile_json: String,
    pub preferences: Option<String>,
}

//TODO use as exemple of things should be Optional
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

// New struct that exactly matches the json_users table structure
#[derive(Debug, Clone, PartialEq)]
pub struct JsonUsersRow {
    pub id: UserId,
    pub name: String,
    pub profile_json: sqlx::types::Json<sqlx::types::JsonValue>, // Use JsonValue for JSON columns
    pub preferences: Option<sqlx::types::Json<sqlx::types::JsonValue>>, // Optional JSON column
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
    // Basic JSON operations
    #[dml("SELECT id, name, profile_json, preferences FROM json_users WHERE id = $1")]
    async fn find_user_by_id(&self, id: i64) -> Result<User>;

    #[dml("SELECT id, name, json_extract(profile_json, '$.email') as email FROM json_users")]
    async fn get_users_with_email(
        &self,
        params: impl IntoParams,
    ) -> Result<Serial<(UserId, String, Option<String>)>>;

    // JSON extraction and filtering
    #[dml("SELECT * FROM json_users WHERE json_extract(profile_json, '$.age') > $1")]
    async fn find_users_older_than(
        &self,
        age: i64,
        params: impl IntoParams,
    ) -> Result<Serial<User>>;

    #[dml("SELECT * FROM json_users WHERE json_extract(profile_json, '$.department') = $1")]
    async fn find_users_by_department(&self, department: String) -> Result<Vec<User>>;

    // JSON modification
    #[dml(
        "UPDATE json_users SET profile_json = json_set(profile_json, '$.lastLogin', $2) WHERE id = $1"
    )]
    async fn update_last_login(&self, id: i64, timestamp: String) -> Result<QueryResult>;

    #[dml(
        "UPDATE json_users SET preferences = json_set(COALESCE(preferences, '{}'), $2, $3) WHERE id = $1"
    )]
    async fn set_user_preference(&self, id: i64, key: String, value: String)
    -> Result<QueryResult>;

    // JSON aggregation
    #[dml(
        "SELECT json_group_array(json_object('id', id, 'name', name, 'email', json_extract(profile_json, '$.email'))) as 'users_json!: String' FROM json_users"
    )]
    async fn get_users_as_json_array(&self) -> Result<String>;

    // Complex JSON queries - DISABLED: SQLx doesn't support static analysis of json_each
    //#[dml("SELECT key as 'field_name!: String', value as 'field_value!: String' FROM json_users, json_each(profile_json) WHERE json_users.id = $1")]
    //async fn get_user_profile_fields(&self, id: i64) -> Result<Vec<JsonField>>;

    //TODO this one is not working properly yet - need to use unchecked macro
    //#[dml("SELECT json_users.id, key as preference_key, value as preference_value FROM json_users, json_each(preferences) WHERE json_users.id = $1 AND preferences IS NOT NULL")]
    //async fn get_user_preferences(&self, id: i64) -> Result<Vec<UserPreference>>;

    // JSON validation and type checking
    #[dml("SELECT * FROM json_users WHERE json_valid(profile_json) = 1")]
    async fn find_users_with_valid_json(&self) -> Result<Vec<User>>;

    #[dml("SELECT * FROM json_users WHERE json_type(profile_json, '$.age') = 'integer'")]
    async fn find_users_with_numeric_age(&self) -> Result<Vec<User>>;

    // JSON array operations
    #[dml(
        "SELECT * FROM json_users WHERE json_array_length(json_extract(profile_json, '$.skills')) > $1"
    )]
    async fn find_users_with_many_skills(&self, min_skills: i64) -> Result<Vec<User>>;

    //TODO this one is not working yet - need to use FilterBuilder
    // Query with vector parameter using IN clause
    //#[dml("SELECT * FROM json_users WHERE json_extract(profile_json, '$.department') IN ($1)")]
    //async fn find_users_by_departments(&self, departments: Vec<String>) -> Result<Vec<User>>;

    #[dml("INSERT INTO json_users (name, profile_json, preferences) VALUES ($1, $2, $3)")]
    async fn create_user(
        &self,
        name: String,
        profile_json: String,
        preferences: Option<String>,
    ) -> Result<QueryResult>;

    #[dml(
        "INSERT INTO json_users (name, profile_json, preferences) VALUES ($1, $2, $3) RETURNING id"
    )]
    async fn create_user_returning_id(
        &self,
        name: String,
        profile_json: String,
        preferences: Option<String>,
    ) -> Result<i64>;

    // Connection/Transaction variations
    #[dml("SELECT * FROM json_users WHERE json_extract(profile_json, '$.active') = true")]
    async fn find_active_users_with_conn(&self, conn: &mut Connection) -> Result<Vec<User>>;

    #[dml("DELETE FROM json_users WHERE json_extract(profile_json, '$.toDelete') = true")]
    async fn delete_marked_users_with_tx(&self, tx: &mut Transaction<'_>) -> Result<QueryResult>;

    // Advanced JSON methods with Serde support
    #[dml("INSERT INTO json_users (name, profile_json) VALUES ($1, $2)")]
    async fn insert_json_value(
        &self,
        name: String,
        profile_json: sqlx::types::JsonValue,
    ) -> Result<QueryResult>;

    #[dml("SELECT profile_json FROM json_users WHERE id = $1")]
    async fn get_profile_json(&self, id: i64) -> Result<String>;

    #[dml("UPDATE json_users SET profile_json = json_set(profile_json, '$.age', $2) WHERE id = $1")]
    async fn update_profile_age(&self, id: i64, age: u32) -> Result<QueryResult>;

    //TODO good example of !: operator usage with Json<T>
    #[dml(
        "SELECT id, name, profile_json as 'profile_json!: sqlx::types::Json<sqlx::types::JsonValue>', preferences as 'preferences: sqlx::types::Json<sqlx::types::JsonValue>' FROM json_users WHERE id = $1"
    )]
    async fn get_json_user_row(&self, id: i64) -> Result<JsonUsersRow>;

    #[dml(
        "SELECT id, name, profile_json as 'profile_json: sqlx::types::Json<sqlx::types::JsonValue>', preferences as 'preferences: sqlx::types::Json<sqlx::types::JsonValue>' FROM json_users"
    )]
    async fn get_all_json_user_rows(&self) -> Result<Vec<JsonUsersRow>>;
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
    use serde_json::Value;

    use super::*;

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("json_users"))
    )]
    async fn test_basic_json_extraction(pool: Pool) {
        let repo = JsonUserApp { pool };

        let user = repo.find_user_by_id(1).await.unwrap();
        assert_eq!(user.name, "Alice Johnson");
        assert!(user.profile_json.contains("alice@example.com"));
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
    async fn test_json_modification(pool: Pool) {
        let repo = JsonUserApp { pool };

        let timestamp = "2024-01-15T10:30:00Z";
        let result = repo
            .update_last_login(1, timestamp.to_string())
            .await
            .unwrap();
        assert_eq!(result.rows_affected(), 1);

        let user = repo.find_user_by_id(1).await.unwrap();
        assert!(user.profile_json.contains("lastLogin"));
        assert!(user.profile_json.contains(timestamp));
    }

    // #[tokio::test]
    // async fn test_user_preferences() {
    //     let pool = setup_json_test_db().await;
    //     let repo = JsonUserApp { pool };

    //     // Set a new preference
    //     repo.set_user_preference(1, "fontSize".to_string(), "14".to_string())
    //         .await
    //         .unwrap();

    //     // Get user preferences
    //     let preferences = repo.get_user_preferences(1).await.unwrap();
    //     assert!(preferences.len() >= 3); // theme, notifications, language + fontSize

    //     let font_size_pref = preferences
    //         .iter()
    //         .find(|p| p.preference_key == "fontSize");
    //     assert!(font_size_pref.is_some());
    //     assert_eq!(font_size_pref.unwrap().preference_value, "14");
    // }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("json_users"))
    )]
    async fn test_json_aggregation(pool: Pool) {
        let repo = JsonUserApp { pool };

        let json_array = repo.get_users_as_json_array().await.unwrap();
        assert!(json_array.starts_with('['));
        assert!(json_array.ends_with(']'));
        assert!(json_array.contains("Alice Johnson"));
        assert!(json_array.contains("alice@example.com"));
    }

    //#[tokio::test]
    //async fn test_json_each_profile_fields() {
    //    let pool = setup_json_test_db().await;
    //    let repo = JsonUserApp { pool };
    //
    //    let fields = repo.get_user_profile_fields(1).await.unwrap();
    //    assert!(fields.len() >= 5); // email, age, department, skills, active
    //
    //    let email_field = fields.iter().find(|f| f.field_name == "email");
    //    assert!(email_field.is_some());
    //    assert_eq!(email_field.unwrap().field_value, "alice@example.com");
    //}

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("json_users"))
    )]
    async fn test_json_validation(pool: Pool) {
        let repo = JsonUserApp { pool };

        let valid_users = repo.find_users_with_valid_json().await.unwrap();
        assert_eq!(valid_users.len(), 4); // All test users have valid JSON

        let numeric_age_users = repo.find_users_with_numeric_age().await.unwrap();
        assert_eq!(numeric_age_users.len(), 4); // All have numeric age
    }

    //#[tokio::test]
    //async fn test_json_null_fields() {
    //    let pool = setup_json_test_db().await;
    //    let repo = JsonUserApp { pool };
    //
    //    // Test getting profile fields for user 3 (Carol) who has null preferences
    //    let fields = repo.get_user_profile_fields(3).await.unwrap();
    //    assert!(fields.len() >= 5); // Carol has profile fields like email, age, department, etc.
    //
    //    // Check that we can extract specific fields even when some might be null
    //    let email_field = fields.iter().find(|f| f.field_name == "email");
    //    assert!(email_field.is_some());
    //    assert_eq!(email_field.unwrap().field_value, "carol@example.com");
    //
    //    // Test that toDelete field exists (Carol has this field)
    //    let to_delete_field = fields.iter().find(|f| f.field_name == "toDelete");
    //    assert!(to_delete_field.is_some());
    //}

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("json_users"))
    )]
    async fn test_json_array_operations(pool: Pool) {
        let repo = JsonUserApp { pool };

        let skilled_users = repo.find_users_with_many_skills(2).await.unwrap();
        // Alice has 3 skills, Carol has 3 skills
        assert_eq!(skilled_users.len(), 2);
        assert!(skilled_users.iter().any(|u| u.name == "Alice Johnson"));
        assert!(skilled_users.iter().any(|u| u.name == "Carol Davis"));
    }

    // #[tokio::test]
    // async fn test_vector_parameter_in_clause() {
    //     let pool = setup_json_test_db().await;
    //     let repo = JsonUserApp { pool };

    //     // Test finding users by multiple departments using Vec<String> parameter
    //     let departments = vec!["Engineering".to_string(), "Sales".to_string()];
    //     let users = repo.find_users_by_departments(departments).await.unwrap();

    //     // Should find Alice, Carol (Engineering) and David (Sales)
    //     assert_eq!(users.len(), 3);
    //     assert!(users.iter().any(|u| u.name == "Alice Johnson"));
    //     assert!(users.iter().any(|u| u.name == "Carol Davis"));
    //     assert!(users.iter().any(|u| u.name == "David Wilson"));

    //     // Bob (Marketing) should not be included
    //     assert!(!users.iter().any(|u| u.name == "Bob Smith"));
    // }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("json_users"))
    )]
    async fn test_create_user_with_json(pool: Pool) {
        let repo = JsonUserApp { pool };

        let new_profile = r#"{"email": "eve@example.com", "age": 26, "department": "Design", "skills": ["UI/UX", "Figma"], "active": true}"#;
        let new_preferences = Some(r#"{"theme": "system", "notifications": true}"#.to_string());

        let result = repo
            .create_user(
                "Eve Taylor".to_string(),
                new_profile.to_string(),
                new_preferences,
            )
            .await
            .unwrap();

        assert_eq!(result.rows_affected(), 1);

        // Verify user was created
        let user = repo.find_user_by_id(5).await.unwrap();
        assert_eq!(user.name, "Eve Taylor");
        assert!(user.profile_json.contains("eve@example.com"));
        assert!(user.preferences.is_some());
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("json_users"))
    )]
    async fn test_json_with_connection(pool: Pool) {
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
    async fn test_json_with_transaction(pool: Pool) {
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
    async fn test_pagination_with_json_filters(pool: Pool) {
        let repo = JsonUserApp { pool };

        // Test pagination with JSON filtering
        let params = SerialParams::new(1, 2); // First page, 2 items
        let result = repo.find_users_older_than(25, params).await.unwrap();

        assert_eq!(result.data.len(), 2); // Should get first 2 users
        assert_eq!(result.total_items, 3); // Total 3 users older than 25
        assert_eq!(result.page, 1);
        assert_eq!(result.size, 2);
        assert!(result.total_pages > 1);
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("json_users"))
    )]
    async fn test_serde_json_integration(pool: Pool) {
        let repo = JsonUserApp { pool };

        // Method 1: Using serde_json::Value serialized to string
        let json_data: Value = serde_json::json!({
            "age": 30,
            "city": "São Paulo",
            "preferences": {
                "theme": "dark",
                "notifications": true
            }
        });

        let json_string_maria = serde_json::to_string(&json_data).unwrap();
        let maria_id = repo
            .create_user_returning_id("Maria".to_string(), json_string_maria, None)
            .await
            .unwrap();

        // Verify Maria's data
        let maria_profile_str = repo.get_profile_json(maria_id).await.unwrap();
        let maria_profile: Profile = serde_json::from_str(&maria_profile_str).unwrap();
        assert_eq!(maria_profile.age, 30);
        assert_eq!(maria_profile.city, "São Paulo");

        // Method 2: Using custom types with Serialize/Deserialize
        let profile = Profile {
            age: 25,
            city: "Rio".to_string(),
            preferences: Preferences {
                theme: "light".to_string(),
                notifications: false,
            },
        };

        // Serialize to JSON string
        let json_string = serde_json::to_string(&profile).unwrap();
        let carlos_id = repo
            .create_user_returning_id("Carlos".to_string(), json_string, None)
            .await
            .unwrap();

        // Method 3: Query and deserialize JSON
        let profile_str = repo.get_profile_json(carlos_id).await.unwrap(); // Carlos using returned ID
        let parsed_profile: Profile = serde_json::from_str(&profile_str).unwrap();

        assert_eq!(parsed_profile.age, 25);
        assert_eq!(parsed_profile.city, "Rio");
        assert_eq!(parsed_profile.preferences.theme, "light");
        assert!(!parsed_profile.preferences.notifications);

        // Method 4: Update JSON field using JSON1 functions
        let update_result = repo.update_profile_age(carlos_id, 35).await.unwrap();
        assert_eq!(update_result.rows_affected(), 1);

        // Verify the update worked
        let updated_profile_str = repo.get_profile_json(carlos_id).await.unwrap();
        let updated_profile: Profile = serde_json::from_str(&updated_profile_str).unwrap();
        assert_eq!(updated_profile.age, 35);
        assert_eq!(updated_profile.city, "Rio"); // Should remain unchanged
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("json_users"))
    )]
    async fn test_select_star_with_json_values(pool: Pool) {
        let repo = JsonUserApp { pool };

        // Test SELECT * with exact table structure mapping
        let json_row = repo.get_json_user_row(1).await.unwrap(); // Alice

        // Verify the structure fields
        assert_eq!(json_row.id.0, 1);
        assert_eq!(json_row.name, "Alice Johnson");

        // Access JSON fields as JsonValue
        println!("Profile JSON: {:?}", json_row.profile_json);
        println!("Preferences JSON: {:?}", json_row.preferences);

        // Verify JSON content (profile_json should contain Alice's data)
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

        // Carol should have JSON profile data
        let carol_profile = carol.profile_json.to_string();
        assert!(carol_profile.contains("carol@example.com"));
        assert!(carol_profile.contains("toDelete"));
    }
}
