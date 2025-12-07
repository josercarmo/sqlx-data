-- Enable UUID extension for UUID generation functions
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- Products table for UUID testing with PostgreSQL native UUID type
CREATE TABLE products (
    id BIGSERIAL PRIMARY KEY,
    uuid UUID NOT NULL UNIQUE,                           -- PostgreSQL native UUID type
    name TEXT NOT NULL,
    category_uuid UUID,                                   -- Foreign key to categories (nullable)
    supplier_uuid UUID,                                   -- Foreign key to suppliers (nullable)
    price DECIMAL(10,2) NOT NULL,                        -- Precise decimal for currency
    active BOOLEAN NOT NULL DEFAULT true,                -- Boolean type
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),       -- Timestamp with time zone
    updated_at TIMESTAMPTZ,                              -- Nullable timestamp with time zone
    metadata_json JSONB                                   -- Optional metadata as JSONB
);

-- Indexes for efficient UUID lookups
CREATE INDEX idx_products_uuid ON products(uuid);
CREATE INDEX idx_products_category_uuid ON products(category_uuid);
CREATE INDEX idx_products_supplier_uuid ON products(supplier_uuid);
CREATE INDEX idx_products_active ON products(active);
CREATE INDEX idx_products_created_at ON products(created_at);

-- GIN index for JSONB metadata queries
CREATE INDEX idx_products_metadata_gin ON products USING GIN (metadata_json);

-- Categories table for UUID relationships
CREATE TABLE categories (
    id BIGSERIAL PRIMARY KEY,
    uuid UUID NOT NULL UNIQUE DEFAULT uuid_generate_v4(),
    name TEXT NOT NULL,
    description TEXT
);

-- Suppliers table for UUID relationships
CREATE TABLE suppliers (
    id BIGSERIAL PRIMARY KEY,
    uuid UUID NOT NULL UNIQUE DEFAULT uuid_generate_v4(),
    name TEXT NOT NULL,
    contact_email TEXT
);

-- Add foreign key constraints
ALTER TABLE products
    ADD CONSTRAINT fk_products_category
    FOREIGN KEY (category_uuid)
    REFERENCES categories(uuid)
    ON DELETE SET NULL;

ALTER TABLE products
    ADD CONSTRAINT fk_products_supplier
    FOREIGN KEY (supplier_uuid)
    REFERENCES suppliers(uuid)
    ON DELETE SET NULL;