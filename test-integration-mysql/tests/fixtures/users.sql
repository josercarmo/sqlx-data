-- Comprehensive test data fixture for all test files - MySQL version
-- Based on analysis of all setup_test_db functions
-- Supports: integration_tests, tuple_tests, alias_tests, pagination_tests, etc.

INSERT INTO users (id, name, email, age, birth_year) VALUES
-- Core test users (used in integration_tests, tuple_tests, file_based_tests)
(1, 'Alice', 'alice@example.com', 30, 1993),
(2, 'Bob', 'bob@example.com', 25, 1998),
(3, 'Charlie', 'charlie@example.com', 35, NULL),

-- Additional users for pagination and comprehensive testing
(4, 'Diana', 'diana@example.com', 28, 1996),
(5, 'Eve', 'eve@example.com', 42, 1982),
(6, 'Frank', 'frank@example.com', 35, 1989),
(7, 'Grace', 'grace@example.com', 28, 1996),
(8, 'Henry', 'henry@example.com', 19, 2005),
(9, 'Ivy', 'ivy@example.com', 33, 1991),
(10, 'Jack', 'jack@example.com', 21, 2003),

-- Extra users for extensive pagination testing
(11, 'Karen', 'karen@example.com', 24, 2000),
(12, 'Liam', 'liam@example.com', 26, 1998),
(13, 'Maya', 'maya@example.com', 27, 1997),
(14, 'Noah', 'noah@example.com', 31, 1993),
(15, 'Olivia', 'olivia@example.com', 29, 1995),

-- Users with NULL birth_year for testing nullable fields
(16, 'Paul', 'paul@example.com', 32, NULL),
(17, 'Quinn', 'quinn@example.com', 36, NULL),

-- Users for age filtering tests (need users > 20, > 25, > 30, etc.)
(18, 'Rachel', 'rachel@example.com', 22, 2002),
(19, 'Sam', 'sam@example.com', 38, 1986),
(20, 'Tina', 'tina@example.com', 40, 1984);