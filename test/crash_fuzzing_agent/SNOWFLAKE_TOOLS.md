# Snowflake Query Execution Tools

This document describes the Snowflake query execution tools that have been added to the fuzzing agent, enabling comparative testing between Embucket and Snowflake.

## Overview

The Snowflake tools allow the fuzzing agent to:
- Execute SQL queries against Snowflake
- Validate Snowflake connections
- Capture query performance metrics
- Compare results between Embucket and Snowflake
- Test SQL compatibility across systems

## Tools Available

### 1. `execute_query_against_snowflake`

Executes a SQL query against Snowflake and returns detailed results.

**Parameters:**
- `sql_query` (required): The SQL query to execute
- `account` (optional): Snowflake account identifier (defaults to `SNOWFLAKE_ACCOUNT` env var)
- `user` (optional): Snowflake username (defaults to `SNOWFLAKE_USER` env var)
- `password` (optional): Snowflake password (defaults to `SNOWFLAKE_PASSWORD` env var)
- `warehouse` (optional): Snowflake warehouse (defaults to `SNOWFLAKE_WAREHOUSE` env var)
- `database` (optional): Snowflake database (defaults to `SNOWFLAKE_DATABASE` env var)
- `schema` (optional): Snowflake schema (defaults to `SNOWFLAKE_SCHEMA` env var)
- `capture_query_id` (optional): Whether to capture query ID for performance tracking (default: true)

**Returns:**
JSON string with execution results including:
- `success`: Boolean indicating if query succeeded
- `query_id`: Snowflake query ID (for performance tracking)
- `rows_affected`: Number of rows affected/returned
- `execution_time`: Total execution time in seconds
- `compilation_time`: Query compilation time in milliseconds (if available)
- `total_elapsed_time`: Total elapsed time in milliseconds (if available)
- `result_data`: Query results for SELECT queries (limited to first 100 rows)
- `error_type`: Error classification if query failed
- `error_message`: Detailed error message if query failed

### 2. `validate_snowflake_connection`

Validates the Snowflake connection configuration.

**Parameters:**
- `account` (optional): Snowflake account identifier (defaults to env var)
- `user` (optional): Snowflake username (defaults to env var)
- `password` (optional): Snowflake password (defaults to env var)
- `warehouse` (optional): Snowflake warehouse (defaults to env var)

**Returns:**
JSON string with validation results:
- `success`: Boolean indicating if connection succeeded
- `message`: Descriptive message about the connection status
- `connection_time`: Time taken to establish connection in seconds

## Configuration

### Environment Variables

Add the following environment variables to your `.env` file:

```bash
# Snowflake Configuration
SNOWFLAKE_ACCOUNT=your_account_identifier
SNOWFLAKE_USER=your_username
SNOWFLAKE_PASSWORD=your_password
SNOWFLAKE_WAREHOUSE=COMPUTE_WH
SNOWFLAKE_DATABASE=your_database
SNOWFLAKE_SCHEMA=public
```

### Required Dependencies

The tools require the `snowflake-connector-python` package, which has been added to `requirements.txt`:

```bash
pip install snowflake-connector-python==3.15.0
```

## Usage Examples

### Basic Query Execution

```python
from tools.snowflake_query_execution_tool import execute_query_against_snowflake

# Execute a simple query
result = execute_query_against_snowflake("SELECT 1 as test_column")
result_data = json.loads(result)

if result_data['success']:
    print(f"Query succeeded! Query ID: {result_data['query_id']}")
    print(f"Execution time: {result_data['execution_time']:.3f}s")
else:
    print(f"Query failed: {result_data['error_message']}")
```

### Connection Validation

```python
from tools.snowflake_query_execution_tool import validate_snowflake_connection

# Validate connection
result = validate_snowflake_connection()
result_data = json.loads(result)

if result_data['success']:
    print("Snowflake connection is valid!")
else:
    print(f"Connection failed: {result_data['message']}")
```

### Comparative Testing with Agent

```python
import asyncio
from agents import Agent, Runner
from tools.snowflake_query_execution_tool import execute_query_against_snowflake
from tools.query_execution_tool import execute_query_against_embucket

class ComparativeAgent(Agent):
    def __init__(self):
        super().__init__(
            name="ComparativeAgent",
            instructions="Compare query execution between Embucket and Snowflake",
            tools=[execute_query_against_embucket, execute_query_against_snowflake]
        )

async def compare_systems():
    agent = ComparativeAgent()
    instruction = """
    Execute this query on both Embucket and Snowflake and compare results:
    SELECT 1 as id, 'test' as name, CURRENT_TIMESTAMP() as created_at
    """
    result = await Runner.run(agent, instruction, max_turns=5)
    print(result.final_output)

asyncio.run(compare_systems())
```

## Testing

### Test Scripts

1. **`test_snowflake_tool.py`**: Basic functionality tests for the Snowflake tools
2. **`example_snowflake_usage.py`**: Complete example showing comparative testing

Run the tests:

```bash
# Test basic Snowflake functionality
python test_snowflake_tool.py

# Run comparative testing example
python example_snowflake_usage.py
```

### Test Coverage

The test scripts cover:
- Connection validation
- Simple query execution
- Complex query execution with CTEs and window functions
- Error handling with invalid queries
- Performance metric capture
- Comparative testing between systems

## Error Handling

The tools provide comprehensive error handling for:

- **Configuration errors**: Missing required environment variables
- **Connection errors**: Network issues, invalid credentials
- **SQL errors**: Syntax errors, invalid table/column references
- **Database errors**: Snowflake-specific database errors
- **Timeout errors**: Query or connection timeouts
- **Dependency errors**: Missing snowflake-connector-python package

Error types are classified for easy identification and handling in fuzzing scenarios.

## Performance Metrics

When `capture_query_id=True` (default), the tools capture:
- Query ID for tracking in Snowflake's query history
- Compilation time (query parsing and optimization)
- Execution time (actual query execution)
- Total elapsed time (end-to-end time)

These metrics enable performance comparison between Embucket and Snowflake.

## Integration with Fuzzing Agent

The Snowflake tools are automatically available in the main `EmbucketFuzzingAgent` class, enabling:

1. **Comparative fuzzing**: Run the same generated queries on both systems
2. **Compatibility testing**: Identify SQL syntax differences
3. **Performance benchmarking**: Compare execution times
4. **Error analysis**: Compare error handling between systems
5. **Result validation**: Verify query results match between systems

## Troubleshooting

### Common Issues

1. **Missing dependencies**: Install `snowflake-connector-python`
2. **Configuration errors**: Verify all required environment variables are set
3. **Connection timeouts**: Check network connectivity to Snowflake
4. **Authentication failures**: Verify credentials and account identifier
5. **Warehouse issues**: Ensure the specified warehouse exists and is accessible

### Debug Tips

- Use `validate_snowflake_connection()` first to verify basic connectivity
- Check Snowflake query history for failed queries using the captured query IDs
- Enable detailed logging in the Snowflake connector for connection debugging
- Verify warehouse is running and not suspended

## Future Enhancements

Potential improvements for the Snowflake tools:
- Support for multiple Snowflake accounts/environments
- Batch query execution for performance testing
- Advanced performance profiling and analysis
- Integration with Snowflake's query optimization features
- Support for Snowflake-specific SQL features and functions
