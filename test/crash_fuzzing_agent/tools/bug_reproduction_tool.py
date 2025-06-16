"""
Bug reproduction tool for verifying if crashes can be reproduced with standalone queries.
"""

import json
import time
from typing import Dict, Any
from .server_lifecycle_tool import start_embucket_server, stop_embucket_server
from .database_setup_tool import setup_test_database
from .query_execution_tool import execute_query_against_embucket


def reproduce_bug(
    problematic_query: str,
    original_error_type: str,
    host: str = "localhost",
    port: int = 3000,
    database: str = "embucket",
    db_schema: str = "public"
) -> Dict[str, Any]:
    """
    Attempt to reproduce a bug by restarting the server and running the problematic query in isolation.
    
    This function:
    1. Stops the current server
    2. Starts a fresh server instance
    3. Re-runs the database setup
    4. Executes only the problematic query
    5. Compares the result with the original error
    
    Args:
        problematic_query: The SQL query that caused the original bug
        original_error_type: The error type from the original execution
        host: Embucket server host
        port: Embucket server port
        database: Database name to use
        db_schema: Schema name to use
        
    Returns:
        Dictionary containing reproduction results:
        - reproduction_attempted: bool
        - reproduction_successful: bool
        - reproduction_error_type: str (if reproduced)
        - reproduction_error_message: str
        - original_error_type: str
        - execution_time: float
        - details: str (human-readable summary)
    """
    print(f"\n🔄 ATTEMPTING BUG REPRODUCTION")
    print(f"Original error type: {original_error_type}")
    print(f"Query to reproduce: {problematic_query}")
    
    reproduction_result = {
        "reproduction_attempted": True,
        "reproduction_successful": False,
        "reproduction_error_type": None,
        "reproduction_error_message": "",
        "original_error_type": original_error_type,
        "execution_time": 0.0,
        "details": ""
    }
    
    start_time = time.time()
    embucket_url = f"http://{host}:{port}"
    
    try:
        # Step 1: Stop current server
        print("🛑 Stopping current server for clean reproduction...")
        stop_result = stop_embucket_server()
        print(f"Server stop result: {stop_result}")
        
        # Wait a moment for clean shutdown
        time.sleep(2)
        
        # Step 2: Start fresh server (with automatic port cleanup)
        print("🚀 Starting fresh server for reproduction...")
        start_success = start_embucket_server(host, port, kill_existing=True)
        
        if not start_success:
            reproduction_result["reproduction_error_message"] = "Failed to start server for reproduction"
            reproduction_result["details"] = "Bug reproduction failed: Could not start fresh server"
            return reproduction_result
        
        print("✅ Fresh server started successfully")
        
        # Step 3: Re-setup database
        print("🗄️ Re-setting up test database...")
        db_setup_result = setup_test_database(embucket_url, include_sample_data=True, database=database, db_schema=db_schema)
        
        if "successfully" not in db_setup_result.lower():
            reproduction_result["reproduction_error_message"] = f"Database setup failed: {db_setup_result}"
            reproduction_result["details"] = "Bug reproduction failed: Could not setup test database"
            return reproduction_result
        
        print("✅ Test database re-setup completed")
        
        # Step 4: Execute the problematic query in isolation
        print("🎯 Executing problematic query in isolation...")
        result_json = execute_query_against_embucket(problematic_query, host, port)
        result = json.loads(result_json)
        
        reproduction_result["execution_time"] = time.time() - start_time
        
        # Step 5: Analyze reproduction result
        reproduction_error_type = result.get("error_type")
        
        if reproduction_error_type == original_error_type:
            # Bug reproduced with same error type
            reproduction_result["reproduction_successful"] = True
            reproduction_result["reproduction_error_type"] = reproduction_error_type
            reproduction_result["reproduction_error_message"] = result.get("error_message", "")
            reproduction_result["details"] = f"✅ Bug REPRODUCED: Same error type '{original_error_type}' occurred in isolation"
            print(f"✅ BUG REPRODUCED: {original_error_type} error reproduced successfully")
            
        elif reproduction_error_type is None:
            # Query succeeded in isolation - bug not reproduced
            reproduction_result["reproduction_successful"] = False
            reproduction_result["reproduction_error_message"] = "Query executed successfully in isolation"
            reproduction_result["details"] = f"❌ Bug NOT reproduced: Query succeeded when run in isolation (original: {original_error_type})"
            print(f"❌ BUG NOT REPRODUCED: Query succeeded in isolation (original error: {original_error_type})")
            
        elif reproduction_error_type != original_error_type:
            # Different error occurred - partial reproduction
            reproduction_result["reproduction_successful"] = False
            reproduction_result["reproduction_error_type"] = reproduction_error_type
            reproduction_result["reproduction_error_message"] = result.get("error_message", "")
            reproduction_result["details"] = f"⚠️ Different error in isolation: {reproduction_error_type} (original: {original_error_type})"
            print(f"⚠️ DIFFERENT ERROR: Got {reproduction_error_type} instead of {original_error_type}")
        
        return reproduction_result
        
    except Exception as e:
        reproduction_result["execution_time"] = time.time() - start_time
        reproduction_result["reproduction_error_message"] = f"Reproduction attempt failed: {str(e)}"
        reproduction_result["details"] = f"❌ Bug reproduction failed due to unexpected error: {str(e)}"
        print(f"❌ REPRODUCTION FAILED: {str(e)}")
        return reproduction_result
