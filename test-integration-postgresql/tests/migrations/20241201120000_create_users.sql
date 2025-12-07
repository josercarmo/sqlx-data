-- Main users table used across most tests with PostgreSQL strong types
CREATE TABLE users (
    id BIGSERIAL PRIMARY KEY,              -- Auto-incrementing 64-bit integer
    name TEXT NOT NULL,                    -- Variable-length string
    email TEXT NOT NULL,                   -- Email stored as text
    age SMALLINT NOT NULL,                 -- 16-bit integer (age range 0-255)
    birth_year SMALLINT                    -- Optional birth year
);

CREATE UNIQUE INDEX idx_users_email ON users(email);

-- JSON users table for JSONB tests
CREATE TABLE json_users (
    id BIGSERIAL PRIMARY KEY,
    name TEXT NOT NULL,
    profile_json JSONB NOT NULL,           -- Binary JSON for efficient queries
    preferences JSONB                      -- Optional preferences as JSONB
);

-- Create indexes on JSONB columns for better performance
CREATE INDEX idx_json_users_profile ON json_users USING GIN (profile_json);
CREATE INDEX idx_json_users_preferences ON json_users USING GIN (preferences);

-- Files table for BYTEA blob tests
CREATE TABLE files (
    id BIGSERIAL PRIMARY KEY,
    name TEXT NOT NULL,
    content BYTEA,                             -- Binary data storage
    size INTEGER NOT NULL DEFAULT 0,          -- File size in bytes
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Index for file operations
CREATE INDEX idx_files_size ON files(size);
CREATE INDEX idx_files_name ON files(name);

-- Additional tables for comprehensive PostgreSQL testing

-- Test arrays table
CREATE TABLE test_arrays (
    id BIGSERIAL PRIMARY KEY,
    name TEXT NOT NULL,
    numbers INTEGER[],
    texts TEXT[]
);

-- User stats table
CREATE TABLE user_stats (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL,
    avg_age NUMERIC(5,2),
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- User categories table
CREATE TABLE user_categories (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL,
    category TEXT NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- UUID records table
CREATE TABLE uuid_records (
    id BIGSERIAL PRIMARY KEY,
    uuid_id UUID UNIQUE NOT NULL,
    name TEXT NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Computed values table
CREATE TABLE computed_values (
    id BIGSERIAL PRIMARY KEY,
    input_value INTEGER NOT NULL,
    computed INTEGER NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Create indexes for performance
CREATE INDEX idx_user_stats_user_id ON user_stats(user_id);
CREATE INDEX idx_user_categories_user_id ON user_categories(user_id);
CREATE INDEX idx_uuid_records_uuid_id ON uuid_records(uuid_id);