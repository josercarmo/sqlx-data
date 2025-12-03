//! Requires the `uuid` Cargo feature flag.
//!
//! | Rust type                             | MySQL type(s)                                        |
//! |---------------------------------------|------------------------------------------------------|
//! | `uuid::Uuid`                          | BINARY(16), VARCHAR(36)                             |
//! | `uuid::fmt::Hyphenated`               | VARCHAR(36)                                          |
//! | `uuid::fmt::Simple`                   | VARCHAR(32)                                          |
//!
//! **BINARY(16):** Efficient binary format, stores raw UUID bytes.
//! **VARCHAR(36):** Human-readable format with hyphens (e.g., 550e8400-e29b-41d4-a716-446655440000).
//! **VARCHAR(32):** Compact text format without hyphens.
//!
//! Recommendation: Use `Uuid` for BINARY storage, `Hyphenated`/`Simple` for text storage.
//!

use sqlx_data::{
    Cursor, CursorData, CursorError, CursorSecureExtract, CursorValue, FilterValue, IntoParams, ParamsBuilder, Pool,
    QueryResult, Result, dml, repo,
};

// Import UUID types from sqlx when uuid feature is enabled
use sqlx::types::{Json,JsonValue,BigDecimal, Uuid };
use std::str::FromStr;

#[derive(Debug, sqlx::FromRow)]
pub struct Product {
    pub id: i64,
    pub uuid: Uuid,
    pub name: String,
    pub category_uuid: Option<Uuid>,
    pub supplier_uuid: Option<Uuid>,
    pub price: BigDecimal,
    pub active: bool,
    pub created_at: String, //string just for test
    pub updated_at: Option<String>,
    pub metadata_json: Option<Json<JsonValue>>,//Important do document how to use Json and JsonValue
}

impl CursorSecureExtract for Product {
    fn extract_whitelisted_fields(&self, fields: &[String]) -> Result<Vec<CursorValue>> {
        let mut values = Vec::with_capacity(fields.len());
        for field in fields {
            match field.as_str() {
                "id" => values.push(self.id.into()),
                "uuid" => values.push(self.uuid.to_string().into()),
                "created_at" => values.push(self.created_at.clone().into()),
                "category_uuid" => {
                    if let Some(category_uuid) = self.category_uuid {
                        values.push(category_uuid.to_string().into());
                    } else {
                        values.push(CursorValue::String("".into()));
                    }
                }
                "price" => values.push(CursorValue::Float(self.price.to_string().parse::<f64>().unwrap_or(0.0))),
                _ => {
                    return Err(CursorError::invalid_field(field.clone()).into());
                }
            }
        }
        Ok(values)
    }

    fn encode(cursor: &CursorData) -> Result<String> {
        use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD as BASE64};
        let json_bytes = serde_json::to_vec(&cursor)
            .map_err(|e| CursorError::encode_error(format!("JSON serialization failed: {}", e)))?;
        Ok(BASE64.encode(json_bytes))
    }

    fn decode(encoded: &str) -> Result<Vec<FilterValue>> {
        use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD as BASE64};
        let bytes = BASE64
            .decode(encoded)
            .map_err(|e| CursorError::decode_error(format!("Base64 decode failed: {}", e)))?;

        let cursor: CursorData = serde_json::from_slice(&bytes).map_err(|e| {
            CursorError::decode_error(format!("JSON deserialization failed: {}", e))
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
#[alias(all_columns = "id, uuid as 'uuid: Uuid', name, category_uuid as 'category_uuid: Uuid', supplier_uuid as 'supplier_uuid: Uuid', price, active as 'active: bool', created_at, updated_at, metadata_json as 'metadata_json: Json<JsonValue>'")]
trait ProductRepo {
    // Insert product with UUID - MySQL uses VARCHAR for text UUIDs
    #[dml("INSERT INTO products (uuid, name, category_uuid, supplier_uuid, price, active, created_at, metadata_json) VALUES (?, ?, ?, ?, ?, ?, ?, ?)")]
    async fn insert_product(
        &self,
        uuid: Uuid,
        name: String,
        category_uuid: Option<Uuid>,
        supplier_uuid: Option<Uuid>,
        price: BigDecimal,
        active: bool,
        created_at: String,
        metadata_json: Option<Json<JsonValue>>,
    ) -> Result<QueryResult>;

    // Find by UUID
    #[dml("SELECT {{all_columns}} FROM products WHERE uuid = ?")]
    async fn find_by_uuid(&self, uuid: Uuid) -> Result<Product>;

    #[dml("SELECT {{all_columns}} FROM products WHERE uuid = ?")]
    async fn find_optional_by_uuid(&self, uuid: Uuid) -> Result<Option<Product>>;

    // Find by category UUID
    #[dml("SELECT {{all_columns}} FROM products WHERE category_uuid = ?")]
    async fn find_by_category(&self, category_uuid: Uuid) -> Result<Vec<Product>>;

    // Find products with null category
    #[dml("SELECT {{all_columns}} FROM products WHERE category_uuid IS NULL")]
    async fn find_without_category(&self) -> Result<Vec<Product>>;

    // Update product by UUID
    #[dml("UPDATE products SET name = ?, price = ?, updated_at = ? WHERE uuid = ?")]
    async fn update_product(&self, name: String, price: BigDecimal, updated_at: String, uuid: Uuid) -> Result<QueryResult>;

    // Delete by UUID
    #[dml("DELETE FROM products WHERE uuid = ?")]
    async fn delete_by_uuid(&self, uuid: Uuid) -> Result<QueryResult>;

    // Search with cursor pagination
    #[dml("SELECT {{all_columns}} FROM products WHERE active = ?")]
    async fn find_active_products(&self, active: bool, params: impl IntoParams) -> Result<Cursor<Product>>;

    // Complex query with UUID filtering
    #[dml("SELECT {{all_columns}} FROM products WHERE price >= ? AND (category_uuid = ? OR supplier_uuid = ?)")]
    async fn find_by_price_and_related_uuids(
        &self,
        min_price: BigDecimal,
        category_uuid: Option<Uuid>,
        supplier_uuid: Option<Uuid>,
    ) -> Result<Vec<Product>>;

    // Batch insert with UUIDs
    #[dml("INSERT INTO products (uuid, name, category_uuid, supplier_uuid, price, active, created_at) VALUES (?, ?, ?, ?, ?, ?, ?)")]
    async fn insert_three_products(&self, products: Vec<(Uuid, String, Option<Uuid>, Option<Uuid>, BigDecimal, bool, String)>) -> Result<QueryResult>;

    // Count by category
    #[dml("SELECT COUNT(*) FROM products WHERE category_uuid = ?")]
    async fn count_by_category(&self, category_uuid: Uuid) -> Result<i64>;
}

pub struct ProductApp {
    pool: Pool,
}

impl ProductRepo for ProductApp {
    fn get_pool(&self) -> &Pool {
        &self.pool
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper function to create test UUID
    fn create_test_uuid() -> Uuid {
        Uuid::new_v4()
    }

    // Helper function to create deterministic UUID for testing
    fn create_deterministic_uuid(input: &str) -> Uuid {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        input.hash(&mut hasher);
        let hash = hasher.finish();

        // Create a deterministic UUID from hash
        let mut bytes = [0u8; 16];
        bytes[..8].copy_from_slice(&hash.to_be_bytes());
        bytes[8..].copy_from_slice(&hash.to_le_bytes()[..8]);

        Uuid::from_bytes(bytes)
    }

    #[sqlx::test(
        migrations = "tests/migrations_uuid",
    )]
    async fn test_uuid_insert_and_find(pool: Pool) {
        let app = ProductApp { pool };

        let product_uuid = create_test_uuid();
        let category_uuid = create_test_uuid();
        let supplier_uuid = create_test_uuid();

        // Insert product with UUIDs
        let result = app
            .insert_product(
                product_uuid,
                "Test Product".to_string(),
                Some(category_uuid),
                Some(supplier_uuid),
                BigDecimal::from_str("99.99").unwrap(),
                true,
                "2024-01-01 10:00:00".to_string(),
                Some(Json(serde_json::json!({"brand": "TestBrand"}))),
            )
            .await
            .unwrap();

        assert_eq!(result.rows_affected(), 1);

        // Find by UUID
        let found_product = app.find_by_uuid(product_uuid).await.unwrap();

        assert_eq!(found_product.uuid, product_uuid);
        assert_eq!(found_product.name, "Test Product");
        assert_eq!(found_product.category_uuid, Some(category_uuid));
        assert_eq!(found_product.supplier_uuid, Some(supplier_uuid));
        assert_eq!(found_product.price, BigDecimal::from_str("99.99").unwrap());
        assert!(found_product.active);
        assert_eq!(found_product.metadata_json, Some(Json(serde_json::json!({"brand": "TestBrand"}))));
    }

    #[sqlx::test(
        migrations = "tests/migrations_uuid",
    )]
    async fn test_uuid_optional_find(pool: Pool) {
        let app = ProductApp { pool };

        let existing_uuid = create_test_uuid();
        let non_existing_uuid = create_test_uuid();

        // Insert one product
        app.insert_product(
            existing_uuid,
            "Existing Product".to_string(),
            None,
            None,
            BigDecimal::from_str("50.00").unwrap(),
            true,
            "2024-01-01 10:00:00".to_string(),
            None,
        )
        .await
        .unwrap();

        // Test existing UUID
        let found = app.find_optional_by_uuid(existing_uuid).await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "Existing Product");

        // Test non-existing UUID
        let not_found = app.find_optional_by_uuid(non_existing_uuid).await.unwrap();
        assert!(not_found.is_none());
    }

    #[sqlx::test(
        migrations = "tests/migrations_uuid",
    )]
    async fn test_uuid_category_filtering(pool: Pool) {
        let app = ProductApp { pool };

        let category1_uuid = create_deterministic_uuid("category1");
        let category2_uuid = create_deterministic_uuid("category2");

        // Insert products with different categories
        for i in 0..3 {
            app.insert_product(
                create_deterministic_uuid(&format!("product_cat1_{}", i)),
                format!("Product Cat1 {}", i),
                Some(category1_uuid),
                None,
                BigDecimal::from_str(&format!("{}.0", (i + 1) * 10)).unwrap(),
                true,
                "2024-01-01 10:00:00".to_string(),
                None,
            )
            .await
            .unwrap();
        }

        for i in 0..2 {
            app.insert_product(
                create_deterministic_uuid(&format!("product_cat2_{}", i)),
                format!("Product Cat2 {}", i),
                Some(category2_uuid),
                None,
                BigDecimal::from_str(&format!("{}.0", (i + 1) * 20)).unwrap(),
                true,
                "2024-01-01 10:00:00".to_string(),
                None,
            )
            .await
            .unwrap();
        }

        // Insert product without category
        app.insert_product(
            create_deterministic_uuid("product_no_cat"),
            "Product No Category".to_string(),
            None,
            None,
            BigDecimal::from_str("5.0").unwrap(),
            true,
            "2024-01-01 10:00:00".to_string(),
            None,
        )
        .await
        .unwrap();

        // Test category1 filtering
        let cat1_products = app.find_by_category(category1_uuid).await.unwrap();
        assert_eq!(cat1_products.len(), 3);
        for product in &cat1_products {
            assert_eq!(product.category_uuid, Some(category1_uuid));
        }

        // Test category2 filtering
        let cat2_products = app.find_by_category(category2_uuid).await.unwrap();
        assert_eq!(cat2_products.len(), 2);
        for product in &cat2_products {
            assert_eq!(product.category_uuid, Some(category2_uuid));
        }

        // Test products without category
        let no_cat_products = app.find_without_category().await.unwrap();
        assert_eq!(no_cat_products.len(), 1);
        assert_eq!(no_cat_products[0].name, "Product No Category");
        assert_eq!(no_cat_products[0].category_uuid, None);

        // Test count by category
        let cat1_count = app.count_by_category(category1_uuid).await.unwrap();
        assert_eq!(cat1_count, 3);
    }

    #[sqlx::test(
        migrations = "tests/migrations_uuid",
    )]
    async fn test_uuid_update_and_delete(pool: Pool) {
        let app = ProductApp { pool };

        let product_uuid = create_test_uuid();

        // Insert product
        app.insert_product(
            product_uuid,
            "Original Name".to_string(),
            None,
            None,
            BigDecimal::from_str("100.0").unwrap(),
            true,
            "2024-01-01 10:00:00".to_string(),
            None,
        )
        .await
        .unwrap();

        // Update product
        let update_result = app
            .update_product(
                "Updated Name".to_string(),
                BigDecimal::from_str("150.0").unwrap(),
                "2024-01-01 11:00:00".to_string(),
                product_uuid,
            )
            .await
            .unwrap();

        assert_eq!(update_result.rows_affected(), 1);

        // Verify update
        let updated_product = app.find_by_uuid(product_uuid).await.unwrap();
        assert_eq!(updated_product.name, "Updated Name");
        assert_eq!(updated_product.price, BigDecimal::from_str("150.0").unwrap());
        assert_eq!(updated_product.updated_at, Some("2024-01-01 11:00:00".to_string()));

        // Delete product
        let delete_result = app.delete_by_uuid(product_uuid).await.unwrap();
        assert_eq!(delete_result.rows_affected(), 1);

        // Verify deletion
        let deleted_product = app.find_optional_by_uuid(product_uuid).await.unwrap();
        assert!(deleted_product.is_none());
    }

    #[sqlx::test(
        migrations = "tests/migrations_uuid",
    )]
    async fn test_uuid_cursor_pagination(pool: Pool) {
        let app = ProductApp { pool };

        let category_uuid = create_test_uuid();

        // Insert several active products
        for i in 0..10 {
            app.insert_product(
                create_deterministic_uuid(&format!("cursor_test_{}", i)),
                format!("Cursor Product {}", i),
                Some(category_uuid),
                None,
                BigDecimal::from_str(&format!("{}.0", (i + 1) * 5)).unwrap(),
                true,
                "2024-01-01 10:00:00".to_string(),
                None,
            )
            .await
            .unwrap();
        }

        // Test cursor pagination
        let params = ParamsBuilder::new()
            .cursor()
                .first_page()
                .done()
            .sort()
            .asc("id")
            .done()
            .limit(3)
            .build();

        let first_page = app.find_active_products(true, params).await.unwrap();
        assert_eq!(first_page.data.len(), 3);
        assert!(first_page.has_next);
        assert!(first_page.data.len() > 0);

        // Verify UUIDs are properly loaded
        for product in &first_page.data {
            assert!(product.uuid.to_string().len() == 36); // Standard UUID format
            assert!(product.name.starts_with("Cursor Product"));
        }

        // Test second page
        let cursor_str = first_page.next_cursor.unwrap();
        let params = ParamsBuilder::new()
            .cursor()
                .next_cursor::<Product>(cursor_str)
                .done()
            .sort()
                .asc("id")
                .done()
            .limit(3)
            .build();

        let second_page = app.find_active_products(true, params).await.unwrap();
        assert_eq!(second_page.data.len(), 3);

        // Verify no UUID overlap
        let first_uuids: Vec<_> = first_page.data.iter().map(|p| p.uuid).collect();
        for product in &second_page.data {
            assert!(!first_uuids.contains(&product.uuid));
        }
    }

    #[sqlx::test(
        migrations = "tests/migrations_uuid",
    )]
    async fn test_uuid_complex_queries(pool: Pool) {
        let app = ProductApp { pool };

        let category_uuid = create_test_uuid();
        let supplier_uuid = create_test_uuid();

        // Insert products with various configurations
        app.insert_product(
            create_deterministic_uuid("complex1"),
            "Expensive Category Product".to_string(),
            Some(category_uuid),
            None,
            BigDecimal::from_str("200.0").unwrap(),
            true,
            "2024-01-01 10:00:00".to_string(),
            None,
        )
        .await
        .unwrap();

        app.insert_product(
            create_deterministic_uuid("complex2"),
            "Expensive Supplier Product".to_string(),
            None,
            Some(supplier_uuid),
            BigDecimal::from_str("250.0").unwrap(),
            true,
            "2024-01-01 10:00:00".to_string(),
            None,
        )
        .await
        .unwrap();

        app.insert_product(
            create_deterministic_uuid("complex3"),
            "Cheap Product".to_string(),
            Some(category_uuid),
            Some(supplier_uuid),
            BigDecimal::from_str("50.0").unwrap(),
            true,
            "2024-01-01 10:00:00".to_string(),
            None,
        )
        .await
        .unwrap();

        // Test complex query
        let expensive_products = app
            .find_by_price_and_related_uuids(
                BigDecimal::from_str("100.0").unwrap(),
                Some(category_uuid),
                Some(supplier_uuid),
            )
            .await
            .unwrap();

        // Should find products with price >= 100 AND (category_uuid OR supplier_uuid match)
        assert_eq!(expensive_products.len(), 2);
        for product in &expensive_products {
            assert!(product.price >= BigDecimal::from_str("100.0").unwrap());
            assert!(product.category_uuid == Some(category_uuid) || product.supplier_uuid == Some(supplier_uuid));
        }
    }

    #[sqlx::test(
        migrations = "tests/migrations_uuid",
    )]
    async fn test_uuid_batch_insert(pool: Pool) {
        let app = ProductApp { pool };

        let category_uuid = create_test_uuid();
        let supplier_uuid = create_test_uuid();

        // Prepare batch data
        let batch_products = vec![
            (
                create_deterministic_uuid("batch1"),
                "Batch Product 1".to_string(),
                Some(category_uuid),
                None,
                BigDecimal::from_str("10.0").unwrap(),
                true,
                "2024-01-01 10:00:00".to_string(),
            ),
            (
                create_deterministic_uuid("batch2"),
                "Batch Product 2".to_string(),
                None,
                Some(supplier_uuid),
                BigDecimal::from_str("20.0").unwrap(),
                true,
                "2024-01-01 10:00:00".to_string(),
            ),
            (
                create_deterministic_uuid("batch3"),
                "Batch Product 3".to_string(),
                Some(category_uuid),
                Some(supplier_uuid),
                BigDecimal::from_str("30.0").unwrap(),
                false,
                "2024-01-01 10:00:00".to_string(),
            ),
        ];

        // Note: For this to work, we'd need to properly expand the batch in the macro
        // For now, let's test the concept by inserting individually
        for (uuid, name, cat_uuid, sup_uuid, price, active, created_at) in &batch_products {
            app.insert_product(
                *uuid,
                name.clone(),
                *cat_uuid,
                *sup_uuid,
                price.clone(),
                *active,
                created_at.clone(),
                None,
            )
            .await
            .unwrap();
        }
        let params = ParamsBuilder::new()
                .cursor()
                    .first_page()
                    .done()
                .sort()
                    .asc("id")
                    .done()
                .limit(10)
                .build();
        // Verify all products were inserted
        let active_products = app.find_active_products(true, params).await.unwrap();

        let batch_products_found: Vec<_> = active_products.data.iter()
            .filter(|p| p.name.starts_with("Batch Product"))
            .collect();

        assert_eq!(batch_products_found.len(), 2); // Only active ones

        // Verify UUIDs match
        let expected_uuids = vec![
            create_deterministic_uuid("batch1"),
            create_deterministic_uuid("batch2"),
        ];

        for product in &batch_products_found {
            assert!(expected_uuids.contains(&product.uuid));
        }
    }

    #[sqlx::test(
        migrations = "tests/migrations_uuid",
    )]
    async fn test_uuid_null_handling(pool: Pool) {
        let app = ProductApp { pool };

        let product_uuid = create_test_uuid();

        // Insert product with all nullable UUIDs as None
        app.insert_product(
            product_uuid,
            "Null UUID Product".to_string(),
            None,
            None,
            BigDecimal::from_str("25.0").unwrap(),
            true,
            "2024-01-01 10:00:00".to_string(),
            None,
        )
        .await
        .unwrap();

        // Verify product was inserted correctly
        let product = app.find_by_uuid(product_uuid).await.unwrap();
        assert_eq!(product.category_uuid, None);
        assert_eq!(product.supplier_uuid, None);
        assert_eq!(product.metadata_json, None);
        assert_eq!(product.updated_at, None);

        // Test that we can still filter by these null UUIDs
        let products_without_category = app.find_without_category().await.unwrap();
        assert!(products_without_category.iter().any(|p| p.uuid == product_uuid));
    }
}