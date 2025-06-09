"""
Snowflake query execution tool for running SQL against Snowflake.
"""

import time
import json
import os
from typing import Optional, Dict, Any
from dotenv import load_dotenv
from agents import function_tool

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


@function_tool
def execute_query_against_snowflake(
    sql_query: str,
    account: Optional[str] = None,
    user: Optional[str] = None,
    password: Optional[str] = None,
    warehouse: Optional[str] = None,
    database: Optional[str] = None,
    schema: Optional[str] = None,
    capture_query_id: bool = True
) -> str:
    """
    Execute a SQL query against Snowflake.

    Args:
        sql_query: The SQL query to execute
        account: Snowflake account identifier (defaults to env SNOWFLAKE_ACCOUNT)
        user: Snowflake username (defaults to env SNOWFLAKE_USER)
        password: Snowflake password (defaults to env SNOWFLAKE_PASSWORD)
        warehouse: Snowflake warehouse (defaults to env SNOWFLAKE_WAREHOUSE)
        database: Snowflake database (defaults to env SNOWFLAKE_DATABASE)
        schema: Snowflake schema (defaults to env SNOWFLAKE_SCHEMA)
        capture_query_id: Whether to capture the query ID for performance tracking

    Returns:
        JSON string containing execution result with keys:
        - success: bool
        - query_id: str (Snowflake query ID, if captured)
        - rows_affected: int (number of rows affected/returned)
        - execution_time: float (in seconds)
        - compilation_time: float (in milliseconds, if available)
        - total_elapsed_time: float (in milliseconds, if available)
        - result_data: list (query results for SELECT queries, limited to first 100 rows)
        - error_type: str (if error occurred)
        - error_message: str (detailed error message for display)
    """
    if not SNOWFLAKE_AVAILABLE:
        return json.dumps({
            "success": False,
            "error_type": "dependency_error",
            "error_message": "snowflake-connector-python is not installed. Please install it to use this tool.",
            "execution_time": 0.0
        })

    # Load environment variables
    load_dotenv()
    
    # Use provided parameters or fall back to environment variables
    config = {
        'account': account or os.getenv('SNOWFLAKE_ACCOUNT'),
        'user': user or os.getenv('SNOWFLAKE_USER'),
        'password': password or os.getenv('SNOWFLAKE_PASSWORD'),
        'warehouse': warehouse or os.getenv('SNOWFLAKE_WAREHOUSE'),
        'database': database or os.getenv('SNOWFLAKE_DATABASE'),
        'schema': schema or os.getenv('SNOWFLAKE_SCHEMA')
    }

    # Validate required configuration
    required_fields = ['account', 'user', 'password']
    missing_fields = [field for field in required_fields if not config[field]]
    if missing_fields:
        return json.dumps({
            "success": False,
            "error_type": "configuration_error",
            "error_message": f"Missing required Snowflake configuration: {', '.join(missing_fields)}. "
                           f"Please set environment variables: {', '.join(f'SNOWFLAKE_{field.upper()}' for field in missing_fields)}",
            "execution_time": 0.0
        })

    print(f"Executing query against Snowflake: {sql_query}")
    start_time = time.time()

    connection = None
    try:
        # Establish connection to Snowflake
        connection = connect(
            account=config['account'],
            user=config['user'],
            password=config['password'],
            warehouse=config['warehouse'],
            database=config['database'],
            schema=config['schema'],
            timeout=30  # 30 second connection timeout
        )

        with connection.cursor() as cursor:
            # Execute the query
            cursor.execute(sql_query)
            
            execution_time = time.time() - start_time
            
            # Capture query ID if requested
            query_id = None
            compilation_time = None
            total_elapsed_time = None
            
            if capture_query_id:
                try:
                    cursor.execute("SELECT LAST_QUERY_ID()")
                    query_id = cursor.fetchone()[0]
                    
                    # Get query performance metrics
                    if query_id:
                        perf_query = f"""
                        SELECT 
                            COMPILATION_TIME,
                            EXECUTION_TIME,
                            TOTAL_ELAPSED_TIME
                        FROM TABLE(SNOWFLAKE.INFORMATION_SCHEMA.QUERY_HISTORY())
                        WHERE QUERY_ID = '{query_id}'
                        """
                        cursor.execute(perf_query)
                        perf_result = cursor.fetchone()
                        if perf_result:
                            compilation_time = float(perf_result[0]) if perf_result[0] is not None else None
                            total_elapsed_time = float(perf_result[2]) if perf_result[2] is not None else None
                            
                except Exception as e:
                    print(f"Warning: Could not capture query performance metrics: {e}")

            # Get result data and row count
            result_data = []
            rows_affected = cursor.rowcount if cursor.rowcount is not None else 0
            
            # For SELECT queries, fetch results (limit to first 100 rows for safety)
            if sql_query.strip().upper().startswith('SELECT'):
                try:
                    # Reset cursor to beginning and fetch results
                    cursor.execute(sql_query)
                    rows = cursor.fetchmany(100)  # Limit to 100 rows
                    
                    # Convert rows to list of dictionaries for JSON serialization
                    if rows and cursor.description:
                        column_names = [desc[0] for desc in cursor.description]
                        result_data = [dict(zip(column_names, row)) for row in rows]
                        rows_affected = len(rows)
                        
                except Exception as e:
                    print(f"Warning: Could not fetch result data: {e}")

            print("✓ Query executed successfully against Snowflake")
            if query_id:
                print(f"  Query ID: {query_id}")
            print(f"  Rows affected/returned: {rows_affected}")
            if compilation_time is not None:
                print(f"  Compilation time: {compilation_time:.2f}ms")
            if total_elapsed_time is not None:
                print(f"  Total elapsed time: {total_elapsed_time:.2f}ms")

            result = {
                "success": True,
                "query_id": query_id,
                "rows_affected": rows_affected,
                "execution_time": execution_time,
                "compilation_time": compilation_time,
                "total_elapsed_time": total_elapsed_time,
                "result_data": result_data,
                "error_type": None,
                "error_message": None
            }

    except ProgrammingError as e:
        execution_time = time.time() - start_time
        error_type = "sql_error"
        error_message = str(e)
        print(f"✗ SQL error in Snowflake query: {error_message}")
        
        result = {
            "success": False,
            "query_id": None,
            "rows_affected": 0,
            "execution_time": execution_time,
            "compilation_time": None,
            "total_elapsed_time": None,
            "result_data": [],
            "error_type": error_type,
            "error_message": f"SQL Error: {error_message}"
        }

    except DatabaseError as e:
        execution_time = time.time() - start_time
        error_type = "database_error"
        error_message = str(e)
        print(f"✗ Database error in Snowflake: {error_message}")
        
        result = {
            "success": False,
            "query_id": None,
            "rows_affected": 0,
            "execution_time": execution_time,
            "compilation_time": None,
            "total_elapsed_time": None,
            "result_data": [],
            "error_type": error_type,
            "error_message": f"Database Error: {error_message}"
        }

    except (InterfaceError, OperationalError) as e:
        execution_time = time.time() - start_time
        error_type = "connection_error"
        error_message = str(e)
        print(f"✗ Connection error to Snowflake: {error_message}")
        
        result = {
            "success": False,
            "query_id": None,
            "rows_affected": 0,
            "execution_time": execution_time,
            "compilation_time": None,
            "total_elapsed_time": None,
            "result_data": [],
            "error_type": error_type,
            "error_message": f"Connection Error: {error_message}"
        }

    except Exception as e:
        execution_time = time.time() - start_time
        error_type = "unknown_error"
        error_message = str(e)
        print(f"✗ Unexpected error in Snowflake query execution: {error_message}")
        
        result = {
            "success": False,
            "query_id": None,
            "rows_affected": 0,
            "execution_time": execution_time,
            "compilation_time": None,
            "total_elapsed_time": None,
            "result_data": [],
            "error_type": error_type,
            "error_message": f"Unexpected Error: {error_message}"
        }

    finally:
        # Clean up connection
        if connection:
            try:
                connection.close()
            except Exception as e:
                print(f"Warning: Error closing Snowflake connection: {e}")

    return json.dumps(result)


@function_tool
def validate_snowflake_connection(
    account: Optional[str] = None,
    user: Optional[str] = None,
    password: Optional[str] = None,
    warehouse: Optional[str] = None
) -> str:
    """
    Validate the Snowflake connection configuration.

    Args:
        account: Snowflake account identifier (defaults to env SNOWFLAKE_ACCOUNT)
        user: Snowflake username (defaults to env SNOWFLAKE_USER)
        password: Snowflake password (defaults to env SNOWFLAKE_PASSWORD)
        warehouse: Snowflake warehouse (defaults to env SNOWFLAKE_WAREHOUSE)

    Returns:
        JSON string with validation result:
        - success: bool
        - message: str
        - connection_time: float (in seconds)
    """
    if not SNOWFLAKE_AVAILABLE:
        return json.dumps({
            "success": False,
            "message": "snowflake-connector-python is not installed",
            "connection_time": 0.0
        })

    # Load environment variables
    load_dotenv()
    
    # Use provided parameters or fall back to environment variables
    config = {
        'account': account or os.getenv('SNOWFLAKE_ACCOUNT'),
        'user': user or os.getenv('SNOWFLAKE_USER'),
        'password': password or os.getenv('SNOWFLAKE_PASSWORD'),
        'warehouse': warehouse or os.getenv('SNOWFLAKE_WAREHOUSE')
    }

    # Validate required configuration
    required_fields = ['account', 'user', 'password']
    missing_fields = [field for field in required_fields if not config[field]]
    if missing_fields:
        return json.dumps({
            "success": False,
            "message": f"Missing required configuration: {', '.join(missing_fields)}",
            "connection_time": 0.0
        })

    start_time = time.time()
    connection = None
    
    try:
        print("Validating Snowflake connection...")
        connection = connect(
            account=config['account'],
            user=config['user'],
            password=config['password'],
            warehouse=config['warehouse'],
            timeout=10  # 10 second timeout for validation
        )
        
        # Test with a simple query
        with connection.cursor() as cursor:
            cursor.execute("SELECT 1")
            cursor.fetchone()
        
        connection_time = time.time() - start_time
        print(f"✓ Snowflake connection validated successfully in {connection_time:.2f}s")
        
        return json.dumps({
            "success": True,
            "message": f"Connection successful in {connection_time:.2f}s",
            "connection_time": connection_time
        })

    except Exception as e:
        connection_time = time.time() - start_time
        error_message = str(e)
        print(f"✗ Snowflake connection validation failed: {error_message}")
        
        return json.dumps({
            "success": False,
            "message": f"Connection failed: {error_message}",
            "connection_time": connection_time
        })

    finally:
        if connection:
            try:
                connection.close()
            except Exception as e:
                print(f"Warning: Error closing validation connection: {e}")
