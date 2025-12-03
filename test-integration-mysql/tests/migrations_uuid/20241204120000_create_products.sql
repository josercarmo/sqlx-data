-- Create products table for UUID testing (MySQL version with BINARY(16))
CREATE TABLE IF NOT EXISTS products (
    id BIGINT PRIMARY KEY AUTO_INCREMENT,
    uuid BINARY(16) NOT NULL UNIQUE,
    name VARCHAR(255) NOT NULL,
    category_uuid BINARY(16),
    supplier_uuid BINARY(16),
    price DECIMAL(10,2) NOT NULL,
    active BOOLEAN NOT NULL DEFAULT 1,
    created_at VARCHAR(255) NOT NULL,
    updated_at VARCHAR(255),
    metadata_json JSON
);

-- Create indexes for UUID fields for better performance
-- Note: MySQL 8.0 doesn't support IF NOT EXISTS for indexes
CREATE INDEX idx_products_uuid ON products(uuid);
CREATE INDEX idx_products_category_uuid ON products(category_uuid);
CREATE INDEX idx_products_supplier_uuid ON products(supplier_uuid);