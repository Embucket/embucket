#!/usr/bin/env python3
"""
Test script for the comprehensive fuzzing tool.

This script tests the comprehensive fuzzing tool without requiring an OpenAI API key.
It verifies that the tool can be imported and has the correct structure.
"""

import sys
import os

# Add the current directory to the path so we can import tools
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

def test_comprehensive_tool_import():
    """Test that the comprehensive fuzzing tool can be imported."""
    try:
        from tools.comprehensive_fuzzing_tool import run_comprehensive_fuzzing_session
        print("✅ Successfully imported run_comprehensive_fuzzing_session")
        return True
    except ImportError as e:
        print(f"❌ Failed to import comprehensive fuzzing tool: {e}")
        return False

def test_tool_signature():
    """Test that the tool has the expected function signature."""
    try:
        from tools.comprehensive_fuzzing_tool import run_comprehensive_fuzzing_session

        # The function is wrapped by @function_tool, so check the tool's schema
        if hasattr(run_comprehensive_fuzzing_session, 'params_json_schema'):
            schema = run_comprehensive_fuzzing_session.params_json_schema
            params = list(schema.get('properties', {}).keys())

            expected_params = ['num_queries', 'complexity', 'host', 'port', 'output_dir']

            if params == expected_params:
                print("✅ Tool has correct function signature")
                return True
            else:
                print(f"❌ Tool signature mismatch. Expected: {expected_params}, Got: {params}")
                return False
        else:
            print("❌ Tool missing params_json_schema")
            return False

    except Exception as e:
        print(f"❌ Error checking tool signature: {e}")
        return False

def test_tool_decorator():
    """Test that the tool has the @function_tool decorator."""
    try:
        from tools.comprehensive_fuzzing_tool import run_comprehensive_fuzzing_session

        # Check if the function has the agents framework attributes
        if hasattr(run_comprehensive_fuzzing_session, 'params_json_schema') and hasattr(run_comprehensive_fuzzing_session, 'name'):
            print("✅ Tool has @function_tool decorator")
            return True
        else:
            print("❌ Tool missing @function_tool decorator")
            return False

    except Exception as e:
        print(f"❌ Error checking tool decorator: {e}")
        return False

def test_agent_import():
    """Test that the fuzzing agent can be imported."""
    try:
        from fuzzing_agent import EmbucketFuzzingAgent
        print("✅ Successfully imported EmbucketFuzzingAgent")
        return True
    except ImportError as e:
        print(f"❌ Failed to import fuzzing agent: {e}")
        return False

def test_agent_tools():
    """Test that the agent has the comprehensive tool configured."""
    try:
        # Mock the environment variables to avoid requiring .env file
        os.environ.setdefault('OPENAI_API_KEY', 'test-key-for-testing')
        
        from fuzzing_agent import EmbucketFuzzingAgent
        
        agent = EmbucketFuzzingAgent()
        
        if len(agent.tools) == 1:
            tool_name = agent.tools[0].name if hasattr(agent.tools[0], 'name') else str(agent.tools[0])
            print(f"✅ Agent has 1 tool configured: {tool_name}")
            return True
        else:
            print(f"❌ Agent has {len(agent.tools)} tools, expected 1")
            return False
            
    except Exception as e:
        print(f"❌ Error testing agent tools: {e}")
        return False

def main():
    """Run all tests."""
    print("🧪 Testing Comprehensive Fuzzing Tool Architecture")
    print("=" * 50)
    
    tests = [
        test_comprehensive_tool_import,
        test_tool_signature,
        test_tool_decorator,
        test_agent_import,
        test_agent_tools
    ]
    
    passed = 0
    total = len(tests)
    
    for test in tests:
        try:
            if test():
                passed += 1
        except Exception as e:
            print(f"❌ Test {test.__name__} failed with exception: {e}")
        print()
    
    print("=" * 50)
    print(f"📊 Test Results: {passed}/{total} tests passed")
    
    if passed == total:
        print("🎉 All tests passed! The comprehensive fuzzing tool is ready to use.")
        return 0
    else:
        print("⚠️ Some tests failed. Please check the errors above.")
        return 1

if __name__ == "__main__":
    sys.exit(main())
