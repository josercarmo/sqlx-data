-- Create files table with MySQL-specific types for BLOB testing
CREATE TABLE files (
    id INT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    content_type VARCHAR(100) NOT NULL,
    file_size INT UNSIGNED NOT NULL,
    data MEDIUMBLOB NOT NULL,
    is_compressed BOOLEAN NOT NULL DEFAULT FALSE
);