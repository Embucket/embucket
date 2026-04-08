# Embucket

Embucket is a Snowflake-compatible query engine built on [Apache DataFusion](https://datafusion.apache.org/). It runs as an AWS Lambda function and uses S3 Tables (Apache Iceberg) for storage. Connect with any Snowflake-compatible tool or the [dbt-embucket](https://github.com/Embucket/dbt-embucket) adapter.

## Key features

- Snowflake v1 REST API compatibility
- SQL dialect support via Apache DataFusion
- AWS Lambda serverless deployment
- S3 Tables (Iceberg/Parquet) storage
- Official dbt adapter for analytics workflows
- Snowflake CLI compatibility

## Quick start

Run Embucket locally with Docker:

```bash
docker run --name embucket --rm -p 3000:3000 embucket/embucket
```

See the full [Quick Start guide](docs/src/content/docs/getting-started/quick-start.mdx) for next steps.

## Deploy

Deploy Embucket to AWS Lambda:

```bash
make -C crates/embucket-lambda deploy
```

See the [AWS Lambda deployment guide](docs/src/content/docs/deploy/aws-lambda.mdx) for configuration and production setup.

## Connect

- **Snowflake CLI** -- connect any Snowflake-compatible client to your Embucket endpoint. See the [Snowflake CLI guide](docs/src/content/docs/connect/snowflake-cli.mdx).
- **dbt adapter** -- use `dbt-embucket` for analytics workflows. See the [dbt guide](docs/src/content/docs/connect/dbt.mdx).

## Documentation

- [Architecture](docs/src/content/docs/reference/architecture.mdx)
- [Snowflake compatibility](docs/src/content/docs/reference/snowflake.mdx)
- [Troubleshooting](docs/src/content/docs/reference/troubleshooting.mdx)

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).

## License

Embucket is licensed under the [Apache License 2.0](LICENSE).
