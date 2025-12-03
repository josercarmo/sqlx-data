-- MySQL query with ORDER BY and LIMIT, strong types
SELECT
    id as 'id!: Id',
    name,
    email,
    age as 'age: u8',
    birth_year as 'birth_year: u16'
FROM users
ORDER BY age DESC
LIMIT ?