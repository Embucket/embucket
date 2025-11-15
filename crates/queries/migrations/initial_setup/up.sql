CREATE TYPE result_format_type AS ENUM (
    'json',
    'arrow'
);

CREATE TYPE query_status_type AS ENUM (
    'created',
    'limit_exceeded',
    'queued',
    'running',
    'successful',
    'failed',
    'canceled',
    'timed_out'
);


CREATE TABLE IF NOT EXISTS queries (
    id UUID PRIMARY KEY,
    request_id UUID NOT NULL,
    request_metadata JSONB NOT NULL,
    sql TEXT NOT NULL,
    source SMALLINT NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE,
    queued_at TIMESTAMP WITH TIME ZONE,
    running_at TIMESTAMP WITH TIME ZONE,
    successful_at TIMESTAMP WITH TIME ZONE,
    failed_at TIMESTAMP WITH TIME ZONE,
    cancelled_at TIMESTAMP WITH TIME ZONE,
    timedout_at TIMESTAMP WITH TIME ZONE,
    duration_ms BIGINT NOT NULL,
    rows_count BIGINT NOT NULL,
    result_format result_format_type NOT NULL,
    status query_status_type NOT NULL,
    error TEXT
);