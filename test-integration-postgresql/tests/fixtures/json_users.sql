-- PostgreSQL JSONB test fixture data
INSERT INTO json_users (name, profile_json, preferences) VALUES
(
    'Alice Johnson',
    '{"email": "alice@example.com", "age": 30, "department": "Engineering", "skills": ["Rust", "PostgreSQL", "Docker"], "active": true}'::jsonb,
    '{"theme": "dark", "notifications": true, "language": "en"}'::jsonb
),
(
    'Bob Smith',
    '{"email": "bob@example.com", "age": 25, "department": "Marketing", "skills": ["Analytics", "SEO"], "active": true}'::jsonb,
    '{"theme": "light", "notifications": false, "language": "en"}'::jsonb
),
(
    'Carol Davis',
    '{"email": "carol@example.com", "age": 35, "department": "Engineering", "skills": ["Python", "Kubernetes", "Monitoring"], "active": false, "toDelete": true}'::jsonb,
    NULL
),
(
    'David Wilson',
    '{"email": "david@example.com", "age": 28, "department": "Sales", "skills": ["Communication"], "active": true}'::jsonb,
    '{"theme": "auto", "notifications": true, "language": "es"}'::jsonb
);