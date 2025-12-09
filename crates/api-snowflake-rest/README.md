# api-snowflake-rest

Provides a REST API compatible with the Snowflake V1 SQL API, allowing Snowflake SDK clients and tooling to connect and interact with Embucket.

## Purpose

This crate allows tools and applications that use the Snowflake client SDKs to connect to Embucket as if it were a Snowflake instance, enabling query execution and other interactions via the Snowflake SQL API.

## `snow sql` programmatic API
`snow_sql` function provides a programmatic API to interact with the server
via Snowflake REST API in similar way as `snow sql` command line tool does.

There is also a `sql_test` macro which is a wrapper around `snow_sql` function. Test suite uses this helper macro to run SQLs like it was executed from `snow sql`.

## Testing
Set `METASTORE_CONFIG_JSON` env var to stress test on real s3_tables.
``` shell
export METASTORE_CONFIG_JSON="$(yq eval -o=json config/metastore.yaml)"
```

Run tests as usual:
```
cargo test --workspace
```
