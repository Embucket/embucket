# Embucket

Embucket is a Snowflake-compatible query engine built on [Apache DataFusion](https://datafusion.apache.org/). It runs as an AWS Lambda function and uses S3 Tables (Apache Iceberg) for storage. Connect with any Snowflake-compatible tool or the [dbt-embucket](https://github.com/Embucket/dbt-embucket) adapter.

## Key features

- SQL dialect support via Apache DataFusion
- AWS Lambda serverless deployment
- S3 Tables (Iceberg/Parquet) storage
- [dbt-embucket](https://github.com/Embucket/dbt-embucket) adapter
- Snowflake CLI compatibility

## Quick start

Run Embucket locally with Docker:

```bash
docker run --name embucket --rm -p 3000:3000 embucket/embucket
```

See the full [Quick Start guide](https://docs.embucket.com/getting-started/quick-start/) for next steps.

## Deploy

Deploy Embucket to AWS Lambda with the pre-built zip:

```bash
aws s3 cp s3://embucket-releases/lambda/embucket-lambda-latest.zip .
```

See the [AWS Lambda deployment guide](https://docs.embucket.com/deploy/aws-lambda/) for configuration and production setup.

## Connect

- **Snowflake CLI** -- connect any Snowflake-compatible client to your Embucket endpoint. See the [Snowflake CLI guide](https://docs.embucket.com/connect/snowflake-cli/).
- **dbt adapter** -- use `dbt-embucket` for dbt pipelines. See the [dbt guide](https://docs.embucket.com/connect/dbt/).

## Documentation

- [Architecture](https://docs.embucket.com/reference/architecture/)
- [Snowflake compatibility](https://docs.embucket.com/reference/snowflake/)
- [Troubleshooting](https://docs.embucket.com/reference/troubleshooting/)

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).

## License

Embucket is licensed under the [Apache License 2.0](LICENSE).
