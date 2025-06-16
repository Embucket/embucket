# Fuzzing Agent Verbose Mode

## Overview

The fuzzing agents now support a verbose mode that can be controlled via environment variable. By default, the agents run in quiet mode to reduce terminal output and focus on essential information.

## Environment Variables

Set `FUZZING_VERBOSE=true` to enable verbose output:

```bash
export FUZZING_VERBOSE=true
```

Or set it to `false` (default) for quiet mode:

```bash
export FUZZING_VERBOSE=false
```

Set `FUZZING_DEBUG=true` to enable full debug output (shows complete agent responses):

```bash
export FUZZING_DEBUG=true
```

## Output Differences

### Quiet Mode (Default)
- Shows only essential progress indicators
- Always shows bug detection alerts (🚨)
- Always shows final summaries
- Always shows iteration completion status
- Minimal logging to terminal

### Verbose Mode
- Shows detailed query execution logs
- Shows individual query results
- Shows detailed error messages
- Shows server lifecycle messages
- Shows backup creation details
- Shows detailed change analysis between iterations

## What's Always Shown (Both Modes)

Regardless of verbose mode, these important messages are always displayed:

1. **Bug Detection**: `🚨 ACTUAL BUG DETECTED`
2. **Crash Detection**: Connection errors that may indicate server crashes
3. **Timeout Detection**: Query execution timeouts
4. **Final Summaries**: Session completion statistics
5. **Iteration Progress**: Which iteration is running and completion status
6. **Success/Failure**: Whether bugs were found
7. **Iteration Changes**: What changes were made to SQL generation and why (compact format in quiet mode)

## What's Hidden in Quiet Mode

To reduce noise, these are only shown in verbose mode:

1. Individual query execution details
2. Detailed error messages for expected errors
3. Server build/start/stop messages
4. Database setup details
5. Query generation progress
6. Backup creation messages
7. Log file locations
8. Full detailed change lists (quiet mode shows first 3 changes, verbose shows all)

## Usage Examples

### Run in quiet mode (default):
```bash
python iterative_improvement_agent.py
```

### Run in verbose mode:
```bash
FUZZING_VERBOSE=true python iterative_improvement_agent.py
```

### Set in .env file:
```
FUZZING_VERBOSE=true
```

## Benefits

- **Quiet Mode**: Easier to monitor overall progress without being overwhelmed by logs
- **Verbose Mode**: Full debugging information when needed for troubleshooting
- **Consistent**: All important information (bugs, crashes, summaries) always visible
- **Flexible**: Can be toggled without code changes
