#![cfg(all(feature = "json"))]

use serde::{Deserialize, Serialize};
use sqlx::types::Json;
use sqlx_data::{Pool, Result, dml, repo};

#[derive(Debug, Serialize, Deserialize)]
pub struct Profile {
    pub email: String,
    pub age: u32,
    pub city: String,
    pub department: String,
    pub skills: Vec<String>,
    pub active: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Preferences {
    pub theme: String,
    pub notifications: bool,
    pub language: Option<String>,
}

#[repo]
trait JsonUserRepo {
    #[dml(
        r#"
        INSERT INTO json_users (name, profile_json, preferences)
        VALUES (?, ?, ?)
        "#
    )]
    async fn create_user_with_json_struct(
        &self,
        name: impl Into<String>,
        profile: Json<Profile>,
        preferences: Option<Json<Preferences>>,
    ) -> Result<u64>;

    #[dml(
        r#"
        INSERT INTO json_users (name, profile_json, preferences)
        VALUES (?, ?, ?)
        "#,
        json
    )]
    async fn create_user_with_json_direct(
        &self,
        name: String,
        profile: Profile,
        preferences: Option<Preferences>,
    ) -> Result<u64>;
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
    use super::*;

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_create_user_with_json_struct(pool: Pool) {
        let repo = JsonUserApp { pool };

        let profile = Profile {
            email: "eve@example.com".into(),
            age: 26,
            department: "Design".into(),
            skills: vec!["UI/UX".into(), "Figma".into(), "Design Systems".into()],
            active: true,
            city: "San Francisco".into(),
        };

        let preferences = Preferences {
            theme: "system".into(),
            notifications: true,
            language: Some("en".into()),
        };

        let user_id = repo
            .create_user_with_json_struct("Eve Taylor", Json(profile), Some(Json(preferences)))
            .await
            .unwrap();

        assert!(user_id > 0);
    }

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_create_user_with_json_direct(pool: Pool) {
        let repo = JsonUserApp { pool };

        let profile = Profile {
            email: "frank@example.com".into(),
            age: 32,
            department: "Engineering".into(),
            skills: vec!["Rust".into(), "Python".into(), "SQL".into()],
            active: true,
            city: "Austin".into(),
        };

        let preferences = Preferences {
            theme: "dark".into(),
            notifications: false,
            language: Some("en".into()),
        };

        let user_id = repo
            .create_user_with_json_direct("Frank Wilson".to_string(), profile, Some(preferences))
            .await
            .unwrap();

        assert!(user_id > 0);
    }
}