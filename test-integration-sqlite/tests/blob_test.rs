use bytes::Bytes;
use sqlx_data::{
    IntoParams, ParamsBuilder, QueryResult, Result, Serial, SerialParams, Slice, SliceParams,
};
use sqlx_data::{dml, repo};

#[derive(Debug, sqlx::FromRow)]
pub struct FileView {
    pub id: i64,
    pub name: String,
    pub content_type: String,
    #[sqlx(try_from = "Vec<u8>")] // Convert from Vec<u8> to Bytes
    pub data: Bytes,
}

//require to prevent issues working with Vec<u8>
//Vec<u8> could be anything: age list, but can also be a bytes list
pub type Blob = Vec<u8>;

#[repo]
trait FileRepo {
    // Create operations
    //#[dml("INSERT INTO files (name, content_type, data) VALUES ($1, $2, $3) RETURNING id")]
    //async fn create_file_with_batches(&self, name: Vec<String>, content_type: Vec<String>, data: Vec<Bytes>) -> Result<Vec<i64>>;

    #[dml("INSERT INTO files (name, content_type, data) VALUES ($1, $2, $3) RETURNING id")]
    async fn create_file_with_into(
        &self,
        name: String,
        content_type: impl Into<String>,
        data: impl Into<Bytes>,
    ) -> Result<i64>;

    #[dml("INSERT INTO files (name, content_type, data) VALUES ($1, $2, $3) RETURNING id")]
    async fn create_file_with_all(
        &self,
        name: String,
        content_type: impl Into<String>,
        data: impl Into<Bytes>,
    ) -> Result<Vec<i64>>;

    #[dml("INSERT INTO files (name, content_type, data) VALUES ($1, $2, $3) RETURNING id")]
    async fn create_file_with_into_option(
        &self,
        name: String,
        content_type: String,
        data: Option<impl Into<Bytes>>,
    ) -> Result<i64>;

    #[dml("INSERT INTO files (name, content_type, data) VALUES ($1, $2, $3) RETURNING id")]
    async fn create_file_with_vec(
        &self,
        name: String,
        content_type: String,
        data: Vec<u8>,
    ) -> Result<i64>;

    #[dml("INSERT INTO files (name, content_type, data) VALUES ($1, $2, $3) RETURNING id")]
    async fn create_file_with_bytes(
        &self,
        name: String,
        content_type: String,
        data: Bytes,
    ) -> Result<i64>;

    #[dml("INSERT INTO files (name, content_type, data) VALUES ($1, $2, $3)")]
    async fn create_file_vec_no_return(
        &self,
        name: String,
        content_type: String,
        data: Vec<u8>,
    ) -> Result<QueryResult>;

    #[dml("INSERT INTO files (name, content_type, data) VALUES ($1, $2, $3)")]
    async fn create_file_bytes_no_return(
        &self,
        name: String,
        content_type: String,
        data: Bytes,
    ) -> Result<QueryResult>;

    // Update operations
    #[dml("UPDATE files SET name = $2, content_type = $3, data = $4 WHERE id = $1")]
    async fn update_file_with_vec(
        &self,
        id: i64,
        name: String,
        content_type: String,
        data: Vec<u8>,
    ) -> Result<QueryResult>;

    #[dml("UPDATE files SET name = $2, content_type = $3, data = $4 WHERE id = $1")]
    async fn update_file_with_bytes(
        &self,
        id: i64,
        name: String,
        content_type: String,
        data: Bytes,
    ) -> Result<QueryResult>;

    #[dml("UPDATE files SET data = $2 WHERE id = $1")]
    async fn update_file_data_vec(&self, id: i64, data: Vec<u8>) -> Result<QueryResult>;

    #[dml("UPDATE files SET data = $2 WHERE id = $1")]
    async fn update_file_data_bytes(&self, id: i64, data: Bytes) -> Result<QueryResult>;

    // Read operations
    #[dml("SELECT id, name, content_type, data FROM files ORDER BY id")]
    async fn find_files_serial(&self, params: SerialParams) -> Result<Serial<FileView>>;

    #[dml("SELECT id, name, content_type, data FROM files ORDER BY id")]
    async fn find_files_slice(&self, params: SliceParams) -> Result<Slice<FileView>>;

    #[dml("SELECT id, name, content_type, data FROM files ORDER BY id")]
    async fn find_files_builder(&self, params: impl IntoParams) -> Result<Slice<FileView>>;

    //TODO REVIEW: support Bytes in tuple?
    //async fn find_files_tuple(&self, params: SerialParams) -> Result<Serial<(i64, String, String, Bytes)>, sqlx::Error>;❌
    #[dml("SELECT id, name, content_type, data FROM files ORDER BY id")]
    async fn find_files_tuple(
        &self,
        params: SerialParams,
    ) -> Result<Serial<(i64, String, String, Vec<u8>)>>;

    #[dml("SELECT id, name, content_type, data FROM files ORDER BY id LIMIT 10")]
    async fn find_all_files(&self) -> Result<Vec<FileView>>;

    #[dml("SELECT id, data FROM files ORDER BY id LIMIT 10")] // add any field, here we use id
    async fn find_many_files(&self) -> Result<Vec<(i64, Vec<u8>)>>;

    // Test multiple BLOBs - now properly supported as Vec<Vec<u8>>
    #[dml("SELECT data FROM files ORDER BY id LIMIT 10")]
    async fn find_many_files_vec(&self) -> Result<Vec<Blob>>;

    // Test single BLOB - USE TUPLE and not just Vec<u8>!!!!
    #[dml("SELECT id, data FROM files ORDER BY id LIMIT 1")] // add any field, here we use id
    async fn find_one_file(&self) -> Result<(i64, Vec<u8>)>;

    //Just define a pub Blob = Vec<u8>; and everything works fine with Vec<u8>
    #[dml("SELECT data FROM files ORDER BY id LIMIT 1")] // add any field, here we use id
    async fn find_one_file_byte(&self) -> Result<Blob>;

    // Test multiple BLOBs - now properly supported as Vec<Vec<u8>> with blob
    #[dml("SELECT data FROM files ORDER BY id LIMIT 10")]
    async fn find_many_files_blob(&self) -> Result<Vec<Blob>>;
}

#[cfg(test)]
mod tests {
    use sqlx_data::{Pool, SerialParams, SliceParams};

    use super::*;

    #[tokio::test]
    async fn test_blob_serial_pagination() {
        let pool = setup_test_db_with_files().await;
        let repo = TestFileRepo { pool: &pool };

        // Serial with page_size = 2, should fetch exactly 2 files with BLOBs
        let params = SerialParams::new(1, 2);
        let page = repo.find_files_serial(params).await.unwrap();

        println!(
            "Serial BLOB: Page {}, Size {}, Total {}, Pages {}",
            page.page, page.size, page.total_items, page.total_pages
        );
        assert_eq!(page.size, 2);
        assert_eq!(page.total_items, 3); // We have 3 files in total
        assert_eq!(page.data.len(), 2); // Retorna exatamente 2 (sem +1)

        // Verify that BLOBs were loaded correctly
        for file in &page.data {
            assert!(!file.data.is_empty(), "BLOB data should not be empty");
            assert!(
                file.name.starts_with("file"),
                "File name should start with 'file'"
            );
            println!(
                "File: {} ({}) - {} bytes",
                file.name,
                file.content_type,
                file.data.len()
            );
        }
    }

    #[tokio::test]
    async fn test_blob_slice_pagination() {
        let pool = setup_test_db_with_files().await;
        let repo = TestFileRepo { pool: &pool };

        // Slice with page_size = 2, should fetch 3 items (2 + 1) to detect has_next
        let params = SliceParams::new(1, 2);
        let page = repo.find_files_slice(params).await.unwrap();

        println!(
            "Slice BLOB: Page {}, Size {}, HasNext {}",
            page.page, page.size, page.has_next
        );
        assert_eq!(page.size, 2);
        assert_eq!(page.data.len(), 2); // Returns 2 (the +1 is removed if has_next=true)
        assert!(page.has_next); // Should have next page (3 total, requested 2, 1 remaining)

        // Verify that BLOBs were loaded correctly
        for file in &page.data {
            assert!(!file.data.is_empty(), "BLOB data should not be empty");
            let data_preview = if file.data.len() > 10 {
                format!(
                    "{}...",
                    std::str::from_utf8(&file.data[..10]).unwrap_or("binary")
                )
            } else {
                std::str::from_utf8(&file.data[..])
                    .unwrap_or("binary")
                    .to_string()
            };
            println!(
                "File: {} - {} bytes - Preview: '{}'",
                file.name,
                file.data.len(),
                data_preview
            );
        }
    }

    #[tokio::test]
    async fn test_blob_large_files_no_next_page() {
        let pool = setup_test_db_with_files().await;
        let repo = TestFileRepo { pool: &pool };

        // Slice with page_size = 10, more than the 3 available files
        let params = SliceParams::new(1, 10);
        let page = repo.find_files_slice(params).await.unwrap();

        println!(
            "Slice Large BLOB: Page {}, Size {}, HasNext {}",
            page.page, page.size, page.has_next
        );
        assert_eq!(page.size, 10);
        assert_eq!(page.data.len(), 3); // Returns all 3 available files
        assert!(!page.has_next); // No next page

        // Verify that all BLOBs were loaded
        for file in &page.data {
            assert!(!file.data.is_empty(), "BLOB data should not be empty");
            println!("File: {} - {} bytes", file.name, file.data.len());
        }
    }

    #[tokio::test]
    async fn test_blob_builder_params() {
        let pool = setup_test_db_with_files().await;
        let repo = TestFileRepo { pool: &pool };

        // Builder slice to test with total count disabled
        let params = ParamsBuilder::new().slice().page(1, 2).done().build();

        let page = repo.find_files_builder(params).await.unwrap();

        println!(
            "Slice Builder BLOB: Page {}, Size {}, HasNext {}, HasPrevious {}, Total {:?}",
            page.page, page.size, page.has_next, page.has_previous, page.total_items
        );
        assert_eq!(page.size, 2);
        assert_eq!(page.data.len(), 2);
        assert!(page.has_next);
        assert!(page.total_items.is_none()); // Slice doesn't count total by default
        assert!(!page.has_previous); // First page should not have previous

        // Verify BLOBs
        for file in &page.data {
            assert!(!file.data.is_empty(), "BLOB data should not be empty");
            // Different files have different content_types
            assert!(
                file.content_type == "text/plain"
                    || file.content_type == "application/json"
                    || file.content_type == "application/octet-stream"
            );
        }
    }

    #[tokio::test]
    async fn test_blob_tuple_pagination() {
        let pool = setup_test_db_with_files().await;
        let repo = TestFileRepo { pool: &pool };

        // Test with tuple - basic types supported by SQLx
        let params = SerialParams::new(1, 2);
        let page = repo.find_files_tuple(params).await.unwrap();

        println!(
            "Tuple BLOB: Page {}, Size {}, Total {}, Pages {}",
            page.page, page.size, page.total_items, page.total_pages
        );
        assert_eq!(page.size, 2);
        assert_eq!(page.total_items, 3);
        assert_eq!(page.data.len(), 2);

        // Verify that BLOBs were loaded correctly como Vec<u8>
        for (id, name, content_type, data) in &page.data {
            assert!(!data.is_empty(), "BLOB data should not be empty");
            assert!(
                name.starts_with("file"),
                "File name should start with 'file'"
            );
            println!(
                "Tuple - ID: {}, File: {} ({}) - {} bytes",
                id,
                name,
                content_type,
                data.len()
            );
        }
    }

    #[tokio::test]
    async fn test_simple_fileview_query() {
        let pool = setup_test_db_with_files().await;
        let repo = TestFileRepo { pool: &pool };

        // Simple FileView test without pagination
        let files = repo.find_all_files().await.unwrap();

        assert_eq!(files.len(), 3, "Should return all 3 files");

        for file in &files {
            assert!(!file.data.is_empty(), "BLOB data should not be empty");
            assert!(
                file.name.starts_with("file"),
                "File name should start with 'file'"
            );
            println!(
                "Simple FileView - ID: {}, File: {} ({}) - {} bytes",
                file.id,
                file.name,
                file.content_type,
                file.data.len()
            );

            // Verify that the Vec<u8> -> Bytes conversion worked
            assert!(file.data.len() > 0);
        }

        // Verify specific contents
        let file1 = files.iter().find(|f| f.name == "file1.txt").unwrap();
        let file1_content = std::str::from_utf8(&file1.data).unwrap();
        assert!(file1_content.contains("Este e o conteudo do arquivo 1"));

        let file2 = files.iter().find(|f| f.name == "file2.json").unwrap();
        let file2_content = std::str::from_utf8(&file2.data).unwrap();
        assert!(file2_content.contains("message"));

        let file3 = files.iter().find(|f| f.name == "file3.bin").unwrap();
        assert_eq!(file3.data.len(), 100); // Binary file with 100 zero bytes
    }

    #[tokio::test]
    async fn test_many_files_tuple() {
        let pool = setup_test_db_with_files().await;
        let repo = TestFileRepo { pool: &pool };

        // Test Vec<(i64, Vec<u8>)> - multiple BLOBs with ID
        let files = repo.find_many_files().await.unwrap();

        assert_eq!(files.len(), 3, "Should return 3 files");

        for (id, data) in &files {
            assert!(!data.is_empty(), "BLOB data should not be empty");
            assert!(*id > 0, "ID should be positive");
            println!("File ID: {}, Data size: {} bytes", id, data.len());
        }

        // Verify specific file contents
        let file1 = files.iter().find(|(id, _)| *id == 1).unwrap();
        let file1_content = std::str::from_utf8(&file1.1).unwrap();
        assert!(file1_content.contains("Este e o conteudo do arquivo 1"));

        let file2 = files.iter().find(|(id, _)| *id == 2).unwrap();
        let file2_content = std::str::from_utf8(&file2.1).unwrap();
        assert!(file2_content.contains("message"));

        let file3 = files.iter().find(|(id, _)| *id == 3).unwrap();
        assert_eq!(file3.1.len(), 100); // Binary file with 100 zero bytes
    }

    #[tokio::test]
    async fn test_many_files_vec_vec() {
        let pool = setup_test_db_with_files().await;
        let repo = TestFileRepo { pool: &pool };

        // Test Vec<Vec<u8>> - multiple BLOBs (data only)
        let files = repo.find_many_files_vec().await.unwrap();

        assert_eq!(files.len(), 3, "Should return 3 files");

        for (index, data) in files.iter().enumerate() {
            assert!(!data.is_empty(), "BLOB data should not be empty");
            println!("File {}: Data size: {} bytes", index + 1, data.len());
        }

        // Verify specific file contents (files are returned in id order)
        let file1_content = std::str::from_utf8(&files[0]).unwrap();
        assert!(file1_content.contains("Este e o conteudo do arquivo 1"));

        let file2_content = std::str::from_utf8(&files[1]).unwrap();
        assert!(file2_content.contains("message"));

        assert_eq!(files[2].len(), 100); // Binary file with 100 zero bytes
    }

    #[tokio::test]
    async fn test_one_file_tuple() {
        let pool = setup_test_db_with_files().await;
        let repo = TestFileRepo { pool: &pool };

        // Test (i64, Vec<u8>) - single BLOB with ID
        let (id, data) = repo.find_one_file().await.unwrap();

        assert_eq!(id, 1, "Should return first file with ID 1");
        assert!(!data.is_empty(), "BLOB data should not be empty");

        let content = std::str::from_utf8(&data).unwrap();
        assert!(content.contains("Este e o conteudo do arquivo 1"));
        println!("Single file - ID: {}, Content: {}", id, content);
    }

    #[tokio::test]
    async fn test_create_file_with_vec() {
        let pool = setup_test_db_with_files().await;
        let repo = TestFileRepo { pool: &pool };

        let file_data = vec![0u8, 1, 2, 3, 4, 5];
        let file_id = repo
            .create_file_with_vec(
                "test_vec.bin".to_string(),
                "application/octet-stream".to_string(),
                file_data.clone(),
            )
            .await
            .unwrap();

        assert!(file_id > 0, "Should return a valid file ID");

        // Verify file was created correctly by checking all files
        let all_files = repo.find_all_files().await.unwrap();
        let created_file = all_files.iter().find(|f| f.id == file_id).unwrap();
        assert_eq!(created_file.data.to_vec(), file_data);
        assert_eq!(created_file.name, "test_vec.bin");
    }

    #[tokio::test]
    async fn test_create_file_with_bytes() {
        let pool = setup_test_db_with_files().await;
        let repo = TestFileRepo { pool: &pool };

        let file_data = Bytes::from(vec![10u8, 20, 30, 40, 50]);
        let file_id = repo
            .create_file_with_bytes(
                "test_bytes.bin".to_string(),
                "application/octet-stream".to_string(),
                file_data.clone(),
            )
            .await
            .unwrap();

        assert!(file_id > 0, "Should return a valid file ID");

        // Verify file was created correctly by checking all files
        let all_files = repo.find_all_files().await.unwrap();
        let created_file = all_files.iter().find(|f| f.id == file_id).unwrap();
        assert_eq!(created_file.data, file_data);
        assert_eq!(created_file.name, "test_bytes.bin");
    }

    #[tokio::test]
    async fn test_create_file_with_into() {
        let pool = setup_test_db_with_files().await;
        let repo = TestFileRepo { pool: &pool };

        // Test with Vec<u8> (implements Into<Bytes>)
        let vec_data = vec![100u8, 101, 102, 103, 104];
        let file_id_1 = repo
            .create_file_with_into(
                "test_into_vec.bin".to_string(),
                "application/octet-stream".to_string(),
                vec_data.clone(),
            )
            .await
            .unwrap();

        // Test with Bytes (implements Into<Bytes>)
        let bytes_data = Bytes::from(vec![200u8, 201, 202, 203, 204]);
        let file_id_2 = repo
            .create_file_with_into(
                "test_into_bytes.bin".to_string(),
                "application/octet-stream".to_string(),
                bytes_data.clone(),
            )
            .await
            .unwrap();

        assert!(file_id_1 > 0, "Should return valid file ID for Vec<u8>");
        assert!(file_id_2 > 0, "Should return valid file ID for Bytes");

        // Verify both files were created correctly
        let all_files = repo.find_all_files().await.unwrap();

        let created_file_1 = all_files.iter().find(|f| f.id == file_id_1).unwrap();
        assert_eq!(created_file_1.data.to_vec(), vec_data);
        assert_eq!(created_file_1.name, "test_into_vec.bin");

        let created_file_2 = all_files.iter().find(|f| f.id == file_id_2).unwrap();
        assert_eq!(created_file_2.data, bytes_data);
        assert_eq!(created_file_2.name, "test_into_bytes.bin");
    }

    #[tokio::test]
    async fn test_create_file_with_into_option() {
        let pool = setup_test_db_with_files().await;
        let repo = TestFileRepo { pool: &pool };

        // Test with Some(Vec<u8>)
        let vec_data = vec![50u8, 51, 52, 53, 54];
        let file_id_1 = repo
            .create_file_with_into_option(
                "test_option_some.bin".to_string(),
                "application/octet-stream".to_string(),
                Some(vec_data.clone()),
            )
            .await
            .unwrap();

        // Test with Some(Bytes)
        let bytes_data = Bytes::from(vec![60u8, 61, 62, 63, 64]);
        let file_id_2 = repo
            .create_file_with_into_option(
                "test_option_bytes.bin".to_string(),
                "application/octet-stream".to_string(),
                Some(bytes_data.clone()),
            )
            .await
            .unwrap();

        assert!(
            file_id_1 > 0,
            "Should return valid file ID for Some(Vec<u8>)"
        );
        assert!(file_id_2 > 0, "Should return valid file ID for Some(Bytes)");

        // Verify both files were created correctly
        let all_files = repo.find_all_files().await.unwrap();

        let created_file_1 = all_files.iter().find(|f| f.id == file_id_1).unwrap();
        assert_eq!(created_file_1.data.to_vec(), vec_data);
        assert_eq!(created_file_1.name, "test_option_some.bin");

        let created_file_2 = all_files.iter().find(|f| f.id == file_id_2).unwrap();
        assert_eq!(created_file_2.data, bytes_data);
        assert_eq!(created_file_2.name, "test_option_bytes.bin");
    }

    #[tokio::test]
    async fn test_create_file_with_slice_and_none() {
        let pool = setup_test_db_with_files().await;
        let repo = TestFileRepo { pool: &pool };

        // Test with &[u8] slice + impl Into<String> for content_type
        let slice_data: &[u8] = &[70u8, 71, 72, 73, 74];
        let file_id_1 = repo
            .create_file_with_into(
                "test_slice.bin".to_string(),
                "binary/slice", // &str implements Into<String>
                slice_data,     // &[u8] implements Into<Bytes> via Vec<u8>
            )
            .await
            .unwrap();

        // Test with Option<impl Into<Bytes>> = None
        let file_id_2 = repo
            .create_file_with_into_option(
                "test_none.bin".to_string(),
                "application/empty".to_string(),
                None::<Vec<u8>>, // Explicitly None for Option<impl Into<Bytes>>
            )
            .await
            .unwrap();

        assert!(file_id_1 > 0, "Should return valid file ID for &[u8]");
        assert!(file_id_2 > 0, "Should return valid file ID for None data");

        // Verify files were created correctly
        let all_files = repo.find_all_files().await.unwrap();

        // Verify slice file
        let created_file_1 = all_files.iter().find(|f| f.id == file_id_1).unwrap();
        assert_eq!(created_file_1.data.to_vec(), slice_data.to_vec());
        assert_eq!(created_file_1.name, "test_slice.bin");
        assert_eq!(created_file_1.content_type, "binary/slice");

        // Verify None file (should have empty/null data)
        let created_file_2 = all_files.iter().find(|f| f.id == file_id_2).unwrap();
        assert_eq!(created_file_2.name, "test_none.bin");
        assert_eq!(created_file_2.content_type, "application/empty");

        // Debug: check what SQLite does with NULL BLOB when reading as Bytes
        println!("None BLOB data length: {}", created_file_2.data.len());
        println!("None BLOB data content: {:?}", created_file_2.data.to_vec());

        // SQLite NULL BLOB is read as empty Bytes
        assert_eq!(
            created_file_2.data.len(),
            0,
            "NULL BLOB should read as empty Bytes"
        );
    }

    #[tokio::test]
    async fn test_create_file_no_return_vec() {
        let pool = setup_test_db_with_files().await;
        let repo = TestFileRepo { pool: &pool };

        let file_data = vec![100u8, 101, 102];
        let result = repo
            .create_file_vec_no_return(
                "no_return_vec.txt".to_string(),
                "text/plain".to_string(),
                file_data,
            )
            .await
            .unwrap();

        assert!(result.rows_affected() > 0, "Should affect at least one row");
    }

    #[tokio::test]
    async fn test_create_file_no_return_bytes() {
        let pool = setup_test_db_with_files().await;
        let repo = TestFileRepo { pool: &pool };

        let file_data = Bytes::from("Hello Bytes!".as_bytes().to_vec());
        let result = repo
            .create_file_bytes_no_return(
                "no_return_bytes.txt".to_string(),
                "text/plain".to_string(),
                file_data,
            )
            .await
            .unwrap();

        assert!(result.rows_affected() > 0, "Should affect at least one row");
    }

    #[tokio::test]
    async fn test_update_file_with_vec() {
        let pool = setup_test_db_with_files().await;
        let repo = TestFileRepo { pool: &pool };

        let new_data = vec![255u8, 254, 253, 252];
        let result = repo
            .update_file_with_vec(
                1,
                "updated_file.bin".to_string(),
                "application/updated".to_string(),
                new_data.clone(),
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
    }

    #[tokio::test]
    async fn test_update_file_with_bytes() {
        let pool = setup_test_db_with_files().await;
        let repo = TestFileRepo { pool: &pool };

        let new_data = Bytes::from("Updated content with Bytes".as_bytes().to_vec());
        let result = repo
            .update_file_with_bytes(
                2,
                "updated_bytes.txt".to_string(),
                "text/updated".to_string(),
                new_data.clone(),
            )
            .await
            .unwrap();

        assert!(result.rows_affected() > 0, "Should update at least one row");

        // Verify the update
        let files = repo.find_all_files().await.unwrap();
        let updated_file = files.iter().find(|f| f.id == 2).unwrap();
        assert_eq!(updated_file.name, "updated_bytes.txt");
        assert_eq!(updated_file.content_type, "text/updated");
        assert_eq!(updated_file.data, new_data);
    }

    #[tokio::test]
    async fn test_update_file_data_only_vec() {
        let pool = setup_test_db_with_files().await;
        let repo = TestFileRepo { pool: &pool };

        let new_data = vec![1u8, 3, 5, 7, 9, 11];
        let result = repo
            .update_file_data_vec(3, new_data.clone())
            .await
            .unwrap();

        assert!(result.rows_affected() > 0, "Should update at least one row");

        // Verify only data was updated (name and content_type should remain the same)
        let files = repo.find_all_files().await.unwrap();
        let updated_file = files.iter().find(|f| f.id == 3).unwrap();
        assert_eq!(updated_file.name, "file3.bin"); // Should remain unchanged
        assert_eq!(updated_file.content_type, "application/octet-stream"); // Should remain unchanged
        assert_eq!(updated_file.data.to_vec(), new_data); // Should be updated
    }

    #[tokio::test]
    async fn test_update_file_data_only_bytes() {
        let pool = setup_test_db_with_files().await;
        let repo = TestFileRepo { pool: &pool };

        let new_data = Bytes::from("Only data update with Bytes!".as_bytes().to_vec());
        let result = repo
            .update_file_data_bytes(1, new_data.clone())
            .await
            .unwrap();

        assert!(result.rows_affected() > 0, "Should update at least one row");

        // Verify only data was updated
        let files = repo.find_all_files().await.unwrap();
        let updated_file = files.iter().find(|f| f.id == 1).unwrap();
        assert_eq!(updated_file.name, "file1.txt"); // Should remain unchanged
        assert_eq!(updated_file.content_type, "text/plain"); // Should remain unchanged
        assert_eq!(updated_file.data, new_data); // Should be updated
    }

    #[tokio::test]
    async fn test_large_blob_operations() {
        let pool = setup_test_db_with_files().await;
        let repo = TestFileRepo { pool: &pool };

        // Test with a larger blob (1MB)
        let large_data: Vec<u8> = (0..1024 * 1024).map(|i| (i % 256) as u8).collect();

        // Create with Vec<u8>
        let file_id = repo
            .create_file_with_vec(
                "large_file.bin".to_string(),
                "application/octet-stream".to_string(),
                large_data.clone(),
            )
            .await
            .unwrap();

        // Update with Bytes
        let updated_data = Bytes::from(vec![42u8; 512 * 1024]); // 512KB of 42s
        let result = repo
            .update_file_data_bytes(file_id, updated_data.clone())
            .await
            .unwrap();
        assert!(result.rows_affected() > 0);

        // Verify the large update worked by checking all files for our specific file
        let all_files = repo.find_all_files().await.unwrap();
        let updated_file = all_files.iter().find(|f| f.id == file_id).unwrap();
        assert_eq!(updated_file.data.len(), 512 * 1024);
        assert!(
            updated_file.data.iter().all(|&b| b == 42),
            "All bytes should be 42"
        );
    }

    struct TestFileRepo<'a> {
        pool: &'a Pool,
    }

    impl<'a> FileRepo for TestFileRepo<'a> {
        fn get_pool(&self) -> &Pool {
            self.pool
        }
    }

    async fn setup_test_db_with_files() -> Pool {
        let pool = Pool::connect(":memory:").await.unwrap();

        // Create files table with BLOB (nullable)
        sqlx::query(
            r#"
            CREATE TABLE files (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                content_type TEXT NOT NULL,
                data BLOB
            )
        "#,
        )
        .execute(&pool)
        .await
        .unwrap();

        // Insert 3 files with different sized BLOB data
        let file1_data = "Este e o conteudo do arquivo 1 com texto simples.";
        let file2_data =
            r#"{"message": "Este e um arquivo JSON com dados estruturados", "size": 1024}"#;
        let file3_data = vec![0u8; 100];

        let files = vec![
            ("file1.txt", "text/plain", file1_data.as_bytes()),
            ("file2.json", "application/json", file2_data.as_bytes()),
            (
                "file3.bin",
                "application/octet-stream",
                file3_data.as_slice(),
            ),
        ];

        for (i, (name, content_type, data)) in files.iter().enumerate() {
            sqlx::query("INSERT INTO files (id, name, content_type, data) VALUES (?, ?, ?, ?)")
                .bind((i + 1) as i64)
                .bind(*name)
                .bind(*content_type)
                .bind(*data)
                .execute(&pool)
                .await
                .unwrap();
        }

        pool
    }
}
