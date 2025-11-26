# Embucket

**Run Snowflake SQL dialect on your data lake in 30 seconds. Zero dependencies.**

[![License](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)

## Introduction

Embucket is a single binary lakehouse that provides a wire-compatible Snowflake replacement and works with Apache Iceberg open table format. Perfect for simple deployments. Built on proven open source:

- [Apache DataFusion](https://datafusion.apache.org/) for SQL execution
- [Apache Iceberg](https://iceberg.apache.org/) for ACID table metadata

Embucket features:

- **Snowflake SQL dialect and API**: Use your existing queries, dbt projects, and BI tools
- **Apache Iceberg storage**: Your data stays in Apache Iceberg format on object storage, no lock-in
- **Radical simplicity** - Single binary deployment
- **Query-per-node**: Each instance handles complete queries independently
- **Horizontal scaling** - Add nodes for more throughput  

## Quick start

Start Embucket and run your first query in 30 seconds:

```bash
mkdir -p config
cat > config/metastore.yaml <<'EOF'
volumes:
  - ident: embucket
    type: memory
    database: embucket
EOF

docker run --name embucket --rm -p 3000:3000 \
  -v $PWD/config:/app/config \
  embucket/embucket \
  ./embucketd --metastore-config config/metastore.yaml
```

Install and configure the Snowflake CLI against the local endpoint:

```bash
$ pip install snowflake-cli # Install snowflake cli if you haven't already
# Configure the Snowflake CLI to connect to the local endpoint
# Find the config file path
$ snow --info | grep "config_file_path" -A 1
$ CONFIG=<your_config_file_path>
# Update the config file with the following:
echo '
[connections.local]
host = "localhost"
region = "us-east-2"
port = 3000
protocol = "http"
database = "embucket"
schema = "public"
warehouse = "em.wh"
password = "embucket"
account = "acc.local"
user = "embucket"
' >> $CONFIG
$ snow connection test -c local
# Expected output
+-----------------------------+
| key             | value     |
|-----------------+-----------|
| Connection name | local     |
| Status          | OK        |
| Host            | localhost |
| Account         | acc       |
| User            | embucket  |
| Role            | not set   |
| Database        | embucket  |
| Warehouse       | em.wh     |
+-----------------------------+
```

Run a query:

```bash
$ snow sql -c local -q "select dateadd(day, -1, current_timestamp()) as yesterday;"
# Expected output
+----------------------------------+
| yesterday                        |
|----------------------------------|
| 2025-01-02 03:04:05.040000+00:00 |
+----------------------------------+
```

**Done.** You just ran Snowflake SQL dialect against the local Embucket instance.

## External catalogs

Embucket can connect to external catalogs via YAML configuration. At the moment the only supported catalog is AWS S3 table buckets.

**Important**: External catalogs must be defined via YAML configuration file. 

Define catalogs by pointing `embucketd` at a YAML config file.

**Using Docker:**

```bash
docker run --name embucket --rm -p 3000:3000 \
  -v $PWD/config:/app/config \
  embucket/embucket \
  ./embucketd --metastore-config config/metastore.yaml
```

**Using binary:**

```bash
./embucketd \
  --metastore-config config/metastore.yaml
```

**Sample configuration** (`config/metastore.yaml`):

```yaml
volumes:
  # S3 Tables volume - connects to AWS S3 Table Bucket
  - ident: demo
    type: s3-tables
    database: demo
    credentials:
      credential_type: access_key
      aws-access-key-id: YOUR_ACCESS_KEY
      aws-secret-access-key: YOUR_SECRET_KEY
    arn: arn:aws:s3tables:us-east-2:123456789012:bucket/my-table-bucket
```

Query the catalog. Assume the catalog is name `demo` (`database: demo` in the config above). 

```bash
$ snow sql -c local -q "show schemas in demo; show tables in demo.tpch_10;"
show schemas in demo;
+----------------------------------------------------------------------------+
| created_on | name                     | kind | database_name | schema_name |
|------------+--------------------------+------+---------------+-------------|
| None       | public                   | None | demo          | None        |
| None       | public_derived           | None | demo          | None        |
| None       | public_scratch           | None | demo          | None        |
| None       | public_snowplow_manifest | None | demo          | None        |
| None       | sturukin                 | None | demo          | None        |
| None       | tpcds_10                 | None | demo          | None        |
| None       | tpcds_100                | None | demo          | None        |
| None       | tpch_10                  | None | demo          | None        |
| None       | tpch_100                 | None | demo          | None        |
| None       | information_schema       | None | demo          | None        |
+----------------------------------------------------------------------------+

show tables in demo.tpch_10;
+-------------------------------------------------------------+
| created_on | name     | kind  | database_name | schema_name |
|------------+----------+-------+---------------+-------------|
| None       | orders   | TABLE | demo          | tpch_10     |
| None       | nation   | TABLE | demo          | tpch_10     |
| None       | customer | TABLE | demo          | tpch_10     |
| None       | part     | TABLE | demo          | tpch_10     |
| None       | lineitem | TABLE | demo          | tpch_10     |
| None       | partsupp | TABLE | demo          | tpch_10     |
| None       | region   | TABLE | demo          | tpch_10     |
| None       | supplier | TABLE | demo          | tpch_10     |
| None       | t1       | TABLE | demo          | tpch_10     |
| None       | t3       | TABLE | demo          | tpch_10     |
+-------------------------------------------------------------+
```

Update the credentials and ARN/bucket details with your own values for real deployments.

## External Iceberg tables

Embucket can also be configured to work with external Iceberg tables. Define tables in the metastore config file.

**Important**: External tables must be in the same bucket as the `volume` defined in the metastore config file.

```yaml
volumes:
  - ident: lakehouse
    type: s3
    region: us-east-2
    bucket: YOUR_BUCKET_NAME
    credentials:
      credential_type: access_key
      aws-access-key-id: YOUR_ACCESS_KEY
      aws-secret-access-key: YOUR_SECRET_KEY
databases:
  - ident: demo
    volume: lakehouse
schemas:
  - database: demo
    schema: tpch_10
tables:
- database: demo
  schema: tpch_10
  table: customer
  metadata_location: s3://YOUR_BUCKET_NAME/tpch_10/customer/metadata/00001-eea1cccb-38a4-4fe2-8c95-c01dae9d0c60.metadata.json
- database: demo
  schema: tpch_10
  table: lineitem
  metadata_location: s3://YOUR_BUCKET_NAME/tpch_10/lineitem/metadata/00001-d777220e-d508-4033-a229-8c4c8d8fe514.metadata.json
```

## Build from source

```bash
git clone https://github.com/Embucket/embucket.git
cd embucket && cargo build
./target/debug/embucketd
```

## Deployment

Embucket supports all different deployments modes as it is single binary. 

### Lambda

One of the interesting deployment modes is AWS Lambda, Google Cloud Functions, and Azure Functions. At the moment, Embucket can be natively built for AWS Lambda. 

To build for Lambda, make sure you have `cargo-lambda` installed (`cargo install cargo-lambda`) and run the following command:

```bash
$ cargo lambda build --release -p embucket-lambda --arm64
```

To deploy to AWS Lambda, you will need to create a new function in the AWS Console and upload the binary. By default deploy command will use the `bootstrap` binary, use default IAM role `AWSLambdaBasicExecutionRole`, use default memory size `1024`,  default timeout `30` and include `config` directory.

**Important**: Make sure configuration file exists in the `config` directory (i.e. `config/metastore.yaml`).

Use the `cargo lambda deploy` command to deploy the binary to AWS Lambda.

```bash
$ cargo lambda deploy --binary-name bootstrap embucket-lambda
```

It is also recommended to enable function URL to access the API. Read [cargo-lambda](https://www.cargo-lambda.info/commands/deploy.html#function-urls) documentation and configure the IAM role to allow function URL access.

```bash
$ cargo lambda deploy --binary-name bootstrap embucket-lambda --enable-function-url
```

The expected output is:

```bash
✅ function deployed successfully 🎉
🛠️  binary last compiled 1 minute ago
🔍 arn: arn:aws:lambda:us-east-2:123456789012:function:embucket-lambda:1
🎭 version: 1
🔗 url: https://7mh4xw9n2pqjvf5kzrbt8ycusg6dla3e.lambda-url.us-east-2.on.aws/
```

The 32 alphanumeric characters after `url.` are the function URL. You can use it to access the API.

Create a new connection profile for Snowflake CLI:

```bash
$ snow connection add
Enter connection name: lambda
Enter account: acc.lambda
Enter user: embucket
Enter password:
Enter role: em.role
Enter warehouse: em.wh
Enter database: demo
Enter schema: public
Enter host: https://7mh4xw9n2pqjvf5kzrbt8ycusg6dla3e.lambda-url.us-east-2.on.aws
Enter port:
Enter region: us-east-2
Enter authenticator:
Enter workload identity provider:
Enter private key file:
Enter token file path:
```

Now use it to run queries:

```bash
snow sql -c lambda -q "select dateadd(day, -1, current_timestamp()) as yesterday;"
select dateadd(day, -1, current_timestamp()) as yesterday;
+----------------------------------+
| yesterday                        |
|----------------------------------|
| 2025-01-02 03:04:05.040000+00:00 |
+----------------------------------+
```


## Contributing  

Contributions welcome. To get involved:  

1. **Fork** the repository on GitHub  
2. **Create** a new branch for your feature or bug fix  
3. **Submit** a pull request with a detailed description  

For more details, see [CONTRIBUTING.md](CONTRIBUTING.md).  

## License  

This project uses the **Apache 2.0 License**. See [LICENSE](LICENSE) for details.  
