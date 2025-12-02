-- Create products table for UUID testing
CREATE TABLE products (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    uuid TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    category_uuid TEXT,
    supplier_uuid TEXT,
    price DECIMAL(10,2) NOT NULL,
    active BOOLEAN NOT NULL DEFAULT 1,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME,
    metadata_json TEXT
);