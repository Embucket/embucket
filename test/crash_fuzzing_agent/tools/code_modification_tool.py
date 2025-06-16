"""
Code Modification Tool for applying improvements to the query generator.
"""

import os
import re
import json
from typing import Dict, List, Any
from agents import function_tool

@function_tool
def apply_query_generator_improvements(
    improvement_suggestions: str
) -> str:
    """
    Apply suggested improvements to the query generator code.

    This function automatically finds and modifies the random_query_generator_tool.py file.
    No file path parameter is needed as the file location is auto-detected.

    Args:
        improvement_suggestions: JSON string containing suggested modifications

    Returns:
        JSON string containing the results of applying modifications
    """
    try:
        # Auto-detect target file path
        target_file_path = _find_query_generator_file()
        if not target_file_path:
            return json.dumps({
                "success": False,
                "error": "Could not find random_query_generator_tool.py file",
                "modifications_applied": 0
            })

        # Parse the improvement suggestions
        suggestions = json.loads(improvement_suggestions)
        
        if "error" in suggestions:
            return json.dumps({
                "success": False,
                "error": "Cannot apply improvements due to analysis error",
                "modifications_applied": 0,
                "details": suggestions["error"]
            })
        
        # Read the current file content
        if not os.path.exists(target_file_path):
            return json.dumps({
                "success": False,
                "error": f"Target file not found: {target_file_path}",
                "modifications_applied": 0
            })
        
        with open(target_file_path, 'r') as f:
            original_content = f.read()
        
        modified_content = original_content
        modifications_applied = 0
        modification_details = []
        
        # Apply each suggested modification
        suggested_mods = suggestions.get("suggested_modifications", [])

        # Debug: Log what modifications we're trying to apply
        modification_details.append(f"DEBUG: Attempting to apply {len(suggested_mods)} modifications")

        for mod in suggested_mods:
            location = mod.get("location", "")
            change_type = mod.get("change_type", "")
            description = mod.get("description", "")
            code_snippet = mod.get("code_snippet", "")
            
            try:
                # Debug: Log what we're trying to match
                modification_details.append(f"DEBUG: Processing {change_type} modification: {description[:50]}...")

                if change_type == "modify" and ("safe_query_probability" in location or "safe_query_probability" in description):
                    # Modify the safe_query_probability value
                    pattern = r'self\.safe_query_probability\s*=\s*[\d.]+.*?(?=\s*#|$)'
                    if re.search(pattern, modified_content):
                        modified_content = re.sub(pattern, code_snippet.strip(), modified_content)
                        modifications_applied += 1
                        modification_details.append(f"Modified safe_query_probability: {description}")

                elif change_type == "add" and "window functions" in description:
                    # Add window functions to the aggressive query method
                    method_pattern = r'(def _generate_aggressive_complex_query\(self\):.*?)(        # Clean up tracking)'
                    match = re.search(method_pattern, modified_content, re.DOTALL)
                    if match:
                        before_cleanup = match.group(1)
                        cleanup_line = match.group(2)
                        new_content = before_cleanup + code_snippet + "\n\n" + cleanup_line
                        modified_content = modified_content.replace(match.group(0), new_content)
                        modifications_applied += 1
                        modification_details.append(f"Added window functions: {description}")
                
                elif change_type == "add" and "division by zero" in description:
                    # Add edge case conditions to _random_condition method
                    method_pattern = r'(        else:  # is_null\n.*?return col\.is_\(exp\.Null\(\)\)\.not_\(\))'
                    match = re.search(method_pattern, modified_content, re.DOTALL)
                    if match:
                        original_else = match.group(1)
                        new_else = original_else + "\n\n" + code_snippet
                        modified_content = modified_content.replace(original_else, new_else)
                        modifications_applied += 1
                        modification_details.append(f"Added edge case conditions: {description}")
                
                elif change_type == "add" and "recursive CTE" in description:
                    # Add recursive CTE method to the class
                    class_end_pattern = r'(\n\ndef random_query_generator\()'
                    match = re.search(class_end_pattern, modified_content)
                    if match:
                        insertion_point = match.start()
                        new_method = "\n" + code_snippet + "\n"
                        modified_content = modified_content[:insertion_point] + new_method + modified_content[insertion_point:]
                        modifications_applied += 1
                        modification_details.append(f"Added recursive CTE method: {description}")
                
                elif change_type == "modify" and "always use aggressive" in description:
                    # Modify generate_complex_query to always use aggressive mode
                    method_pattern = r'(def generate_complex_query\(self\):.*?)(        else:\n.*?return self\._generate_aggressive_complex_query\(\))'
                    match = re.search(method_pattern, modified_content, re.DOTALL)
                    if match:
                        method_start = match.group(1)
                        new_method = method_start + code_snippet
                        modified_content = modified_content.replace(match.group(0), new_method)
                        modifications_applied += 1
                        modification_details.append(f"Modified to always use aggressive mode: {description}")
                
            except Exception as e:
                modification_details.append(f"Failed to apply modification '{description}': {str(e)}")
        
        # Write the modified content back to the file
        if modifications_applied > 0:
            # Create a backup of the original file
            backup_path = target_file_path + f".backup.{int(__import__('time').time())}"
            with open(backup_path, 'w') as f:
                f.write(original_content)
            
            # Write the modified content
            with open(target_file_path, 'w') as f:
                f.write(modified_content)
            
            result = {
                "success": True,
                "modifications_applied": modifications_applied,
                "backup_created": backup_path,
                "details": modification_details,
                "analysis": suggestions.get("analysis", ""),
                "expected_impact": suggestions.get("expected_impact", ""),
                "iteration": suggestions.get("iteration", "unknown"),
                "current_metrics": suggestions.get("current_metrics", {}),
                "summary": f"Applied {modifications_applied} modifications to query generator in iteration {suggestions.get('iteration', 'unknown')}"
            }
        else:
            result = {
                "success": False,
                "modifications_applied": 0,
                "error": "No modifications could be applied",
                "details": modification_details
            }
        
        return json.dumps(result, indent=2)
        
    except Exception as e:
        return json.dumps({
            "success": False,
            "error": f"Failed to apply improvements: {str(e)}",
            "modifications_applied": 0
        })


@function_tool
def read_query_generator_code() -> str:
    """
    Read the current query generator code.

    This function automatically finds and reads the random_query_generator_tool.py file.
    No parameters are needed as the file location is auto-detected.

    Returns:
        The current source code of the query generator
    """
    try:
        # Auto-detect target file path
        target_file_path = _find_query_generator_file()

        if not target_file_path:
            # Provide detailed debugging information
            cwd = os.getcwd()
            return f"Error: Could not find random_query_generator_tool.py file. Current working directory: {cwd}. Please ensure the file exists in the tools/ subdirectory."

        if not os.path.exists(target_file_path):
            return f"Error: Target file not found: {target_file_path} (absolute: {os.path.abspath(target_file_path)})"

        with open(target_file_path, 'r') as f:
            content = f.read()

        return content

    except Exception as e:
        return f"Error reading file: {str(e)}"


@function_tool
def backup_and_restore_query_generator(
    action: str,
    backup_path: str = None
) -> str:
    """
    Backup or restore the query generator file.

    This function automatically finds the random_query_generator_tool.py file.
    No file path parameter is needed as the file location is auto-detected.

    Args:
        action: Either "backup" or "restore"
        backup_path: Path to backup file (required for restore action)

    Returns:
        JSON string containing the result of the backup/restore operation
    """
    try:
        # Auto-detect target file path
        target_file_path = _find_query_generator_file()
        if not target_file_path:
            return json.dumps({
                "success": False,
                "error": "Could not find random_query_generator_tool.py file"
            })

        if action == "backup":
            if not os.path.exists(target_file_path):
                return json.dumps({
                    "success": False,
                    "error": f"Target file not found: {target_file_path}"
                })
            
            # Create backup with timestamp
            timestamp = int(__import__('time').time())
            backup_path = f"{target_file_path}.backup.{timestamp}"
            
            with open(target_file_path, 'r') as source:
                content = source.read()
            
            with open(backup_path, 'w') as backup:
                backup.write(content)
            
            return json.dumps({
                "success": True,
                "action": "backup",
                "backup_path": backup_path,
                "message": f"Backup created successfully"
            })
        
        elif action == "restore":
            if not backup_path:
                return json.dumps({
                    "success": False,
                    "error": "backup_path is required for restore action"
                })
            
            if not os.path.exists(backup_path):
                return json.dumps({
                    "success": False,
                    "error": f"Backup file not found: {backup_path}"
                })
            
            with open(backup_path, 'r') as backup:
                content = backup.read()
            
            with open(target_file_path, 'w') as target:
                target.write(content)
            
            return json.dumps({
                "success": True,
                "action": "restore",
                "backup_path": backup_path,
                "message": f"File restored from backup successfully"
            })
        
        else:
            return json.dumps({
                "success": False,
                "error": f"Invalid action: {action}. Must be 'backup' or 'restore'"
            })
    
    except Exception as e:
        return json.dumps({
            "success": False,
            "error": f"Failed to {action} file: {str(e)}"
        })


def _find_query_generator_file() -> str:
    """
    Find the random_query_generator_tool.py file by searching in common locations.

    Returns:
        Path to the file, or None if not found
    """
    import sys

    # Get the current working directory and script directory
    cwd = os.getcwd()
    script_dir = os.path.dirname(os.path.abspath(__file__))

    # Debug information (only when file not found)
    debug_info = {
        "cwd": cwd,
        "script_dir": script_dir
    }

    # Common locations to search for the file
    possible_paths = [
        # Relative to current working directory
        "tools/random_query_generator_tool.py",
        "./tools/random_query_generator_tool.py",

        # Relative to script directory (where this tool is located)
        os.path.join(script_dir, "random_query_generator_tool.py"),
        os.path.join(os.path.dirname(script_dir), "tools", "random_query_generator_tool.py"),

        # From workspace root
        "test/crash_fuzzing_agent/tools/random_query_generator_tool.py",

        # From parent directories
        "../tools/random_query_generator_tool.py",
        "../../tools/random_query_generator_tool.py",

        # Absolute paths based on known structure
        os.path.join(cwd, "tools", "random_query_generator_tool.py"),
        os.path.join(os.path.dirname(cwd), "test", "crash_fuzzing_agent", "tools", "random_query_generator_tool.py"),
    ]

    # Remove duplicates while preserving order
    seen = set()
    unique_paths = []
    for path in possible_paths:
        abs_path = os.path.abspath(path)
        if abs_path not in seen:
            seen.add(abs_path)
            unique_paths.append(path)

    # Check each path
    for path in unique_paths:
        if os.path.exists(path):
            return os.path.abspath(path)

    # If not found in common locations, search in current directory and subdirectories
    for root, dirs, files in os.walk("."):
        if "random_query_generator_tool.py" in files:
            found_path = os.path.join(root, "random_query_generator_tool.py")
            return os.path.abspath(found_path)

    # Last resort: search from script directory upwards
    search_dir = script_dir
    for _ in range(5):  # Search up to 5 levels up
        for root, dirs, files in os.walk(search_dir):
            if "random_query_generator_tool.py" in files and "tools" in root:
                found_path = os.path.join(root, "random_query_generator_tool.py")
                return os.path.abspath(found_path)
        search_dir = os.path.dirname(search_dir)
        if search_dir == os.path.dirname(search_dir):  # Reached filesystem root
            break

    # File not found - provide debug information
    print(f"ERROR: Could not find random_query_generator_tool.py file")
    print(f"Current working directory: {debug_info['cwd']}")
    print(f"Script directory: {debug_info['script_dir']}")
    print(f"Searched {len(unique_paths)} locations")

    return None
