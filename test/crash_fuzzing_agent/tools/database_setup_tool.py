"""
Database setup tool for creating test tables before fuzzing.
"""

import requests
import json
from typing import List, Dict, Any
# Table schemas that match the random query generator expectations
TEST_TABLES = [
    {
        "name": "users",
        "columns": [
            ("id", "INTEGER"),
            ("user_id", "INTEGER"), 
            ("name", "VARCHAR(255)"),
            ("email", "VARCHAR(255)"),
            ("age", "INTEGER"),
            ("created_at", "TIMESTAMP"),
            ("updated_at", "TIMESTAMP")
        ]
    },
    {
        "name": "orders", 
        "columns": [
            ("id", "INTEGER"),
            ("order_id", "INTEGER"),
            ("user_id", "INTEGER"),
            ("amount", "DECIMAL(10,2)"),
            ("total", "DECIMAL(10,2)"),
            ("status", "VARCHAR(50)"),
            ("order_date", "TIMESTAMP"),
            ("created_at", "TIMESTAMP")
        ]
    },
    {
        "name": "products",
        "columns": [
            ("id", "INTEGER"),
            ("product_id", "INTEGER"),
            ("name", "VARCHAR(255)"),
            ("description", "TEXT"),
            ("price", "DECIMAL(10,2)"),
            ("quantity", "INTEGER"),
            ("created_at", "TIMESTAMP")
        ]
    },
    {
        "name": "customers",
        "columns": [
            ("id", "INTEGER"),
            ("customer_id", "INTEGER"),
            ("name", "VARCHAR(255)"),
            ("email", "VARCHAR(255)"),
            ("age", "INTEGER"),
            ("created_at", "TIMESTAMP")
        ]
    },
    {
        "name": "sales",
        "columns": [
            ("id", "INTEGER"),
            ("customer_id", "INTEGER"),
            ("product_id", "INTEGER"),
            ("amount", "DECIMAL(10,2)"),
            ("quantity", "INTEGER"),
            ("created_at", "TIMESTAMP")
        ]
    },
    {
        "name": "inventory",
        "columns": [
            ("id", "INTEGER"),
            ("product_id", "INTEGER"),
            ("quantity", "INTEGER"),
            ("count", "INTEGER"),
            ("updated_at", "TIMESTAMP")
        ]
    },
    {
        "name": "employees",
        "columns": [
            ("id", "INTEGER"),
            ("user_id", "INTEGER"),
            ("name", "VARCHAR(255)"),
            ("email", "VARCHAR(255)"),
            ("age", "INTEGER"),
            ("created_at", "TIMESTAMP")
        ]
    },
    {
        "name": "departments",
        "columns": [
            ("id", "INTEGER"),
            ("name", "VARCHAR(255)"),
            ("description", "TEXT"),
            ("created_at", "TIMESTAMP")
        ]
    },
    {
        "name": "categories",
        "columns": [
            ("id", "INTEGER"),
            ("name", "VARCHAR(255)"),
            ("description", "TEXT"),
            ("created_at", "TIMESTAMP")
        ]
    },
    {
        "name": "suppliers",
        "columns": [
            ("id", "INTEGER"),
            ("name", "VARCHAR(255)"),
            ("email", "VARCHAR(255)"),
            ("created_at", "TIMESTAMP")
        ]
    },
    {
        "name": "transactions",
        "columns": [
            ("id", "INTEGER"),
            ("user_id", "INTEGER"),
            ("amount", "DECIMAL(10,2)"),
            ("type", "VARCHAR(50)"),
            ("status", "VARCHAR(50)"),
            ("created_at", "TIMESTAMP")
        ]
    },
    {
        "name": "accounts",
        "columns": [
            ("id", "INTEGER"),
            ("user_id", "INTEGER"),
            ("name", "VARCHAR(255)"),
            ("type", "VARCHAR(50)"),
            ("created_at", "TIMESTAMP")
        ]
    },
    {
        "name": "payments",
        "columns": [
            ("id", "INTEGER"),
            ("order_id", "INTEGER"),
            ("amount", "DECIMAL(10,2)"),
            ("status", "VARCHAR(50)"),
            ("created_at", "TIMESTAMP")
        ]
    },
    {
        "name": "reviews",
        "columns": [
            ("id", "INTEGER"),
            ("product_id", "INTEGER"),
            ("user_id", "INTEGER"),
            ("title", "VARCHAR(255)"),
            ("description", "TEXT"),
            ("created_at", "TIMESTAMP")
        ]
    },
    {
        "name": "sessions",
        "columns": [
            ("id", "INTEGER"),
            ("user_id", "INTEGER"),
            ("status", "VARCHAR(50)"),
            ("created_at", "TIMESTAMP"),
            ("updated_at", "TIMESTAMP")
        ]
    },
    {
        "name": "logs",
        "columns": [
            ("id", "INTEGER"),
            ("user_id", "INTEGER"),
            ("type", "VARCHAR(50)"),
            ("description", "TEXT"),
            ("created_at", "TIMESTAMP")
        ]
    }
]


def _execute_sql_query(embucket_url: str, sql_query: str) -> Dict[str, Any]:
    """Execute a SQL query against Embucket using the correct UI endpoint."""
    try:
        response = requests.post(
            f"{embucket_url}/ui/queries",
            json={"query": sql_query},
            headers={"Content-Type": "application/json"},
            timeout=30
        )

        result = {
            "status_code": response.status_code,
            "success": response.status_code == 200,
            "response_text": response.text
        }

        if response.status_code == 200:
            try:
                result["data"] = response.json()
            except json.JSONDecodeError:
                result["data"] = response.text

        return result

    except requests.exceptions.RequestException as e:
        return {
            "status_code": 0,
            "success": False,
            "error": str(e),
            "response_text": f"Request failed: {e}"
        }


def _setup_embucket_infrastructure(embucket_url: str, database: str = "embucket", schema: str = "public") -> Dict[str, Any]:
    """Set up the basic Embucket infrastructure (volume, database, schema)."""
    headers = {'Content-Type': 'application/json'}

    try:
        # Step 1: Create volume
        print(f"Creating volume 'local'...")
        volume_payload = {
            "type": "memory",
            "ident": "local"
        }
        volume_response = requests.post(
            f"{embucket_url}/v1/metastore/volumes",
            headers=headers,
            json=volume_payload,
            timeout=30
        )
        print(f"Volume creation response: {volume_response.status_code}")

        # Step 2: Create database
        print(f"Creating database '{database}'...")
        database_payload = {
            "ident": database,
            "volume": "local"
        }
        database_response = requests.post(
            f"{embucket_url}/v1/metastore/databases",
            headers=headers,
            json=database_payload,
            timeout=30
        )
        print(f"Database creation response: {database_response.status_code}")

        # Step 3: Create schema using SQL query
        print(f"Creating schema '{schema}'...")
        schema_query = f"CREATE SCHEMA IF NOT EXISTS {database}.{schema}"
        schema_result = _execute_sql_query(embucket_url, schema_query)
        print(f"Schema creation result: {schema_result['success']}")

        return {
            "success": True,
            "volume_status": volume_response.status_code,
            "database_status": database_response.status_code,
            "schema_result": schema_result
        }

    except requests.exceptions.RequestException as e:
        return {
            "success": False,
            "error": str(e)
        }


def setup_test_database(embucket_url: str = "http://localhost:3000",
                       include_sample_data: bool = True,
                       database: str = "embucket",
                       db_schema: str = "public") -> str:
    """
    Set up test database with tables needed for fuzzing queries.

    Creates all the tables that the random query generator expects to exist,
    with appropriate column types and optional sample data.

    Args:
        embucket_url: URL of the Embucket server
        include_sample_data: Whether to insert sample data into tables
        database: Database name to use (default: embucket)
        db_schema: Schema name to use (default: public)

    Returns:
        Status message indicating success or failure
    """
    print(f"Setting up test database at {embucket_url}")

    # Step 1: Set up Embucket infrastructure (volume, database, schema)
    print("Setting up Embucket infrastructure...")
    infra_result = _setup_embucket_infrastructure(embucket_url, database, db_schema)

    if not infra_result["success"]:
        error_msg = f"Failed to set up Embucket infrastructure: {infra_result.get('error', 'Unknown error')}"
        print(error_msg)
        return error_msg

    print("✓ Embucket infrastructure setup completed")

    # Step 2: Create tables
    created_tables = []
    failed_tables = []

    for table_info in TEST_TABLES:
        table_name = table_info["name"]
        columns = table_info["columns"]

        # Build CREATE TABLE statement with fully qualified name
        column_defs = []
        for col_name, col_type in columns:
            column_defs.append(f"{col_name} {col_type}")

        create_sql = f"CREATE TABLE IF NOT EXISTS {database}.{db_schema}.{table_name} ({', '.join(column_defs)})"

        print(f"Creating table: {database}.{db_schema}.{table_name}")
        result = _execute_sql_query(embucket_url, create_sql)

        if result["success"]:
            created_tables.append(table_name)
            print(f"✓ Created table: {table_name}")
        else:
            # Check if the error is about table already existing
            response_text = result.get("response_text", "")
            if "already exists" in response_text:
                created_tables.append(table_name)
                print(f"✓ Table {table_name} already exists (skipped)")
            else:
                failed_tables.append((table_name, response_text))
                print(f"✗ Failed to create table {table_name}: {response_text}")

    # Step 3: Insert sample data if requested
    if include_sample_data and created_tables:
        print("Inserting sample data...")
        _insert_sample_data(embucket_url, created_tables, database, db_schema)

    # Prepare summary
    summary_lines = [
        f"Database setup completed:",
        f"  Infrastructure: {database}.{db_schema}",
        f"  Tables available: {len(created_tables)} tables",
        f"  Failed to create: {len(failed_tables)} tables"
    ]

    if created_tables:
        summary_lines.append(f"  Available tables: {', '.join(created_tables)}")

    if failed_tables:
        summary_lines.append("  Failed tables:")
        for table_name, error in failed_tables:
            summary_lines.append(f"    - {table_name}: {error}")

    summary = "\n".join(summary_lines)
    print(summary)
    return summary


def _insert_sample_data(embucket_url: str, created_tables: List[str], database: str, db_schema: str) -> None:
    """Insert minimal sample data into created tables."""
    sample_data_queries = []

    if "users" in created_tables:
        sample_data_queries.append(
            f"INSERT INTO {database}.{db_schema}.users (id, user_id, name, email, age) VALUES "
            "(1, 1, 'John Doe', 'john@example.com', 30), "
            "(2, 2, 'Jane Smith', 'jane@example.com', 25)"
        )

    if "products" in created_tables:
        sample_data_queries.append(
            f"INSERT INTO {database}.{db_schema}.products (id, product_id, name, description, price, quantity) VALUES "
            "(1, 1, 'Widget A', 'A useful widget', 19.99, 100), "
            "(2, 2, 'Widget B', 'Another widget', 29.99, 50)"
        )

    if "orders" in created_tables:
        sample_data_queries.append(
            f"INSERT INTO {database}.{db_schema}.orders (id, order_id, user_id, amount, total, status) VALUES "
            "(1, 1, 1, 19.99, 19.99, 'completed'), "
            "(2, 2, 2, 29.99, 29.99, 'pending')"
        )

    if "customers" in created_tables:
        sample_data_queries.append(
            f"INSERT INTO {database}.{db_schema}.customers (id, customer_id, name, email, age) VALUES "
            "(1, 1, 'Alice Johnson', 'alice@example.com', 28), "
            "(2, 2, 'Bob Wilson', 'bob@example.com', 35)"
        )

    if "departments" in created_tables:
        sample_data_queries.append(
            f"INSERT INTO {database}.{db_schema}.departments (id, name, description) VALUES "
            "(1, 'Engineering', 'Software development team'), "
            "(2, 'Sales', 'Customer acquisition team')"
        )

    if "employees" in created_tables:
        sample_data_queries.append(
            f"INSERT INTO {database}.{db_schema}.employees (id, user_id, name, email, age) VALUES "
            "(1, 1, 'John Engineer', 'john.eng@company.com', 32), "
            "(2, 2, 'Sarah Sales', 'sarah.sales@company.com', 29)"
        )

    # Execute sample data queries
    for query in sample_data_queries:
        result = _execute_sql_query(embucket_url, query)
        if result["success"]:
            print(f"✓ Inserted sample data")
        else:
            print(f"✗ Failed to insert sample data: {result.get('response_text', 'Unknown error')}")
