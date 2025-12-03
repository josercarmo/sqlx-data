CREATE TABLE time_records (
    id BIGINT PRIMARY KEY AUTO_INCREMENT,
    name VARCHAR(255) NOT NULL,
    created_date DATE NOT NULL,
    created_time TIME NOT NULL,
    created_datetime DATETIME NOT NULL,
    created_offset TIMESTAMP NOT NULL
);