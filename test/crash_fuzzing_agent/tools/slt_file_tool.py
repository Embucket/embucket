"""
SLT file creation tool for saving crash regression tests.
"""

import os
from datetime import datetime
from typing import Optional
def save_crash_slt_file(sql_query: str, error_info: str,
                       test_name: Optional[str] = None,
                       output_dir: str = "test/sql/fuzz_regressions",
                       stdout_log: Optional[str] = None,
                       stderr_log: Optional[str] = None) -> str:
    """
    Save a problematic SQL query as an .slt file for regression testing.

    Args:
        sql_query: The SQL query that caused the issue
        error_info: JSON string containing error details from execute_query_against_embucket
        test_name: Optional custom name for the test file
        output_dir: Directory to save the SLT file
        stdout_log: Optional path to Embucket stdout log file
        stderr_log: Optional path to Embucket stderr log file

    Returns:
        Path to the created .slt file
    """
    import json

    # Ensure output directory exists
    os.makedirs(output_dir, exist_ok=True)

    # Parse error_info from JSON string
    try:
        error_data = json.loads(error_info)
    except json.JSONDecodeError:
        error_data = {"error_type": "unknown", "response": error_info}

    if not test_name:
        # Generate descriptive filename based on error type and timestamp
        error_type = error_data.get("error_type", "unknown")
        timestamp = datetime.now().strftime("%Y_%m_%d_%H%M%S")
        test_name = f"{error_type}_{timestamp}"

    slt_filename = f"{test_name}.slt"
    slt_path = os.path.join(output_dir, slt_filename)

    # Create SLT file content with log file references
    log_info = ""
    if stdout_log or stderr_log:
        log_info = "\n# Embucket server logs:"
        if stdout_log:
            log_info += f"\n# STDOUT: {stdout_log}"
        if stderr_log:
            log_info += f"\n# STDERR: {stderr_log}"
        log_info += "\n"

    slt_content = f"""# Fuzzing regression test
# Generated on: {datetime.now().isoformat()}
# Error type: {error_data.get('error_type', 'unknown')}
# Status code: {error_data.get('status_code', 'N/A')}
# Execution time: {error_data.get('execution_time', 0):.3f}s{log_info}
# Original error response:
# {error_data.get('response', 'No response')}

statement error
{sql_query.strip()}
"""

    # Write the SLT file
    try:
        with open(slt_path, 'w', encoding='utf-8') as f:
            f.write(slt_content)
        print(f"Successfully saved crash SLT file: {slt_path}")
        return slt_path
    except IOError as e:
        error_msg = f"Failed to write SLT file {slt_path}: {e}"
        print(error_msg)
        raise IOError(error_msg)
