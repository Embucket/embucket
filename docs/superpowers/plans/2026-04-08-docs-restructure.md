# Docs Restructure Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Restructure Embucket public docs from a confusing repo-centric layout into a clean product-docs structure: Getting Started, Deploy, Connect, Tutorials, Reference.

**Architecture:** Delete 7 old pages and 2 old directories; create 5 new directories with 9 pages. Update astro sidebar config, CI workflow, and package.json. Each page is written by a docs-writer agent, reviewed by a docs-reviewer agent, then stranger-audited at the end. README.md gets a full rewrite.

**Tech Stack:** Astro Starlight, MDX, Vale linter (Google + write-good styles), pnpm

**Spec:** `docs/superpowers/specs/2026-04-08-docs-restructure-design.md`

---

### Task 1: Scaffold directories and update config

**Files:**
- Modify: `docs/astro.config.mjs`
- Modify: `docs/package.json`
- Modify: `.github/workflows/docs-ci.yml`
- Delete: `docs/scripts/validate-docs-smoke.mjs`
- Delete: `docs/src/content/docs/essentials/` (entire directory)
- Delete: `docs/src/content/docs/guides/` (entire directory)
- Delete: `docs/src/content/docs/development/` (entire directory)
- Create: `docs/src/content/docs/getting-started/`
- Create: `docs/src/content/docs/deploy/`
- Create: `docs/src/content/docs/connect/`
- Create: `docs/src/content/docs/tutorials/`
- Create: `docs/src/content/docs/reference/`

- [ ] **Step 1: Create new directories**

```bash
mkdir -p docs/src/content/docs/getting-started
mkdir -p docs/src/content/docs/deploy
mkdir -p docs/src/content/docs/connect
mkdir -p docs/src/content/docs/tutorials
mkdir -p docs/src/content/docs/reference
```

- [ ] **Step 2: Copy images to new locations**

Images referenced by pages need to move with them:

```bash
# Architecture diagram stays with architecture page
cp docs/src/content/docs/essentials/architecture.png docs/src/content/docs/reference/architecture.png
# Quick start UI screenshot
cp docs/src/content/docs/essentials/quick-start-ui.png docs/src/content/docs/getting-started/quick-start-ui.png
# S3 tables query screenshot goes to deploy (absorbed into Lambda page)
cp docs/src/content/docs/guides/s3-tables-query.png docs/src/content/docs/deploy/s3-tables-query.png
# Copy any create-volume images
cp docs/src/content/docs/essentials/create-volume-*.png docs/src/content/docs/deploy/ 2>/dev/null || true
```

- [ ] **Step 3: Delete old directories and files**

```bash
rm -rf docs/src/content/docs/essentials
rm -rf docs/src/content/docs/guides
rm -rf docs/src/content/docs/development
rm docs/scripts/validate-docs-smoke.mjs
```

- [ ] **Step 4: Update astro.config.mjs sidebar**

Replace the sidebar config in `docs/astro.config.mjs`:

```javascript
sidebar: [
  {
    label: 'Getting Started',
    autogenerate: { directory: 'getting-started' },
  },
  {
    label: 'Deploy',
    autogenerate: { directory: 'deploy' },
  },
  {
    label: 'Connect',
    autogenerate: { directory: 'connect' },
  },
  {
    label: 'Tutorials',
    autogenerate: { directory: 'tutorials' },
  },
  {
    label: 'Reference',
    autogenerate: { directory: 'reference' },
  },
],
```

Also update the root redirect:

```javascript
redirects: {
  '/': '/getting-started/quick-start/',
},
```

- [ ] **Step 5: Remove smoke script from package.json**

Remove the `"smoke"` entry from `scripts` in `docs/package.json`:

```json
"smoke": "node ./scripts/validate-docs-smoke.mjs"
```

Delete that line.

- [ ] **Step 6: Update CI workflow**

In `.github/workflows/docs-ci.yml`:

1. Delete the entire `smoke` job (lines 54-89)
2. In the `build` job, change `needs: [format, smoke]` to `needs: [format]`

- [ ] **Step 7: Verify scaffold builds**

```bash
cd docs && pnpm install && pnpm check
```

Expected: passes (no content files yet, but config is valid)

- [ ] **Step 8: Commit scaffold**

```bash
git add -A docs/ .github/workflows/docs-ci.yml
git commit -m "docs: scaffold new directory structure, remove smoke checks"
```

---

### Task 2: Write Quick Start page

**Files:**
- Create: `docs/src/content/docs/getting-started/quick-start.mdx`

**Source material:**
- Old file: `essentials/quick-start.mdx` (read from git: `git show HEAD~1:docs/src/content/docs/essentials/quick-start.mdx`)
- Spec section: "Getting Started / Quick Start"

- [ ] **Step 1: Draft page with docs-writer agent**

Dispatch a docs-writer agent with this prompt:

> Write a Starlight MDX page for `docs/src/content/docs/getting-started/quick-start.mdx`.
>
> **Frontmatter:** title "Quick Start", description "Try Embucket locally with Docker and run your first query.", sidebar order 0.
>
> **Purpose:** Try Embucket locally in 2 minutes before deploying to AWS.
>
> **Content to include (in order):**
> 1. One-line intro: "Try Embucket locally before deploying to AWS."
> 2. Step 1: Start Embucket — `docker run --name embucket --rm -p 3000:3000 embucket/embucket` with expected startup log showing `Listening on http://0.0.0.0:3000`
> 3. Step 2: Configure Snowflake CLI — install with `python -m pip install snowflake-cli`, find config with `snow --info`, add TOML connection block (host=localhost, region=us-east-2, port=3000, protocol=http, database=embucket, schema=public, warehouse=em.wh, account=acc.local, user=embucket, password=embucket). Test with `snow connection test -c local`.
> 4. Step 3: Run first query — `snow sql -c local -q "select dateadd(day, -1, current_timestamp()) as yesterday;"` with expected output.
> 5. Next steps section with exactly 3 links: Deploy to AWS Lambda (`/deploy/aws-lambda/`), Connect with Snowflake CLI (`/connect/snowflake-cli/`), Connect with dbt (`/connect/dbt/`)
>
> **Rules:**
> - No "Owner/Last reviewed" blockquotes
> - No web UI mention
> - No hedging language ("test and evaluation", "not production-ready")
> - No self-hosted binary references
> - Use Starlight `Steps` component for the numbered steps
> - Import `{ Aside, Steps }` from `@astrojs/starlight/components`
> - Write as clean product documentation
> - Vale linter uses Google style guide — use second person ("you"), active voice, present tense

- [ ] **Step 2: Review page with docs-reviewer agent**

Dispatch a docs-reviewer agent with this prompt:

> Review `docs/src/content/docs/getting-started/quick-start.mdx` against these criteria:
> 1. No "Owner:", "Last reviewed:", or internal metadata visible
> 2. No repo-centric language ("the repo", "current docs", "currently documented")
> 3. No hedging ("test and evaluation path", "not production-ready", "the confusion around")
> 4. No web UI or HTTP surface mention
> 5. No self-hosted binary references
> 6. Next steps has exactly 3 links to /deploy/aws-lambda/, /connect/snowflake-cli/, /connect/dbt/
> 7. Uses Starlight Steps component
> 8. All code blocks have language tags
> 9. Would pass Vale with Google style (second person, active voice, present tense)
> Report issues as a numbered list. If clean, say "PASS".

- [ ] **Step 3: Fix any review findings**

- [ ] **Step 4: Verify build**

```bash
cd docs && pnpm build 2>&1 | head -50
```

Expected: build succeeds, no broken links for this page

- [ ] **Step 5: Commit**

```bash
git add docs/src/content/docs/getting-started/quick-start.mdx
git commit -m "docs: add Quick Start page"
```

---

### Task 3: Write AWS Lambda deployment page

**Files:**
- Create: `docs/src/content/docs/deploy/aws-lambda.mdx`

**Source material:**
- Old files (from git): `guides/aws-lambda.mdx`, `guides/s3-tables.mdx`, `essentials/runtime-modes.mdx`
- `crates/embucket-lambda/Cargo.toml` — Lambda deploy metadata (memory 3008, timeout 30)
- `crates/embucket-lambda/Makefile` — deploy targets and variables
- `crates/state-store/README.md` — DynamoDB setup, env vars, table schema

- [ ] **Step 1: Draft page with docs-writer agent**

Dispatch a docs-writer agent with this prompt:

> Write a Starlight MDX page for `docs/src/content/docs/deploy/aws-lambda.mdx`.
>
> **Frontmatter:** title "AWS Lambda", description "Deploy Embucket to AWS Lambda with S3 Tables storage.", sidebar order 0.
>
> **Purpose:** Single page covering the full path from zero to a running Embucket Lambda with S3 Tables.
>
> **Content sections (in order):**
>
> 1. **Intro** (2 sentences): Embucket runs as an AWS Lambda function using S3 Tables (Apache Iceberg) for storage. This guide covers the full deployment from creating your S3 table bucket through verifying a working query.
>
> 2. **Prerequisites**: Rust toolchain, `cargo-lambda` (`cargo install cargo-lambda`), AWS CLI installed and configured, AWS credentials with Lambda/S3Tables/IAM permissions.
>
> 3. **Create an S3 table bucket**: Use `aws s3tables create-table-bucket --name my-table-bucket --region us-east-2`. Show the JSON response with ARN. Tell user to save bucket name, region, and ARN.
>
> 4. **Configure the metastore**: Create a YAML config file at `config/metastore.yaml`:
> ```yaml
> volumes:
>   - ident: embucket
>     type: s3-tables
>     database: demo
>     credentials:
>       credential_type: access_key
>       aws-access-key-id: ACCESS_KEY
>       aws-secret-access-key: SECRET_ACCESS_KEY
>     arn: arn:aws:s3tables:us-east-2:123456789012:bucket/my-table-bucket
> ```
> Add an Aside tip to replace placeholders with real values.
>
> 5. **Build and deploy**: From repo root: `make -C crates/embucket-lambda deploy`. Show env file variant: `ENV_FILE=config/.env.lambda make -C crates/embucket-lambda deploy`. Show role ARN variant: `AWS_LAMBDA_ROLE_ARN=arn:aws:iam::123456789012:role/embucket-lambda-role make -C crates/embucket-lambda deploy`. Mention the Makefile variables: `FUNCTION_NAME` (default: embucket-lambda), `ENV_FILE`, `AWS_LAMBDA_ROLE_ARN`, `FEATURES`, `LAYERS`, `WITH_OTEL_CONFIG`.
>
> 6. **Verify the deployment**: `make -C crates/embucket-lambda verify` (runs `snow sql -c lambda -q "SELECT 1 as test_column"`). Also `make -C crates/embucket-lambda logs` to tail CloudWatch. Show the direct curl login-request example for HTTP-level validation.
>
> 7. **IAM and access**: Two concerns: (a) deployer permissions — identity running `cargo lambda deploy`; (b) execution role — the Lambda function itself. Mention that `dbt-embucket` uses AWS credentials to invoke Lambda directly by ARN. Plan for: permission to deploy/update, permission to read logs, permission for dbt clients to invoke.
>
> 8. **Lambda sizing**: Default memory is 3008 MB, timeout is 30 seconds (set in Cargo.toml deploy metadata). 3008 MB is the standard Lambda maximum — increasing beyond that requires an AWS support ticket (up to 10 GB). Override memory/timeout through `cargo lambda deploy` flags or AWS Console. For larger datasets or complex queries, consider requesting a memory increase. Tracing is set to Active by default.
>
> 9. **Statestore (optional)**: For persistent query state across invocations, build with the state-store feature flag: `FEATURES=state-store-query make -C crates/embucket-lambda deploy`. Requires a DynamoDB table. Show the `aws dynamodb create-table` command creating table `embucket-statestore` with PK/SK keys and GSIs for query_id, request_id, session_id. Environment variables: `STATESTORE_TABLE_NAME` (default: embucket-statestore), `STATESTORE_DYNAMODB_ENDPOINT` (for local testing: http://localhost:8000), `AWS_DDB_ACCESS_KEY_ID`, `AWS_DDB_SECRET_ACCESS_KEY`, `AWS_DDB_SESSION_TOKEN` (optional, for temporary credentials).
>
> 10. **Production ingress**: For production traffic, keep the Lambda private and put an API Gateway in front. Show the CloudFormation skeleton:
> ```yaml
> Parameters:
>   LambdaFunctionName:
>     Type: String
>     Default: embucket-lambda
>   VpcId:
>     Type: AWS::EC2::VPC::Id
>   SubnetIds:
>     Type: List<AWS::EC2::Subnet::Id>
>   VpcCidr:
>     Type: String
>     Default: 10.0.0.0/16
>
> Resources:
>   ExecuteApiVpcEndpoint:
>     Type: AWS::EC2::VPCEndpoint
>   PrivateApi:
>     Type: AWS::ApiGateway::RestApi
>   LambdaInvokePermission:
>     Type: AWS::Lambda::Permission
> ```
> Note: provisions a private API Gateway, VPC endpoint for execute-api, Lambda proxy integration, stage named v1. Caution Aside: a public Function URL is acceptable for testing but should not be used for production traffic.
>
> 11. **Rollback and redeploy**: Keep previous env file and metastore config in version control. Redeploy with previous config if a change causes regression. Re-run verification after rollback.
>
> 12. **Cleanup**: `aws lambda delete-function-url-config --function-name embucket-lambda` to remove Function URL. Also clean up Lambda function, CloudWatch log group, API Gateway and VPC endpoint if used, telemetry layers.
>
> 13. **Troubleshooting** (inline at bottom): Deploy succeeds but queries fail → check METASTORE_CONFIG points to real file. dbt cannot connect → check AWS credentials and EMBUCKET_FUNCTION_ARN. Timeouts or truncated responses → review timeout and memory settings. No useful traces → verify RUST_LOG, TRACING_LEVEL, OTEL config.
>
> **Rules:**
> - No "Owner/Last reviewed" blockquotes
> - No repo-centric language
> - Import `{ Aside, Steps }` from `@astrojs/starlight/components`
> - Use Steps component for the main deployment flow
> - Vale Google style: second person, active voice, present tense
> - This is the most important page in the docs — it must be thorough and clear

- [ ] **Step 2: Review page with docs-reviewer agent**

Dispatch a docs-reviewer agent:

> Review `docs/src/content/docs/deploy/aws-lambda.mdx` against the spec at `docs/superpowers/specs/2026-04-08-docs-restructure-design.md`, section "Deploy / AWS Lambda". Check:
> 1. All 12 content sections present (prerequisites through troubleshooting)
> 2. S3 table bucket creation absorbed from old s3-tables guide
> 3. Statestore section with feature flag `-F state-store` and DynamoDB setup
> 4. Lambda sizing section mentioning 3008 MB default and AWS support ticket for >3008
> 5. No "Owner/Last reviewed", no repo-centric language, no internal commentary
> 6. All code blocks have language tags
> 7. Asides used appropriately (caution for Function URL, tips for config)
> Report issues as a numbered list. If clean, say "PASS".

- [ ] **Step 3: Fix any review findings**

- [ ] **Step 4: Verify build**

```bash
cd docs && pnpm build 2>&1 | head -50
```

- [ ] **Step 5: Commit**

```bash
git add docs/src/content/docs/deploy/aws-lambda.mdx
git commit -m "docs: add AWS Lambda deployment page"
```

---

### Task 4: Write Configuration page

**Files:**
- Create: `docs/src/content/docs/deploy/configuration.mdx`

**Source material:**
- Old file (from git): `essentials/configuration.mdx`
- `crates/embucketd/src/cli.rs` — for flag/env var reference
- `crates/embucket-lambda/Makefile` — Makefile variables
- `crates/state-store/README.md` — statestore env vars

- [ ] **Step 1: Draft page with docs-writer agent**

Dispatch a docs-writer agent:

> Write a Starlight MDX page for `docs/src/content/docs/deploy/configuration.mdx`.
>
> **Frontmatter:** title "Configuration", description "Environment variables, flags, and metastore settings for Embucket.", sidebar order 1.
>
> **Purpose:** Reference for all configuration options.
>
> **Content sections:**
>
> 1. **Configuration precedence**: flags (highest) > environment variables > .env file (loaded at startup via dotenv).
>
> 2. **Core runtime settings** table:
>
> | Purpose | Flag | Environment variable | Default |
> |---|---|---|---|
> | metastore config path | `--metastore-config` | `METASTORE_CONFIG` | unset |
> | bind host | `--host` | `BUCKET_HOST` | `localhost` |
> | bind port | `--port` | `BUCKET_PORT` | `3000` |
> | result serialization | `--data-format` | `DATA_FORMAT` | `json` |
> | parser dialect | `--sql-parser-dialect` | `SQL_PARSER_DIALECT` | `snowflake` |
> | query concurrency | `--max-concurrency-level` | `MAX_CONCURRENCY_LEVEL` | `8` |
> | query timeout | `--query-timeout-secs` | `QUERY_TIMEOUT_SECS` | `1200` |
> | demo user | `--auth-demo-user` | `AUTH_DEMO_USER` | `embucket` |
> | demo password | `--auth-demo-password` | `AUTH_DEMO_PASSWORD` | `embucket` |
> | JWT signing secret | `--jwt-secret` | `JWT_SECRET` | unset |
> | tracing level | `--tracing-level` | `TRACING_LEVEL` | `info` |
> | service idle timeout | `--idle-timeout-seconds` | `IDLE_TIMEOUT_SECONDS` | `18000` |
>
> 3. **Metastore configuration**: Explain METASTORE_CONFIG pointing to a YAML file. Show minimal example (`volumes: []`). Show S3 Tables volume example:
> ```yaml
> volumes:
>   - ident: embucket
>     type: s3-tables
>     database: demo
>     credentials:
>       credential_type: access_key
>       aws-access-key-id: ACCESS_KEY
>       aws-secret-access-key: SECRET_ACCESS_KEY
>     arn: arn:aws:s3tables:us-east-2:123456789012:bucket/my-table-bucket
> ```
> Show external Iceberg tables on S3 example:
> ```yaml
> volumes:
>   - ident: lakehouse
>     type: s3
>     region: us-east-2
>     bucket: YOUR_BUCKET_NAME
>     credentials:
>       credential_type: access_key
>       aws-access-key-id: YOUR_ACCESS_KEY
>       aws-secret-access-key: YOUR_SECRET_KEY
>
> databases:
>   - ident: demo
>     volume: lakehouse
>
> schemas:
>   - database: demo
>     schema: tpch_10
>
> tables:
>   - database: demo
>     schema: tpch_10
>     table: customer
>     metadata_location: s3://YOUR_BUCKET_NAME/tpch_10/customer/metadata/00001.metadata.json
> ```
>
> 4. **Statestore settings** table:
>
> | Variable | Purpose | Default |
> |---|---|---|
> | `STATESTORE_TABLE_NAME` | DynamoDB table name | `embucket-statestore` |
> | `STATESTORE_DYNAMODB_ENDPOINT` | DynamoDB endpoint (local testing) | unset |
> | `AWS_DDB_ACCESS_KEY_ID` | DynamoDB access key | unset |
> | `AWS_DDB_SECRET_ACCESS_KEY` | DynamoDB secret key | unset |
> | `AWS_DDB_SESSION_TOKEN` | Temporary credentials token | unset |
>
> 5. **Lambda deploy-time variables** table:
>
> | Variable | Purpose |
> |---|---|
> | `FUNCTION_NAME` | Override Lambda function name (default: embucket-lambda) |
> | `ENV_FILE` | Env file path (default: config/.env.lambda) |
> | `AWS_LAMBDA_ROLE_ARN` | Execution role ARN for new functions |
> | `WITH_OTEL_CONFIG` | OpenTelemetry collector config file path |
> | `FEATURES` | Cargo features, comma-separated (e.g., `state-store-query`) |
> | `LAYERS` | Additional Lambda layer ARNs |
>
> 6. **Memory and performance tuning** table:
>
> | Variable | Purpose | Default |
> |---|---|---|
> | `MEM_POOL_TYPE` | Memory pool type | unset |
> | `MEM_POOL_SIZE_MB` | Memory pool size in MB | unset |
> | `DISK_POOL_SIZE_MB` | Disk pool size in MB | unset |
> | `AWS_SDK_CONNECT_TIMEOUT_SECS` | AWS SDK connection timeout | unset |
> | `AWS_SDK_OPERATION_TIMEOUT_SECS` | AWS SDK operation timeout | unset |
> | `OBJECT_STORE_TIMEOUT_SECS` | Object store operation timeout | 30 |
> | `OBJECT_STORE_CONNECT_TIMEOUT_SECS` | Object store connection timeout | 3 |
>
> 7. **Authentication defaults**: Demo credentials are `embucket`/`embucket`. Override with `AUTH_DEMO_USER` and `AUTH_DEMO_PASSWORD`. Use stronger credentials in shared environments.
>
> **Rules:**
> - No "Owner/Last reviewed" blockquotes
> - No "backed by the repository" or "visible in cli.rs" language
> - Import `{ Aside }` from `@astrojs/starlight/components`
> - Vale Google style

- [ ] **Step 2: Review page with docs-reviewer agent**

> Review `docs/src/content/docs/deploy/configuration.mdx`. Check: all 7 sections present, statestore vars included, Lambda Makefile vars included, no internal language, tables are complete and accurate.

- [ ] **Step 3: Fix any review findings**

- [ ] **Step 4: Verify build and commit**

```bash
cd docs && pnpm build 2>&1 | head -50
git add docs/src/content/docs/deploy/configuration.mdx
git commit -m "docs: add Configuration reference page"
```

---

### Task 5: Write Snowflake CLI page

**Files:**
- Create: `docs/src/content/docs/connect/snowflake-cli.mdx`

**Source material:**
- Old file (from git): `guides/snowflake-cli.mdx`

- [ ] **Step 1: Draft page with docs-writer agent**

Dispatch a docs-writer agent:

> Write a Starlight MDX page for `docs/src/content/docs/connect/snowflake-cli.mdx`.
>
> **Frontmatter:** title "Snowflake CLI", description "Connect the Snowflake command-line tool to Embucket.", sidebar order 0.
>
> **Purpose:** Connect the standard Snowflake CLI to a running Embucket instance.
>
> **Content:**
> 1. Intro (one line): The standard Snowflake command-line tool works with Embucket through its Snowflake-compatible REST API.
> 2. Prerequisites: Python 3.8+, Snowflake CLI (`python -m pip install snowflake-cli`), a running Embucket instance (local via Docker or deployed Lambda)
> 3. Configure connection: Find config with `snow --info`. Show TOML block for local: `[connections.local]` with host=localhost, region=us-east-2, port=3000, protocol=http, database=embucket, schema=public, warehouse=em.wh, account=acc.local, user=embucket, password=embucket. Test with `snow connection test -c local`.
> 4. Run a query: `snow sql -c local -q "SELECT 1 AS ok"` with expected output.
> 5. Troubleshooting (inline): Protocol errors → set protocol=http. Auth failures → use embucket/embucket. Connection refused → check Docker is running on port 3000. No data → configure metastore or see S3 table bucket setup in deploy guide.
>
> Wherever a Function URL or Lambda deployment detail is mentioned, add an `<Aside type="tip">` linking to the [AWS Lambda deployment guide](/deploy/aws-lambda/).
>
> **Rules:**
> - No "Owner/Last reviewed"
> - No "current local test and evaluation client flow" framing
> - No references to runtime-modes or support-matrix
> - Import `{ Aside, Steps }` from `@astrojs/starlight/components`
> - Use Steps for the setup flow
> - Vale Google style

- [ ] **Step 2: Review, fix, build, commit**

```bash
cd docs && pnpm build 2>&1 | head -50
git add docs/src/content/docs/connect/snowflake-cli.mdx
git commit -m "docs: add Snowflake CLI connection page"
```

---

### Task 6: Write dbt adapter page

**Files:**
- Create: `docs/src/content/docs/connect/dbt.mdx`

**Source material:**
- Old files (from git): `guides/dbt.mdx`, `guides/end-to-end-dbt.mdx`

- [ ] **Step 1: Draft page with docs-writer agent**

Dispatch a docs-writer agent:

> Write a Starlight MDX page for `docs/src/content/docs/connect/dbt.mdx`.
>
> **Frontmatter:** title "dbt adapter", description "Connect dbt to Embucket using the dbt-embucket adapter.", sidebar order 1.
>
> **Purpose:** Connect dbt to Embucket via Lambda invoke — no public endpoint needed.
>
> **Content:**
> 1. Intro: "[dbt](https://docs.getdbt.com/) is a SQL-first transformation tool for analytics engineering. The `dbt-embucket` adapter connects to Embucket by invoking the Lambda function directly through AWS APIs, so no public endpoint is required."
> 2. Prerequisites: A deployed Embucket Lambda (Aside tip linking to /deploy/aws-lambda/), the Lambda function ARN, AWS credentials that can invoke the function, Python and dbt installed locally.
> 3. Step 1 — Install adapter: `python -m pip install dbt-embucket`
> 4. Step 2 — Create minimal project: mkdir, create `dbt_project.yml` (name: embucket_demo, version: 1.0.0, config-version: 2, profile: embucket, model-paths: ['models'], models: embucket_demo: +materialized: view), create `models/hello_embucket.sql` (`select 1 as id, 'hello embucket' as message`).
> 5. Step 3 — Configure profile: Add to `profiles.yml`:
> ```yaml
> embucket:
>   target: dev
>   outputs:
>     dev:
>       type: embucket
>       function_arn: "{{ env_var('EMBUCKET_FUNCTION_ARN') }}"
>       account: "{{ env_var('EMBUCKET_ACCOUNT', 'embucket') }}"
>       user: "{{ env_var('EMBUCKET_USER', 'embucket') }}"
>       password: "{{ env_var('EMBUCKET_PASSWORD', 'embucket') }}"
>       database: "{{ env_var('EMBUCKET_DATABASE', 'demo') }}"
>       schema: public
>       threads: 1
> ```
> Export env vars: `export EMBUCKET_FUNCTION_ARN=arn:aws:lambda:us-east-2:123456789012:function:embucket-lambda` (and optionally EMBUCKET_ACCOUNT, EMBUCKET_USER, EMBUCKET_PASSWORD, EMBUCKET_DATABASE).
> 6. Profile field reference table: type (required, must be embucket), function_arn (required, target Lambda ARN), account (required, logical account identifier), user (required, login name), password (required, login password), database (required, default database), schema (required, default schema), threads (required, dbt concurrency).
> 7. Step 4 — Check connection: `dbt debug`. Expected: "Connection test: [OK connection ok] / All checks passed!"
> 8. Step 5 — Run a model: `dbt run`. Expected: builds hello_embucket successfully.
> 9. Step 6 — Verify result: If Snowflake CLI is configured for same environment: `snow sql -c lambda -q "select * from demo.public.hello_embucket"`. Expected: one row with id=1, message='hello embucket'. Aside tip: the adapter talks to Lambda by ARN, no public Function URL needed.
> 10. Caveats (stated as facts, no hedging): The adapter uses Lambda invoke transport (not a TCP connection). Python models are not supported. Use AWS credentials that can invoke the target Lambda function.
> 11. Troubleshooting: profiles.yml not found → check dbt profiles directory. EMBUCKET_FUNCTION_ARN missing → export it before dbt debug. AWS credentials missing → adapter needs Lambda invoke permission. Auth failures → check EMBUCKET_USER and EMBUCKET_PASSWORD. Runs succeed but data missing → check database, schema, and metastore config on Lambda side.
> 12. Next step: For a complete analytics pipeline example, see [Snowplow web analytics](/tutorials/snowplow/).
>
> **Rules:**
> - Must include "what is dbt" in intro
> - Aside linking to Deploy page wherever deployment is mentioned
> - No "recommended client path in the current docs" hedging
> - Import `{ Aside, Steps }` from `@astrojs/starlight/components`
> - Use Steps for setup flow
> - Vale Google style

- [ ] **Step 2: Review, fix, build, commit**

```bash
cd docs && pnpm build 2>&1 | head -50
git add docs/src/content/docs/connect/dbt.mdx
git commit -m "docs: add dbt adapter connection page"
```

---

### Task 7: Write Snowplow tutorial page

**Files:**
- Create: `docs/src/content/docs/tutorials/snowplow.mdx`

**Source material:**
- Old file (from git): `guides/snowplow.mdx`

- [ ] **Step 1: Draft page with docs-writer agent**

Dispatch a docs-writer agent:

> Write a Starlight MDX page for `docs/src/content/docs/tutorials/snowplow.mdx`.
>
> **Frontmatter:** title "Snowplow web analytics", description "Build a real-world analytics pipeline with Embucket, dbt, and Snowplow.", sidebar order 0.
>
> **Purpose:** Flagship tutorial — build a complete analytics pipeline on Embucket.
>
> **Content** (mostly preserved from old page, with cleanup):
> 1. Intro: Build a complete web analytics pipeline using Embucket on AWS Lambda with the dbt-embucket adapter. This tutorial follows the [embucket-snowplow](https://github.com/Embucket/embucket-snowplow) repository.
> 2. What you'll build: Deploy a Snowplow analytics runtime on Lambda, run dbt transformations, inspect derived analytics tables.
> 3. Prerequisites: AWS credentials, an S3 Table Bucket ARN, `uv` or Python environment, Git. Aside tip linking to /deploy/aws-lambda/ for Lambda deployment.
> 4. Step 1: Clone repo — `git clone https://github.com/Embucket/embucket-snowplow.git && cd embucket-snowplow`
> 5. Step 2: Set deploy values — STACK_NAME and BUCKET_ARN variables.
> 6. Step 3: Deploy Lambda stack — `aws cloudformation deploy` command with template, stack name, capabilities, parameter overrides. Capture LAMBDA_ARN from stack outputs.
> 7. Step 4: Install dependencies — `uv sync`
> 8. Step 5: Configure dbt profile — `cp profiles.yml.example profiles.yml` and sed the ARN.
> 9. Step 6: Install dbt packages — `uv run dbt deps --profiles-dir .`
> 10. Step 7: Patch if needed — `./scripts/patch_snowplow.sh` (compatibility workaround for packages checking target.type == 'snowflake'). Note: skip if your packages already support embucket.
> 11. Step 8: Load example data — `uv run python scripts/load_data.py "$LAMBDA_ARN"`
> 12. Step 9: Run pipeline — `uv run dbt seed --profiles-dir .` then `uv run dbt run --profiles-dir .`
> 13. Step 10: Verify — `uv run dbt show` against snowplow_web_page_views, snowplow_web_sessions, snowplow_web_users in demo.atomic_derived.
> 14. Cleanup: `aws cloudformation delete-stack --stack-name "$STACK_NAME"`. Note: doesn't remove data from S3 Table Bucket.
>
> **Rules:**
> - No "Owner/Last reviewed"
> - No "current recommended runtime and client path" language
> - No references to runtime-modes, support-matrix, or deleted pages
> - Aside linking to /deploy/aws-lambda/ wherever deployment is mentioned
> - Import `{ Aside, Steps }` from `@astrojs/starlight/components`
> - Use Steps for the tutorial flow
> - Vale Google style

- [ ] **Step 2: Review, fix, build, commit**

```bash
cd docs && pnpm build 2>&1 | head -50
git add docs/src/content/docs/tutorials/snowplow.mdx
git commit -m "docs: add Snowplow web analytics tutorial"
```

---

### Task 8: Write Architecture page

**Files:**
- Create: `docs/src/content/docs/reference/architecture.mdx`

**Source material:**
- Old file (from git): `essentials/architecture.mdx`
- Copy `architecture.png` to reference/ (done in Task 1)

- [ ] **Step 1: Draft page with docs-writer agent**

Dispatch a docs-writer agent:

> Write a Starlight MDX page for `docs/src/content/docs/reference/architecture.mdx`.
>
> **Frontmatter:** title "Architecture", description "How Embucket handles queries, metadata, storage, and authentication.", sidebar order 0.
>
> **Purpose:** Explain how Embucket works internally using a five-layer model.
>
> **Content:**
> 1. Intro: Embucket exposes a Snowflake-compatible API over lakehouse data. It separates into five layers: runtime, metadata, storage, query execution, and authentication.
> 2. Include the architecture diagram: `![Embucket architecture](architecture.png)`
> 3. **Runtime**: Embucket runs as an AWS Lambda function (`embucket-lambda`) for production use. A local binary (`embucketd`) is available for evaluation via Docker. Both share the same Snowflake-compatible API router. Aside linking to /deploy/aws-lambda/.
> 4. **Metadata**: Embucket loads metadata from configuration or external catalogs. Supported paths: YAML metastore config (via METASTORE_CONFIG), AWS S3 Tables as an external catalog, external Iceberg table definitions in the metastore YAML. Aside linking to /deploy/configuration/.
> 5. **Storage**: Data stays in your object storage. Embucket uses Apache Iceberg metadata, Parquet data files, and AWS S3 or S3-compatible storage.
> 6. **Query execution**: Embucket executes Snowflake-flavored SQL through Apache DataFusion. Query execution is single-node per request — each invocation handles complete queries independently. There is no distributed query plan across nodes.
> 7. **Authentication and sessions**: Snowflake-compatible HTTP surface with endpoints: /session/v1/login-request, /session, /queries/v1/query-request, /queries/v1/abort-request. Default demo credentials: embucket/embucket. JWT token lifetime: 3 days. Session inactivity expiry: 60 seconds.
>
> **Rules:**
> - No "Owner/Last reviewed"
> - No "The repo currently ships two runtime artifacts" — describe the system, not the repo
> - No repo-centric language at all
> - Aside tips linking to deploy/configuration where appropriate
> - Import `{ Aside }` from `@astrojs/starlight/components`
> - Vale Google style

- [ ] **Step 2: Review, fix, build, commit**

```bash
cd docs && pnpm build 2>&1 | head -50
git add docs/src/content/docs/reference/architecture.mdx
git commit -m "docs: add Architecture reference page"
```

---

### Task 9: Write Snowflake compatibility page

**Files:**
- Create: `docs/src/content/docs/reference/snowflake.mdx`

**Source material:**
- Current file (from git HEAD): `essentials/snowflake.mdx`
- Previous version with full content: `git show eebec735:docs/src/content/docs/essentials/snowflake.mdx`

- [ ] **Step 1: Draft page with docs-writer agent**

Dispatch a docs-writer agent:

> Write a Starlight MDX page for `docs/src/content/docs/reference/snowflake.mdx`.
>
> **Frontmatter:** title "Snowflake compatibility", description "Snowflake compatibility status, known differences, and development roadmap.", sidebar order 1.
>
> **Purpose:** What works, what differs from Snowflake, known limitations. This is a comprehensive reference.
>
> **Content (restore full content from previous versions):**
>
> 1. **Intro**: Embucket provides Snowflake compatibility through SQL dialect support and API compatibility. Key compatibility features: Snowflake v1 REST API, SQL dialect via Apache DataFusion, compatible with snowflake-connector-python and dependent tools, integration with dbt, snowflake-cli, and Apache Superset.
>
> 2. **API compatibility**: Snowflake v1 REST API that works with any Snowflake client. Tested primarily with snowflake-connector-python and tools that depend on it.
>
> 3. **SQL engine compatibility**: Based on Apache DataFusion (Apache Arrow-native query engine). Aims for 100% SQL dialect compatibility with Snowflake but supports a subset of features. Includes some built-in functions that Snowflake doesn't provide.
>
> 4. **Compatibility testing**: Two test methods: SQL Logic Tests (verify SQL engine compatibility) and dbt integration tests (verify REST API compatibility). Uses the dbt GitLab project as a compatibility benchmark. Repository displays compatibility badges.
>
> 5. **Architecture differences**: Compare with Snowflake (managed analytics database built on FoundationDB and S3, see [whitepaper](https://www.cs.cmu.edu/~15721-f24/papers/Snowflake.pdf)). Embucket is open source using Apache DataFusion, Apache Iceberg, Apache Arrow, and Parquet. Key structural differences: metadata storage (external catalogs vs FoundationDB), data format (Iceberg/Parquet vs proprietary), query execution (single-node DataFusion with Arrow in-memory representation).
>
> 6. **Current limitations**:
>    - Architecture: single-node execution (one node's memory and CPU), no distributed parallelism, single writer (one instance writes to a table at a time)
>    - Data types: VARIANT stored as JSON-serialized TEXT (Parquet/Iceberg/Arrow don't support VARIANT natively), numeric type coercion differences, fixed nanosecond timestamp precision, no collation/charset support (UTF-8 only)
>    - Error handling: error messages don't match Snowflake format
>    - Backslash escaping differs — show three examples:
>      - Literal backslashes: Snowflake `'\\b'` → `\b`, Embucket `'\\\\b'` → `\b`
>      - Special characters: Snowflake `'\b'` → backspace, Embucket `'\b'` → backspace
>      - Single trailing backslash: Snowflake `'\\'` → `\`, Embucket `'\\'` → error "Unterminated string literal"
>
> 7. **VARIANT data type support**: Create table with VARIANT (`create table t2 (c1 variant) as values (parse_json('{"k1":1}'))`), read back, inspect Arrow type with `arrow_typeof(c1)` showing `Utf8`. Use Steps component.
>
> 8. **Numeric type handling**: DataFusion uses different numeric types than Snowflake for coercion. Uses Decimal128 for closest behavior to Snowflake NUMBER. Show comparison: Embucket `avg()` returns `Decimal128(7,5)`, Snowflake returns `NUMBER(20,6)[SB16]`. Include both SQL examples.
>
> 9. **Timestamp handling**: Arrow timestamp type: 64-bit integer, nanosecond precision, fixed. Snowflake uses variable precision. Timezone differences: Snowflake stores offset per value, Embucket stores offset per column.
>
> 10. **Error message format**: Arrow and DataFusion generate errors that don't match Snowflake format.
>
> 11. **Development roadmap**: VARIANT (native storage when dependencies support it), timestamp (per-value timezone), numeric (dynamic precision Decimal), error format (align with Snowflake).
>
> **Rules:**
> - No "Owner/Last reviewed"
> - This is the most comprehensive page — restore ALL content from previous versions
> - Import `{ Aside, Steps }` from `@astrojs/starlight/components`
> - Use Steps for VARIANT example
> - Vale Google style
> - Use vale comment delimiters `{/* vale Google.Headings = NO */}` before "SQL engine compatibility" heading if needed

- [ ] **Step 2: Review, fix, build, commit**

```bash
cd docs && pnpm build 2>&1 | head -50
git add docs/src/content/docs/reference/snowflake.mdx
git commit -m "docs: add Snowflake compatibility reference (restored full content)"
```

---

### Task 10: Write Troubleshooting page

**Files:**
- Create: `docs/src/content/docs/reference/troubleshooting.mdx`

**Source material:**
- Old file (from git): `guides/troubleshooting.mdx`

- [ ] **Step 1: Draft page with docs-writer agent**

Dispatch a docs-writer agent:

> Write a Starlight MDX page for `docs/src/content/docs/reference/troubleshooting.mdx`.
>
> **Frontmatter:** title "Troubleshooting", description "Common issues and solutions for Embucket deployment and client connections.", sidebar order 2.
>
> **Purpose:** Aggregated catch-all for when things break. Individual pages have inline troubleshooting too.
>
> **Content organized by symptom:**
>
> 1. **Local startup issues**: Use port 3000. Docker command: `docker run --name embucket --rm -p 3000:3000 embucket/embucket`. Browser surface at `http://127.0.0.1:3000/`.
>
> 2. **Snowflake CLI protocol or SSL errors**: Set `protocol = "http"` in connection config.
>
> 3. **Authentication failures**: Default demo credentials are embucket/embucket. Check AUTH_DEMO_USER and AUTH_DEMO_PASSWORD if overridden.
>
> 4. **Data doesn't appear**: Check METASTORE_CONFIG points to a YAML file with actual volumes, databases, schemas, or tables. For external catalog, see S3 table bucket setup in [AWS Lambda deployment](/deploy/aws-lambda/).
>
> 5. **Lambda deploy succeeds but runtime fails**: Check METASTORE_CONFIG points to a packaged file, function has correct memory and timeout, log group contains expected errors, ingress path matches client path (Function URL for Snowflake CLI, direct invoke for dbt).
>
> 6. **dbt can't connect**: Install dbt-embucket, set EMBUCKET_FUNCTION_ARN, verify AWS credentials can invoke Lambda, profile has type: embucket. Run `dbt debug`. Also check: AWS credential chain, ARN region, Lambda is deployed and responding, demo credentials match profile.
>
> 7. **dbt runs but model doesn't appear**: Check target database and schema in profile. Verify with: `snow sql -c lambda -q "select * from demo.public.hello_embucket"`.
>
> 8. **Snowplow package target.type check fails**: Some dbt packages check `target.type == 'snowflake'` and don't recognize embucket. Use the patch script in the Snowplow tutorial. This is a compatibility workaround.
>
> 9. **Sessions feel short-lived**: JWT token lifetime is 3 days, session inactivity expiry is 60 seconds. For long-running workflows, prefer automated client paths like dbt over interactive sessions.
>
> 10. **Legacy embucket-labs references**: Use `embucket/embucket` for the container image and `embucket-lambda` for the Lambda artifact. Treat embucket-labs references as outdated.
>
> **Rules:**
> - No "Owner/Last reviewed"
> - No support-matrix references (page deleted)
> - No "current docs treat it as" language
> - Import `{ Aside }` from `@astrojs/starlight/components`
> - Aside linking to /deploy/aws-lambda/ for Lambda-related issues
> - Vale Google style

- [ ] **Step 2: Review, fix, build, commit**

```bash
cd docs && pnpm build 2>&1 | head -50
git add docs/src/content/docs/reference/troubleshooting.mdx
git commit -m "docs: add Troubleshooting reference page"
```

---

### Task 11: Rewrite README.md

**Files:**
- Modify: `README.md` (repo root)

- [ ] **Step 1: Draft README with docs-writer agent**

Dispatch a docs-writer agent:

> Rewrite `/Users/ramp/vcs/embucket/README.md` as a clean product README for GitHub visitors.
>
> **Content:**
> 1. **Title and badge area** (keep any existing badges)
> 2. **One-paragraph description**: Embucket is a Snowflake-compatible query engine built on Apache DataFusion. It runs as an AWS Lambda function and uses S3 Tables (Apache Iceberg) for storage. Connect with any Snowflake-compatible tool or the dbt-embucket adapter.
> 3. **Key features** (bullet list): Snowflake v1 REST API, SQL dialect compatibility via DataFusion, AWS Lambda serverless deployment, S3 Tables (Iceberg/Parquet) storage, dbt adapter for analytics workflows, Snowflake CLI compatibility
> 4. **Quick start**: Docker command to try locally: `docker run --name embucket --rm -p 3000:3000 embucket/embucket`. Link to full Quick Start in docs.
> 5. **Deploy**: One-liner: `make -C crates/embucket-lambda deploy`. Link to full deployment guide in docs.
> 6. **Connect**: Two paths — Snowflake CLI (link to docs) and dbt adapter (link to docs).
> 7. **Documentation links**: Architecture, Snowflake compatibility, Troubleshooting, Contributing (link to CONTRIBUTING.md)
> 8. **License** section if one exists
>
> **Rules:**
> - No "The repo currently ships two runtime artifacts" framing
> - No internal path-chooser language or "choose your path" sections
> - No "current docs should" or "currently documented" language
> - No references to deleted pages (runtime-modes, support-matrix, self-hosted, etc.)
> - Write for someone seeing this repo for the first time on GitHub
> - Keep it concise — README should be scannable, not a full guide

- [ ] **Step 2: Review, fix, commit**

```bash
git add README.md
git commit -m "docs: rewrite README for product clarity"
```

---

### Task 12: Update docs/README.md

**Files:**
- Modify: `docs/README.md`

- [ ] **Step 1: Update docs README to reflect new structure**

Read current `docs/README.md` and update the project structure section to match the new directory layout. Remove any references to the smoke script or deleted directories. Keep the dev setup instructions (pnpm dev, pnpm build, etc.) but remove the `pnpm smoke` command.

- [ ] **Step 2: Commit**

```bash
git add docs/README.md
git commit -m "docs: update docs README for new structure"
```

---

### Task 13: Stranger-user review

**Files:** All new MDX files in docs/src/content/docs/

- [ ] **Step 1: Run stranger-user audit**

Spawn an agent with this prompt:

> You are a developer who has never seen Embucket before. You just found it and are reading the docs for the first time. Read every MDX file in `docs/src/content/docs/` and flag anything that would confuse you or seem unprofessional:
>
> 1. Internal metadata visible to users ("Owner:", "Last reviewed:")
> 2. Repo-centric language ("the repo ships", "current docs", "currently documented")
> 3. Internal commentary ("the confusion around", "this is not production-ready", "the current docs should not call it")
> 4. Undefined jargon or acronyms not explained on first use
> 5. Broken or circular links
> 6. References to deleted pages (runtime-modes, support-matrix, self-hosted, s3-tables, end-to-end-dbt, docs-maintenance, tracing)
> 7. Hedging language that makes you doubt the product works
> 8. Anything that makes you think "this is internal documentation, not for me"
>
> For each file, report: file path, line number, the problematic text, and why it's a problem. If a file is clean, say "PASS".

- [ ] **Step 2: Fix all findings**

Address every issue found by the stranger review.

- [ ] **Step 3: Re-run stranger review on fixed files**

Re-run only on files that had findings to confirm fixes.

---

### Task 14: Final build verification

- [ ] **Step 1: Full docs build**

```bash
cd docs && pnpm build
```

Expected: clean build, no errors, no broken links (Starlight links validator runs during build).

- [ ] **Step 2: Format check**

```bash
cd docs && pnpm prettier --check .
```

Expected: all files formatted correctly. If not, run `pnpm format` and commit.

- [ ] **Step 3: Astro check**

```bash
cd docs && pnpm check
```

Expected: no type errors.

- [ ] **Step 4: Vale lint**

```bash
cd /Users/ramp/vcs/embucket && vale docs/src/content/
```

Expected: no errors. Warnings acceptable but minimize them. Fix any errors.

- [ ] **Step 5: Final commit if needed**

```bash
git add -A docs/
git commit -m "docs: fix formatting and linting issues"
```

- [ ] **Step 6: Verify all links resolve**

Open the dev server and manually check that the sidebar renders correctly with all 5 sections and all pages load:

```bash
cd docs && pnpm dev
```

Check: Getting Started (1 page), Deploy (2 pages), Connect (2 pages), Tutorials (1 page), Reference (3 pages).
