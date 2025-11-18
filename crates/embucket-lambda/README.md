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
- Environment variables: `[package.metadata.lambda.deploy.env]`
- Included files: `include = ["config"]`

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


