-- Snowflake-like DDL for TPC-H NATION
CREATE OR REPLACE TABLE {{TABLE_FQN}} (
  n_nationkey INT,
  n_name VARCHAR(25),
  n_regionkey INT,
  n_comment VARCHAR(152)
);