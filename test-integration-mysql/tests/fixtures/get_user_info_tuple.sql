-- MySQL tuple query with type casting
SELECT
    name,
    age as 'age: u8'
FROM users
WHERE id = ?