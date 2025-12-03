-- Insert some initial test data with chrono types for MySQL
INSERT INTO customers (name, email, age, birth_date, created_at, updated_at, last_login) VALUES
('John Doe', 'john@example.com', 35, '1988-06-15', '2024-01-01 10:00:00', '2024-01-01 10:00:00', '2024-01-15 14:30:00'),
('Jane Smith', 'jane@example.com', 28, '1995-03-22', '2024-01-02 11:15:00', '2024-01-02 11:15:00', NULL),
('Bob Wilson', 'bob@example.com', 42, '1981-12-08', '2024-01-03 09:45:00', '2024-01-03 09:45:00', '2024-01-20 16:20:00'),
('Alice Johnson', 'alice@example.com', 31, '1992-11-30', '2024-01-04 15:30:00', '2024-01-04 15:30:00', '2024-01-22 09:10:00'),
('Charlie Brown', 'charlie@example.com', 25, '1998-07-14', '2024-01-05 08:20:00', '2024-01-05 08:20:00', NULL);