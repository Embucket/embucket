# Bug Reproduction Feature

## Overview

The fuzzing agent now includes automatic bug reproduction functionality to help distinguish between deterministic bugs (reproducible with specific queries) and load-dependent bugs (only occur under system stress).

## How It Works

When the fuzzing agent detects a bug (crash, server error, timeout, or unknown error), it automatically attempts to reproduce the bug using this algorithm:

1. **Stop Current Server** - Cleanly shut down the current Embucket server instance
2. **Start Fresh Server** - Launch a new, clean Embucket server instance  
3. **Re-setup Database** - Re-run all database setup queries to recreate the test environment
4. **Execute Problematic Query** - Run only the specific query that caused the original bug
5. **Compare Results** - Analyze if the same error type occurs in isolation

## Results Classification

### ✅ Bug Reproduced
- Same error type occurs when query is run in isolation
- Indicates a **deterministic bug** that can be triggered by the specific query
- These bugs are typically easier to debug and fix

### ❌ Bug Not Reproduced  
- Query executes successfully when run in isolation
- Indicates a **load-dependent bug** that only occurs under system stress
- May be related to concurrency, memory pressure, or timing issues

### ⚠️ Different Error
- A different error type occurs when query is run in isolation
- May indicate multiple issues or environment-dependent behavior

## New Output Fields

The fuzzing session results now include additional fields:

### Execution Summary
```json
{
  "execution_summary": {
    "reproduction_attempts": 2,
    "successful_reproductions": 1,
    // ... other fields
  }
}
```

### Bug Details
Each bug in `bug_details` now includes reproduction information:
```json
{
  "query_number": 5,
  "error_type": "crash",
  "query": "SELECT * FROM ...",
  "reproduction_attempted": true,
  "reproduction_successful": true,
  "reproduction_error_type": "crash",
  "reproduction_error_message": "Server crashed during query execution",
  "original_error_type": "crash",
  "execution_time": 2.5,
  "details": "✅ Bug REPRODUCED: Same error type 'crash' occurred in isolation"
}
```

### Recommendations
The system now provides reproduction-specific recommendations:

- **All bugs reproducible**: "All X bugs were reproducible with standalone queries - indicates deterministic issues"
- **No bugs reproducible**: "None of X bugs were reproducible - indicates load-dependent or timing issues"  
- **Mixed results**: "X/Y bugs were reproducible - mixed deterministic and load-dependent issues"

## Benefits

1. **Better Bug Classification** - Quickly identify which bugs are deterministic vs load-dependent
2. **Debugging Priority** - Focus on reproducible bugs first as they're easier to fix
3. **Root Cause Analysis** - Understand if issues are query-specific or system-stress related
4. **Test Case Quality** - Reproducible bugs make better regression tests

## Usage

The reproduction feature is automatically enabled in the comprehensive fuzzing tool. No additional configuration is required.

```python
# The reproduction happens automatically when bugs are detected
result = await agent.run_fuzzing_session(num_queries=10, complexity="complex")

# Check reproduction results in the output
session_data = json.loads(result.final_output)
for bug in session_data["bug_details"]:
    if bug["reproduction_attempted"]:
        print(f"Bug {bug['error_type']}: {'Reproduced' if bug['reproduction_successful'] else 'Not reproduced'}")
```

## Implementation Details

- **New File**: `tools/bug_reproduction_tool.py` - Contains the reproduction logic
- **Updated File**: `tools/comprehensive_fuzzing_tool.py` - Integrated reproduction into main workflow
- **Server Lifecycle**: Uses existing server management tools for clean restart
- **Database Setup**: Reuses existing database setup to ensure consistent test environment
- **Error Handling**: Robust error handling for reproduction failures

## Performance Impact

- Reproduction adds ~2-5 seconds per bug detected (server restart + database setup)
- Only runs when actual bugs are found (not for expected errors)
- Early termination still occurs after reproduction attempt
- Minimal impact on overall fuzzing performance since bugs trigger early termination anyway
