#!/usr/bin/env python3
"""
Comprehensive Fuzzing Tool for Embucket

This tool handles the entire fuzzing workflow in a single function by importing
and orchestrating the individual tool functions.
"""

import os
import time
import json
from typing import Dict, List, Any
from agents import function_tool
from dotenv import load_dotenv

# Load environment variables
load_dotenv()

# Import the individual tool functions (without decorators)
from .server_build_tool import build_embucket_server
from .server_lifecycle_tool import start_embucket_server, stop_embucket_server, get_current_log_files
from .database_setup_tool import setup_test_database
from .random_query_generator_tool import random_query_generator
from .query_execution_tool import execute_query_against_embucket
from .slt_file_tool import save_crash_slt_file
from .bug_reproduction_tool import reproduce_bug


def _run_comprehensive_fuzzing_session_impl(
    num_queries: int = 10,
    complexity: str = "medium",
    host: str = "localhost",
    port: int = 3000,
    output_dir: str = "test/sql/fuzz_regressions",
    safe_query_probability: float = None
) -> str:
    """
    Internal implementation of the comprehensive fuzzing session.
    This is the actual function that does the work.
    """
    # Get safe_query_probability from environment if not provided
    if safe_query_probability is None:
        safe_query_probability = float(os.getenv("DEFAULT_SAFE_QUERY_PROBABILITY", "0.3"))

    # Check if verbose mode is enabled
    verbose_mode = os.getenv("FUZZING_VERBOSE", "false").lower() == "true"

    if verbose_mode:
        print(f"🚀 Starting comprehensive fuzzing session with {num_queries} queries...")
        print(f"🎯 Using safe_query_probability: {safe_query_probability}")

    # Initialize results structure
    session_results = {
        "session_info": {
            "num_queries": num_queries,
            "complexity": complexity,
            "host": host,
            "port": port,
            "output_dir": output_dir,
            "start_time": time.strftime("%Y-%m-%d %H:%M:%S")
        },
        "server_lifecycle": {
            "build_success": False,
            "start_success": False,
            "database_setup_success": False,
            "stop_success": False,
            "stdout_log_file": None,
            "stderr_log_file": None
        },
        "execution_summary": {
            "queries_executed": 0,
            "successful_queries": 0,
            "expected_errors": 0,
            "actual_bugs": 0,
            "crashes_found": 0,
            "server_errors_found": 0,
            "timeouts_found": 0,
            "reproduction_attempts": 0,
            "successful_reproductions": 0,
            "early_termination": False,
            "early_termination_reason": None
        },
        "detailed_logs": [],
        "bug_details": [],
        "slt_files_created": [],
        "recommendations": []
    }

    embucket_url = f"http://{host}:{port}"

    try:
        # Step 1: Build Embucket server
        if verbose_mode:
            print("📦 Building Embucket server...")
        build_result = build_embucket_server()
        session_results["server_lifecycle"]["build_success"] = "successfully" in build_result.lower()
        # Only log build result if it failed (to reduce context length)
        if not session_results["server_lifecycle"]["build_success"]:
            session_results["detailed_logs"].append({
                "step": "build_server",
                "timestamp": time.strftime("%H:%M:%S"),
                "result": build_result[:200] + "..." if len(build_result) > 200 else build_result
            })

        if not session_results["server_lifecycle"]["build_success"]:
            session_results["execution_summary"]["early_termination"] = True
            session_results["execution_summary"]["early_termination_reason"] = "Server build failed"
            session_results["recommendations"].append("Fix server build issues before running fuzzing")
            return json.dumps(session_results)

        # Step 2: Start Embucket server (with automatic port cleanup)
        if verbose_mode:
            print(f"🔧 Starting Embucket server on {host}:{port}...")
        start_success = start_embucket_server(host, port, kill_existing=True)
        session_results["server_lifecycle"]["start_success"] = start_success

        # Get log file paths
        stdout_log, stderr_log = get_current_log_files()
        session_results["server_lifecycle"]["stdout_log_file"] = stdout_log
        session_results["server_lifecycle"]["stderr_log_file"] = stderr_log

        # Only log server start if it failed (to reduce context length)
        if not start_success:
            session_results["detailed_logs"].append({
                "step": "start_server",
                "timestamp": time.strftime("%H:%M:%S"),
                "result": f"Server start failed",
                "stdout_log": stdout_log,
                "stderr_log": stderr_log
            })

        if not start_success:
            session_results["execution_summary"]["early_termination"] = True
            session_results["execution_summary"]["early_termination_reason"] = "Server failed to start"
            session_results["recommendations"].append("Check if port is available and server binary exists")
            return json.dumps(session_results)

        # Step 3: Setup test database
        if verbose_mode:
            print("🗄️ Setting up test database...")

        # Capture the database setup output to suppress verbose logging
        import io
        import sys
        from contextlib import redirect_stdout, redirect_stderr

        # Capture stdout and stderr to suppress verbose output
        captured_output = io.StringIO()
        try:
            with redirect_stdout(captured_output), redirect_stderr(captured_output):
                db_setup_result = setup_test_database(embucket_url)
        except Exception as e:
            db_setup_result = f"Database setup failed: {str(e)}"

        session_results["server_lifecycle"]["database_setup_success"] = "successfully" in db_setup_result.lower() or "completed" in db_setup_result.lower()

        # Print simple success/failure message only in verbose mode
        if verbose_mode:
            if session_results["server_lifecycle"]["database_setup_success"]:
                print("✅ Database setup completed successfully")
            else:
                print("❌ Database setup failed")

        # Only log database setup if it failed (to reduce context length)
        if not session_results["server_lifecycle"]["database_setup_success"]:
            session_results["detailed_logs"].append({
                "step": "setup_database",
                "timestamp": time.strftime("%H:%M:%S"),
                "result": db_setup_result[:200] + "..." if len(db_setup_result) > 200 else db_setup_result
            })

        # Step 4: Generate and execute queries
        if verbose_mode:
            print(f"🎯 Generating and executing {num_queries} queries...")

        for i in range(num_queries):
            query_num = i + 1
            if verbose_mode:
                print(f"\n--- Query {query_num}/{num_queries} ---")

            try:
                # Generate query with configurable success rate (always complex now)
                if verbose_mode:
                    print(f"Generating complex query...")
                query = random_query_generator(database="embucket", db_schema="public", safe_query_probability=safe_query_probability)

                # Execute query and get detailed result
                if verbose_mode:
                    print(f"Executing query against Embucket...")
                result_json = execute_query_against_embucket(query, host, port)
                result = json.loads(result_json)

                session_results["execution_summary"]["queries_executed"] += 1

                # Only log failed queries and bugs to reduce context length
                if not result.get("success", False) or result.get("error_type") in ["crash", "server_error", "timeout", "unknown_error"]:
                    query_log = {
                        "step": f"execute_query_{query_num}",
                        "timestamp": time.strftime("%H:%M:%S"),
                        "query": query,  # Keep full query for failed/bug queries
                        "success": result.get("success", False),
                        "error_type": result.get("error_type"),
                        "execution_time": result.get("execution_time", 0)
                    }
                    session_results["detailed_logs"].append(query_log)

                # Analyze result and classify errors
                error_type = result.get("error_type")

                if error_type is None or error_type == "success":
                    session_results["execution_summary"]["successful_queries"] += 1
                    if verbose_mode:
                        print("✅ Query executed successfully")

                elif error_type in ["schema_error", "validation_error", "bad_request", "unauthorized", "forbidden", "not_found", "client_error", "request_error"]:
                    # Expected errors from fuzzy query generation or client-side issues
                    session_results["execution_summary"]["expected_errors"] += 1
                    if verbose_mode:
                        print(f"⚠️ Expected error: {error_type}")

                elif error_type in ["crash", "server_error", "timeout", "unknown_error"]:
                    # Actual bugs that need investigation
                    session_results["execution_summary"]["actual_bugs"] += 1

                    if error_type == "crash":
                        session_results["execution_summary"]["crashes_found"] += 1
                    elif error_type == "server_error":
                        session_results["execution_summary"]["server_errors_found"] += 1
                    elif error_type == "timeout":
                        session_results["execution_summary"]["timeouts_found"] += 1

                    # Always print bug detection (this is important)
                    print(f"🚨 ACTUAL BUG DETECTED: {error_type}")
                    if verbose_mode:
                        print(f"Error message: {result.get('error_message', 'No message')}")

                    # Save concise bug information (keep full queries)
                    bug_detail = {
                        "query_number": query_num,
                        "error_type": error_type,
                        "error_message": result.get("error_message", "")[:200] + "..." if len(result.get("error_message", "")) > 200 else result.get("error_message", ""),
                        "query": query,  # Keep full query for debugging
                        "timestamp": time.strftime("%H:%M:%S"),
                        "status_code": result.get("status_code"),
                        "execution_time": result.get("execution_time", 0)
                    }

                    # Attempt to reproduce the bug
                    if verbose_mode:
                        print(f"🔄 Attempting to reproduce bug...")
                    session_results["execution_summary"]["reproduction_attempts"] += 1

                    reproduction_result = reproduce_bug(
                        problematic_query=query,
                        original_error_type=error_type,
                        host=host,
                        port=port
                    )

                    # Add concise reproduction results to bug detail
                    bug_detail.update({
                        "reproduction_successful": reproduction_result.get("reproduction_successful", False),
                        "reproduction_details": reproduction_result.get("details", "")[:100] + "..." if len(reproduction_result.get("details", "")) > 100 else reproduction_result.get("details", "")
                    })

                    if reproduction_result["reproduction_successful"]:
                        session_results["execution_summary"]["successful_reproductions"] += 1
                        if verbose_mode:
                            print(f"✅ Bug reproduction: {reproduction_result['details']}")
                    else:
                        if verbose_mode:
                            print(f"❌ Bug reproduction: {reproduction_result['details']}")

                    session_results["bug_details"].append(bug_detail)

                    # Save SLT file for the bug
                    try:
                        test_name = f"{error_type}_{int(time.time())}_{query_num}"
                        stdout_log, stderr_log = get_current_log_files()
                        slt_result = save_crash_slt_file(
                            query,
                            json.dumps(result),
                            test_name,
                            output_dir,
                            stdout_log,
                            stderr_log
                        )
                        session_results["slt_files_created"].append(f"{test_name}.slt")
                        if verbose_mode:
                            print(f"💾 Saved SLT file: {test_name}.slt")
                            if stdout_log or stderr_log:
                                print(f"📋 Log files referenced in SLT file:")
                                if stdout_log:
                                    print(f"   STDOUT: {stdout_log}")
                                if stderr_log:
                                    print(f"   STDERR: {stderr_log}")
                    except Exception as e:
                        if verbose_mode:
                            print(f"⚠️ Failed to save SLT file: {e}")

                    # Terminate early when any actual bug is found
                    print(f"🚨 ACTUAL BUG DETECTED - Terminating fuzzing session early")
                    session_results["execution_summary"]["early_termination"] = True
                    session_results["execution_summary"]["early_termination_reason"] = f"{error_type.title()} detected on query {query_num}"
                    break

                else:
                    # Handle any unexpected error types that might be added in the future
                    session_results["execution_summary"]["expected_errors"] += 1
                    if verbose_mode:
                        print(f"⚠️ Unrecognized error type (treating as expected): {error_type}")
                        print(f"Error message: {result.get('error_message', 'No message')}")

                # Small delay between queries
                time.sleep(0.1)

            except Exception as e:
                if verbose_mode:
                    print(f"❌ Unexpected error during query {query_num}: {e}")
                session_results["detailed_logs"].append({
                    "step": f"execute_query_{query_num}",
                    "timestamp": time.strftime("%H:%M:%S"),
                    "error": str(e)[:100] + "..." if len(str(e)) > 100 else str(e)
                })
                break

    except Exception as e:
        if verbose_mode:
            print(f"❌ Fatal error during fuzzing session: {e}")
        session_results["execution_summary"]["early_termination"] = True
        session_results["execution_summary"]["early_termination_reason"] = f"Fatal error: {str(e)}"

    finally:
        # Step 5: Always try to stop the server
        if verbose_mode:
            print("🛑 Stopping Embucket server...")
        try:
            stop_result = stop_embucket_server()
            session_results["server_lifecycle"]["stop_success"] = "stopped" in stop_result.lower()
            # Only log server stop if it failed (to reduce context length)
            if not session_results["server_lifecycle"]["stop_success"]:
                session_results["detailed_logs"].append({
                    "step": "stop_server",
                    "timestamp": time.strftime("%H:%M:%S"),
                    "result": stop_result[:100] + "..." if len(stop_result) > 100 else stop_result
                })
        except Exception as e:
            if verbose_mode:
                print(f"⚠️ Error stopping server: {e}")
            session_results["detailed_logs"].append({
                "step": "stop_server",
                "timestamp": time.strftime("%H:%M:%S"),
                "error": str(e)[:100] + "..." if len(str(e)) > 100 else str(e)
            })

    # Add final recommendations
    if session_results["execution_summary"]["crashes_found"] > 0:
        session_results["recommendations"].append("CRITICAL: Crashes detected - investigate immediately")
    if session_results["execution_summary"]["server_errors_found"] > 0:
        session_results["recommendations"].append("Server errors found - check server logs")
    if session_results["execution_summary"]["actual_bugs"] == 0:
        session_results["recommendations"].append("No critical bugs found - Embucket appears stable for this test set")

    # Add reproduction-specific recommendations
    reproduction_attempts = session_results["execution_summary"]["reproduction_attempts"]
    successful_reproductions = session_results["execution_summary"]["successful_reproductions"]

    if reproduction_attempts > 0:
        if successful_reproductions == reproduction_attempts:
            session_results["recommendations"].append(f"All {reproduction_attempts} bugs were reproducible with standalone queries - indicates deterministic issues")
        elif successful_reproductions == 0:
            session_results["recommendations"].append(f"None of {reproduction_attempts} bugs were reproducible - indicates load-dependent or timing issues")
        else:
            session_results["recommendations"].append(f"{successful_reproductions}/{reproduction_attempts} bugs were reproducible - mixed deterministic and load-dependent issues")

    session_results["session_info"]["end_time"] = time.strftime("%Y-%m-%d %H:%M:%S")

    # Always show final summary (this is important for monitoring progress)
    print(f"\n🏁 Fuzzing session completed!")
    print(f"📊 Summary: {session_results['execution_summary']['queries_executed']} queries executed")
    print(f"✅ Successful: {session_results['execution_summary']['successful_queries']}")
    print(f"⚠️ Expected errors: {session_results['execution_summary']['expected_errors']}")
    print(f"🚨 Actual bugs: {session_results['execution_summary']['actual_bugs']}")

    # Show log file locations only in verbose mode
    if verbose_mode:
        stdout_log = session_results["server_lifecycle"]["stdout_log_file"]
        stderr_log = session_results["server_lifecycle"]["stderr_log_file"]
        if stdout_log or stderr_log:
            print(f"\n📋 Embucket server logs saved:")
            if stdout_log:
                print(f"   STDOUT: {stdout_log}")
            if stderr_log:
                print(f"   STDERR: {stderr_log}")

    return json.dumps(session_results)


@function_tool
def run_comprehensive_fuzzing_session(
    num_queries: int = 10,
    complexity: str = "medium",
    host: str = "localhost",
    port: int = 3000,
    output_dir: str = "test/sql/fuzz_regressions",
    safe_query_probability: float = None
) -> str:
    """
    Run a comprehensive fuzzing session that handles the entire workflow.

    This function:
    1. Builds and starts the Embucket server
    2. Sets up the test database
    3. Generates and executes SQL queries
    4. Monitors for crashes and collects detailed logs
    5. When a bug is found, attempts to reproduce it with a fresh server
    6. Saves SLT files for any crashes found
    7. Stops the server when done
    8. Returns early if a crash is detected

    Args:
        num_queries: Number of queries to generate and test
        complexity: Query complexity level (ignored - always generates complex queries now)
        host: Embucket server host
        port: Embucket server port
        output_dir: Directory to save SLT files
        safe_query_probability: Probability (0.0-1.0) of generating safer queries (default: reads from DEFAULT_SAFE_QUERY_PROBABILITY env var, fallback 0.3)

    Returns:
        JSON string containing comprehensive fuzzing session results with detailed logs and reproduction results
    """
    return _run_comprehensive_fuzzing_session_impl(num_queries, complexity, host, port, output_dir, safe_query_probability)
