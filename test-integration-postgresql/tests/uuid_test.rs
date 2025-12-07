#![cfg(all(feature = "json", feature = "uuid"))]
//! PostgreSQL UUID type support test
//!
//! | Rust type                             | PostgreSQL type(s)                              |
//! |---------------------------------------|------------------------------------------------|
//! | `uuid::Uuid`                          | UUID                                           |
//! | `sqlx::types::Uuid`                   | UUID                                           |
//!
//! **UUID:** PostgreSQL's native UUID type, efficient 16-byte binary storage.
//!
//! Recommendation: Use PostgreSQL's native UUID type for optimal performance and storage.
//!

use sqlx_data::{
    Cursor, CursorData, CursorError, CursorSecureExtract, CursorValue, FilterValue, IntoParams, ParamsBuilder, Pool,
    QueryResult, Result, dml, repo,
};

// Import UUID types from sqlx
use sqlx::types::Uuid;

#[derive(Debug, sqlx::FromRow)]
pub struct Product {
    pub id: i64,
    pub uuid: Uuid,
    pub name: String,
    pub category_uuid: Option<Uuid>,
    pub supplier_uuid: Option<Uuid>,
    pub price: f64,
    pub active: bool,
    pub created_at: chrono::DateTime<chrono::Utc>, // PostgreSQL TIMESTAMPTZ
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
    pub metadata_json: Option<serde_json::Value>,
}

impl CursorSecureExtract for Product {
    fn extract_whitelisted_fields(&self, fields: &[String]) -> Result<Vec<CursorValue>> {
        let mut values = Vec::with_capacity(fields.len());
        for field in fields {
            match field.as_str() {
                "id" => values.push(self.id.into()),
                "uuid" => values.push(self.uuid.to_string().into()),
                "created_at" => values.push(self.created_at.to_rfc3339().into()),
                "category_uuid" => {
                    if let Some(category_uuid) = self.category_uuid {
                        values.push(category_uuid.to_string().into());
                    } else {
                        values.push(CursorValue::String("".into()));
                    }
                }
                "supplier_uuid" => {
                    if let Some(supplier_uuid) = self.supplier_uuid {
                        values.push(supplier_uuid.to_string().into());
                    } else {
                        values.push(CursorValue::String("".into()));
                    }
                }
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
#[alias(
    all_columns = "id, uuid, name, category_uuid, supplier_uuid, price, active, created_at, updated_at, metadata_json"
)]
trait ProductRepo {
    #[dml(
        "INSERT INTO products (uuid, name, category_uuid, supplier_uuid, price, active, created_at, metadata_json)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
         RETURNING id"
    )]
    async fn insert_product(
        &self,
        uuid: Uuid,
        name: String,
        category_uuid: Option<Uuid>,
        supplier_uuid: Option<Uuid>,
        price: f64,
        active: bool,
        created_at: chrono::DateTime<chrono::Utc>,
        metadata_json: Option<serde_json::Value>,
    ) -> Result<i64>;

    #[dml("SELECT {{all_columns}} FROM products WHERE uuid = $1")]
    async fn find_product_by_uuid(&self, uuid: Uuid) -> Result<Option<Product>>;

    #[dml("SELECT {{all_columns}} FROM products WHERE category_uuid = $1")]
    async fn find_products_by_category(&self, category_uuid: Uuid) -> Result<Vec<Product>>;

    #[dml("SELECT uuid FROM products WHERE active = true")]
    async fn find_active_product_uuids(&self) -> Result<Vec<Uuid>>;

    #[dml("SELECT COUNT(*) FROM products WHERE supplier_uuid = $1")]
    async fn count_products_by_supplier(&self, supplier_uuid: Uuid) -> Result<i64>;

    #[dml("SELECT COUNT(*) > 0 FROM products WHERE uuid = $1")]
    async fn product_exists(&self, uuid: Uuid) -> Result<bool>;

    #[dml("SELECT uuid FROM products WHERE id = $1")]
    async fn get_product_uuid_by_id(&self, id: i64) -> Result<Option<Uuid>>;

    #[dml(
        "UPDATE products
         SET updated_at = $2, supplier_uuid = $3
         WHERE uuid = $1"
    )]
    async fn update_product_supplier(
        &self,
        uuid: Uuid,
        updated_at: chrono::DateTime<chrono::Utc>,
        supplier_uuid: Option<Uuid>,
    ) -> Result<QueryResult>;

    #[dml("SELECT {{all_columns}} FROM products WHERE supplier_uuid IS NOT NULL")]
    async fn find_products_with_supplier(&self) -> Result<Vec<Product>>;

    #[dml("SELECT {{all_columns}} FROM products WHERE category_uuid IS NULL")]
    async fn find_products_without_category(&self) -> Result<Vec<Product>>;

    #[dml(
        "SELECT
            uuid,
            name,
            CASE
                WHEN supplier_uuid IS NOT NULL THEN true
                ELSE false
            END as has_supplier
         FROM products"
    )]
    async fn get_products_with_supplier_flag(&self) -> Result<Vec<(Uuid, String, bool)>>;

    #[dml(
        "SELECT DISTINCT category_uuid FROM products WHERE category_uuid IS NOT NULL ORDER BY category_uuid"
    )]
    async fn get_distinct_categories(&self) -> Result<Vec<Uuid>>;

    // Test direct field return types
    #[dml("SELECT uuid FROM products WHERE id = $1")]
    async fn get_uuid(&self, id: i64) -> Result<Option<Uuid>>;

    #[dml("SELECT category_uuid FROM products WHERE id = $1")]
    async fn get_category_uuid(&self, id: i64) -> Result<Option<Uuid>>;

    #[dml("SELECT supplier_uuid FROM products WHERE id = $1")]
    async fn get_supplier_uuid(&self, id: i64) -> Result<Option<Uuid>>;

    // Cursor pagination methods for UUID fields
    #[dml("SELECT {{all_columns}} FROM products ORDER BY uuid, id")]
    async fn find_products_cursor_by_uuid(
        &self,
        params: impl IntoParams,
    ) -> Result<Cursor<Product>>;

    #[dml(
        "SELECT {{all_columns}} FROM products WHERE category_uuid IS NOT NULL ORDER BY category_uuid, id"
    )]
    async fn find_products_cursor_by_category_uuid(
        &self,
        params: impl IntoParams,
    ) -> Result<Cursor<Product>>;

    #[dml(
        "SELECT {{all_columns}} FROM products WHERE supplier_uuid IS NOT NULL ORDER BY supplier_uuid, id"
    )]
    async fn find_products_cursor_by_supplier_uuid(
        &self,
        params: impl IntoParams,
    ) -> Result<Cursor<Product>>;
}

pub struct ProductRepoImpl<'a> {
    pool: &'a Pool,
}

impl<'a> ProductRepo for ProductRepoImpl<'a> {
    fn get_pool(&self) -> & Pool {
        &self.pool
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[sqlx::test(
        migrations = "tests/migrations_uuid",
        fixtures(path = "fixtures", scripts("products"))
    )]
    async fn test_uuid_roundtrip(pool: Pool) -> Result<()> {
        let repo = ProductRepoImpl { pool: &pool };
        let product_uuid = Uuid::new_v4();
        let category_uuid = Uuid::new_v4();
        let supplier_uuid = Uuid::new_v4();

        let inserted_id = repo
            .insert_product(
                product_uuid,
                "Test Product".into(),
                Some(category_uuid),
                Some(supplier_uuid),
                99.99,
                true,
                Utc::now(),
                Some(serde_json::json!({"key": "value"})),
            )
            .await?;

        assert!(inserted_id > 0);

        let product = repo
            .find_product_by_uuid(product_uuid)
            .await?
            .expect("Product should be found");

        assert_eq!(product.id, inserted_id);
        assert_eq!(product.uuid, product_uuid);
        assert_eq!(product.name, "Test Product");
        assert_eq!(product.category_uuid, Some(category_uuid));
        assert_eq!(product.supplier_uuid, Some(supplier_uuid));
        assert_eq!(product.price, 99.99);
        assert!(product.active);

        Ok(())
    }

    #[sqlx::test(
        migrations = "tests/migrations_uuid",
        fixtures(path = "fixtures", scripts("products"))
    )]
    async fn test_find_products_by_category(pool: Pool) -> Result<()> {
        let repo = ProductRepoImpl { pool: &pool };
        let category_uuid = Uuid::new_v4();

        // Insert test product
        let product_uuid = Uuid::new_v4();
        repo.insert_product(
            product_uuid,
            "Category Test Product".into(),
            Some(category_uuid),
            None,
            49.99,
            true,
            Utc::now(),
            None,
        )
        .await?;

        let products = repo.find_products_by_category(category_uuid).await?;

        assert!(!products.is_empty());
        for product in products {
            assert_eq!(product.category_uuid, Some(category_uuid));
        }

        Ok(())
    }

    #[sqlx::test(
        migrations = "tests/migrations_uuid",
        fixtures(path = "fixtures", scripts("products"))
    )]
    async fn test_uuid_operations(pool: Pool) -> Result<()> {
        let repo = ProductRepoImpl { pool: &pool };
        let product_uuid = Uuid::new_v4();

        // Test existence check
        let exists_before = repo.product_exists(product_uuid).await?;
        assert!(!exists_before);

        // Insert product
        let inserted_id = repo
            .insert_product(
                product_uuid,
                "Existence Test".into(),
                None,
                None,
                19.99,
                true,
                Utc::now(),
                None,
            )
            .await?;

        // Test existence check after insert
        let exists_after = repo.product_exists(product_uuid).await?;
        assert!(exists_after);

        // Test get UUID by ID
        let retrieved_uuid = repo.get_uuid(inserted_id).await?;
        assert_eq!(retrieved_uuid, Some(product_uuid));

        Ok(())
    }

    #[sqlx::test(
        migrations = "tests/migrations_uuid",
        fixtures(path = "fixtures", scripts("products"))
    )]
    async fn test_supplier_operations(pool: Pool) -> Result<()> {
        let repo = ProductRepoImpl { pool: &pool };
        let product_uuid = Uuid::new_v4();
        let supplier_uuid = Uuid::new_v4();

        // Insert product without supplier
        let inserted_id = repo
            .insert_product(
                product_uuid,
                "Supplier Test".into(),
                None,
                None,
                29.99,
                true,
                Utc::now(),
                None,
            )
            .await?;

        // Update with supplier
        repo.update_product_supplier(
            product_uuid,
            Utc::now(),
            Some(supplier_uuid),
        )
        .await?;

        // Test count by supplier
        let count = repo.count_products_by_supplier(supplier_uuid).await?;
        assert!(count > 0);

        // Test get supplier UUID
        let retrieved_supplier = repo.get_supplier_uuid(inserted_id).await?;
        assert_eq!(retrieved_supplier, Some(supplier_uuid));

        Ok(())
    }

    #[sqlx::test(
        migrations = "tests/migrations_uuid",
        fixtures(path = "fixtures", scripts("products"))
    )]
    async fn test_active_product_uuids(pool: Pool) -> Result<()> {
        let repo = ProductRepoImpl { pool: &pool };

        // Insert active product
        let active_uuid = Uuid::new_v4();
        repo.insert_product(
            active_uuid,
            "Active Product".into(),
            None,
            None,
            39.99,
            true,
            Utc::now(),
            None,
        )
        .await?;

        let active_uuids = repo.find_active_product_uuids().await?;

        // Should include our active product plus any from fixtures
        assert!(!active_uuids.is_empty());
        assert!(active_uuids.contains(&active_uuid));

        Ok(())
    }

    #[sqlx::test(
        migrations = "tests/migrations_uuid",
        fixtures(path = "fixtures", scripts("products"))
    )]
    async fn test_products_with_supplier_flag(pool: Pool) -> Result<()> {
        let repo = ProductRepoImpl { pool: &pool };
        let product_uuid = Uuid::new_v4();
        let supplier_uuid = Uuid::new_v4();

        // Insert product with supplier
        repo.insert_product(
            product_uuid,
            "Supplier Flag Test".into(),
            None,
            Some(supplier_uuid),
            59.99,
            true,
            Utc::now(),
            None,
        )
        .await?;

        let products_with_flags = repo.get_products_with_supplier_flag().await?;

        assert!(!products_with_flags.is_empty());

        // Find our test product
        let test_product = products_with_flags
            .iter()
            .find(|(uuid, _, _)| *uuid == product_uuid)
            .expect("Test product should be found");

        let (_, name, has_supplier) = test_product;
        assert_eq!(name, "Supplier Flag Test");
        assert!(*has_supplier);

        Ok(())
    }

    #[sqlx::test(
        migrations = "tests/migrations_uuid",
        fixtures(path = "fixtures", scripts("products"))
    )]
    async fn test_distinct_categories(pool: Pool) -> Result<()> {
        let repo = ProductRepoImpl { pool: &pool };
        let category1 = Uuid::new_v4();
        let category2 = Uuid::new_v4();

        // Insert products with different categories
        repo.insert_product(
            Uuid::new_v4(),
            "Product 1".into(),
            Some(category1),
            None,
            19.99,
            true,
            Utc::now(),
            None,
        )
        .await?;

        repo.insert_product(
            Uuid::new_v4(),
            "Product 2".into(),
            Some(category2),
            None,
            29.99,
            true,
            Utc::now(),
            None,
        )
        .await?;

        repo.insert_product(
            Uuid::new_v4(),
            "Product 3".into(),
            Some(category1), // Same category as Product 1
            None,
            39.99,
            true,
            Utc::now(),
            None,
        )
        .await?;

        let categories = repo.get_distinct_categories().await?;

        // Should have our 2 categories plus any from fixtures
        assert!(categories.len() >= 2);
        assert!(categories.contains(&category1));
        assert!(categories.contains(&category2));

        Ok(())
    }

    #[sqlx::test(
        migrations = "tests/migrations_uuid",
        fixtures(path = "fixtures", scripts("products"))
    )]
    async fn test_cursor_pagination_by_uuid(pool: Pool) -> Result<()> {
        let repo = ProductRepoImpl { pool: &pool };

        // Insert test products with different UUIDs
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();
        let uuid3 = Uuid::new_v4();

        repo.insert_product(
            uuid1,
            "Product UUID 1".into(),
            None,
            None,
            19.99,
            true,
            Utc::now(),
            None,
        )
        .await?;

        repo.insert_product(
            uuid2,
            "Product UUID 2".into(),
            None,
            None,
            29.99,
            true,
            Utc::now(),
            None,
        )
        .await?;

        repo.insert_product(
            uuid3,
            "Product UUID 3".into(),
            None,
            None,
            39.99,
            true,
            Utc::now(),
            None,
        )
        .await?;

        // First page - get first 2 products ordered by UUID, id
        let params1 = ParamsBuilder::new()
            .sort()
            .asc("uuid")
            .asc("id")
            .done()
            .cursor()
            .first_page()
            .done()
            .limit(2)
            .build();

        let page1 = repo.find_products_cursor_by_uuid(params1).await?;
        assert_eq!(page1.data.len(), 2);

        // Verify order - should be in UUID ascending order
        let first_product = &page1.data[0];
        let second_product = &page1.data[1];
        assert!(
            first_product.uuid < second_product.uuid
                || (first_product.uuid == second_product.uuid
                    && first_product.id < second_product.id),
            "First page should be ordered by UUID, then id"
        );

        // Second page if available
        if page1.has_next {
            let cursor_token = page1.next_cursor.expect("Should have next cursor");
            let params2 = ParamsBuilder::new()
                .sort()
                .asc("uuid")
                .asc("id")
                .done()
                .cursor()
                .next_cursor::<Product>(&cursor_token)
                .done()
                .limit(2)
                .build();

            let page2 = repo.find_products_cursor_by_uuid(params2).await?;

            if !page2.data.is_empty() {
                // Verify we got different products (no overlap)
                let page1_uuids: Vec<Uuid> = page1.data.iter().map(|p| p.uuid).collect();
                let page2_uuids: Vec<Uuid> = page2.data.iter().map(|p| p.uuid).collect();

                for uuid in &page2_uuids {
                    assert!(
                        !page1_uuids.contains(uuid),
                        "Page 2 should not contain products from page 1"
                    );
                }

                // Verify ordering continuity
                let last_page1 = page1.data.last().unwrap();
                let first_page2 = page2.data.first().unwrap();
                assert!(
                    last_page1.uuid < first_page2.uuid
                        || (last_page1.uuid == first_page2.uuid && last_page1.id < first_page2.id),
                    "Page 2 should continue after page 1"
                );
            }
        }

        Ok(())
    }

    #[sqlx::test(
        migrations = "tests/migrations_uuid",
        fixtures(path = "fixtures", scripts("products"))
    )]
    async fn test_get_field_types(pool: Pool) -> Result<()> {
        let repo = ProductRepoImpl { pool: &pool };
        let product_uuid = Uuid::new_v4();
        let category_uuid = Uuid::new_v4();

        let inserted_id = repo
            .insert_product(
                product_uuid,
                "Field Test".into(),
                Some(category_uuid),
                None,
                19.99,
                true,
                Utc::now(),
                None,
            )
            .await?;

        // Test getting UUID
        let uuid_retrieved = repo.get_uuid(inserted_id).await?;
        assert_eq!(uuid_retrieved, Some(product_uuid));

        // Test getting category UUID
        let category = repo.get_category_uuid(inserted_id).await?;
        assert_eq!(category, Some(category_uuid));

        // Test getting supplier UUID (should be None)
        let supplier = repo.get_supplier_uuid(inserted_id).await?;
        assert_eq!(supplier, None);

        Ok(())
    }
}