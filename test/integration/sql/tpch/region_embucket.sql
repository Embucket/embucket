-- Snowflake-like DDL for TPC-H REGION
CREATE OR REPLACE TABLE {{TABLE_FQN}} (
  r_regionkey INT,
  r_name VARCHAR(25),
  r_comment VARCHAR(152)
);