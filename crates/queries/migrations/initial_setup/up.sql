DROP TYPE IF EXISTS result_format_type;
CREATE TYPE result_format_type AS ENUM (
    'json',
    'arrow'
);

DROP TYPE IF EXISTS query_status_type;
CREATE TYPE query_status_type AS ENUM (
    'created',
    'limit_exceeded',
    'queued',
    'running',
    'successful',
    'failed',
    'cancelled',
    'timed_out'
);

CREATE TABLE IF NOT EXISTS queries (
    id UUID PRIMARY KEY,
    sql TEXT NOT NULL,
    status query_status_type NOT NULL,
    source SMALLINT NOT NULL,
    result_format result_format_type NOT NULL,    
    request_id UUID,
    request_metadata JSONB NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL, -- NOT NULL
    limit_exceeded_at TIMESTAMP WITH TIME ZONE,
    queued_at TIMESTAMP WITH TIME ZONE,
    running_at TIMESTAMP WITH TIME ZONE,
    successful_at TIMESTAMP WITH TIME ZONE,
    failed_at TIMESTAMP WITH TIME ZONE,
    cancelled_at TIMESTAMP WITH TIME ZONE,
    timedout_at TIMESTAMP WITH TIME ZONE,
    duration_ms BIGINT NOT NULL,
    rows_count BIGINT NOT NULL,
    error TEXT
);