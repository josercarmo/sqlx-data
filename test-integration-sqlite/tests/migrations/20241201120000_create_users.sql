-- Main users table used across most tests
CREATE TABLE users (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    email TEXT NOT NULL,
    age INTEGER NOT NULL,
    birth_year INTEGER
);

CREATE UNIQUE INDEX idx_users_email ON users(email);

-- JSON users table for JSON tests
CREATE TABLE json_users (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    profile_json TEXT NOT NULL CHECK (json_valid(profile_json)),
    preferences TEXT CHECK (preferences IS NULL OR json_valid(preferences))
);
