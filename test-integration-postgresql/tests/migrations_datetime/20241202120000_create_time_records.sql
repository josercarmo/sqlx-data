-- Time records table for PostgreSQL datetime testing
CREATE TABLE time_records (
    id BIGSERIAL PRIMARY KEY,
    name TEXT NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    event_date DATE NOT NULL,
    event_time TIME NOT NULL,
    modified_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Indexes for time-based queries
CREATE INDEX idx_time_records_created_at ON time_records(created_at);
CREATE INDEX idx_time_records_event_date ON time_records(event_date);
CREATE INDEX idx_time_records_event_time ON time_records(event_time);

-- PostgreSQL-specific function for last day of month
CREATE OR REPLACE FUNCTION LAST_DAY(input_date DATE)
RETURNS DATE AS $$
BEGIN
    RETURN (DATE_TRUNC('MONTH', input_date) + INTERVAL '1 MONTH - 1 day')::DATE;
END;
$$ LANGUAGE plpgsql IMMUTABLE;