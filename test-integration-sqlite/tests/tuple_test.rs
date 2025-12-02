use sqlx::Pool;
use sqlx_data::{FilterParams, Result, Serial, dml};

// Example of a newtype for testing transparent types
#[allow(dead_code)]
#[derive(Clone, PartialEq, Eq, Debug, sqlx::Type)]
#[sqlx(transparent)]
pub struct Id(i64);

#[allow(dead_code)]
impl From<i64> for Id {
    fn from(value: i64) -> Self {
        Id(value)
    }
}

// User model for tests
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub struct User {
    pub id: Id,
    pub name: String,
    pub email: String,
    pub age: u8,
    pub birth_year: Option<u16>,
}

// Trait focused on tuple return types and tuple-based operations
#[sqlx_data::repo]
trait TupleRepo {
    // === TUPLE RETURN TYPE TESTS ===

    // Basic tuple queries with type casting
    #[dml("SELECT COUNT(id) as count, AVG(age) as 'average: f64' FROM users")]
    async fn average_age(&self) -> Result<(i32, Option<f64>)>;

    #[dml("SELECT id, age, name FROM users LIMIT 2")]
    async fn get_all_ages(&self) -> Result<Vec<(i32, u8, String)>>;

    // Test optimization: no casting needed (i64, String)
    #[dml("SELECT id, name FROM users LIMIT 2")]
    async fn get_id_names(&self) -> Result<Vec<(i32, String)>>;

    #[dml("SELECT id, age, name FROM users LIMIT 1")]
    async fn get_one_age(&self) -> Result<(i32, i32, String)>;

    // CTE with tuple return
    #[dml("WITH temp AS (SELECT 1) SELECT name, birth_year FROM users WHERE id = $1")]
    async fn get_one_birth(&self, id: i32) -> Result<(String, Option<i32>)>;

    // Tuple with casting
    #[dml("SELECT name, birth_year FROM users WHERE id = $1")]
    async fn get_one_birth_with_cast(&self, id: i32) -> Result<(String, Option<u16>)>;

    // Complex aggregates with casting in tuple
    #[dml(
        "SELECT MIN(age) as min_age, MAX(age) as max_age, COUNT(*) as count, SUM(age) as sum  FROM users"
    )]
    async fn stats_all_types(&self) -> Result<(Option<u8>, Option<u8>, u32, Option<u64>)>;

    // CASE WHEN with different types in tuple
    #[dml("SELECT CASE WHEN age > 30 THEN 1 ELSE 0 END as is_senior, age FROM users")]
    async fn case_when_types(&self) -> Result<Vec<(i64, i64)>>;

    // COALESCE function with nullable in tuple
    #[dml("SELECT COALESCE(birth_year, 2000) as year, name FROM users WHERE id = $1")]
    async fn coalesce_nullable(&self, id: i64) -> Result<(u16, String)>;

    // String functions with casting in tuple
    #[dml(
        "SELECT LENGTH(name) as 'name_len: u8', UPPER(email) as 'email: String' FROM users LIMIT 1"
    )]
    async fn string_functions(&self) -> Result<(Option<u8>, Option<String>)>;

    // Window function tuple
    #[dml("SELECT id, age, ROW_NUMBER() OVER (ORDER BY age) as rank FROM users")]
    async fn window_function(&self) -> Result<Vec<(i32, u8, u16)>>;

    // Null handling in tuple
    #[dml("SELECT name, birth_year, email FROM users WHERE birth_year IS NULL")]
    async fn null_handling_tuple(&self) -> Result<Vec<(String, Option<u16>, String)>>;

    // LIMIT and OFFSET with casting in tuple
    #[dml("SELECT id, age FROM users ORDER BY age DESC LIMIT $1 OFFSET $2")]
    async fn limit_offset(&self, limit: u8, offset: u16) -> Result<Vec<(i32, u8)>>;

    // Multiple nullable columns in tuple
    #[dml("SELECT birth_year, name, email FROM users WHERE id = $1")]
    async fn multiple_nullable(
        &self,
        id: i64,
    ) -> Result<(Option<u16>, Option<String>, Option<String>)>;

    // Boolean-like casting in tuple
    #[dml("SELECT (age > 18) as is_adult, name FROM users")]
    async fn boolean_cast(&self) -> Result<Vec<(u8, String)>>;

    // Complex ORDER BY in tuple
    #[dml(
        "SELECT id, age FROM users ORDER BY CASE WHEN birth_year IS NULL THEN 1 ELSE 0 END, age DESC"
    )]
    async fn complex_order(&self) -> Result<Vec<(i32, u8)>>;

    // Complex GROUP BY with HAVING returning tuple
    #[dml(
        "SELECT birth_year, CAST(AVG(age) AS REAL) as avg_age FROM users WHERE birth_year IS NOT NULL GROUP BY birth_year HAVING CAST(AVG(age) AS REAL) > $1 "
    )]
    async fn group_having_avg(&self, min_avg: f32) -> Result<Vec<(Option<u16>, Option<f32>)>>;

    // Complex CASE with casting in tuple
    #[dml(
        "SELECT CASE WHEN age < 20 THEN 1 WHEN age < 40 THEN 2 ELSE 3 END as age_group, COUNT(*) as count FROM users GROUP BY age_group"
    )]
    async fn case_group_count(&self) -> Result<Vec<(u8, u32)>>;

    // Hexadecimal and binary operations in tuple
    #[dml("SELECT (age & $1) as bitwise_and, (age | $2) as bitwise_or FROM users WHERE id = $3")]
    async fn bitwise_ops(&self, mask1: u8, mask2: u8, id: i64) -> Result<(Option<u8>, Option<u8>)>;

    // Edge case: empty result with casting in tuple
    #[dml("SELECT age, name FROM users WHERE 1=0", unchecked)]
    async fn empty_result_cast(&self) -> Result<Vec<(u8, String)>>;

    // Mixed: tuple with some explicit, some implicit casting
    #[dml("SELECT COUNT(*) as 'explicit_count: u32', MAX(age) as implicit_max FROM users")]
    async fn mixed_casting_tuple(&self) -> Result<(u32, Option<u8>)>;

    // === TUPLE-BASED PAGINATION TESTS ===

    // FilterParams with tuple return in pagination
    #[dml("SELECT id, name FROM users")]
    async fn find_by_ids_filter(&self, ids: FilterParams) -> Result<Serial<(i32, String)>>;
}

// Test implementation
pub struct TupleTestApp {
    pool: Pool<sqlx::Sqlite>,
}

impl TupleRepo for TupleTestApp {
    fn get_pool(&self) -> &sqlx::Pool<sqlx::Sqlite> {
        &self.pool
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::SqlitePool;

    async fn setup_test_db() -> SqlitePool {
        // In-memory SQLite database - much faster for tests
        let pool = SqlitePool::connect(":memory:").await.unwrap();

        // Create test table
        sqlx::query(
            r#"
            CREATE TABLE users (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                email TEXT NOT NULL,
                age INTEGER NOT NULL,
                birth_year INTEGER
            )
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();

        // Insert test data
        sqlx::query("INSERT INTO users (id, name, email, age, birth_year) VALUES (1, 'Alice', 'alice@example.com', 30, 1993)")
            .execute(&pool)
            .await
            .unwrap();

        sqlx::query("INSERT INTO users (id, name, email, age, birth_year) VALUES (2, 'Bob', 'bob@example.com', 25, 1998)")
            .execute(&pool)
            .await
            .unwrap();

        // Insert user with NULL birth_year
        sqlx::query("INSERT INTO users (id, name, email, age, birth_year) VALUES (3, 'Charlie', 'charlie@example.com', 35, NULL)")
            .execute(&pool)
            .await
            .unwrap();

        pool
    }

    #[tokio::test]
    async fn test_tuple_return_type() {
        let pool = setup_test_db().await;
        let repo = TupleTestApp { pool };

        // Test tuple return type with compile-time validation
        let result = repo.get_all_ages().await;
        assert!(result.is_ok());

        let tuples = result.unwrap();
        assert_eq!(tuples.len(), 2); // LIMIT 2

        // Verify data types and values - use references to avoid moving String
        let (first_id, first_age, first_name) = &tuples[0];
        let (second_id, second_age, second_name) = &tuples[1];

        assert_eq!(*first_id, 1); // Alice
        assert_eq!(*first_age, 30); // Alice's age (i64 -> i32 casting)
        assert_eq!(first_name, "Alice"); // Alice's name

        assert_eq!(*second_id, 2); // Bob
        assert_eq!(*second_age, 25); // Bob's age (i64 -> i32 casting)
        assert_eq!(second_name, "Bob"); // Bob's name
    }

    #[tokio::test]
    async fn test_optimized_tuple_no_casting() {
        let pool = setup_test_db().await;
        let repo = TupleTestApp { pool };

        // Test tuple with no casting needed - should use optimized path
        let result = repo.get_id_names().await;
        assert!(result.is_ok());

        let tuples = result.unwrap();
        assert_eq!(tuples.len(), 2); // LIMIT 2

        // Verify data types and values - i64 and String don't need casting
        let (first_id, first_name) = &tuples[0];
        let (second_id, second_name) = &tuples[1];

        assert_eq!(*first_id, 1); // Alice
        assert_eq!(first_name, "Alice");

        assert_eq!(*second_id, 2); // Bob
        assert_eq!(second_name, "Bob");
    }

    #[tokio::test]
    async fn test_single_tuple_return() {
        let pool = setup_test_db().await;
        let repo = TupleTestApp { pool };

        // Test single tuple return type with casting
        let result = repo.get_one_age().await;
        assert!(result.is_ok());

        let (id, age, name) = result.unwrap();

        // Should return the first row from the database
        assert_eq!(id, 1); // Alice's ID
        assert_eq!(age, 30); // Alice's age (i64 -> i32 casting)
        assert_eq!(name, "Alice"); // Alice's name
    }

    #[tokio::test]
    async fn test_single_tuple_with_nullable() {
        let pool = setup_test_db().await;
        let repo = TupleTestApp { pool };

        // Test Charlie (id=3) who has NULL birth_year
        let result = repo.get_one_birth(3).await;
        assert!(result.is_ok());

        let (name, birth_year) = result.unwrap();

        assert_eq!(name, "Charlie"); // Charlie's name
        assert_eq!(birth_year, None); // birth_year is NULL, should be None

        // Test Alice (id=1) who has non-NULL birth_year
        let result = repo.get_one_birth(1).await;
        assert!(result.is_ok());

        let (name, birth_year) = result.unwrap();

        assert_eq!(name, "Alice"); // Alice's name
        assert_eq!(birth_year, Some(1993)); // Alice's birth_year
    }

    #[tokio::test]
    async fn test_option_tuple_with_casting() {
        let pool = setup_test_db().await;
        let repo = TupleTestApp { pool };

        // Test Charlie (id=3) who has NULL birth_year - should cast None correctly
        let result = repo.get_one_birth_with_cast(3).await;
        assert!(result.is_ok());

        let (name, birth_year) = result.unwrap();

        assert_eq!(name, "Charlie"); // Charlie's name
        assert_eq!(birth_year, None); // birth_year is NULL, should be None (no casting needed for None)

        // Test Alice (id=1) who has non-NULL birth_year - should cast i64->i16
        let result = repo.get_one_birth_with_cast(1).await;
        assert!(result.is_ok());

        let (name, birth_year) = result.unwrap();

        assert_eq!(name, "Alice"); // Alice's name
        assert_eq!(birth_year, Some(1993u16)); // Alice's birth_year casted from i64 to u16
    }

    #[tokio::test]
    async fn test_type_overrides() {
        let pool = setup_test_db().await;
        let repo = TupleTestApp { pool };

        // Test type overrides with COUNT and AVG
        let result = repo.average_age().await;
        assert!(result.is_ok());

        let (count, average) = result.unwrap();

        // COUNT should be i32 (type override from i64)
        assert_eq!(count, 3i32); // 3 users in test data

        // AVG should be Option<f64>
        assert!(average.is_some());
        assert!(average.unwrap() > 0.0);
    }

    #[tokio::test]
    async fn test_window_function() {
        let pool = setup_test_db().await;
        let repo = TupleTestApp { pool };

        // Test window function with ROW_NUMBER() OVER
        let result = repo.window_function().await;
        assert!(result.is_ok());

        let rows = result.unwrap();
        assert_eq!(rows.len(), 3); // Alice, Bob, Charlie

        // Should be ordered by age: Bob(25), Alice(30), Charlie(35)
        // With row numbers: 1, 2, 3
        assert_eq!(rows[0], (2, 25, 1)); // Bob: id=2, age=25, rank=1
        assert_eq!(rows[1], (1, 30, 2)); // Alice: id=1, age=30, rank=2
        assert_eq!(rows[2], (3, 35, 3)); // Charlie: id=3, age=35, rank=3

        // Option 1: Direct destructuring with if let
        if let Some((id, age, rank)) = rows.first() {
            println!("First user: id={}, age={}, rank={}", id, age, rank);
        }

        // Option 2: Using iterator with destructuring
        for (id, age, rank) in &rows {
            println!("User: id={}, age={}, rank={}", id, age, rank);
        }

        // Option 3: With enumerate for automatic index
        for (index, (id, age, rank)) in rows.iter().enumerate() {
            println!("#{}: User id={}, age={}, rank={}", index + 1, id, age, rank);
        }

        // Option 4: Map + destructuring
        let user_info: Vec<String> = rows
            .iter()
            .map(|(id, age, rank)| format!("User {}: age {}, rank {}", id, age, rank))
            .collect();
        println!("User info: {:?}", user_info);

        // Option 5: Pattern matching with slice
        match rows.as_slice() {
            [
                (id1, _age1, _rank1),
                (id2, _age2, _rank2),
                (id3, _age3, _rank3),
            ] => {
                println!("Bob: {}, Alice: {}, Charlie: {}", id1, id2, id3);
            }
            _ => panic!("Expected exactly 3 users"),
        }

        // Verify casting worked: i64->i32 for id, i64->u8 for age, i64->u16 for rank
        for (id, age, rank) in &rows {
            assert!(*id > 0);
            assert!(*age > 0 && *age < 255); // u8 range
            assert!(*rank > 0 && *rank <= 3); // u16 range, should be 1-3
        }
    }

    #[tokio::test]
    async fn test_in_clause_with_filter_params() {
        use sqlx_data::FilterBuilder;

        let pool = setup_test_db().await;
        let repo = TupleTestApp { pool };

        // Test IN clause with FilterBuilder - multiple ids
        let filters = FilterBuilder::new().r#in("id", vec![1, 3]).build();

        let result = repo.find_by_ids_filter(filters).await;
        assert!(result.is_ok());

        let users = result.unwrap();
        println!("Users found: {:?}", users);
        assert_eq!(users.data.len(), 2);

        // Should find Alice (id=1) and Charlie (id=3)
        assert!(
            users
                .data
                .iter()
                .any(|(id, name)| *id == 1 && name == "Alice")
        );
        assert!(
            users
                .data
                .iter()
                .any(|(id, name)| *id == 3 && name == "Charlie")
        );

        // Test with single ID
        let single_filters = FilterBuilder::new().r#in("id", vec![2]).build();

        let single_filter = single_filters;

        let single_result = repo.find_by_ids_filter(single_filter).await;
        assert!(single_result.is_ok());
        let single_users = single_result.unwrap();
        assert_eq!(single_users.data.len(), 1);
        assert_eq!(single_users.data[0], (2, "Bob".to_string()));
    }

    #[tokio::test]
    async fn test_mixed_casting_tuple() {
        let pool = setup_test_db().await;
        let repo = TupleTestApp { pool };

        // Test tuple with mixed explicit and implicit casting
        let (explicit_count, implicit_max) = repo.mixed_casting_tuple().await.unwrap();

        // explicit_count has 'explicit_count: u32' - no additional casting should be done
        assert_eq!(explicit_count, 3u32);

        // implicit_max has no explicit cast - automatic casting should be applied
        assert!(implicit_max.is_some());
        assert_eq!(implicit_max.unwrap(), 35u8); // Charlie's age is max
    }

    #[tokio::test]
    async fn test_stats_all_types() {
        let pool = setup_test_db().await;
        let repo = TupleTestApp { pool };

        // Test aggregation functions in tuple
        let result = repo.stats_all_types().await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), (Some(25), Some(35), 3, Some(90))); // MIN, MAX, COUNT, SUM
    }

    #[tokio::test]
    async fn test_complex_tuple_operations() {
        let pool = setup_test_db().await;
        let repo = TupleTestApp { pool };

        // Test COALESCE in tuple
        let result = repo.coalesce_nullable(1).await.unwrap();
        assert_eq!(result, (1993u16, "Alice".to_string()));

        // Test string functions in tuple
        let result = repo.string_functions().await.unwrap();
        assert!(result.0.is_some()); // name length
        assert!(result.1.is_some()); // email upper

        // Test bitwise operations in tuple
        let result = repo.bitwise_ops(0b1111, 0b0001, 1).await.unwrap();
        assert!(result.0.is_some()); // age & mask1
        assert!(result.1.is_some()); // age | mask2
    }

    #[tokio::test]
    async fn test_nullable_tuple_handling() {
        let pool = setup_test_db().await;
        let repo = TupleTestApp { pool };

        // Test NULL handling in tuple - Charlie has NULL birth_year
        let result = repo.null_handling_tuple().await.unwrap();
        assert_eq!(result.len(), 1); // Only Charlie matches WHERE birth_year IS NULL

        let (name, birth_year, email) = &result[0];
        assert_eq!(name, "Charlie");
        assert_eq!(*birth_year, None);
        assert_eq!(email, "charlie@example.com");

        // Test multiple nullable columns
        let result = repo.multiple_nullable(3).await.unwrap();
        assert_eq!(result.0, None); // birth_year is NULL for Charlie
        assert!(result.1.is_some()); // name should be Some
        assert!(result.2.is_some()); // email should be Some
    }

    #[tokio::test]
    async fn test_pagination_and_ordering_tuples() {
        let pool = setup_test_db().await;
        let repo = TupleTestApp { pool };

        // Test LIMIT and OFFSET
        let result = repo.limit_offset(2, 0).await.unwrap();
        assert_eq!(result.len(), 2);

        // Should be ordered by age DESC: Charlie(35), Alice(30)
        assert_eq!(result[0].1, 35); // Charlie's age
        assert_eq!(result[1].1, 30); // Alice's age

        // Test complex ordering
        let result = repo.complex_order().await.unwrap();
        assert_eq!(result.len(), 3);
        // Verify the complex ORDER BY works (NULL birth_years first, then age DESC)
    }
}
