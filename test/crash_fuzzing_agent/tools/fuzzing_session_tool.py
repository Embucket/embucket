"""
Fuzzing session tool that handles automatic server restart logic.
"""

import json
import time
from agents import function_tool
from .random_query_generator_tool import random_query_generator
from .server_lifecycle_tool import start_embucket_server, stop_embucket_server
from .query_execution_tool import execute_query_against_embucket
from .slt_file_tool import save_crash_slt_file


@function_tool
def execute_fuzzing_queries(num_queries: int, complexity: str = "medium", 
                           host: str = "localhost", port: int = 3000,
                           output_dir: str = "test/sql/fuzz_regressions") -> str:
    """
    Execute a batch of fuzzing queries with automatic crash recovery.
    
    This function handles the core fuzzing loop with deterministic restart logic:
    - Generates and executes SQL queries
    - Automatically detects crashes and restarts server when needed
    - Saves problematic queries as SLT files
    - Returns a summary of the fuzzing session
    
    Args:
        num_queries: Number of queries to generate and test
        complexity: Query complexity level ("simple", "medium", "complex")
        host: Embucket server host
        port: Embucket server port
        output_dir: Directory to save SLT files
        
    Returns:
        JSON string containing fuzzing session results
    """
    print(f"Starting fuzzing session with {num_queries} queries...")
    
    session_results = {
        "total_queries": num_queries,
        "queries_executed": 0,
        "crashes_found": 0,
        "server_errors_found": 0,
        "schema_errors_found": 0,
        "timeouts_found": 0,
        "successful_queries": 0,
        "server_restarts": 0,
        "slt_files_created": []
    }
    
    for i in range(num_queries):
        print(f"\n--- Query {i+1}/{num_queries} ---")
        
        try:
            # Generate query
            print(f"Generating query with complexity: {complexity}")
            query = random_query_generator(complexity)
            
            # Execute query
            print(f"Executing query against Embucket...")
            result_json = execute_query_against_embucket(query, host, port)
            result = json.loads(result_json)
            
            session_results["queries_executed"] += 1
            
            # Check result and handle accordingly
            if result["success"]:
                print("✓ Query executed successfully")
                session_results["successful_queries"] += 1
            else:
                error_type = result.get("error_type", "unknown")
                print(f"✗ Query failed with error_type: {error_type}")
                
                # Count error types
                if error_type == "crash":
                    session_results["crashes_found"] += 1
                elif error_type == "server_error":
                    session_results["server_errors_found"] += 1
                elif error_type == "schema_error":
                    session_results["schema_errors_found"] += 1
                elif error_type == "timeout":
                    session_results["timeouts_found"] += 1
                
                # Save as SLT file for all errors
                print(f"Saving problematic query as SLT file...")
                slt_path = save_crash_slt_file(query, result_json, output_dir=output_dir)
                session_results["slt_files_created"].append(slt_path)
                print(f"Saved SLT file: {slt_path}")
                
                # DETERMINISTIC RESTART LOGIC - Only restart on actual crashes
                if error_type == "crash":
                    print("🔄 CRASH DETECTED - Restarting server...")
                    print("Stopping crashed server...")
                    stop_embucket_server()
                    
                    print("Waiting 2 seconds before restart...")
                    time.sleep(2)
                    
                    print("Starting server after crash...")
                    restart_success = start_embucket_server(host, port)
                    
                    if restart_success:
                        session_results["server_restarts"] += 1
                        print("✓ Server restarted successfully after crash")
                    else:
                        print("✗ Failed to restart server after crash")
                        break  # Stop fuzzing if we can't restart
                else:
                    print(f"ℹ️  No restart needed for {error_type} (server still running)")
            
            # Small delay between queries
            time.sleep(0.1)
            
        except Exception as e:
            print(f"Unexpected error during query {i+1}: {e}")
            break
    
    # Generate summary
    print(f"\n{'='*60}")
    print(f"FUZZING SESSION COMPLETED")
    print(f"{'='*60}")
    print(f"Total queries planned: {session_results['total_queries']}")
    print(f"Queries executed: {session_results['queries_executed']}")
    print(f"Successful queries: {session_results['successful_queries']}")
    print(f"Crashes found: {session_results['crashes_found']}")
    print(f"Server errors found: {session_results['server_errors_found']}")
    print(f"Schema errors found: {session_results['schema_errors_found']}")
    print(f"Timeouts found: {session_results['timeouts_found']}")
    print(f"Server restarts performed: {session_results['server_restarts']}")
    print(f"SLT files created: {len(session_results['slt_files_created'])}")
    
    if session_results['slt_files_created']:
        print(f"\nSLT files saved:")
        for slt_file in session_results['slt_files_created']:
            print(f"  - {slt_file}")
    
    return json.dumps(session_results, indent=2)
