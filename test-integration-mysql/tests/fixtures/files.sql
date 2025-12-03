-- Insert test data with MySQL-specific content for files table
INSERT INTO files (id, name, content_type, file_size, data, is_compressed) VALUES
(1, 'file1.txt', 'text/plain', 49, 'MySQL content for file 1 - testing unsigned types', FALSE),
(2, 'file2.json', 'application/json', 81, '{"database": "mysql", "engine": "InnoDB", "charset": "utf8mb4", "version": "8.0"}', FALSE),
(3, 'file3.bin', 'application/octet-stream', 50, REPEAT(CHAR(42), 50), TRUE);