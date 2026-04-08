# Embucket

Embucket exposes a Snowflake-compatible API over lakehouse data. The repo currently ships two runtime artifacts:

- `embucketd` for local and self-hosted runs
- `embucket-lambda` for AWS Lambda deployments

## Choose your path

- **Start locally** if you want the fastest test or evaluation loop.
- **Run from source** if you want to build `embucketd` yourself for local evaluation.
- **Deploy on AWS Lambda** if you want the current serverless runtime.
- **Connect dbt** if you want the recommended client path.
- **Run Snowplow web analytics** if you want a fuller example on the Lambda + dbt path.
- **Use S3 Tables** if you want the currently documented external catalog.
- **Troubleshoot** if your client, auth, or runtime setup does not behave as expected.

Relevant guides live under `docs/src/content/docs/`:

- `essentials/quick-start.mdx`
- `essentials/runtime-modes.mdx`
- `guides/aws-lambda.mdx`
- `guides/dbt.mdx`
- `guides/self-hosted.mdx`
- `guides/snowplow.mdx`
- `guides/s3-tables.mdx`
- `guides/troubleshooting.mdx`

If you want to build the local binary instead of using Docker, start with `docs/src/content/docs/guides/self-hosted.mdx`.

If you want a fuller example on the recommended client path, start with `docs/src/content/docs/guides/snowplow.mdx`.

## Support summary

The current docs should make these distinctions explicit:

- **Local mode** is the fastest path for tests and evaluation.
- **AWS Lambda + dbt-embucket** is verified and is the recommended client path.
- **AWS Lambda + Snowflake CLI over Function URL** is tested, but not production-ready because the Function URL is publicly reachable.
- **Production-facing Lambda deployments** should avoid a public Function URL. The AWS Lambda guide includes an anonymized private API Gateway example.
- **AWS S3 Tables** is the currently documented external catalog path.

## Local quick start

Run Embucket locally:

```bash
docker run --name embucket --rm -p 3000:3000 embucket/embucket
```

Expected startup log:

```text
{"timestamp":"2025-07-01T15:35:05.687807Z","level":"INFO","fields":{"message":"Listening on http://0.0.0.0:3000"},"target":"embucketd"}
```

Configure Snowflake CLI for the local endpoint:

```bash
snow --info

# Add this connection block to your Snowflake CLI config file.
[connections.local]
host = "localhost"
region = "us-east-2"
port = 3000
protocol = "http"
database = "embucket"
schema = "public"
warehouse = "em.wh"
account = "acc.local"
user = "embucket"
password = "embucket"
```

Validate the connection and run a query:

```bash
snow connection test -c local
snow sql -c local -q "SELECT 1 AS ok"
```

You can also open `http://127.0.0.1:3000/` to inspect the current Swagger/OpenAPI surface served by `embucketd`.

## AWS Lambda quick pointer

If you want the current serverless path, start with `docs/src/content/docs/guides/aws-lambda.mdx`.

The current runtime is built from `crates/embucket-lambda` and can be deployed with:

```bash
make -C crates/embucket-lambda deploy
```

For test-only validation, you can expose a Function URL and connect Snowflake CLI to it. For production-facing traffic, keep the Lambda private and put an API gateway layer in front of it.

## dbt quick pointer

If you want the recommended client workflow, start with `docs/src/content/docs/guides/dbt.mdx`.

The official adapter lives in the sibling repository `Embucket/dbt-embucket` and uses:

- `type: embucket`
- `function_arn` to reach the deployed Lambda
- `dbt debug` and `dbt run` as the verified end-to-end checks

## S3 Tables quick pointer

The current docs treat AWS S3 Tables as the supported external catalog path. Start with `docs/src/content/docs/guides/s3-tables.mdx` for the YAML shape, AWS prerequisites, and query flow.
