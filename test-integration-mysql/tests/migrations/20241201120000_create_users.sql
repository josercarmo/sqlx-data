-- Grant permissions for test user to create databases for sqlx::test
-- GRANT ALL PRIVILEGES ON *.* TO 'test_user'@'%' WITH GRANT OPTION;
-- FLUSH PRIVILEGES;

-- Main users table adapted for MySQL with strong types
CREATE TABLE users (
    id BIGINT PRIMARY KEY AUTO_INCREMENT,
    name VARCHAR(255) NOT NULL,
    email VARCHAR(255) NOT NULL UNIQUE,
    age TINYINT UNSIGNED NOT NULL,
    birth_year SMALLINT UNSIGNED
);

-- JSON users table for JSON tests using MySQL JSON type
CREATE TABLE json_users (
    id BIGINT PRIMARY KEY AUTO_INCREMENT,
    name VARCHAR(255) NOT NULL,
    profile_json JSON NOT NULL,
    preferences JSON
);