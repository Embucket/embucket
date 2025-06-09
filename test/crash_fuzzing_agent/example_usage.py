#!/usr/bin/env python3
"""
Example usage of the refactored fuzzing agent.

This example shows how to use the new streamlined architecture where:
1. A single comprehensive tool handles the entire fuzzing workflow
2. The AI agent analyzes the results and provides insights
3. Only 1-2 AI turns are needed (no more max_turns issues)
"""

import asyncio
import os
from dotenv import load_dotenv

# Load environment variables
load_dotenv()

from fuzzing_agent import EmbucketFuzzingAgent


async def main():
    """Example of running the refactored fuzzing agent."""
    
    # Check if OpenAI API key is set
    if not os.getenv("OPENAI_API_KEY"):
        print("❌ OPENAI_API_KEY not set. Please set it in your .env file.")
        print("This example requires an actual OpenAI API key to run the agent.")
        print("You can still test the tool architecture with test_comprehensive_tool.py")
        return
    
    try:
        print("🚀 Creating Embucket Fuzzing Agent...")
        agent = EmbucketFuzzingAgent()
        
        print("📋 Agent Configuration:")
        print(f"  Host: {agent.embucket_host}")
        print(f"  Port: {agent.embucket_port}")
        print(f"  Output Directory: {agent.slt_output_dir}")
        print(f"  Tools: {len(agent.tools)} (comprehensive fuzzing tool)")
        print()
        
        # Get configuration from environment variables
        num_queries = int(os.getenv("DEFAULT_NUM_QUERIES", "5"))
        complexity = os.getenv("DEFAULT_COMPLEXITY", "medium")
        
        print(f"🎯 Starting fuzzing session:")
        print(f"  Queries: {num_queries}")
        print(f"  Complexity: {complexity}")
        print()
        
        print("🔄 Running comprehensive fuzzing session...")
        print("This will:")
        print("  1. Build and start Embucket server")
        print("  2. Generate and execute SQL queries")
        print("  3. Monitor for crashes and collect logs")
        print("  4. Save SLT files for any bugs found")
        print("  5. Stop the server")
        print("  6. Analyze results with AI")
        print()
        
        # Run the fuzzing session
        result = await agent.run_fuzzing_session(
            num_queries=num_queries,
            complexity=complexity
        )
        
        print("✅ Fuzzing session completed!")
        print("📊 Check the agent's analysis above for insights and recommendations.")
        
    except Exception as e:
        print(f"❌ Error running fuzzing agent: {e}")
        return 1
    
    return 0


if __name__ == "__main__":
    asyncio.run(main())
