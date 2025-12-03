use serde::{Deserialize, Serialize};
use sqlx::types::Json;
use sqlx_data::{JsonValue, Pool, QueryResult, Result, dml, repo};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UserProfile {
    pub email: String,
    pub age: u32,
    pub department: String,
    pub skills: Vec<String>,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Settings {
    pub theme: String,
    pub notifications: bool,
    pub language: Option<String>,
}

#[repo]
trait JsonMySqlRepo {
    #[dml("INSERT INTO json_users (name, profile_json, preferences) VALUES (?, ?, ?)")]
    async fn insert_with_json_types(
        &self,
        name: String,
        profile: Json<UserProfile>,
        preferences: Option<Json<Settings>>,
    ) -> Result<QueryResult>;

    #[dml("SELECT id, name, profile_json, preferences FROM json_users WHERE id = ?")]
    async fn find_raw_json(&self, id: i64) -> Result<Option<(i64, String, Json<JsonValue>, Option<JsonValue>)>>;

    #[dml("SELECT JSON_EXTRACT(profile_json, '$.email') as email FROM json_users WHERE id = ?")]
    async fn extract_email(&self, id: i64) -> Result<Option<JsonValue>>;

    #[dml("SELECT (CAST(JSON_UNQUOTE(JSON_EXTRACT(profile_json, '$.age')) AS UNSIGNED)+0) as 'age: u32' FROM json_users WHERE id = ?")]
    async fn extract_age(&self, id: i64) -> Result<Option<u32>>;

    #[dml("SELECT JSON_EXTRACT(profile_json, '$.skills') as skills FROM json_users WHERE id = ?")]
    async fn extract_skills(&self, id: i64) -> Result<Option<JsonValue>>;

    #[dml("SELECT COUNT(*) FROM json_users WHERE CAST(JSON_EXTRACT(profile_json, '$.active') AS UNSIGNED) = ?")]
    async fn count_by_active_status(&self, active: bool) -> Result<i64>;

    #[dml("UPDATE json_users SET profile_json = JSON_SET(profile_json, '$.active', ?) WHERE id = ?")]
    async fn update_active_status(&self, active: bool, id: i64) -> Result<QueryResult>;

    #[dml("SELECT id FROM json_users WHERE JSON_EXTRACT(profile_json, '$.department') = ?")]
    async fn find_by_department(&self, department: String) -> Result<Vec<i64>>;

    #[dml("SELECT id FROM json_users WHERE JSON_CONTAINS(profile_json, ?, '$.skills')")]
    async fn find_by_skill(&self, skill: String) -> Result<Vec<i64>>;

    #[dml("INSERT INTO json_users (name, profile_json) VALUES (?, JSON_OBJECT('email', ?, 'age', ?, 'department', ?, 'skills', JSON_ARRAY(?), 'active', ?))")]
    async fn insert_with_json_object(
        &self,
        name: String,
        email: String,
        age: u32,
        department: String,
        skill: String,
        active: bool,
    ) -> Result<QueryResult>;

    #[dml("UPDATE json_users SET preferences = ? WHERE id = ?")]
    async fn update_preferences(&self, preferences: Json<Settings>, id: i64) -> Result<QueryResult>;

    #[dml("SELECT cast(JSON_UNQUOTE(JSON_EXTRACT(preferences, '$.theme')) AS CHAR) as 'theme: String' FROM json_users WHERE id = ?")]
    async fn get_theme(&self, id: i64) -> Result<Option<String>>;
}

pub struct JsonMySqlApp {
    pool: Pool,
}

impl JsonMySqlRepo for JsonMySqlApp {
    fn get_pool(&self) -> &Pool {
        &self.pool
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_insert_and_extract_json(pool: Pool) {
        let app = JsonMySqlApp { pool };

        let profile = UserProfile {
            email: "test@example.com".to_string(),
            age: 28,
            department: "Engineering".to_string(),
            skills: vec!["Rust".to_string(), "MySQL".to_string(), "JSON".to_string()],
            active: true,
        };

        let settings = Settings {
            theme: "dark".to_string(),
            notifications: true,
            language: Some("en".to_string()),
        };

        let result = app
            .insert_with_json_types("JSON Test User".to_string(), Json(profile.clone()), Some(Json(settings.clone())))
            .await
            .unwrap();

        let id = result.last_insert_id() as i64;

        let email = app.extract_email(id).await.unwrap();
        assert_eq!(email, Some(JsonValue::String(profile.email.clone())));

        let age = app.extract_age(id).await.unwrap();
        assert_eq!(age, Some(profile.age));

        let skills = app.extract_skills(id).await.unwrap();
        assert!(skills.is_some());
        let skills_str = skills.unwrap().to_string();
        assert!(skills_str.contains("Rust"));
        assert!(skills_str.contains("MySQL"));

        let theme = app.get_theme(id).await.unwrap();
        assert_eq!(theme, Some(settings.theme));
    }

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_mysql_json_object_function(pool: Pool) {
        let app = JsonMySqlApp { pool };

        let result = app
            .insert_with_json_object(
                "Direct JSON".to_string(),
                "direct@example.com".to_string(),
                32,
                "Marketing".to_string(),
                "Analytics".to_string(),
                true,
            )
            .await
            .unwrap();

        let id = result.last_insert_id() as i64;

        let email = app.extract_email(id).await.unwrap();
        assert_eq!(email, Some(JsonValue::String("direct@example.com".to_string())));

        let age = app.extract_age(id).await.unwrap();
        assert_eq!(age, Some(32));

        let department_users = app.find_by_department("Marketing".to_string()).await.unwrap();
        assert!(department_users.contains(&id));
    }

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_json_contains_queries(pool: Pool) {
        let app = JsonMySqlApp { pool };

        let profile1 = UserProfile {
            email: "dev1@example.com".to_string(),
            age: 25,
            department: "Engineering".to_string(),
            skills: vec!["Rust".to_string(), "Docker".to_string()],
            active: true,
        };

        let profile2 = UserProfile {
            email: "dev2@example.com".to_string(),
            age: 30,
            department: "Engineering".to_string(),
            skills: vec!["Python".to_string(), "Kubernetes".to_string()],
            active: true,
        };

        let result1 = app
            .insert_with_json_types("Dev 1".to_string(), Json(profile1), None)
            .await
            .unwrap();

        let result2 = app
            .insert_with_json_types("Dev 2".to_string(), Json(profile2), None)
            .await
            .unwrap();

        let id1 = result1.last_insert_id() as i64;
        let id2 = result2.last_insert_id() as i64;

        let rust_users = app.find_by_skill("\"Rust\"".to_string()).await.unwrap();
        assert!(rust_users.contains(&id1));
        assert!(!rust_users.contains(&id2));

        let python_users = app.find_by_skill("\"Python\"".to_string()).await.unwrap();
        assert!(!python_users.contains(&id1));
        assert!(python_users.contains(&id2));
    }

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_json_updates(pool: Pool) {
        let app = JsonMySqlApp { pool };

        let profile = UserProfile {
            email: "update@example.com".to_string(),
            age: 35,
            department: "Sales".to_string(),
            skills: vec!["Communication".to_string()],
            active: true,
        };

        let result = app
            .insert_with_json_types("Update Test".to_string(), Json(profile), None)
            .await
            .unwrap();

        let id = result.last_insert_id() as i64;

        let active_count_before = app.count_by_active_status(true).await.unwrap();

        let update_result = app.update_active_status(false, id).await.unwrap();
        assert_eq!(update_result.rows_affected(), 1);

        let active_count_after = app.count_by_active_status(true).await.unwrap();
        assert_eq!(active_count_after, active_count_before - 1);

        let new_settings = Settings {
            theme: "light".to_string(),
            notifications: false,
            language: Some("es".to_string()),
        };

        let prefs_result = app.update_preferences(Json(new_settings.clone()), id).await.unwrap();
        assert_eq!(prefs_result.rows_affected(), 1);

        let theme = app.get_theme(id).await.unwrap();
        assert_eq!(theme, Some(new_settings.theme));
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("json_users"))
    )]
    async fn test_query_fixture_data(pool: Pool) {
        let app = JsonMySqlApp { pool };

        let alice_email = app.extract_email(1).await.unwrap();
        assert_eq!(alice_email, Some(JsonValue::String("alice@example.com".to_string())));

        let alice_age = app.extract_age(1).await.unwrap();
        assert_eq!(alice_age, Some(30));

        let engineering_users = app.find_by_department("Engineering".to_string()).await.unwrap();
        assert!(engineering_users.contains(&1));

        let active_users = app.count_by_active_status(true).await.unwrap();
        assert!(active_users > 0);
    }

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_raw_json_retrieval(pool: Pool) {
        let app = JsonMySqlApp { pool };

        let profile = UserProfile {
            email: "raw@example.com".to_string(),
            age: 40,
            department: "HR".to_string(),
            skills: vec!["Management".to_string()],
            active: true,
        };

        let result = app
            .insert_with_json_types("Raw Test".to_string(), Json(profile.clone()), None)
            .await
            .unwrap();

        let id = result.last_insert_id() as i64;

        let raw_data = app.find_raw_json(id).await.unwrap();
        assert!(raw_data.is_some());

        let (db_id, name, profile_json, preferences) = raw_data.unwrap();
        assert_eq!(db_id, id);
        assert_eq!(name, "Raw Test");
        let profile_json_str = profile_json.to_string();
        assert!(profile_json_str.contains("raw@example.com"));
        assert!(profile_json_str.contains("HR"));
        assert!(preferences.is_none());
    }
}