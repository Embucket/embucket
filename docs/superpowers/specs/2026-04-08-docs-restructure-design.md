# Docs restructure design

## Context

Embucket's public documentation has structural and content problems:

- Internal metadata ("Owner", "Last reviewed") visible to users
- Internal development section in public docs
- Confusing guide organization (troubleshooting in guides, self-hosted binary as a "guide")
- Repo-centric language ("the repo ships", "current docs treat it as")
- Missing documentation for statestore feature flag and Lambda sizing
- Snowflake compatibility page was stripped of useful content
- No clear product framing: the product is **embucket-lambda** on AWS with S3 Tables as primary storage
- README needs cleanup

## Product framing

- **Product:** embucket-lambda — a Snowflake-compatible DataFusion-based query engine running in AWS Lambda, using S3 Tables (Iceberg) as primary storage
- **Local Docker path:** exists only for trying Embucket locally before deploying
- **Two client paths:** Snowflake-compatible REST API (via Function URL, API Gateway, or custom gateway) and dbt adapter (via Lambda invoke). Both well-tested. Different security models, not a quality hierarchy.

## Target audience

Both developers evaluating Embucket and data engineers adopting it. Docs serve the evaluation-to-production journey.

## New sidebar structure

```
Getting Started/
  Quick Start

Deploy/
  AWS Lambda
  Configuration

Connect/
  Snowflake CLI
  dbt adapter

Tutorials/
  Snowplow web analytics

Reference/
  Architecture
  Snowflake compatibility
  Troubleshooting
```

## Implementation constraint: stranger review

Before any page ships, spawn an agent assuming a stranger-user role. That agent audits every page for internal-facing language, leaked metadata, confusing jargon, repo-centric framing, and anything a new user would find confusing or unprofessional. Fix all findings before completing.

## Pages: detailed spec

### Getting Started / Quick Start

**File:** `getting-started/quick-start.mdx`

**Purpose:** Try Embucket locally in 2 minutes.

**Content:**

1. Start Embucket via Docker (`docker run`)
2. Install and configure Snowflake CLI
3. Run a first query
4. Call to action: "Ready to deploy? See AWS Lambda deployment"

**Changes from current:**

- Remove "Owner/Last reviewed" blockquote
- Remove hedging ("test and evaluation path, not recommended production...") — say "Try Embucket locally before deploying to AWS"
- Drop web UI mention (Step 4: Inspect the HTTP surface)
- Trim "Next steps" to 3 links max: deploy, Snowflake CLI, dbt adapter
- Remove self-hosted binary link

### Deploy / AWS Lambda

**File:** `deploy/aws-lambda.mdx`

**Purpose:** Deploy embucket-lambda to AWS with S3 Tables storage.

**Content — single page covering the full deployment path:**

1. Prerequisites — Rust toolchain, cargo-lambda, AWS credentials
2. Create an S3 table bucket — absorbed from current `s3-tables.mdx` (bucket creation, ARN capture)
3. Configure the metastore — volume config YAML with S3 Tables credentials and ARN
4. Build and deploy — `make -C crates/embucket-lambda deploy`, env file options, role ARN
5. Verify the deployment — `make verify`, CloudWatch logs
6. IAM and access — deployer permissions, execution role, invoke permissions for dbt clients
7. Lambda sizing — memory, timeout, disk recommendations and how to tune them (NEW content)
8. Statestore (optional) — `-F state-store` feature flag, DynamoDB table setup, env vars: `STATESTORE_TABLE_NAME`, `STATESTORE_DYNAMODB_ENDPOINT`, `AWS_DDB_ACCESS_KEY_ID`, `AWS_DDB_SECRET_ACCESS_KEY` (NEW content, sourced from `crates/state-store/README.md`)
9. Production ingress — private API Gateway pattern (CloudFormation template), why to avoid public Function URL for production
10. Rollback and redeploy guidance
11. Cleanup
12. Inline troubleshooting — Lambda-specific failure modes

**Absorbs content from:** `aws-lambda.mdx`, `s3-tables.mdx`, relevant parts of `runtime-modes.mdx`

### Deploy / Configuration

**File:** `deploy/configuration.mdx`

**Purpose:** Reference for all configuration options.

**Content:**

1. Configuration precedence (flags > env vars > .env)
2. Core runtime settings table (host, port, auth, timeouts, tracing, idle timeout)
3. Metastore YAML format — volumes, databases, schemas, tables sections with examples
4. S3 Tables volume config example
5. S3 volume config example (for external Iceberg tables — this IS supported per codebase)
6. Statestore env vars reference
7. Lambda-specific deploy-time variables (from Makefile: `FUNCTION_NAME`, `ENV_FILE`, `AWS_LAMBDA_ROLE_ARN`, `WITH_OTEL_CONFIG`, `FEATURES`, `LAYERS`)
8. Memory and timeout tuning vars (`MEM_POOL_TYPE`, `MEM_POOL_SIZE_MB`, `DISK_POOL_SIZE_MB`, AWS SDK timeouts, object store timeouts)
9. Authentication defaults

**Changes from current:**

- Remove "Owner/Last reviewed" blockquote
- Remove "backed by the repository" and "visible in cli.rs" language
- Add statestore env vars (missing today)
- Add Lambda Makefile variables (currently only in aws-lambda guide)
- Clean up language throughout

### Connect / Snowflake CLI

**File:** `connect/snowflake-cli.mdx`

**Purpose:** Connect the Snowflake CLI to Embucket.

**Content:**

1. Brief intro — standard Snowflake command-line tool works with Embucket's REST API
2. Prerequisites
3. Configure connection — TOML blocks for local and Lambda Function URL
4. Test connection, run a query
5. Inline troubleshooting

**Changes from current:**

- Remove "Owner/Last reviewed"
- Aside linking to Deploy page wherever Function URL is mentioned
- Remove "current local test and evaluation client flow" framing
- Clean up language

### Connect / dbt adapter

**File:** `connect/dbt.mdx`

**Purpose:** Connect dbt to Embucket via the dbt-embucket adapter.

**Content:**

1. One-line explanation of dbt for users who don't know it
2. Explain the adapter connects to Lambda via AWS invoke (no public endpoint)
3. Prerequisites — deployed Lambda, ARN, AWS credentials, Python + dbt
4. Install adapter
5. Create minimal project (dbt_project.yml, model)
6. Configure profile (profiles.yml with env vars)
7. Run `dbt debug` and `dbt run`
8. Verify result (query from Snowflake CLI)
9. Profile field reference table
10. Current caveats stated as facts (Lambda transport only, no Python models, etc.)
11. Inline troubleshooting
12. Next step: Snowplow tutorial for a full analytics example

**Absorbs content from:** `dbt.mdx` and useful parts of `end-to-end-dbt.mdx` (the deploy-first step, export/verify flow)

**Changes from current:**

- Add "what is dbt" intro (one line)
- Aside linking to Deploy page wherever deployment mentioned
- Remove hedging language ("recommended client path in the current docs")
- Absorb e2e-dbt content so this page is self-contained

### Tutorials / Snowplow web analytics

**File:** `tutorials/snowplow.mdx`

**Purpose:** Flagship tutorial — build a real analytics pipeline with Embucket.

**Content:** Stays mostly as-is:

1. Clone embucket-snowplow repo
2. Set deploy-time values
3. Deploy Lambda stack via CloudFormation
4. Install deps, configure dbt profile
5. Install dbt packages, patch if needed
6. Load example data
7. Run pipeline (`dbt seed`, `dbt run`)
8. Verify with `dbt show`
9. Cleanup

**Changes:**

- Remove "Owner/Last reviewed"
- Aside linking to Deploy page for deployment references
- Clean up language (no "current recommended runtime and client path")
- Remove back-references to deleted pages (runtime-modes, support-matrix)

### Reference / Architecture

**File:** `reference/architecture.mdx`

**Purpose:** How Embucket works internally.

**Content:** Keep five-layer breakdown:

1. Runtime — embucket-lambda is the production artifact, local embucketd for evaluation
2. Metadata — YAML metastore config, S3 Tables external catalog, external Iceberg definitions
3. Storage — Iceberg metadata, Parquet data, S3
4. Query — DataFusion, single-node per request
5. Auth/session — JWT tokens, session model, demo credentials

**Changes:**

- Remove "Owner/Last reviewed"
- Remove "The repo currently ships two runtime artifacts" — describe the system, not the repo
- Clean up all repo-centric language
- Aside to Deploy page where appropriate

### Reference / Snowflake compatibility

**File:** `reference/snowflake.mdx`

**Purpose:** What works, what differs from Snowflake, known limitations.

**Content — restore from previous versions and keep current:**

1. Intro — Snowflake v1 REST API, DataFusion SQL engine, compatible clients
2. API compatibility — tested with snowflake-connector-python, dbt, snowflake-cli, Superset
3. SQL engine compatibility — DataFusion-based, aims for 100% dialect compat, currently a subset
4. Compatibility testing — SQL Logic Tests, dbt integration tests, dbt GitLab benchmark, badges
5. Architecture differences — comparison with Snowflake (FoundationDB vs open stack), Snowflake whitepaper link
6. Current limitations:
   - Architecture: single-node, no distributed parallelism, single writer
   - Data types: VARIANT as JSON TEXT, numeric coercion, timestamp precision, no collation/charset
   - Error handling: different format
   - Backslash escaping: detailed examples (literal backslashes, special chars, trailing backslash)
7. VARIANT data type support — create, read, inspect Arrow type
8. Numeric type handling — Decimal128 vs Snowflake NUMBER with SQL examples
9. Timestamp handling — nanosecond precision, timezone per-column vs per-value
10. Error message format
11. Development roadmap — VARIANT, timestamps, numeric, error format

### Reference / Troubleshooting

**File:** `reference/troubleshooting.mdx`

**Purpose:** Aggregated common issues when things break.

**Content — organized by symptom:**

1. Local: startup issues, browser/port, protocol errors
2. Authentication failures
3. Data not appearing (metastore config)
4. Lambda: deploy succeeds but runtime fails, timeouts, no traces
5. Snowflake CLI: connection, SSL, protocol
6. dbt: can't connect, model doesn't appear, Snowplow package checks
7. Sessions feel short-lived

**Changes:**

- Remove "Owner/Last reviewed"
- Remove support-matrix references
- Clean up language
- Individual pages still have inline troubleshooting; this is the catch-all

### README.md (repo root)

**Purpose:** First thing people see on GitHub. Should clearly explain what Embucket is and how to get started.

**Content:**

1. One-paragraph product description: Snowflake-compatible query engine on AWS Lambda with S3 Tables
2. Key features (bullet list)
3. Quick start: link to docs Quick Start
4. Deploy: link to docs AWS Lambda page
5. Connect: links to Snowflake CLI and dbt pages
6. Links to architecture, compatibility, contributing

**Changes:**

- Remove "The repo currently ships two runtime artifacts" framing
- Remove internal path-chooser language
- Write for a GitHub visitor, not a repo maintainer

## Pages to delete

- `essentials/runtime-modes.mdx` — content absorbed into Deploy intro and Quick Start next-steps
- `essentials/support-matrix.mdx` — verified paths info folded into relevant pages
- `guides/self-hosted.mdx` — source build is a contributor concern, not user-facing
- `guides/s3-tables.mdx` — absorbed into Lambda deployment page
- `guides/end-to-end-dbt.mdx` — absorbed into dbt adapter page
- `development/docs-maintenance.mdx` — internal, not public
- `development/tracing.mdx` — internal, not public

## Files to update

- `astro.config.mjs` — new sidebar structure (Getting Started, Deploy, Connect, Tutorials, Reference), remove Development section
- `docs/README.md` — update to reflect new structure

## Files to delete (non-docs)

- `scripts/validate-docs-smoke.mjs` — brittle string-assertion checks, enforces patterns we're removing. Starlight links validator and Vale already cover quality.
- Remove the `smoke` script from `package.json`

## Directory structure change

```
docs/src/content/docs/
  getting-started/
    quick-start.mdx
  deploy/
    aws-lambda.mdx
    configuration.mdx
  connect/
    snowflake-cli.mdx
    dbt.mdx
  tutorials/
    snowplow.mdx
  reference/
    architecture.mdx
    snowflake.mdx
    troubleshooting.mdx
```

## Implementation approach

1. **Stranger-user review:** Before any page ships, spawn an agent in the stranger-user role that audits every page for internal language, leaked metadata, confusing jargon, and "WTF" moments. Fix all findings.
2. **Writer/reviewer pairs:** Use docs-writer agent to draft each page, then docs-reviewer agent to audit it. Work in pairs per page.
3. **CI must be green:** Vale linter is strict about warnings. Run `pnpm build` after changes. All checks must pass.
4. **Lambda sizing source:** The NEW Lambda sizing content should be sourced from `crates/embucket-lambda/Cargo.toml` defaults (memory 3008 MB, timeout 30s) and the Makefile variables. Note that 3008 MB is the default Lambda max — increasing beyond that requires an AWS support ticket (up to 10 GB). Include guidance on when and how to request higher limits.

## Cross-cutting rules

1. No "Owner/Last reviewed" blockquotes in rendered content
2. No repo-centric language ("the repo ships", "current docs", "currently documented")
3. No internal commentary or hedging ("this is not production-ready", "the confusion around...")
4. Wherever Function URL or deployment is mentioned outside Deploy section, add an Aside linking to the Deploy page
5. Write like product documentation, not an internal audit
6. Stranger-user review agent audits every page before shipping
