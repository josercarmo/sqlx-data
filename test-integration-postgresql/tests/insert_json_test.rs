#![cfg(all(feature = "json"))]

use serde::{Deserialize, Serialize};
use sqlx_data::{Pool, Result, dml, repo};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Profile {
    pub email: String,
    pub age: i32,
    pub city: String,
    pub department: String,
    pub skills: Vec<String>,
    pub active: bool,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Preferences {
    pub theme: String,
    pub notifications: bool,
    pub language: Option<String>,
}

#[repo]
trait JsonUserRepo {
    // PostgreSQL JSONB direct insertion - most efficient
    #[dml(
        r#"
        INSERT INTO json_users (name, profile_json, preferences)
        VALUES ($1, $2, $3)
        RETURNING id
        "#
    )]
    async fn create_user_with_jsonb(
        &self,
        name: impl Into<String>,
        profile: serde_json::Value,
        preferences: Option<serde_json::Value>,
    ) -> Result<i64>;

    // With automatic JSON serialization using the json attribute
    #[dml(
        r#"
        INSERT INTO json_users (name, profile_json, preferences)
        VALUES ($1, $2, $3)
        RETURNING id
        "#,
        json
    )]
    async fn create_user_with_json_direct(
        &self,
        name: String,
        profile: Profile,
        preferences: Option<Preferences>,
    ) -> Result<i64>;

    // Query to retrieve and deserialize JSONB
    #[dml("SELECT id, name, profile_json, preferences FROM json_users WHERE id = $1")]
    async fn find_user_by_id(&self, id: i64) -> Result<Option<(i64, String, serde_json::Value, Option<serde_json::Value>)>>;

    // Query with JSON path operations (PostgreSQL specific)
    #[dml("SELECT id, name FROM json_users WHERE profile_json->>'department' = $1")]
    async fn find_users_by_department(&self, department: String) -> Result<Vec<(i64, String)>>;

    // Query with JSONB containment operator (PostgreSQL specific)
    #[dml("SELECT id, name FROM json_users WHERE profile_json @> $1")]
    async fn find_users_with_profile_match(&self, profile_filter: serde_json::Value) -> Result<Vec<(i64, String)>>;

    // Query with JSONB array operations
    #[dml("SELECT id, name FROM json_users WHERE profile_json->'skills' ? $1")]
    async fn find_users_with_skill(&self, skill: String) -> Result<Vec<(i64, String)>>;

    // Update JSONB field using PostgreSQL jsonb_set function
    #[dml("UPDATE json_users SET profile_json = jsonb_set(profile_json, '{email}', $2) WHERE id = $1")]
    async fn update_user_email(&self, id: i64, new_email: serde_json::Value) -> Result<sqlx_data::QueryResult>;
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
    async fn test_create_user_with_jsonb(pool: Pool) {
        let repo = JsonUserApp { pool };

        let profile = serde_json::json!({
            "email": "eve@example.com",
            "age": 26,
            "department": "Design",
            "skills": ["UI/UX", "Figma", "Design Systems"],
            "active": true,
            "city": "San Francisco"
        });

        let preferences = serde_json::json!({
            "theme": "system",
            "notifications": true,
            "language": "en"
        });

        // Test with direct JSONB values
        let user_id = repo
            .create_user_with_jsonb("Eve Taylor", profile, Some(preferences))
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

        // Test with direct struct - automatic JSON serialization
        let user_id = repo
            .create_user_with_json_direct("Frank Wilson".to_string(), profile, Some(preferences))
            .await
            .unwrap();

        assert!(user_id > 0);
    }

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_json_retrieval_and_deserialization(pool: Pool) {
        let repo = JsonUserApp { pool };

        let profile = Profile {
            email: "alice@example.com".into(),
            age: 28,
            department: "Marketing".into(),
            skills: vec!["Content".into(), "Analytics".into()],
            active: true,
            city: "New York".into(),
        };

        let preferences = Preferences {
            theme: "light".into(),
            notifications: true,
            language: Some("en".into()),
        };

        // Insert user
        let user_id = repo
            .create_user_with_json_direct("Alice Smith".to_string(), profile.clone(), Some(preferences.clone()))
            .await
            .unwrap();

        // Retrieve user
        let retrieved = repo
            .find_user_by_id(user_id)
            .await
            .unwrap()
            .expect("User should be found");

        let (_id, name, profile_json, preferences_json) = retrieved;
        assert_eq!(name, "Alice Smith");

        // Deserialize and verify profile
        let retrieved_profile: Profile = serde_json::from_value(profile_json).unwrap();
        assert_eq!(retrieved_profile, profile);

        // Deserialize and verify preferences
        if let Some(prefs_value) = preferences_json {
            let retrieved_preferences: Preferences = serde_json::from_value(prefs_value).unwrap();
            assert_eq!(retrieved_preferences, preferences);
        } else {
            panic!("Preferences should not be null");
        }
    }

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_jsonb_path_operations(pool: Pool) {
        let repo = JsonUserApp { pool };

        // Insert users with different departments
        let profile1 = Profile {
            email: "dev1@example.com".into(),
            age: 30,
            department: "Engineering".into(),
            skills: vec!["Rust".into()],
            active: true,
            city: "Seattle".into(),
        };

        let profile2 = Profile {
            email: "design1@example.com".into(),
            age: 25,
            department: "Design".into(),
            skills: vec!["Figma".into()],
            active: true,
            city: "Portland".into(),
        };

        repo.create_user_with_json_direct("Dev User".to_string(), profile1, None).await.unwrap();
        repo.create_user_with_json_direct("Design User".to_string(), profile2, None).await.unwrap();

        // Test JSON path query
        let engineering_users = repo
            .find_users_by_department("Engineering".to_string())
            .await
            .unwrap();

        assert!(!engineering_users.is_empty());
        assert!(engineering_users.iter().any(|(_, name)| name == "Dev User"));

        let design_users = repo
            .find_users_by_department("Design".to_string())
            .await
            .unwrap();

        assert!(!design_users.is_empty());
        assert!(design_users.iter().any(|(_, name)| name == "Design User"));
    }

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_jsonb_containment_operator(pool: Pool) {
        let repo = JsonUserApp { pool };

        let profile = Profile {
            email: "backend@example.com".into(),
            age: 35,
            department: "Engineering".into(),
            skills: vec!["Rust".into(), "PostgreSQL".into()],
            active: true,
            city: "Denver".into(),
        };

        repo.create_user_with_json_direct("Backend Engineer".to_string(), profile, None).await.unwrap();

        // Test JSONB containment - find users with specific profile attributes
        let filter = serde_json::json!({
            "department": "Engineering",
            "active": true
        });

        let matching_users = repo
            .find_users_with_profile_match(filter)
            .await
            .unwrap();

        assert!(!matching_users.is_empty());
        assert!(matching_users.iter().any(|(_, name)| name == "Backend Engineer"));
    }

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_jsonb_array_operations(pool: Pool) {
        let repo = JsonUserApp { pool };

        let profile_rust = Profile {
            email: "rust@example.com".into(),
            age: 30,
            department: "Engineering".into(),
            skills: vec!["Rust".into(), "Docker".into()],
            active: true,
            city: "Austin".into(),
        };

        let profile_python = Profile {
            email: "python@example.com".into(),
            age: 28,
            department: "Engineering".into(),
            skills: vec!["Python".into(), "Django".into()],
            active: true,
            city: "San Francisco".into(),
        };

        repo.create_user_with_json_direct("Rust Developer".to_string(), profile_rust, None).await.unwrap();
        repo.create_user_with_json_direct("Python Developer".to_string(), profile_python, None).await.unwrap();

        // Test JSONB array containment
        let rust_users = repo
            .find_users_with_skill("Rust".to_string())
            .await
            .unwrap();

        assert!(!rust_users.is_empty());
        assert!(rust_users.iter().any(|(_, name)| name == "Rust Developer"));

        let python_users = repo
            .find_users_with_skill("Python".to_string())
            .await
            .unwrap();

        assert!(!python_users.is_empty());
        assert!(python_users.iter().any(|(_, name)| name == "Python Developer"));

        // Skill that doesn't exist
        let no_users = repo
            .find_users_with_skill("COBOL".to_string())
            .await
            .unwrap();

        assert!(no_users.is_empty());
    }

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_jsonb_update_operations(pool: Pool) {
        let repo = JsonUserApp { pool };

        let profile = Profile {
            email: "old@example.com".into(),
            age: 30,
            department: "Engineering".into(),
            skills: vec!["Rust".into()],
            active: true,
            city: "Seattle".into(),
        };

        let user_id = repo
            .create_user_with_json_direct("Update Test User".to_string(), profile, None)
            .await
            .unwrap();

        // Update email using PostgreSQL jsonb_set function
        let new_email = serde_json::json!("new@example.com");
        repo.update_user_email(user_id, new_email.clone()).await.unwrap();

        // Verify the update
        let updated_user = repo
            .find_user_by_id(user_id)
            .await
            .unwrap()
            .expect("User should be found");

        let (_id, _name, profile_json, _preferences) = updated_user;
        let updated_profile: Profile = serde_json::from_value(profile_json).unwrap();

        assert_eq!(updated_profile.email, "new@example.com");
        assert_eq!(updated_profile.department, "Engineering"); // Other fields unchanged
    }

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_null_preferences_handling(pool: Pool) {
        let repo = JsonUserApp { pool };

        let profile = Profile {
            email: "nullprefs@example.com".into(),
            age: 25,
            department: "Marketing".into(),
            skills: vec!["Content".into()],
            active: true,
            city: "Boston".into(),
        };

        // Insert user with null preferences
        let user_id = repo
            .create_user_with_json_direct("No Prefs User".to_string(), profile.clone(), None)
            .await
            .unwrap();

        // Retrieve and verify null handling
        let retrieved = repo
            .find_user_by_id(user_id)
            .await
            .unwrap()
            .expect("User should be found");

        let (_id, name, profile_json, preferences_json) = retrieved;
        assert_eq!(name, "No Prefs User");

        let retrieved_profile: Profile = serde_json::from_value(profile_json).unwrap();
        assert_eq!(retrieved_profile, profile);

        assert!(preferences_json.is_none(), "Preferences should be null");
    }
}