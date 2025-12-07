-- File-based query for alias_test.rs
-- Uses the same table structure as other tests
SELECT id, name, email, age, birth_year
FROM {{user_table}}
WHERE age >= 30
ORDER BY age DESC
