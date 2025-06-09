#!/usr/bin/env python3
"""
Random Crash Fuzzing Agent for Embucket

This agent uses sqlsmith to generate SQL queries and tests them against Embucket,
saving any queries that cause crashes or unexpected responses as .slt files.
"""

import os
import time
import json
from dotenv import load_dotenv

from agents import Agent, Runner

# Load environment variables from .env file
load_dotenv()

# Import the comprehensive fuzzing tool
from tools.comprehensive_fuzzing_tool import run_comprehensive_fuzzing_session

# Import Snowflake query execution tools
from tools.snowflake_query_execution_tool import (
    execute_query_against_snowflake,
    validate_snowflake_connection
)


class EmbucketFuzzingAgent(Agent):
    """AI agent that performs random crash fuzzing on Embucket using random query generator."""

    def __init__(self):
        # Load configuration from environment variables
        self.embucket_host = os.getenv("EMBUCKET_HOST", "localhost")
        self.embucket_port = int(os.getenv("EMBUCKET_PORT", "3000"))
        self.embucket_url = f"http://{self.embucket_host}:{self.embucket_port}"
        self.slt_output_dir = os.getenv("SLT_OUTPUT_DIR", "test/sql/fuzz_regressions")

        # Verify OpenAI API key is set
        openai_api_key = os.getenv("OPENAI_API_KEY")
        if not openai_api_key:
            raise ValueError(
                "OPENAI_API_KEY environment variable is required. "
                "Please set it in the .env file or as an environment variable."
            )

        super().__init__(
            name="EmbucketFuzzingAgent",
            instructions="You are a fuzzing agent that runs comprehensive SQL fuzzing sessions against Embucket to find crashes and bugs, then analyzes the results. You also have access to Snowflake query execution tools for comparative testing.",
            tools=[
                run_comprehensive_fuzzing_session,
                execute_query_against_snowflake,
                validate_snowflake_connection
            ]
        )

        # Ensure output directory exists
        os.makedirs(self.slt_output_dir, exist_ok=True)


    
    async def run_fuzzing_session(self, num_queries: int = 10, complexity: str = "medium"):
        """
        Run a complete fuzzing session using the agent framework.

        Args:
            num_queries: Number of queries to generate and test
            complexity: Complexity level for generated queries
        """
        print(f"Starting fuzzing session with {num_queries} queries...")

        # Create the simplified fuzzing instruction
        instruction = f"""
        You are a fuzzing agent for Embucket. Please perform the following:

        1. Run a comprehensive fuzzing session using run_comprehensive_fuzzing_session with:
           - num_queries: {num_queries}
           - complexity: "{complexity}"
           - host: "{self.embucket_host}"
           - port: {self.embucket_port}
           - output_dir: "{self.slt_output_dir}"

        2. After the fuzzing session completes, analyze the results and provide:
           - A summary of what was tested
           - Classification of any bugs found (crashes vs expected errors)
           - Recommendations for next steps
           - Assessment of Embucket's stability based on the results

        The comprehensive fuzzing tool will handle all the server lifecycle, query generation,
        execution, and logging. Your job is to analyze the results and provide insights.
        """

        # Run the agent with much fewer turns needed (just 1-2 turns)
        result = await Runner.run(self, instruction, max_turns=5)
        print(f"Fuzzing session result: {result.final_output}")
        return result


async def main():
    # Example usage with configuration from environment variables
    try:
        agent = EmbucketFuzzingAgent()

        # Get configuration from environment variables with defaults
        num_queries = int(os.getenv("DEFAULT_NUM_QUERIES", "5"))
        complexity = os.getenv("DEFAULT_COMPLEXITY", "medium")
        safe_query_probability = float(os.getenv("DEFAULT_SAFE_QUERY_PROBABILITY", "0.3"))

        print(f"Configuration:")
        print(f"  Embucket Host: {agent.embucket_host}")
        print(f"  Embucket Port: {agent.embucket_port}")
        print(f"  SLT Output Dir: {agent.slt_output_dir}")
        print(f"  Number of Queries: {num_queries}")
        print(f"  Complexity: {complexity}")
        print(f"  Safe Query Probability: {safe_query_probability}")
        print()

        await agent.run_fuzzing_session(num_queries=num_queries, complexity=complexity)

    except ValueError as e:
        print(f"Configuration Error: {e}")
        print("\nPlease check your .env file and ensure OPENAI_API_KEY is set.")
        return 1
    except Exception as e:
        print(f"Error: {e}")
        return 1

    return 0

if __name__ == "__main__":
    import asyncio
    asyncio.run(main())
