-- Insert some initial test data with UUID types
INSERT INTO products (uuid, name, category_uuid, supplier_uuid, price, active, created_at, metadata_json) VALUES
('550e8400-e29b-41d4-a716-446655440001', 'Laptop Computer', '550e8400-e29b-41d4-a716-446655440010', '550e8400-e29b-41d4-a716-446655440020', 999.99, 1, '2024-01-01 10:00:00', '{"brand": "TechCorp", "warranty": "2 years"}'),
('550e8400-e29b-41d4-a716-446655440002', 'Wireless Mouse', '550e8400-e29b-41d4-a716-446655440011', NULL, 29.99, 1, '2024-01-02 11:15:00', '{"color": "black", "dpi": 1600}'),
('550e8400-e29b-41d4-a716-446655440003', 'Mechanical Keyboard', '550e8400-e29b-41d4-a716-446655440011', '550e8400-e29b-41d4-a716-446655440021', 149.99, 0, '2024-01-03 09:45:00', NULL),
('550e8400-e29b-41d4-a716-446655440004', 'USB-C Cable', NULL, '550e8400-e29b-41d4-a716-446655440020', 19.99, 1, '2024-01-04 14:30:00', '{"length": "2m", "type": "USB-C to USB-A"}'),
('550e8400-e29b-41d4-a716-446655440005', 'Monitor Stand', '550e8400-e29b-41d4-a716-446655440012', NULL, 89.99, 1, '2024-01-05 16:20:00', '{"material": "aluminum", "adjustable": true}');