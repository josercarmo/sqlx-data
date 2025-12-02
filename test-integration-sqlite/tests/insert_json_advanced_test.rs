#![cfg(all(feature = "json"))]

use serde::{Deserialize, Serialize};
use sqlx_data::{Pool, QueryResult, Result, dml, repo};
use std::collections::{BTreeMap, HashMap, HashSet};

#[derive(Debug, Serialize, Deserialize)]
struct Settings {
    theme: String,
    notifications: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct UserData {
    id: i64,
    name: String,
}

#[repo]
trait HashMapRepo {
    #[dml(
        "INSERT INTO json_users (name, profile_json) VALUES ('test', $1)",
        json
    )]
    async fn insert_hashmap(&self, settings: HashMap<String, Settings>) -> Result<QueryResult>;

    #[dml(
        "INSERT INTO json_users (name, profile_json) VALUES ('test', $1) RETURNING id",
        json
    )]
    async fn insert_hashmap_returning(&self, data: HashMap<String, i32>) -> Result<i64>;

    #[dml("UPDATE json_users SET profile_json = $1 WHERE id = $2", json)]
    async fn update_hashmap(
        &self,
        data: HashMap<String, Vec<String>>,
        id: i64,
    ) -> Result<QueryResult>;

    #[dml(
        "DELETE FROM json_users WHERE name IN (SELECT key FROM json_each($1))",
        json
    )]
    async fn delete_with_hashmap(&self, filters: HashMap<String, bool>) -> Result<QueryResult>;

    #[dml(
        "SELECT json_extract(profile_json, '$.email') as \"email: String\" FROM json_users WHERE id = $1",
        json
    )]
    async fn select_json_extract_string(&self, id: i64) -> Result<Option<String>>;

    #[dml(
        "SELECT json_extract(profile_json, '$.age') as \"age: i32\" FROM json_users WHERE id = $1",
        json
    )]
    async fn select_json_extract_number(&self, id: i64) -> Result<Option<i32>>;

    #[dml("SELECT profile_json ->> '$.name' as \"name: String\" FROM json_users WHERE id = $1")]
    async fn select_json_arrow_text(&self, id: i64) -> Result<Option<String>>;

    #[dml("SELECT profile_json -> '$.skills' as \"skills: String\" FROM json_users WHERE id = $1")]
    async fn select_json_arrow_object(&self, id: i64) -> Result<Option<String>>;

    #[dml("SELECT COUNT(*) FROM json_users WHERE json_extract(profile_json, '$.active') = $1")]
    async fn count_by_json_field(&self, active: bool) -> Result<i64>;

    #[dml(
        "UPDATE json_users SET profile_json = json_set(profile_json, '$.updated', $1) WHERE id = $2"
    )]
    async fn update_json_field(&self, timestamp: String, id: i64) -> Result<QueryResult>;

    #[dml(
        "INSERT INTO json_users (name, profile_json) VALUES ('test', json_object('name', $1, 'active', $2))"
    )]
    async fn insert_json_object(&self, name: String, active: bool) -> Result<QueryResult>;

    //BUG NO SQLX - no support to json_each yet
    //#[dml("SELECT json_each.key, json_each.value FROM json_users, json_each(profile_json) WHERE json_users.id = $1")]
    //async fn select_json_each(&self, id: i64) -> Result<Vec<(String, String)>>;

    #[dml("DELETE FROM json_users WHERE json_extract(profile_json, '$.toDelete') = $1")]
    async fn delete_by_json_flag(&self, to_delete: bool) -> Result<QueryResult>;

    #[dml(
        "INSERT INTO json_users (name, profile_json) VALUES ('test', $1)",
        json
    )]
    async fn insert_btreemap(&self, data: BTreeMap<String, Settings>) -> Result<QueryResult>;

    #[dml(
        "INSERT INTO json_users (name, profile_json) VALUES ('test', $1)",
        json
    )]
    async fn insert_hashmap_vec(&self, data: HashMap<String, Vec<UserData>>)
    -> Result<QueryResult>;

    #[dml(
        "INSERT INTO json_users (name, profile_json) VALUES ('test', $1)",
        json
    )]
    async fn insert_hashmap_option(
        &self,
        data: HashMap<String, Option<Settings>>,
    ) -> Result<QueryResult>;

    #[dml(
        "INSERT INTO json_users (name, profile_json) VALUES ('test', $1)",
        json
    )]
    async fn insert_nested_hashmap(
        &self,
        data: HashMap<String, HashMap<String, i32>>,
    ) -> Result<QueryResult>;

    //#[dml("SELECT profile_json FROM json_users WHERE id = $1")]
    //async fn select_complex_hashmap(&self, id: i64) -> Result<HashMap<i64, Vec<Option<Settings>>>>;

    #[dml(
        "INSERT INTO json_users (name, profile_json) VALUES ('test', $1)",
        json
    )]
    async fn insert_hashmap_primitives(&self, data: HashMap<i32, String>) -> Result<QueryResult>;

    #[dml(
        "INSERT INTO json_users (name, profile_json) VALUES ('test', $1)",
        json
    )]
    async fn insert_hashmap_mixed(
        &self,
        data: HashMap<String, (i32, String, bool)>,
    ) -> Result<QueryResult>;

    #[dml(
        "INSERT INTO json_users (name, profile_json) VALUES ('test', $1)",
        json
    )]
    async fn insert_hashset(&self, data: HashSet<String>) -> Result<QueryResult>;

    #[dml(
        "UPDATE json_users SET preferences = $1, profile_json = $2 WHERE id = $3",
        json
    )]
    async fn update_multi_hashmap(
        &self,
        prefs: Option<HashMap<String, bool>>,
        data: HashMap<String, UserData>,
        id: i64,
    ) -> Result<QueryResult>;

    //#[dml("INSERT INTO json_users (name, profile_json) VALUES ('test', $1) RETURNING name, id")]
    //async fn insert_returning_tuple(&self, data: HashMap<String, Settings>) -> Result<(String, i64)>;

    //#[dml("SELECT profile_json FROM json_users WHERE id = $1")]
    //async fn select_weird_return(&self, id: i64) -> Result<Vec<Option<HashMap<String, Vec<Settings>>>>>;

    #[dml(
        "INSERT INTO json_users (name, profile_json) VALUES ('test', $1)",
        json
    )]
    async fn insert_rc_hashmap(
        &self,
        data: std::rc::Rc<HashMap<String, Settings>>,
    ) -> Result<QueryResult>;

    #[dml(
        "INSERT INTO json_users (name, profile_json) VALUES ('test', $1)",
        json
    )]
    async fn insert_box_hashmap(&self, data: Box<HashMap<String, UserData>>)
    -> Result<QueryResult>;

    //#[dml("SELECT name FROM json_users WHERE profile_json = $1")]
    //async fn select_with_param_hashmap(&self, filter: HashMap<String, String>) -> Result<String>;
}

pub struct HashMapApp {
    pool: Pool,
}

impl HashMapRepo for HashMapApp {
    fn get_pool(&self) -> &Pool {
        &self.pool
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{BTreeMap, HashMap, HashSet};
    use std::rc::Rc;

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_insert_hashmap(pool: Pool) {
        let app = HashMapApp { pool };
        let mut settings_map = HashMap::new();
        settings_map.insert(
            "user1".to_string(),
            Settings {
                theme: "dark".to_string(),
                notifications: true,
            },
        );
        settings_map.insert(
            "user2".to_string(),
            Settings {
                theme: "light".to_string(),
                notifications: false,
            },
        );

        let result = app.insert_hashmap(settings_map).await;
        assert!(result.is_ok());
        let query_result = result.unwrap();
        assert!(query_result.rows_affected() > 0);
    }

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_insert_hashmap_returning(pool: Pool) {
        let app = HashMapApp { pool };
        let mut data = HashMap::new();
        data.insert("count".to_string(), 42);
        data.insert("value".to_string(), 100);

        let result = app.insert_hashmap_returning(data).await;
        assert!(result.is_ok());
        let id = result.unwrap();
        assert!(id > 0);
    }

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_update_hashmap(pool: Pool) {
        let app = HashMapApp { pool };

        // First insert a record
        let mut data = HashMap::new();
        data.insert("test".to_string(), 1);
        let id = app.insert_hashmap_returning(data).await.unwrap();

        // Then update it
        let mut update_data = HashMap::new();
        update_data.insert(
            "skills".to_string(),
            vec!["Rust".to_string(), "SQL".to_string()],
        );

        let result = app.update_hashmap(update_data, id).await;
        assert!(result.is_ok());
    }

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_delete_with_hashmap(pool: Pool) {
        let app = HashMapApp { pool };
        let mut filters = HashMap::new();
        filters.insert("active".to_string(), false);
        filters.insert("toDelete".to_string(), true);

        let result = app.delete_with_hashmap(filters).await;
        assert!(result.is_ok());
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("json_users"))
    )]
    async fn test_json_extract_functions(pool: Pool) {
        let app = HashMapApp { pool };

        // Test string extraction
        let result = app.select_json_extract_string(1).await;
        assert!(result.is_ok());
        let email = result.unwrap();
        assert_eq!(email, Some("alice@example.com".to_string()));

        // Test number extraction
        let result = app.select_json_extract_number(1).await;
        assert!(result.is_ok());
        let age = result.unwrap();
        assert_eq!(age, Some(30));
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("json_users"))
    )]
    async fn test_json_arrow_operators(pool: Pool) {
        let app = HashMapApp { pool };

        // Test ->> operator (text)
        let result = app.select_json_arrow_text(1).await;
        assert!(result.is_ok());
        println!("JSON arrow text result: {:?}", result);

        // Test -> operator (object)
        let result = app.select_json_arrow_object(1).await;
        assert!(result.is_ok());
        println!("JSON arrow object result: {:?}", result);
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("json_users"))
    )]
    async fn test_count_by_json_field(pool: Pool) {
        let app = HashMapApp { pool };

        let result = app.count_by_json_field(true).await;
        assert!(result.is_ok());
        let count = result.unwrap();
        assert!(count >= 0);
        println!("Count of active users: {}", count);
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("json_users"))
    )]
    async fn test_update_json_field(pool: Pool) {
        let app = HashMapApp { pool };

        let timestamp = "2023-12-17T10:00:00Z".to_string();
        let result = app.update_json_field(timestamp, 1).await;
        assert!(result.is_ok());
    }

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_insert_json_object(pool: Pool) {
        let app = HashMapApp { pool };

        let result = app.insert_json_object("test_user".to_string(), true).await;
        assert!(result.is_ok());
        let query_result = result.unwrap();
        assert!(query_result.rows_affected() > 0);
    }

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_delete_by_json_flag(pool: Pool) {
        let app = HashMapApp { pool };

        let result = app.delete_by_json_flag(true).await;
        assert!(result.is_ok());
    }

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_insert_btreemap(pool: Pool) {
        let app = HashMapApp { pool };
        let mut data = BTreeMap::new();
        data.insert(
            "admin".to_string(),
            Settings {
                theme: "system".to_string(),
                notifications: true,
            },
        );

        let result = app.insert_btreemap(data).await;
        assert!(result.is_ok());
    }

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_insert_hashmap_vec(pool: Pool) {
        let app = HashMapApp { pool };
        let mut data = HashMap::new();
        data.insert(
            "team".to_string(),
            vec![
                UserData {
                    id: 1,
                    name: "Alice".to_string(),
                },
                UserData {
                    id: 2,
                    name: "Bob".to_string(),
                },
            ],
        );

        let result = app.insert_hashmap_vec(data).await;
        assert!(result.is_ok());
    }

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_insert_hashmap_option(pool: Pool) {
        let app = HashMapApp { pool };
        let mut data = HashMap::new();
        data.insert(
            "config".to_string(),
            Some(Settings {
                theme: "dark".to_string(),
                notifications: false,
            }),
        );
        data.insert("empty".to_string(), None);

        let result = app.insert_hashmap_option(data).await;
        assert!(result.is_ok());
    }

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_insert_nested_hashmap(pool: Pool) {
        let app = HashMapApp { pool };
        let mut inner_map = HashMap::new();
        inner_map.insert("score".to_string(), 95);
        inner_map.insert("level".to_string(), 3);

        let mut data = HashMap::new();
        data.insert("player1".to_string(), inner_map);

        let result = app.insert_nested_hashmap(data).await;
        assert!(result.is_ok());
    }

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_insert_hashmap_primitives(pool: Pool) {
        let app = HashMapApp { pool };
        let mut data = HashMap::new();
        data.insert(1, "first".to_string());
        data.insert(2, "second".to_string());

        let result = app.insert_hashmap_primitives(data).await;
        assert!(result.is_ok());
    }

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_insert_hashmap_mixed(pool: Pool) {
        let app = HashMapApp { pool };
        let mut data = HashMap::new();
        data.insert("record1".to_string(), (100, "test".to_string(), true));
        data.insert("record2".to_string(), (200, "example".to_string(), false));

        let result = app.insert_hashmap_mixed(data).await;
        assert!(result.is_ok());
    }

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_insert_hashset(pool: Pool) {
        let app = HashMapApp { pool };
        let mut data = HashSet::new();
        data.insert("tag1".to_string());
        data.insert("tag2".to_string());
        data.insert("tag3".to_string());

        let result = app.insert_hashset(data).await;
        assert!(result.is_ok());
    }

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_update_multi_hashmap(pool: Pool) {
        let app = HashMapApp { pool };

        // First create a record
        let mut data = HashMap::new();
        data.insert("test".to_string(), 1);
        let id = app.insert_hashmap_returning(data).await.unwrap();

        let mut prefs = HashMap::new();
        prefs.insert("darkMode".to_string(), true);
        prefs.insert("autoSave".to_string(), false);

        let mut user_data = HashMap::new();
        user_data.insert(
            "user1".to_string(),
            UserData {
                id: 1,
                name: "Test User".to_string(),
            },
        );

        let result = app.update_multi_hashmap(Some(prefs), user_data, id).await;
        assert!(result.is_ok());
    }

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_insert_rc_hashmap(pool: Pool) {
        let app = HashMapApp { pool };
        let mut data = HashMap::new();
        data.insert(
            "shared".to_string(),
            Settings {
                theme: "shared_theme".to_string(),
                notifications: true,
            },
        );
        let rc_data = Rc::new(data);

        let result = app.insert_rc_hashmap(rc_data).await;
        assert!(result.is_ok());
    }

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_insert_box_hashmap(pool: Pool) {
        let app = HashMapApp { pool };
        let mut data = HashMap::new();
        data.insert(
            "boxed".to_string(),
            UserData {
                id: 99,
                name: "Boxed User".to_string(),
            },
        );
        let boxed_data = Box::new(data);

        let result = app.insert_box_hashmap(boxed_data).await;
        assert!(result.is_ok());
    }
}
