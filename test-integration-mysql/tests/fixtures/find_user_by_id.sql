-- MySQL-specific query with type casting for strong types
SELECT
    id as 'id!: Id',
    name,
    email,
    age as 'age: u8',
    birth_year as 'birth_year: u16'
FROM users
WHERE id = ?