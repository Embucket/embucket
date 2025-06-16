"""
Query execution tool for running SQL against Embucket.
"""

import os
import time
import json
from typing import Optional

try:
    from snowflake.connector import connect, SnowflakeConnection
    from snowflake.connector.errors import (
        ProgrammingError,
        DatabaseError,
        InterfaceError,
        OperationalError
    )
    SNOWFLAKE_AVAILABLE = True
except ImportError:
    SNOWFLAKE_AVAILABLE = False
    SnowflakeConnection = None
def execute_query_against_embucket(
    sql_query: str,
    host: str = "localhost",
    port: int = 3000,
    protocol: str = "http",
    account: str = "test",
    user: str = "test",
    password: str = "test",
    warehouse: str = "COMPUTE_WH",
    database: str = "embucket",
    schema: str = "public"
) -> str:
    """
    Execute a SQL query against the running Embucket server using Snowflake connector.

    Args:
        sql_query: The SQL query to execute
        host: Embucket server host
        port: Embucket server port
        protocol: Connection protocol (http/https)
        account: Snowflake account identifier (placeholder for Embucket)
        user: Username for connection
        password: Password for connection
        warehouse: Warehouse name
        database: Database name
        schema: Schema name

    Returns:
        JSON string containing execution result with keys:
        - success: bool
        - status_code: int (HTTP status code equivalent)
        - response: str (response body)
        - error_type: str (if error occurred)
        - error_message: str (detailed error message for display)
        - execution_time: float (in seconds)
    """
    # Check if verbose mode is enabled
    verbose_mode = os.getenv("FUZZING_VERBOSE", "false").lower() == "true"

    if verbose_mode:
        print(f"Executing query against Embucket: {sql_query}")

    if not SNOWFLAKE_AVAILABLE:
        return json.dumps({
            "success": False,
            "status_code": 500,
            "response": "snowflake-connector-python is not installed",
            "error_type": "dependency_error",
            "error_message": "snowflake-connector-python library is required but not installed",
            "execution_time": 0.0
        })

    start_time = time.time()

    # Configure connection parameters for Embucket
    connection_config = {
        'account': account,
        'user': user,
        'password': password,
        'warehouse': warehouse,
        'database': database,
        'schema': schema,
        'host': host,
        'port': port,
        'protocol': protocol,
        'timeout': 30  # 30 second timeout
    }

    connection = None
    try:
        # Establish connection to Embucket using Snowflake connector
        connection = connect(**connection_config)

        with connection.cursor() as cursor:
            # Execute the query
            cursor.execute(sql_query)

            execution_time = time.time() - start_time

            # Fetch results if available
            try:
                results = cursor.fetchall()
                column_names = [desc[0] for desc in cursor.description] if cursor.description else []

                # Format response similar to UI endpoint
                response_data = {
                    "columns": column_names,
                    "rows": results,
                    "rowCount": len(results) if results else 0
                }
                response_text = json.dumps(response_data)

                if verbose_mode:
                    print("✓ Query executed successfully")
                    # For successful queries, show a preview of the response
                    response_preview = response_text[:200] + "..." if len(response_text) > 200 else response_text
                    print(f"  Response preview: {response_preview}")

                result = {
                    "success": True,
                    "status_code": 200,
                    "response": response_text,
                    "error_type": None,
                    "error_message": None,
                    "execution_time": execution_time
                }
            except Exception as fetch_error:
                # Query executed but no results to fetch (e.g., DDL statements)
                execution_time = time.time() - start_time
                if verbose_mode:
                    print("✓ Query executed successfully (no results)")

                result = {
                    "success": True,
                    "status_code": 200,
                    "response": json.dumps({"message": "Query executed successfully", "rowCount": 0}),
                    "error_type": None,
                    "error_message": None,
                    "execution_time": execution_time
                }

    except ProgrammingError as e:
        execution_time = time.time() - start_time
        error_type = _classify_snowflake_error(e)
        if verbose_mode:
            print(f"✗ Query failed with programming error: {error_type}")
            print(f"  Full error message: {str(e)}")
        result = {
            "success": False,
            "status_code": 422,  # Unprocessable Entity for SQL errors
            "response": str(e),
            "error_type": error_type,
            "error_message": f"Programming error ({error_type}): {str(e)}",
            "execution_time": execution_time
        }

    except DatabaseError as e:
        execution_time = time.time() - start_time
        if verbose_mode:
            print("✗ Database error occurred")
            print(f"  Full error message: {str(e)}")
        result = {
            "success": False,
            "status_code": 500,
            "response": str(e),
            "error_type": "database_error",
            "error_message": f"Database error: {str(e)}",
            "execution_time": execution_time
        }

    except InterfaceError as e:
        execution_time = time.time() - start_time
        # Always print crash detection (this is important)
        print("✗ Connection error - server may have crashed")
        if verbose_mode:
            print(f"  Full error message: {str(e)}")
        result = {
            "success": False,
            "status_code": None,
            "response": str(e),
            "error_type": "crash",
            "error_message": f"Connection error (server may have crashed): {str(e)}",
            "execution_time": execution_time
        }

    except OperationalError as e:
        execution_time = time.time() - start_time
        error_msg = str(e)
        if "timeout" in error_msg.lower():
            # Always print timeout detection (this is important)
            print("✗ Query execution timed out")
            if verbose_mode:
                print(f"  Full error message: {error_msg}")
            result = {
                "success": False,
                "status_code": 408,
                "response": error_msg,
                "error_type": "timeout",
                "error_message": f"Query timeout: {error_msg}",
                "execution_time": execution_time
            }
        else:
            if verbose_mode:
                print("✗ Operational error occurred")
                print(f"  Full error message: {error_msg}")
            result = {
                "success": False,
                "status_code": 500,
                "response": error_msg,
                "error_type": "operational_error",
                "error_message": f"Operational error: {error_msg}",
                "execution_time": execution_time
            }

    except Exception as e:
        execution_time = time.time() - start_time
        if verbose_mode:
            print("✗ Unexpected error occurred")
            print(f"  Full error message: {str(e)}")
        result = {
            "success": False,
            "status_code": None,
            "response": f"Unexpected error: {str(e)}",
            "error_type": "unknown_error",
            "error_message": f"Unexpected error: {str(e)}",
            "execution_time": execution_time
        }

    finally:
        # Clean up connection
        if connection:
            try:
                connection.close()
            except Exception:
                pass  # Ignore cleanup errors

    return json.dumps(result)


def _classify_error_type(status_code: int, response_text: str) -> str:
    """
    Classify the error type based on HTTP status code and response content.

    Args:
        status_code: HTTP status code
        response_text: Response body text

    Returns:
        String describing the error type
    """
    if status_code == 422:
        # Check if it's a DataFusion schema error
        if "schema" in response_text.lower() or "datafusion" in response_text.lower():
            return "schema_error"
        else:
            return "validation_error"
    elif status_code == 500:
        return "server_error"
    elif status_code == 400:
        return "bad_request"
    elif status_code == 401:
        return "unauthorized"
    elif status_code == 403:
        return "forbidden"
    elif status_code == 404:
        return "not_found"
    elif status_code == 408:
        return "timeout"
    elif 400 <= status_code < 500:
        return "client_error"
    elif 500 <= status_code < 600:
        return "server_error"
    else:
        return "unknown_error"


def _classify_snowflake_error(error) -> str:
    """
    Classify the type of Snowflake connector error.

    Args:
        error: Snowflake connector exception

    Returns:
        String describing the error type
    """
    error_msg = str(error).lower()

    if "syntax" in error_msg or "parse" in error_msg:
        return "syntax_error"
    elif "schema" in error_msg or "table" in error_msg or "column" in error_msg:
        return "schema_error"
    elif "type" in error_msg or "datatype" in error_msg or "cast" in error_msg:
        return "type_error"
    elif "group by" in error_msg or "aggregate" in error_msg:
        return "aggregation_error"
    elif "join" in error_msg:
        return "join_error"
    elif "function" in error_msg:
        return "function_error"
    else:
        return "sql_error"
