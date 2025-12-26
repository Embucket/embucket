# Embucket Lambda

AWS Lambda function for Embucket using cargo-lambda.

## Configuration

The Lambda function is configured using:
- `Cargo.toml`: Package metadata for build and deployment settings (memory, timeout, env vars, included files)
- `Makefile`: Function name and deployment shortcuts (easily customizable)
- `.envrc`: (Optional) Environment variables for direnv users

## Usage

### Quick Start with Makefile

```bash
cd crates/embucket-lambda

# Build and deploy (function name from Makefile)
make deploy

# Deploy to a different function
make deploy FUNCTION_NAME=my-other-function

# Deploy without rebuilding
make deploy-only

# Verify deployment
make verify

# Watch logs
make logs
```

The function name defaults to `embucket-lambda` but can be overridden:
- Via Makefile variable: `make deploy FUNCTION_NAME=my-function`
- Via environment variable: `export FUNCTION_NAME=my-function && make deploy`

### Manual Commands

All commands should be run from the **workspace root** (`/Users/ramp/vcs/embucket`):

```bash
# Build the Lambda function
cargo lambda build --release --arm64 --manifest-path crates/embucket-lambda/Cargo.toml

# Deploy to AWS (function name and config are in Cargo.toml)
cargo lambda deploy --binary-name bootstrap

# The deployment automatically:
# - Deploys to function "embucket-lambda" (from Cargo.toml)
# - Includes the config directory (from Cargo.toml)
# - Applies all settings: memory, timeout, env vars (from Cargo.toml)
```

**Important**: Due to workspace structure, you still need to specify `--binary-name bootstrap`.

### Customization

**Function Name**: Set via:
1. Positional argument: `cargo lambda deploy --binary-name bootstrap my-function-name`
2. Makefile variable: `FUNCTION_NAME=my-function` (default: `embucket-lambda`)
3. Environment variable: `export CARGO_LAMBDA_FUNCTION_NAME=my-function` (if supported by your cargo-lambda version)

**IAM Role**: Only needed when creating a NEW function. For existing functions, the role is preserved.
- To specify: `export AWS_LAMBDA_ROLE_ARN=arn:aws:iam::account:role/YourRole`

**Other Settings** (in `Cargo.toml`):
- Memory: `memory = 3008`
- Timeout: `timeout = 30`
- Included files: `include = ["config"]`

**Environment Variables**
- Set envs using `ENV_FILE=.env` environment variable: 
  ``` sh
  ENV_FILE=".env.dev" make deploy
  ```
- It will deploy envs from `.env` if `ENV_FILE` not specified

### Observability

#### AWS traces
We send events, spans to stdout log in json format, and in case if AWS X-Ray is enabled it enhances traces.
- `RUST_LOG` - Controls verbosity log level. Default to "INFO", possible values: "OFF", "ERROR", "WARN", "INFO", "DEBUG", "TRACE".

#### OpenTelemetry configuration

To work with Opentelemtry, you need an Opentelemetry Collector running in your environment with open telemetry config. 
The easiest way is to add two layers to your lambda deployment. One of which would be your config file with the remote exporter.

1. Create a folder called collector-config and add a file called `config.yml` with the OpenTelemetry Collector [**configuration**](https://opentelemetry.io/docs/languages/sdk-configuration/otlp-exporter/).
2. After which zip the folder with this ocmmand: `zip -r <filename>.zip collector-config`
3. Then publish it to AWS (change the file name and layer name if you want): `aws lambda publish-layer-version 
  --layer-name <layername> 
  --zip-file fileb://<filename>.zip 
  --compatible-runtimes provided.al2 provided.al2023 
  --compatible-architectures arm64`
4. After which provide this as an external env variable (the first layer is the collector itself): `OTEL_COLLECTOR_LAYERS=arn:aws:lambda:us-east-2:184161586896:layer:opentelemetry-collector-arm64-0_19_0:1,arn:aws:lambda:<region>:<account_id>:layer:<layername>:<version>`
5. Now you can deploy the function with the new layer. 

If you later update the configratuin and publish the layer again remember to change the layer `<version>` number, after the first publish it is `1`.

#### Exporting telemetry spans to [**honeycomb.io**](https://docs.honeycomb.io/send-data/opentelemetry/collector/)

OpenTelemrty Collector config example for Honeycomb:
```yaml
receivers:
  otlp:
    protocols:
      grpc:
        endpoint: localhost:4317
      http:
        endpoint: localhost:4318

processors:
  batch:

exporters:
  otlp:
    # You can name these envs anything you want as long as they are the same as in .env file
    endpoint: "${env:HONEYCOMB_ENDPOINT_URL}"
    headers:
      x-honeycomb-team: "${env:HONEYCOMB_API_KEY}"

service:
  pipelines:
    traces:
      receivers: [otlp]
      processors: [batch]
      exporters: [otlp]
```

- Environment variables configuration:
  * `HONEYCOMB_API_KEY` - this is the full ingestion key (not the key id or management key)
  * `HONEYCOMB_ENDPOINT_URL` - check the region it can start be `api.honeycomb.io` or `api.eu1.honeycomb.io`
  * `OTEL_SERVICE_NAME` - is the x-honeycomb-dataset name

### Test locally

```bash
# Start the function locally
cargo lambda watch

# In another terminal, invoke it
cargo lambda invoke --data-file test-event.json
```

### Verify deployment

```bash
# Using snow CLI
snow sql -c lambda -q "SELECT 1 as test_column"

# Using curl
curl -X POST https://<function-url>.lambda-url.us-east-2.on.aws/session/v1/login-request \
  -H "Content-Type: application/json" \
  -d '{"data": {"ACCOUNT_NAME": "account", "LOGIN_NAME": "embucket", "PASSWORD": "embucket", "CLIENT_APP_ID": "test"}}'

# Check logs
aws logs tail /aws/lambda/embucket-lambda --since 5m --follow
```

## Environment Variables

- `LOG_FORMAT`: json
- `METASTORE_CONFIG`: config/metastore.yaml
- `RUST_LOG`: (optional) Set logging level, defaults to "info"


