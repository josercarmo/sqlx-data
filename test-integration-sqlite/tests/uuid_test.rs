#![cfg(all(feature = "json", feature = "uuid"))]
//! Requires the `uuid` Cargo feature flag.
//!
//! | Rust type                             | Sqlite type(s)                                       |
//! |---------------------------------------|------------------------------------------------------|
//! | `uuid::Uuid`                          | BLOB, TEXT                                           |
//! | `uuid::fmt::Hyphenated`               | TEXT                                                 |
//! | `uuid::fmt::Simple`                   | TEXT                                                 |
//!
//! **BLOB (16 bytes):** Efficient binary format, stores raw UUID bytes.
//! **TEXT (32-36 chars):** Human-readable format, with or without hyphens.
//! 
//! Recommendation: Use `Uuid` for BLOB storage, `Hyphenated`/`Simple` for TEXT storage.
//!

use sqlx_data::{
    Cursor, CursorData, CursorError, CursorSecureExtract, CursorValue, FilterValue, IntoParams, ParamsBuilder, Pool,
    QueryResult, Result, dml, repo,
};

// Import UUID types from sqlx when uuid feature is enabled
use sqlx::types::{Uuid as UuidGenerator, uuid::{self, fmt::Hyphenated as Uuid}};

#[derive(Debug, sqlx::FromRow)]
pub struct Product {
    pub id: i64,
    pub uuid: Uuid,
    pub name: String,
    pub category_uuid: Option<Uuid>,
    pub supplier_uuid: Option<Uuid>,
    pub price: f64,
    pub active: bool,
    pub created_at: String, //string just for test
    pub updated_at: Option<String>,
    pub metadata_json: Option<String>,
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
    all_columns = "id as 'id!' , uuid as 'uuid: Uuid', name, category_uuid as 'category_uuid: Uuid', supplier_uuid as 'supplier_uuid: Uuid', price as 'price!: f64', active, created_at as 'created_at: String', updated_at as 'updated_at: String', metadata_json"
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
        created_at: String,
        metadata_json: Option<String>,
    ) -> Result<i64>;

    #[dml("SELECT {{all_columns}} FROM products WHERE uuid = $1")]
    async fn find_product_by_uuid(&self, uuid: Uuid) -> Result<Option<Product>>;

    #[dml("SELECT {{all_columns}} FROM products WHERE category_uuid = $1")]
    async fn find_products_by_category(&self, category_uuid: Uuid) -> Result<Vec<Product>>;

    #[dml("SELECT uuid as 'uuid: Uuid' FROM products WHERE active = 1")]
    async fn find_active_product_uuids(&self) -> Result<Vec<Uuid>>;

    #[dml("SELECT COUNT(*) FROM products WHERE supplier_uuid = $1")]
    async fn count_products_by_supplier(&self, supplier_uuid: Uuid) -> Result<i64>;

    #[dml("SELECT COUNT(*) > 0 FROM products WHERE uuid = $1")]
    async fn product_exists(&self, uuid: Uuid) -> Result<bool>;

    #[dml("SELECT uuid as 'uuid?: Uuid' FROM products WHERE id = $1")]
    async fn get_product_uuid_by_id(&self, id: i64) -> Result<Option<uuid::fmt::Hyphenated>>;

    #[dml(
        "UPDATE products
         SET updated_at = $2, supplier_uuid = $3
         WHERE uuid = $1"
    )]
    async fn update_product_supplier(
        &self,
        uuid: Uuid,
        updated_at: String,
        supplier_uuid: Option<Uuid>,
    ) -> Result<QueryResult>;

    #[dml("SELECT {{all_columns}} FROM products WHERE supplier_uuid IS NOT NULL")]
    async fn find_products_with_supplier(&self) -> Result<Vec<Product>>;

    #[dml("SELECT {{all_columns}} FROM products WHERE category_uuid IS NULL")]
    async fn find_products_without_category(&self) -> Result<Vec<Product>>;

    #[dml(
        "SELECT
            uuid as 'uuid: Uuid',
            name,
            CASE
                WHEN supplier_uuid IS NOT NULL THEN 1
                ELSE 0
            END as 'has_supplier: bool'
         FROM products"
    )]
    async fn get_products_with_supplier_flag(&self) -> Result<Vec<(Uuid, String, bool)>>;

    #[dml(
        "SELECT category_uuid as 'category_uuid!: Uuid' FROM products WHERE category_uuid IS NOT NULL GROUP BY category_uuid"
    )]
    async fn get_distinct_categories(&self) -> Result<Vec<Uuid>>;

    // Test direct field return types
    #[dml("SELECT uuid FROM products WHERE id = $1")]
    async fn get_uuid(&self, id: i64) -> Result<String>;

    #[dml("SELECT uuid as 'uuid?: Uuid' FROM products WHERE id = $1")] //Deliberated asking for optional
    async fn get_uuid_typed(&self, id: i64) -> Result<Option<Uuid>>;

    #[dml("SELECT category_uuid as 'category_uuid: Uuid' FROM products WHERE id = $1")]
    async fn get_category_uuid(&self, id: i64) -> Result<Option<Uuid>>;

    #[dml("SELECT supplier_uuid as 'supplier_uuid: Uuid' FROM products WHERE id = $1")]
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

    #[sqlx::test(
        migrations = "tests/migrations_uuid",
        fixtures(path = "fixtures", scripts("products"))
    )]
    async fn test_uuid_roundtrip(pool: Pool) -> Result<()> {
        let repo = ProductRepoImpl { pool: &pool };
        let product_uuid = UuidGenerator::new_v4().hyphenated();
        let category_uuid = UuidGenerator::new_v4().hyphenated();
        let supplier_uuid = UuidGenerator::new_v4().hyphenated();

        let inserted_id = repo
            .insert_product(
                product_uuid,
                "Test Product".into(),
                Some(category_uuid),
                Some(supplier_uuid),
                99.99,
                true,
                "2024-01-01T00:00:00Z".into(),
                Some(r#"{"key": "value"}"#.into()),
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
        let category_uuid = UuidGenerator::new_v4().hyphenated();

        // Insert test product
        let product_uuid = UuidGenerator::new_v4().hyphenated();
        repo.insert_product(
            product_uuid,
            "Category Test Product".into(),
            Some(category_uuid),
            None,
            49.99,
            true,
            "2024-01-01T00:00:00Z".into(),
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
        let product_uuid = UuidGenerator::new_v4().hyphenated();

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
                "2024-01-01T00:00:00Z".into(),
                None,
            )
            .await?;

        // Test existence check after insert
        let exists_after = repo.product_exists(product_uuid).await?;
        assert!(exists_after);

        // Test get UUID by ID
        let retrieved_uuid = repo.get_uuid_typed(inserted_id).await?;
        assert_eq!(retrieved_uuid, Some(product_uuid));

        Ok(())
    }

    #[sqlx::test(
        migrations = "tests/migrations_uuid",
        fixtures(path = "fixtures", scripts("products"))
    )]
    async fn test_supplier_operations(pool: Pool) -> Result<()> {
        let repo = ProductRepoImpl { pool: &pool };
        let product_uuid = UuidGenerator::new_v4().hyphenated();
        let supplier_uuid = UuidGenerator::new_v4().hyphenated();

        // Insert product without supplier
        let inserted_id = repo
            .insert_product(
                product_uuid,
                "Supplier Test".into(),
                None,
                None,
                29.99,
                true,
                "2024-01-01T00:00:00Z".into(),
                None,
            )
            .await?;

        // Update with supplier
        repo.update_product_supplier(
            product_uuid,
            "2024-01-02T00:00:00Z".into(),
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
        let active_uuid = UuidGenerator::new_v4().hyphenated();
        repo.insert_product(
            active_uuid,
            "Active Product".into(),
            None,
            None,
            39.99,
            true,
            "2024-01-01T00:00:00Z".into(),
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
        let product_uuid = UuidGenerator::new_v4().hyphenated();
        let supplier_uuid = UuidGenerator::new_v4().hyphenated();

        // Insert product with supplier
        repo.insert_product(
            product_uuid,
            "Supplier Flag Test".into(),
            None,
            Some(supplier_uuid),
            59.99,
            true,
            "2024-01-01T00:00:00Z".into(),
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
        let category1 = UuidGenerator::new_v4().hyphenated();
        let category2 = UuidGenerator::new_v4().hyphenated();

        // Insert products with different categories
        repo.insert_product(
            UuidGenerator::new_v4().hyphenated(),
            "Product 1".into(),
            Some(category1),
            None,
            19.99,
            true,
            "2024-01-01T00:00:00Z".into(),
            None,
        )
        .await?;

        repo.insert_product(
            UuidGenerator::new_v4().hyphenated(),
            "Product 2".into(),
            Some(category2),
            None,
            29.99,
            true,
            "2024-01-01T00:00:00Z".into(),
            None,
        )
        .await?;

        repo.insert_product(
            UuidGenerator::new_v4().hyphenated(),
            "Product 3".into(),
            Some(category1), // Same category as Product 1
            None,
            39.99,
            true,
            "2024-01-01T00:00:00Z".into(),
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
        let uuid1 = UuidGenerator::new_v4().hyphenated();
        let uuid2 = UuidGenerator::new_v4().hyphenated();
        let uuid3 = UuidGenerator::new_v4().hyphenated();

        repo.insert_product(
            uuid1,
            "Product UUID 1".into(),
            None,
            None,
            19.99,
            true,
            "2024-01-01T00:00:00Z".into(),
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
            "2024-01-01T00:00:00Z".into(),
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
            "2024-01-01T00:00:00Z".into(),
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
    async fn test_cursor_pagination_by_category_uuid(pool: Pool) -> Result<()> {
        let repo = ProductRepoImpl { pool: &pool };
        let category1 = UuidGenerator::new_v4().hyphenated();
        let category2 = UuidGenerator::new_v4().hyphenated();

        // Insert products with categories
        repo.insert_product(
            UuidGenerator::new_v4().hyphenated(),
            "Category Product 1".into(),
            Some(category1),
            None,
            19.99,
            true,
            "2024-01-01T00:00:00Z".into(),
            None,
        )
        .await?;

        repo.insert_product(
            UuidGenerator::new_v4().hyphenated(),
            "Category Product 2".into(),
            Some(category2),
            None,
            29.99,
            true,
            "2024-01-01T00:00:00Z".into(),
            None,
        )
        .await?;

        // Test cursor pagination by category_uuid
        let params = ParamsBuilder::default()
            .sort()
            .asc("category_uuid")
            .asc("id")
            .done()
            .cursor()
            .first_page()
            .done()
            .limit(10)
            .build();

        let page = repo.find_products_cursor_by_category_uuid(params).await?;

        // Should only get products with category_uuid (not null)
        for product in &page.data {
            assert!(product.category_uuid.is_some());
        }

        // Verify order - should be in category_uuid ascending order
        if page.data.len() >= 2 {
            let first = &page.data[0];
            let second = &page.data[1];
            assert!(first.category_uuid <= second.category_uuid);
        }

        Ok(())
    }

    #[sqlx::test(
        migrations = "tests/migrations_uuid",
        fixtures(path = "fixtures", scripts("products"))
    )]
    async fn test_cursor_pagination_by_supplier_uuid(pool: Pool) -> Result<()> {
        let repo = ProductRepoImpl { pool: &pool };
        let supplier1 = UuidGenerator::new_v4().hyphenated();
        let supplier2 = UuidGenerator::new_v4().hyphenated();

        // Insert products with suppliers
        repo.insert_product(
            UuidGenerator::new_v4().hyphenated(),
            "Supplier Product 1".into(),
            None,
            Some(supplier1),
            19.99,
            true,
            "2024-01-01T00:00:00Z".into(),
            None,
        )
        .await?;

        repo.insert_product(
            UuidGenerator::new_v4().hyphenated(),
            "Supplier Product 2".into(),
            None,
            Some(supplier2),
            29.99,
            true,
            "2024-01-01T00:00:00Z".into(),
            None,
        )
        .await?;

        // Test cursor pagination by supplier_uuid
        let params = ParamsBuilder::default()
            .sort()
            .asc("supplier_uuid")
            .asc("id")
            .done()
            .cursor()
            .first_page()
            .done()
            .limit(10)
            .build();

        let page = repo.find_products_cursor_by_supplier_uuid(params).await?;

        // Should only get products with supplier_uuid (not null)
        for product in &page.data {
            assert!(product.supplier_uuid.is_some());
        }

        // Verify order - should be in supplier_uuid ascending order
        if page.data.len() >= 2 {
            let first = &page.data[0];
            let second = &page.data[1];
            assert!(first.supplier_uuid <= second.supplier_uuid);
        }

        Ok(())
    }

    #[sqlx::test(
        migrations = "tests/migrations_uuid",
        fixtures(path = "fixtures", scripts("products"))
    )]
    async fn test_get_field_types(pool: Pool) -> Result<()> {
        let repo = ProductRepoImpl { pool: &pool };
        let product_uuid = UuidGenerator::new_v4().hyphenated();
        let category_uuid = UuidGenerator::new_v4().hyphenated();

        let inserted_id = repo
            .insert_product(
                product_uuid,
                "Field Test".into(),
                Some(category_uuid),
                None,
                19.99,
                true,
                "2024-01-01T00:00:00Z".into(),
                None,
            )
            .await?;

        // Test getting UUID as string
        let uuid_string = repo.get_uuid(inserted_id).await?;
        assert_eq!(uuid_string, product_uuid.to_string());

        // Test getting UUID as typed
        let uuid_typed = repo.get_uuid_typed(inserted_id).await?;
        assert_eq!(uuid_typed, Some(product_uuid));

        // Test getting category UUID
        let category = repo.get_category_uuid(inserted_id).await?;
        assert_eq!(category, Some(category_uuid));

        // Test getting supplier UUID (should be None)
        let supplier = repo.get_supplier_uuid(inserted_id).await?;
        assert_eq!(supplier, None);

        Ok(())
    }

    #[sqlx::test(
        migrations = "tests/migrations_uuid",
        fixtures(path = "fixtures", scripts("products"))
    )]
    async fn test_get_product_uuid_by_id(pool: Pool) -> Result<()> {
        let repo = ProductRepoImpl { pool: &pool };
        let product_uuid = UuidGenerator::new_v4().hyphenated();

        // Insert a test product
        let inserted_id = repo
            .insert_product(
                product_uuid,
                "UUID Lookup Test".into(),
                None,
                None,
                49.99,
                true,
                "2024-01-01T00:00:00Z".into(),
                None,
            )
            .await?;

        // Test case 1: Get UUID by existing ID - should return Some(uuid)
        let retrieved_uuid = repo.get_product_uuid_by_id(inserted_id).await?;
        assert_eq!(retrieved_uuid, Some(product_uuid));

        // Test case 2: Get UUID by non-existent ID - should return None
        let non_existent_id = 999999i64; // Very large ID that shouldn't exist
        let no_uuid = repo.get_product_uuid_by_id(non_existent_id).await?;
        assert_eq!(no_uuid, None);

        // Test case 3: Verify with a fixture product (ID should exist from fixtures)
        // We know from fixtures that products with IDs 1-5 should exist
        let fixture_uuid = repo.get_product_uuid_by_id(1).await?;
        assert!(fixture_uuid.is_some());

        // Validate that the returned UUID is a valid UUID (not empty or malformed)
        if let Some(uuid) = fixture_uuid {
            assert_ne!(uuid.to_string(), "");
            assert_eq!(uuid.to_string().len(), 36); // Standard UUID length
        }

        Ok(())
    }
}
