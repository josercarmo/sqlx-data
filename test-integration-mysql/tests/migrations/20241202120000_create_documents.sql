-- Create documents table for BLOB testing
CREATE TABLE documents (
    id BIGINT AUTO_INCREMENT PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    content_type VARCHAR(100) NOT NULL,
    file_size INT UNSIGNED NOT NULL,
    data MEDIUMBLOB NOT NULL,
    checksum VARCHAR(64),
    is_compressed BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Create images table for image-specific BLOB testing
CREATE TABLE images (
    id BIGINT AUTO_INCREMENT PRIMARY KEY,
    filename VARCHAR(255) NOT NULL,
    width SMALLINT UNSIGNED NOT NULL,
    height SMALLINT UNSIGNED NOT NULL,
    format VARCHAR(10) NOT NULL,
    thumbnail BLOB,
    original_data MEDIUMBLOB NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);