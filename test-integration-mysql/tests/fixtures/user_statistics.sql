-- MySQL aggregation query with UNSIGNED types and functions
SELECT
    COUNT(*) as total_users,
    MIN(age) as min_age,
    MAX(age) as max_age,
    AVG(age) as avg_age
FROM users