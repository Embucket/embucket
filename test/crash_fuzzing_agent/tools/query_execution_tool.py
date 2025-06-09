"""
Query execution tool for running SQL against Embucket.
"""

import time
import json
import requests
from typing import Optional
def execute_query_against_embucket(sql_query: str, host: str = "localhost", port: int = 3000) -> str:
    """
    Execute a SQL query against the running Embucket server.

    Args:
        sql_query: The SQL query to execute
        host: Embucket server host
        port: Embucket server port

    Returns:
        JSON string containing execution result with keys:
        - success: bool
        - status_code: int (HTTP status code)
        - response: str (response body)
        - error_type: str (if error occurred)
        - error_message: str (detailed error message for display)
        - execution_time: float (in seconds)
    """
    print(f"Executing query against Embucket: {sql_query}")

    start_time = time.time()

    # Construct the API endpoint URL
    url = f"http://{host}:{port}/ui/queries"

    # Prepare the request payload
    payload = {
        "query": sql_query,
        "worksheetId": None,
        "context": None
    }

    headers = {
        "Content-Type": "application/json"
    }

    try:
        # Make the HTTP request with a timeout
        response = requests.post(
            url,
            headers=headers,
            json=payload,
            timeout=30  # 30 second timeout
        )

        execution_time = time.time() - start_time

        # Handle successful response (200 OK)
        if response.status_code == 200:
            print("✓ Query executed successfully")
            # For successful queries, show a preview of the response
            response_preview = response.text[:200] + "..." if len(response.text) > 200 else response.text
            print(f"  Response preview: {response_preview}")
            result = {
                "success": True,
                "status_code": response.status_code,
                "response": response.text,
                "error_type": None,
                "error_message": None,
                "execution_time": execution_time
            }
        else:
            # Handle HTTP error responses
            error_type = _classify_error_type(response.status_code, response.text)
            print(f"✗ Query failed with HTTP {response.status_code}: {error_type}")
            print(f"  Full error message: {response.text}")

            result = {
                "success": False,
                "status_code": response.status_code,
                "response": response.text,
                "error_type": error_type,
                "error_message": f"HTTP {response.status_code} ({error_type}): {response.text}",
                "execution_time": execution_time
            }

    except requests.exceptions.Timeout:
        execution_time = time.time() - start_time
        print("✗ Query execution timed out")
        print("  Full error message: Request timeout after 30 seconds")
        result = {
            "success": False,
            "status_code": 408,
            "response": "Request timeout after 30 seconds",
            "error_type": "timeout",
            "error_message": "Request timeout after 30 seconds",
            "execution_time": execution_time
        }

    except requests.exceptions.ConnectionError as e:
        execution_time = time.time() - start_time
        print("✗ Connection error - server may have crashed")
        print(f"  Full error message: {str(e)}")
        result = {
            "success": False,
            "status_code": None,
            "response": f"Connection error: {str(e)}",
            "error_type": "crash",
            "error_message": f"Connection error (server may have crashed): {str(e)}",
            "execution_time": execution_time
        }

    except requests.exceptions.RequestException as e:
        execution_time = time.time() - start_time
        print("✗ Request failed with error")
        print(f"  Full error message: {str(e)}")
        result = {
            "success": False,
            "status_code": None,
            "response": f"Request error: {str(e)}",
            "error_type": "request_error",
            "error_message": f"Request error: {str(e)}",
            "execution_time": execution_time
        }

    except Exception as e:
        execution_time = time.time() - start_time
        print("✗ Unexpected error occurred")
        print(f"  Full error message: {str(e)}")
        result = {
            "success": False,
            "status_code": None,
            "response": f"Unexpected error: {str(e)}",
            "error_type": "unknown_error",
            "error_message": f"Unexpected error: {str(e)}",
            "execution_time": execution_time
        }

    return json.dumps(result)


def _classify_error_type(status_code: int, response_text: str) -> str:
    """
    Classify the error type based on HTTP status code and response content.

    Args:
        status_code: HTTP status code
        response_text: Response body text

    Returns:
        String describing the error type
    """
    if status_code == 422:
        # Check if it's a DataFusion schema error
        if "schema" in response_text.lower() or "datafusion" in response_text.lower():
            return "schema_error"
        else:
            return "validation_error"
    elif status_code == 500:
        return "server_error"
    elif status_code == 400:
        return "bad_request"
    elif status_code == 401:
        return "unauthorized"
    elif status_code == 403:
        return "forbidden"
    elif status_code == 404:
        return "not_found"
    elif status_code == 408:
        return "timeout"
    elif 400 <= status_code < 500:
        return "client_error"
    elif 500 <= status_code < 600:
        return "server_error"
    else:
        return "unknown_error"
