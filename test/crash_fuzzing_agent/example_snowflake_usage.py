#!/usr/bin/env python3
"""
Example script demonstrating how to use the Snowflake query execution tool
in the fuzzing agent for comparative testing between Embucket and Snowflake.
"""

import os
import json
import asyncio
from dotenv import load_dotenv

from agents import Agent, Runner

# Load environment variables
load_dotenv()

# Import the tools
from tools.snowflake_query_execution_tool import (
    execute_query_against_snowflake,
    validate_snowflake_connection
)
from tools.query_execution_tool import execute_query_against_embucket


class ComparativeFuzzingAgent(Agent):
    """AI agent that can run queries against both Embucket and Snowflake for comparison."""

    def __init__(self):
        # Verify OpenAI API key is set
        openai_api_key = os.getenv("OPENAI_API_KEY")
        if not openai_api_key:
            raise ValueError(
                "OPENAI_API_KEY environment variable is required. "
                "Please set it in the .env file or as an environment variable."
            )

        super().__init__(
            name="ComparativeFuzzingAgent",
            instructions="""You are a comparative testing agent that can execute SQL queries against both Embucket and Snowflake.
            
            Your capabilities include:
            1. Executing queries against Embucket using execute_query_against_embucket
            2. Executing queries against Snowflake using execute_query_against_snowflake
            3. Validating Snowflake connections using validate_snowflake_connection
            4. Comparing results between the two systems
            5. Identifying differences in behavior, performance, or error handling
            
            When comparing systems:
            - Execute the same query on both systems
            - Compare execution times, result sets, and error behaviors
            - Report any significant differences or compatibility issues
            - Focus on SQL compatibility and correctness
            """,
            tools=[
                execute_query_against_embucket,
                execute_query_against_snowflake,
                validate_snowflake_connection
            ]
        )


async def example_comparative_testing():
    """Example of using the agent for comparative testing."""
    
    print("=== Comparative Testing Example ===")
    
    # Check if both Embucket and Snowflake configurations are available
    embucket_host = os.getenv("EMBUCKET_HOST", "localhost")
    embucket_port = int(os.getenv("EMBUCKET_PORT", "3000"))
    
    snowflake_required = ['SNOWFLAKE_ACCOUNT', 'SNOWFLAKE_USER', 'SNOWFLAKE_PASSWORD']
    snowflake_missing = [var for var in snowflake_required if not os.getenv(var)]
    
    if snowflake_missing:
        print(f"⚠️  Missing Snowflake configuration: {', '.join(snowflake_missing)}")
        print("Comparative testing requires both Embucket and Snowflake to be configured.")
        return
    
    try:
        agent = ComparativeFuzzingAgent()
        
        # Example instruction for comparative testing
        instruction = f"""
        Please perform a comparative test between Embucket and Snowflake using the following approach:
        
        1. First, validate the Snowflake connection to ensure it's working
        
        2. Then execute this simple test query on both systems:
           SELECT 1 as test_id, 'Hello World' as message, CURRENT_TIMESTAMP() as created_at
        
        3. Compare the results and report:
           - Whether both queries succeeded
           - Execution times for each system
           - Any differences in result format or data types
           - Any errors or compatibility issues
        
        4. If the simple query works, try a more complex query:
           WITH test_data AS (
               SELECT 1 as id, 'A' as category
               UNION ALL 
               SELECT 2 as id, 'B' as category
           )
           SELECT 
               id,
               category,
               COUNT(*) OVER() as total_count,
               ROW_NUMBER() OVER(ORDER BY id) as row_num
           FROM test_data
           ORDER BY id
        
        5. Provide a summary of compatibility and performance differences between the systems.
        
        Use the Embucket server at {embucket_host}:{embucket_port} and the configured Snowflake instance.
        """
        
        print("Starting comparative testing...")
        result = await Runner.run(agent, instruction, max_turns=10)
        print(f"\nComparative testing result:\n{result.final_output}")
        
    except ValueError as e:
        print(f"Configuration Error: {e}")
    except Exception as e:
        print(f"Error during comparative testing: {e}")


async def example_snowflake_only_testing():
    """Example of using just the Snowflake tools."""
    
    print("\n=== Snowflake-Only Testing Example ===")
    
    snowflake_required = ['SNOWFLAKE_ACCOUNT', 'SNOWFLAKE_USER', 'SNOWFLAKE_PASSWORD']
    snowflake_missing = [var for var in snowflake_required if not os.getenv(var)]
    
    if snowflake_missing:
        print(f"⚠️  Missing Snowflake configuration: {', '.join(snowflake_missing)}")
        print("Please configure Snowflake credentials in your .env file.")
        return
    
    try:
        agent = ComparativeFuzzingAgent()
        
        # Example instruction for Snowflake-only testing
        instruction = """
        Please test the Snowflake connection and execute some sample queries:
        
        1. First, validate the Snowflake connection
        
        2. Execute a simple query to test basic functionality:
           SELECT 'Snowflake Test' as message, CURRENT_TIMESTAMP() as timestamp
        
        3. Execute a query that tests some Snowflake-specific functions:
           SELECT 
               CURRENT_DATABASE() as current_db,
               CURRENT_SCHEMA() as current_schema,
               CURRENT_WAREHOUSE() as current_warehouse,
               CURRENT_USER() as current_user
        
        4. Test error handling with an invalid query:
           SELECT * FROM non_existent_table_xyz
        
        5. Provide a summary of the Snowflake connection status and query execution results.
        """
        
        print("Starting Snowflake testing...")
        result = await Runner.run(agent, instruction, max_turns=8)
        print(f"\nSnowflake testing result:\n{result.final_output}")
        
    except ValueError as e:
        print(f"Configuration Error: {e}")
    except Exception as e:
        print(f"Error during Snowflake testing: {e}")


async def main():
    """Main function to run examples."""
    
    print("Snowflake Query Execution Tool Examples")
    print("=" * 50)
    
    # Check what configurations are available
    embucket_available = bool(os.getenv("EMBUCKET_HOST"))
    snowflake_available = all(os.getenv(var) for var in ['SNOWFLAKE_ACCOUNT', 'SNOWFLAKE_USER', 'SNOWFLAKE_PASSWORD'])
    
    print(f"Embucket configuration available: {embucket_available}")
    print(f"Snowflake configuration available: {snowflake_available}")
    print()
    
    if snowflake_available and embucket_available:
        print("Both systems configured - running comparative testing...")
        await example_comparative_testing()
    elif snowflake_available:
        print("Only Snowflake configured - running Snowflake-only testing...")
        await example_snowflake_only_testing()
    else:
        print("⚠️  No database systems properly configured.")
        print("\nTo use this example, please configure at least Snowflake in your .env file:")
        print("SNOWFLAKE_ACCOUNT=your_account")
        print("SNOWFLAKE_USER=your_username")
        print("SNOWFLAKE_PASSWORD=your_password")
        print("SNOWFLAKE_WAREHOUSE=COMPUTE_WH")
        print("SNOWFLAKE_DATABASE=your_database")
        print("SNOWFLAKE_SCHEMA=public")
        print("\nOptionally, also configure Embucket for comparative testing:")
        print("EMBUCKET_HOST=localhost")
        print("EMBUCKET_PORT=3000")


if __name__ == "__main__":
    asyncio.run(main())
