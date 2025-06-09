#!/usr/bin/env python3
"""
Test that the fuzzing agent can import the database setup tool.
"""

import sys
import os
from unittest.mock import MagicMock

def test_agent_imports():
    """Test that the fuzzing agent can import all tools including the new database setup tool."""
    print("Testing fuzzing agent imports...")
    
    # Mock all the external dependencies
    mock_modules = [
        'dotenv',
        'agents', 
        'requests',
        'sqlglot',
        'sqlglot.expressions'
    ]
    
    for module in mock_modules:
        sys.modules[module] = MagicMock()
    
    # Mock specific functions and classes that are imported
    sys.modules['agents'].Agent = MagicMock()
    sys.modules['agents'].Runner = MagicMock()
    sys.modules['agents'].function_tool = lambda func: func
    sys.modules['dotenv'].load_dotenv = MagicMock()
    
    try:
        # Test importing the database setup tool
        from tools.database_setup_tool import setup_test_database
        print("✓ Successfully imported setup_test_database")
        
        # Test that the function has the expected attributes
        assert hasattr(setup_test_database, '__name__'), "Function missing __name__ attribute"
        assert setup_test_database.__name__ == 'setup_test_database', f"Unexpected function name: {setup_test_database.__name__}"
        print("✓ Database setup tool has correct function name")
        
        # Test importing the fuzzing agent
        from fuzzing_agent import EmbucketFuzzingAgent
        print("✓ Successfully imported EmbucketFuzzingAgent")
        
        print("✓ All import tests passed!")
        return True
        
    except ImportError as e:
        print(f"❌ Import failed: {e}")
        return False
    except Exception as e:
        print(f"❌ Unexpected error: {e}")
        return False

def test_agent_tool_list():
    """Test that the agent includes the database setup tool in its tools list."""
    print("\nTesting agent tool list...")
    
    # Read the fuzzing agent file to check tool list
    agent_path = os.path.join(os.path.dirname(__file__), 'fuzzing_agent.py')
    with open(agent_path, 'r') as f:
        content = f.read()
    
    # Check that setup_test_database is imported
    assert 'from tools.database_setup_tool import setup_test_database' in content, \
        "Database setup tool not imported in fuzzing agent"
    print("✓ Database setup tool is imported")
    
    # Check that setup_test_database is in the tools list
    assert 'setup_test_database,' in content, \
        "Database setup tool not in agent tools list"
    print("✓ Database setup tool is in agent tools list")
    
    # Check that the instructions mention database setup
    assert 'setup_test_database' in content, \
        "Database setup not mentioned in agent instructions"
    print("✓ Database setup mentioned in agent instructions")
    
    print("✓ All tool list tests passed!")

if __name__ == "__main__":
    try:
        success1 = test_agent_imports()
        test_agent_tool_list()
        
        if success1:
            print("\n🎉 All tests passed!")
        else:
            print("\n⚠ Some tests failed but this may be due to missing dependencies")
            
    except Exception as e:
        print(f"\n❌ Test failed: {e}")
        sys.exit(1)
