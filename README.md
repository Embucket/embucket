# Embucket

**Run Snowflake SQL dialect on your data lake in 30 seconds. Zero dependencies.**

[![License](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)

## Quick start

Start Embucket and run your first query in 30 seconds:

```bash
docker run --name embucket --rm -p 3000:3000 embucket/embucket
```

Run the Snowflake CLI against the local endpoint:

```bash
pip install snowflake-cli
snow sql -c local -a local -u embucket -p embucket -q "select 1;"
```

**Done.** You just ran Snowflake SQL dialect against the local Embucket instance with zero configuration.

### Create external volumes via config

**Important**: External volumes must be created via YAML configuration at startup. 

Define volumes and databases by pointing `embucketd` at a YAML config file.

**Using Docker:**

```bash
docker run --name embucket --rm -p 3000:3000 \
  -v $PWD/config:/app/config \
  embucket/embucket \
  ./embucketd --metastore-config config/metastore.yaml
```

**Using cargo:**

```bash
cargo run -p embucketd -- \
  --no-bootstrap \
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

  # S3 volume - connects to standard S3 bucket
  # - ident: volume
  #   type: s3
  #   bucket: my-data-bucket
  #   credentials:
  #     credential_type: access_key
  #     aws-access-key-id: YOUR_ACCESS_KEY
  #     aws-secret-access-key: YOUR_SECRET_KEY
```

Update the credentials and ARN/bucket details with your own values for real deployments.

## What just happened?

Embucket provides a **single binary** that gives you a **wire-compatible Snowflake replacement**:

- **Snowflake SQL dialect and API**: Use your existing queries, dbt projects, and BI tools
- **Apache Iceberg storage**: Your data stays in open formats on object storage  
- **Zero dependencies**: No databases, no clusters, no configuration files
- **Query-per-node**: Each instance handles complete queries independently

Perfect for teams who want Snowflake's simplicity with bring-your-own-cloud control. Built on proven open source:

- [Apache DataFusion](https://datafusion.apache.org/) for SQL execution
- [Apache Iceberg](https://iceberg.apache.org/) for ACID table metadata  

## Why Embucket?

**Escape the dilemma**: choose between cloud provider lakehouses (Redshift, BigQuery) or operational complexity (do-it-yourself lakehouse).

- **Radical simplicity** - Single binary deployment  
- **Snowflake SQL dialect compatibility** - Works with your existing tools  
- **Open data** - Apache Iceberg format, no lock-in  
- **Horizontal scaling** - Add nodes for more throughput  
- **Zero operations** - No external dependencies to manage

## Build from source

```bash
git clone https://github.com/Embucket/embucket.git
cd embucket && cargo build
./target/debug/embucketd
```

## Contributing  

Contributions welcome. To get involved:  

1. **Fork** the repository on GitHub  
2. **Create** a new branch for your feature or bug fix  
3. **Submit** a pull request with a detailed description  

For more details, see [CONTRIBUTING.md](CONTRIBUTING.md).  

## License  

This project uses the **Apache 2.0 License**. See [LICENSE](LICENSE) for details.  
