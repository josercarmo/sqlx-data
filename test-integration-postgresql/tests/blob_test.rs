use sqlx_data::{Pool, QueryResult, Result, dml, repo};

// PostgreSQL blob repository using BYTEA type
#[repo]
trait BlobRepo {
    // INSERT queries with PostgreSQL BYTEA
    #[dml("INSERT INTO files (name, content, size) VALUES ($1, $2, $3)")]
    async fn insert_file(&self, name: String, content: Vec<u8>, size: i32) -> Result<QueryResult>;

    #[dml("INSERT INTO files (name, content, size) VALUES ($1, $2, $3) RETURNING id")]
    async fn insert_file_returning_id(&self, name: String, content: Vec<u8>, size: i32) -> Result<i64>;

    // SELECT queries with BYTEA
    #[dml("SELECT id, name, content, size FROM files WHERE id = $1")]
    async fn get_file(&self, id: i64) -> Result<Option<(i64, String, Option<Vec<u8>>, i32)>>;

    #[dml("SELECT content FROM files WHERE id = $1")]
    async fn get_file_content(&self, id: i64) -> Result<Option<Vec<u8>>>;

    #[dml("SELECT name, size FROM files WHERE id = $1")]
    async fn get_file_metadata(&self, id: i64) -> Result<Option<(String, i32)>>;

    // BYTEA functions and operations
    #[dml("SELECT LENGTH(content) as content_length FROM files WHERE id = $1")]
    async fn get_content_length(&self, id: i64) -> Result<Option<i32>>;

    #[dml("SELECT SUBSTRING(content FROM 1 FOR $2) as content_prefix FROM files WHERE id = $1")]
    async fn get_content_prefix(&self, id: i64, length: i32) -> Result<Option<Vec<u8>>>;

    // PostgreSQL-specific BYTEA operations
    #[dml("SELECT content || $2 as extended_content FROM files WHERE id = $1")]
    async fn append_content(&self, id: i64, append_data: Vec<u8>) -> Result<Option<Vec<u8>>>;

    // Search operations
    #[dml("SELECT id, name FROM files WHERE size > $1 ORDER BY size DESC")]
    async fn find_large_files(&self, min_size: i32) -> Result<Vec<(i64, String)>>;

    #[dml("SELECT id, name FROM files WHERE content IS NOT NULL")]
    async fn find_files_with_content(&self) -> Result<Vec<(i64, String)>>;

    // PostgreSQL BYTEA specific queries
    #[dml("SELECT COUNT(*) as \"count!: i64\" FROM files WHERE LENGTH(content) = size")]
    async fn count_files_with_matching_size(&self) -> Result<i64>;

    // UPDATE operations with BYTEA
    #[dml("UPDATE files SET content = $2, size = LENGTH($2::bytea) WHERE id = $1")]
    async fn update_file_content(&self, id: i64, content: Vec<u8>) -> Result<QueryResult>;

    #[dml("UPDATE files SET name = $2 WHERE id = $1")]
    async fn update_file_name(&self, id: i64, name: String) -> Result<QueryResult>;

    // DELETE operations
    #[dml("DELETE FROM files WHERE id = $1")]
    async fn delete_file(&self, id: i64) -> Result<QueryResult>;

    #[dml("DELETE FROM files WHERE size = 0")]
    async fn delete_empty_files(&self) -> Result<QueryResult>;

    // PostgreSQL-specific: using BYTEA with aggregation
    #[dml("SELECT SUM(size) as total_size FROM files")]
    async fn get_total_file_size(&self) -> Result<Option<i64>>;

    #[dml("SELECT AVG(LENGTH(content))::INTEGER as avg_content_length FROM files WHERE content IS NOT NULL")]
    async fn get_average_content_length(&self) -> Result<Option<i32>>;
}

pub struct BlobApp {
    pool: Pool,
}

impl BlobRepo for BlobApp {
    fn get_pool(&self) -> &Pool {
        &self.pool
    }
}

#[allow(clippy::unwrap_used)]
#[cfg(test)]
mod tests {
    use super::*;

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_insert_and_retrieve_bytea(pool: Pool) {
        let app = BlobApp { pool };

        // Test with text content
        let text_content = b"Hello, PostgreSQL BYTEA world!";
        let file_id = app
            .insert_file_returning_id(
                "hello.txt".to_string(),
                text_content.to_vec(),
                text_content.len() as i32,
            )
            .await
            .unwrap();

        let file = app.get_file(file_id).await.unwrap().unwrap();
        assert_eq!(file.1, "hello.txt");
        assert_eq!(file.2, Some(text_content.to_vec()));
        assert_eq!(file.3, text_content.len() as i32);

        // Test retrieving just content
        let content = app.get_file_content(file_id).await;
        assert_eq!(content.unwrap(), Some(text_content.to_vec()));
    }

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_binary_data_operations(pool: Pool) {
        let app = BlobApp { pool };

        // Test with binary data (PNG-like header)
        let binary_data = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00];

        let file_id = app
            .insert_file_returning_id(
                "test.png".to_string(),
                binary_data.clone(),
                binary_data.len() as i32,
            )
            .await
            .unwrap();

        // Verify exact binary content
        let retrieved_content = app.get_file_content(file_id).await.unwrap().unwrap();
        assert_eq!(retrieved_content, binary_data);

        // Test content length
        let length = app.get_content_length(file_id).await.unwrap().unwrap();
        assert_eq!(length, binary_data.len() as i32);

        // Test content prefix
        let prefix = app.get_content_prefix(file_id, 4).await.unwrap().unwrap();
        assert_eq!(prefix, &binary_data[0..4]);
    }

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_postgresql_bytea_operations(pool: Pool) {
        let app = BlobApp { pool };

        let original_data = b"Original content";
        let append_data = b" + appended";

        let file_id = app
            .insert_file_returning_id(
                "append_test.txt".to_string(),
                original_data.to_vec(),
                original_data.len() as i32,
            )
            .await
            .unwrap();

        // Test PostgreSQL BYTEA concatenation
        let extended = app
            .append_content(file_id, append_data.to_vec())
            .await
            .unwrap()
            .unwrap();

        let expected = [original_data.as_ref(), append_data.as_ref()].concat();
        assert_eq!(extended, expected);
    }

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_large_file_operations(pool: Pool) {
        let app = BlobApp { pool };

        // Create test files with different sizes
        let small_file = vec![0u8; 100];
        let medium_file = vec![1u8; 1000];
        let large_file = vec![2u8; 10000];

        app.insert_file("small.bin".to_string(), small_file, 100).await.unwrap();
        app.insert_file("medium.bin".to_string(), medium_file, 1000).await.unwrap();
        app.insert_file("large.bin".to_string(), large_file, 10000).await.unwrap();

        // Find large files
        let large_files = app.find_large_files(500).await.unwrap();
        assert_eq!(large_files.len(), 2); // medium and large

        // Verify ordering (DESC by size)
        assert_eq!(large_files[0].1, "large.bin");
        assert_eq!(large_files[1].1, "medium.bin");

        // Test total size
        let total_size = app.get_total_file_size().await.unwrap().unwrap();
        assert_eq!(total_size, 11100); // 100 + 1000 + 10000
    }

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_bytea_with_nulls_and_special_chars(pool: Pool) {
        let app = BlobApp { pool };

        // Test data with null bytes and special characters
        let special_data = vec![0x00, 0xFF, 0x00, 0x7F, 0x80, 0x00, 0x01, 0xFE];

        let file_id = app
            .insert_file_returning_id(
                "special.bin".to_string(),
                special_data.clone(),
                special_data.len() as i32,
            )
            .await
            .unwrap();

        // Verify null bytes are preserved
        let retrieved = app.get_file_content(file_id).await.unwrap().unwrap();
        assert_eq!(retrieved, special_data);
        assert_eq!(retrieved[0], 0x00);
        assert_eq!(retrieved[1], 0xFF);
        assert_eq!(retrieved[5], 0x00);
    }

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_bytea_update_operations(pool: Pool) {
        let app = BlobApp { pool };

        let original_content = b"Original file content";
        let updated_content = b"Updated file content with more data";

        let file_id = app
            .insert_file_returning_id(
                "update_test.txt".to_string(),
                original_content.to_vec(),
                original_content.len() as i32,
            )
            .await
            .unwrap();

        // Update content and verify size is automatically updated
        let update_result = app
            .update_file_content(file_id, updated_content.to_vec())
            .await
            .unwrap();
        assert_eq!(update_result.rows_affected(), 1);

        // Verify updated content and size
        let (_, name, content, size) = app.get_file(file_id).await.unwrap().unwrap();
        assert_eq!(name, "update_test.txt");
        assert_eq!(content, Some(updated_content.to_vec()));
        assert_eq!(size, updated_content.len() as i32);

        // Update just the name
        let name_result = app
            .update_file_name(file_id, "renamed_file.txt".to_string())
            .await
            .unwrap();
        assert_eq!(name_result.rows_affected(), 1);

        let metadata = app.get_file_metadata(file_id).await.unwrap().unwrap();
        assert_eq!(metadata.0, "renamed_file.txt");
        assert_eq!(metadata.1, updated_content.len() as i32);
    }

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_bytea_search_and_aggregation(pool: Pool) {
        let app = BlobApp { pool };

        // Create files with various characteristics
        app.insert_file("file1.txt".to_string(), b"short".to_vec(), 5).await.unwrap();
        app.insert_file("file2.txt".to_string(), b"medium length content".to_vec(), 21).await.unwrap();
        app.insert_file("file3.txt".to_string(), b"".to_vec(), 0).await.unwrap(); // Empty file

        // Test finding files with content
        let files_with_content = app.find_files_with_content().await.unwrap();
        assert_eq!(files_with_content.len(), 3); // All files have content (even empty)

        // Test count with matching sizes
        let matching_count = app.count_files_with_matching_size().await.unwrap();
        assert_eq!(matching_count, 3); // All files should have matching size

        // Test average content length
        let avg_length = app.get_average_content_length().await.unwrap();
        assert!(avg_length.is_some());
        let avg = avg_length.unwrap();
        assert_eq!(avg, 9); // AVG(5, 21, 0) = 8.67 → rounds to 9

        // Delete empty files
        let delete_result = app.delete_empty_files().await.unwrap();
        assert_eq!(delete_result.rows_affected(), 1); // Only file3.txt

        // Verify only 2 files remain
        let remaining_files = app.find_files_with_content().await.unwrap();
        assert_eq!(remaining_files.len(), 2);
    }

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_bytea_edge_cases(pool: Pool) {
        let app = BlobApp { pool };

        // Test empty content
        let empty_id = app
            .insert_file_returning_id("empty.txt".to_string(), vec![], 0)
            .await
            .unwrap();

        let empty_content = app.get_file_content(empty_id).await.unwrap();
        assert_eq!(empty_content, Some(Vec::<u8>::new()));

        // Test very large content (1MB)
        let large_content = vec![0xAAu8; 1024 * 1024];
        let large_id = app
            .insert_file_returning_id(
                "large.bin".to_string(),
                large_content.clone(),
                large_content.len() as i32,
            )
            .await
            .unwrap();

        let retrieved_large = app.get_file_content(large_id).await.unwrap().unwrap();
        assert_eq!(retrieved_large.len(), 1024 * 1024);
        assert_eq!(retrieved_large[0], 0xAA);
        assert_eq!(retrieved_large[1024 * 1024 - 1], 0xAA);

        // Test content prefix with large file
        let prefix = app.get_content_prefix(large_id, 10).await.unwrap().unwrap();
        assert_eq!(prefix, vec![0xAAu8; 10]);
    }

    #[sqlx::test(migrations = "tests/migrations")]
    async fn test_postgresql_bytea_encoding_handling(pool: Pool) {
        let app = BlobApp { pool };

        // Test data that could be problematic with different encodings
        let utf8_bytes = "Hello, 世界! 🦀".as_bytes();
        let latin1_like = vec![0xE0, 0xE1, 0xE2, 0xE3, 0xE4, 0xE5]; // Invalid UTF-8
        let mixed_data = [utf8_bytes, &latin1_like].concat();

        let file_id = app
            .insert_file_returning_id(
                "encoding_test.bin".to_string(),
                mixed_data.clone(),
                mixed_data.len() as i32,
            )
            .await
            .unwrap();

        // PostgreSQL BYTEA should preserve exact bytes regardless of encoding
        let retrieved = app.get_file_content(file_id).await.unwrap().unwrap();
        assert_eq!(retrieved, mixed_data);

        // Verify UTF-8 portion is intact
        let utf8_portion = &retrieved[0..utf8_bytes.len()];
        assert_eq!(std::str::from_utf8(utf8_portion).unwrap(), "Hello, 世界! 🦀");

        // Verify binary portion is intact
        let binary_portion = &retrieved[utf8_bytes.len()..];
        assert_eq!(binary_portion, &latin1_like);
    }
}