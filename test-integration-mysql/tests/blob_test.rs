use bytes::Bytes;
use sqlx_data::{
    IntoParams, ParamsBuilder, QueryResult, Result, Serial, SerialParams, Slice, SliceParams,
};
use sqlx_data::{dml, repo, Pool};

#[derive(Debug, sqlx::FromRow)]
pub struct FileView {
    pub id: u32,                                    // MySQL INT UNSIGNED
    pub name: String,
    pub content_type: String,
    pub file_size: u32,                            // MySQL INT UNSIGNED for file size
    #[sqlx(try_from = "Vec<u8>")]                 // Convert from Vec<u8> to Bytes
    pub data: Bytes,
    pub is_compressed: bool,                       // MySQL BOOLEAN (TINYINT(1))
}

// Especific blob type alias
pub type Blob = Vec<u8>;

#[repo]
trait FileRepo {
    // Create operations with MySQL-specific types
    #[dml("INSERT INTO files (name, content_type, file_size, data, is_compressed) VALUES (?, ?, ?, ?, ?)")]
    async fn create_file_with_into(
        &self,
        name: String,
        content_type: impl Into<String>,
        file_size: u32,
        data: impl Into<Bytes>,
        is_compressed: bool,
    ) -> Result<QueryResult>;

    #[dml("INSERT INTO files (name, content_type, file_size, data, is_compressed) VALUES (?, ?, ?, ?, ?)")]
    async fn create_file_with_vec(
        &self,
        name: String,
        content_type: String,
        file_size: u32,
        data: Vec<u8>,
        is_compressed: bool,
    ) -> Result<QueryResult>;

    #[dml("INSERT INTO files (name, content_type, file_size, data, is_compressed) VALUES (?, ?, ?, ?, ?)")]
    async fn create_file_with_bytes(
        &self,
        name: String,
        content_type: String,
        file_size: u32,
        data: Bytes,
        is_compressed: bool,
    ) -> Result<QueryResult>;

    #[dml("INSERT INTO files (name, content_type, file_size, data, is_compressed) VALUES (?, ?, ?, ?, ?)")]
    async fn create_file_no_return(
        &self,
        name: String,
        content_type: String,
        file_size: u32,
        data: Vec<u8>,
        is_compressed: bool,
    ) -> Result<QueryResult>;

    // Update operations
    #[dml("UPDATE files SET name = ?, content_type = ?, file_size = ?, data = ?, is_compressed = ? WHERE id = ?")]
    async fn update_file_with_vec(
        &self,
        name: String,
        content_type: String,
        file_size: u32,
        data: Vec<u8>,
        is_compressed: bool,
        id: u32,
    ) -> Result<QueryResult>;

    #[dml("UPDATE files SET name = ?, content_type = ?, file_size = ?, data = ?, is_compressed = ? WHERE id = ?")]
    async fn update_file_with_bytes(
        &self,
        name: String,
        content_type: String,
        file_size: u32,
        data: Bytes,
        is_compressed: bool,
        id: u32,
    ) -> Result<QueryResult>;

    #[dml("UPDATE files SET data = ?, file_size = ? WHERE id = ?")]
    async fn update_file_data_vec(&self, data: Vec<u8>, file_size: u32, id: u32) -> Result<QueryResult>;

    #[dml("UPDATE files SET data = ?, file_size = ? WHERE id = ?")]
    async fn update_file_data_bytes(&self, data: Bytes, file_size: u32, id: u32) -> Result<QueryResult>;

    // Read operations with MySQL unsigned types
    #[dml("SELECT id as 'id: u32', name, content_type, file_size as 'file_size: u32', data, is_compressed as 'is_compressed: bool' FROM files ORDER BY id")]
    async fn find_files_serial(&self, params: SerialParams) -> Result<Serial<FileView>>;

    #[dml("SELECT id as 'id: u32', name, content_type, file_size as 'file_size: u32', data, is_compressed as 'is_compressed: bool' FROM files ORDER BY id")]
    async fn find_files_slice(&self, params: SliceParams) -> Result<Slice<FileView>>;

    #[dml("SELECT id as 'id: u32', name, content_type, file_size as 'file_size: u32', data, is_compressed as 'is_compressed: bool' FROM files ORDER BY id")]
    async fn find_files_builder(&self, params: impl IntoParams) -> Result<Slice<FileView>>;

    // Tuple queries with MySQL unsigned types
    #[dml("SELECT id as 'id: u32', name, content_type, file_size as 'file_size: u32', data, is_compressed as 'is_compressed: bool' FROM files ORDER BY id")]
    async fn find_files_tuple(&self, params: SerialParams) -> Result<Serial<(u32, String, String, u32, Vec<u8>, bool)>>;

    #[dml("SELECT id as 'id: u32', name, content_type, file_size as 'file_size: u32', data, is_compressed as 'is_compressed: bool' FROM files ORDER BY id LIMIT 10")]
    async fn find_all_files(&self) -> Result<Vec<FileView>>;

    // MySQL-specific BLOB functions
    #[dml("SELECT id as 'id: u32', LENGTH(data) as 'actual_size: u32' FROM files ORDER BY id LIMIT 10")]
    async fn find_many_files_with_length(&self) -> Result<Vec<(u32, u32)>>;

    #[dml("SELECT data FROM files ORDER BY id LIMIT 10")]
    async fn find_many_files_vec(&self) -> Result<Vec<Blob>>;

    #[dml("SELECT id as 'id: u32', data FROM files ORDER BY id LIMIT 1")]
    async fn find_one_file(&self) -> Result<(u32, Vec<u8>)>;

    #[dml("SELECT data FROM files ORDER BY id LIMIT 1")]
    async fn find_one_file_blob(&self) -> Result<Blob>;

    // MySQL-specific HEX functions for binary data
    #[dml("SELECT id as 'id: u32', HEX(LEFT(data, 8)) as hex_header FROM files WHERE id = ?")]
    async fn get_file_hex_header(&self, id: u32) -> Result<(u32, Option<String>)>;

    // MySQL compression test (if available)
    #[dml("SELECT id as 'id: u32', name, is_compressed as 'is_compressed: bool' FROM files WHERE is_compressed = ?")]
    async fn find_files_by_compression(&self, is_compressed: bool) -> Result<Vec<(u32, String, bool)>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("files"))
    )]
    async fn test_mysql_blob_serial_pagination(pool: Pool) {
        let repo = TestFileRepo { pool: &pool };

        // Serial with page_size = 2, should fetch exactly 2 files with BLOBs
        let params = SerialParams::new(1, 2);
        let page = repo.find_files_serial(params).await.unwrap();

        println!(
            "MySQL Serial BLOB: Page {}, Size {}, Total {}, Pages {}",
            page.page, page.size, page.total_items, page.total_pages
        );
        assert_eq!(page.size, 2);
        assert_eq!(page.total_items, 3); // We have 3 files in total
        assert_eq!(page.data.len(), 2); // Returns exactly 2 (no +1)

        // Verify that BLOBs were loaded correctly with MySQL unsigned types
        for file in &page.data {
            assert!(!file.data.is_empty(), "BLOB data should not be empty");
            assert!(file.id > 0, "ID should be positive u32");
            assert!(file.file_size > 0, "File size should be positive u32");
            assert!(
                file.name.starts_with("file"),
                "File name should start with 'file'"
            );
            println!(
                "MySQL File: {} ({}) - {} bytes, size: {}, compressed: {}",
                file.name, file.content_type, file.data.len(), file.file_size, file.is_compressed
            );
        }
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("files"))
    )]
    async fn test_mysql_blob_slice_pagination(pool: Pool) {
        let repo = TestFileRepo { pool: &pool };

        // Slice with page_size = 2, should fetch 3 items (2 + 1) to detect has_next
        let params = SliceParams::new(1, 2);
        let page = repo.find_files_slice(params).await.unwrap();

        println!(
            "MySQL Slice BLOB: Page {}, Size {}, HasNext {}",
            page.page, page.size, page.has_next
        );
        assert_eq!(page.size, 2);
        assert_eq!(page.data.len(), 2); // Returns 2 (the +1 is removed if has_next=true)
        assert!(page.has_next); // Should have next page (3 total, requested 2, 1 remaining)

        // Verify MySQL-specific data types
        for file in &page.data {
            assert!(!file.data.is_empty(), "BLOB data should not be empty");
            assert!(file.id > 0, "ID should be positive u32");
            assert!(file.file_size > 0, "File size should be positive u32");
            println!(
                "MySQL File: {} - {} bytes, actual size: {}, compressed: {}",
                file.name, file.data.len(), file.file_size, file.is_compressed
            );
        }
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("files"))
    )]
    async fn test_mysql_blob_builder_params(pool: Pool) {
        let repo = TestFileRepo { pool: &pool };

        // Builder slice to test with total count disabled
        let params = ParamsBuilder::new().slice().page(1, 2).done().build();

        let page = repo.find_files_builder(params).await.unwrap();

        println!(
            "MySQL Builder BLOB: Page {}, Size {}, HasNext {}, HasPrevious {}, Total {:?}",
            page.page, page.size, page.has_next, page.has_previous, page.total_items
        );
        assert_eq!(page.size, 2);
        assert_eq!(page.data.len(), 2);
        assert!(page.has_next);
        assert!(page.total_items.is_none()); // Slice doesn't count total by default
        assert!(!page.has_previous); // First page should not have previous

        // Verify MySQL unsigned types
        for file in &page.data {
            assert!(!file.data.is_empty(), "BLOB data should not be empty");
            assert!(file.id > 0, "ID should be positive u32");
            assert!(file.file_size > 0, "File size should be positive u32");
        }
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("files"))
    )]
    async fn test_mysql_blob_tuple_pagination(pool: Pool) {
        let repo = TestFileRepo { pool: &pool };

        // Test with tuple - MySQL unsigned types
        let params = SerialParams::new(1, 2);
        let page = repo.find_files_tuple(params).await.unwrap();

        println!(
            "MySQL Tuple BLOB: Page {}, Size {}, Total {}, Pages {}",
            page.page, page.size, page.total_items, page.total_pages
        );
        assert_eq!(page.size, 2);
        assert_eq!(page.total_items, 3);
        assert_eq!(page.data.len(), 2);

        // Verify MySQL unsigned types in tuple
        for (id, name, content_type, file_size, data, is_compressed) in &page.data {
            assert!(!data.is_empty(), "BLOB data should not be empty");
            assert!(*id > 0, "ID should be positive u32");
            assert!(*file_size > 0, "File size should be positive u32");
            assert!(
                name.starts_with("file"),
                "File name should start with 'file'"
            );
            println!(
                "MySQL Tuple - ID: {}, File: {} ({}) - {} bytes, size: {}, compressed: {}",
                id, name, content_type, data.len(), file_size, is_compressed
            );
        }
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("files"))
    )]
    async fn test_mysql_blob_simple_query(pool: Pool) {
        let repo = TestFileRepo { pool: &pool };

        // Simple FileView test without pagination
        let files = repo.find_all_files().await.unwrap();

        assert_eq!(files.len(), 3, "Should return all 3 files");

        for file in &files {
            assert!(!file.data.is_empty(), "BLOB data should not be empty");
            assert!(file.id > 0, "ID should be positive u32");
            assert!(file.file_size > 0, "File size should be positive u32");
            println!(
                "MySQL FileView - ID: {}, File: {} ({}) - {} bytes, size: {}, compressed: {}",
                file.id, file.name, file.content_type, file.data.len(), file.file_size, file.is_compressed
            );

            // Verify that the Vec<u8> -> Bytes conversion worked
            assert!(file.data.len() > 0);
            assert_eq!(file.data.len(), file.file_size as usize, "Data length should match file_size");
        }

        // Verify specific MySQL contents
        let file1 = files.iter().find(|f| f.name == "file1.txt").unwrap();
        let file1_content = std::str::from_utf8(&file1.data).unwrap();
        assert!(file1_content.contains("MySQL content for file 1"));
        assert!(!file1.is_compressed);

        let file2 = files.iter().find(|f| f.name == "file2.json").unwrap();
        let file2_content = std::str::from_utf8(&file2.data).unwrap();
        assert!(file2_content.contains("mysql"));
        assert!(!file2.is_compressed);

        let file3 = files.iter().find(|f| f.name == "file3.bin").unwrap();
        assert_eq!(file3.data.len(), 50); // Binary file with 50 bytes
        assert!(file3.is_compressed);
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("files"))
    )]
    async fn test_mysql_length_function(pool: Pool) {
        let repo = TestFileRepo { pool: &pool };

        // Test MySQL LENGTH() function
        let files_with_length = repo.find_many_files_with_length().await.unwrap();

        assert_eq!(files_with_length.len(), 3, "Should return 3 files with length");

        for (id, actual_size) in &files_with_length {
            assert!(*id > 0, "ID should be positive u32");
            assert!(*actual_size > 0, "Length should be positive u32");
            println!("MySQL File ID: {}, LENGTH(data): {} bytes", id, actual_size);
        }
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("files"))
    )]
    async fn test_mysql_hex_function(pool: Pool) {
        let repo = TestFileRepo { pool: &pool };

        // Test MySQL HEX() function with the first file
        let (id, hex_header) = repo.get_file_hex_header(1).await.unwrap();

        assert_eq!(id, 1, "Should return file with ID 1");
        assert!(hex_header.is_some(), "HEX header should not be None");
        let hex_str = hex_header.unwrap();
        assert!(!hex_str.is_empty(), "HEX header should not be empty");
        println!("MySQL File ID: {}, HEX header: {}", id, hex_str);

        // The hex should represent the first 8 bytes of the text content
        let expected_start = "4D7953514C"; // "MySQL" in hex
        assert!(hex_str.starts_with(expected_start), "HEX should start with 'MySQL' in hex");
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("files"))
    )]
    async fn test_mysql_compression_filter(pool: Pool) {
        let repo = TestFileRepo { pool: &pool };

        // Test filtering by compression status
        let uncompressed_files = repo.find_files_by_compression(false).await.unwrap();
        let compressed_files = repo.find_files_by_compression(true).await.unwrap();

        println!("Uncompressed files: {}", uncompressed_files.len());
        println!("Compressed files: {}", compressed_files.len());

        assert_eq!(uncompressed_files.len(), 2, "Should have 2 uncompressed files");
        assert_eq!(compressed_files.len(), 1, "Should have 1 compressed file");

        for (id, name, is_compressed) in &uncompressed_files {
            assert!(*id > 0, "ID should be positive u32");
            assert!(!is_compressed, "Should be uncompressed");
            println!("Uncompressed - ID: {}, Name: {}", id, name);
        }

        for (id, name, is_compressed) in &compressed_files {
            assert!(*id > 0, "ID should be positive u32");
            assert!(*is_compressed, "Should be compressed");
            println!("Compressed - ID: {}, Name: {}", id, name);
        }
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("files"))
    )]
    async fn test_mysql_create_file_with_vec(pool: Pool) {
        let repo = TestFileRepo { pool: &pool };

        let file_data = vec![0xFF, 0xD8, 0xFF, 0xE0]; // JPEG header
        let file_size = file_data.len() as u32;
        let result = repo
            .create_file_with_vec(
                "test_jpeg.jpg".to_string(),
                "image/jpeg".to_string(),
                file_size,
                file_data.clone(),
                false,
            )
            .await
            .unwrap();

        let file_id = result.last_insert_id() as u32;
        assert!(file_id > 0, "Should return a valid u32 file ID");

        // Verify file was created correctly
        let all_files = repo.find_all_files().await.unwrap();
        let created_file = all_files.iter().find(|f| f.id == file_id).unwrap();
        assert_eq!(created_file.data.to_vec(), file_data);
        assert_eq!(created_file.name, "test_jpeg.jpg");
        assert_eq!(created_file.file_size, file_size);
        assert!(!created_file.is_compressed);
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("files"))
    )]
    async fn test_mysql_create_file_with_bytes(pool: Pool) {
        let repo = TestFileRepo { pool: &pool };

        let file_data = Bytes::from(vec![0x89, 0x50, 0x4E, 0x47]); // PNG header
        let file_size = file_data.len() as u32;
        let result = repo
            .create_file_with_bytes(
                "test_png.png".to_string(),
                "image/png".to_string(),
                file_size,
                file_data.clone(),
                true, // Mark as compressed
            )
            .await
            .unwrap();

        let file_id = result.last_insert_id() as u32;
        assert!(file_id > 0, "Should return a valid u32 file ID");

        // Verify file was created correctly
        let all_files = repo.find_all_files().await.unwrap();
        let created_file = all_files.iter().find(|f| f.id == file_id).unwrap();
        assert_eq!(created_file.data, file_data);
        assert_eq!(created_file.name, "test_png.png");
        assert_eq!(created_file.file_size, file_size);
        assert!(created_file.is_compressed);
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("files"))
    )]
    async fn test_mysql_update_operations(pool: Pool) {
        let repo = TestFileRepo { pool: &pool };

        // Update with Vec<u8>
        let new_data = vec![0x42; 20]; // 20 bytes of 0x42
        let new_size = new_data.len() as u32;
        let result = repo
            .update_file_with_vec(
                "updated_file.bin".to_string(),
                "application/updated".to_string(),
                new_size,
                new_data.clone(),
                true,
                1, // Update file with ID 1
            )
            .await
            .unwrap();

        assert!(result.rows_affected() > 0, "Should update at least one row");

        // Verify the update
        let files = repo.find_all_files().await.unwrap();
        let updated_file = files.iter().find(|f| f.id == 1).unwrap();
        assert_eq!(updated_file.name, "updated_file.bin");
        assert_eq!(updated_file.content_type, "application/updated");
        assert_eq!(updated_file.data.to_vec(), new_data);
        assert_eq!(updated_file.file_size, new_size);
        assert!(updated_file.is_compressed);
    }

    #[sqlx::test(
        migrations = "tests/migrations",
        fixtures(path = "fixtures", scripts("files"))
    )]
    async fn test_mysql_update_data_only(pool: Pool) {
        let repo = TestFileRepo { pool: &pool };

        // Update only data and file_size
        let new_data = Bytes::from("Updated MySQL content".as_bytes().to_vec());
        let new_size = new_data.len() as u32;
        let result = repo
            .update_file_data_bytes(new_data.clone(), new_size, 2)
            .await
            .unwrap();

        assert!(result.rows_affected() > 0, "Should update at least one row");

        // Verify only data and file_size were updated
        let files = repo.find_all_files().await.unwrap();
        let updated_file = files.iter().find(|f| f.id == 2).unwrap();
        assert_eq!(updated_file.name, "file2.json"); // Should remain unchanged
        assert_eq!(updated_file.content_type, "application/json"); // Should remain unchanged
        assert_eq!(updated_file.data, new_data); // Should be updated
        assert_eq!(updated_file.file_size, new_size); // Should be updated
    }

    struct TestFileRepo<'a> {
        pool: &'a Pool,
    }

    impl<'a> FileRepo for TestFileRepo<'a> {
        fn get_pool(&self) -> &Pool {
            self.pool
        }
    }

}