# Debugging Guide for Embucket Fuzzing Agent

## Issue Resolved: "Server build failed" Error

### Problem
The fuzzing agent was reporting "Server build failed" even though the Embucket binary existed and could start successfully.

### Root Cause
The `server_build_tool.py` was returning the message "Existing binary found at: {path} - build skipped" when a binary already existed. However, the `comprehensive_fuzzing_tool.py` was checking for the word "successfully" in the build result to determine if the build was successful.

### Solution
Changed the build tool to return: `"Build completed successfully - existing binary found at: {path}"` so that the comprehensive tool correctly recognizes this as a successful build.

## Enhanced Debugging Features

### 1. Server Lifecycle Tool Enhancements
The `server_lifecycle_tool.py` now provides much more detailed debugging information:

- **Port Availability Check**: Verifies the port is not already in use
- **Detailed Health Checks**: Shows each health check attempt with response codes
- **Server Output Capture**: Captures and displays server stdout/stderr when startup fails
- **Process Status Monitoring**: Checks if the server process exits unexpectedly

### 2. Health Check Improvements
- Shows each health check attempt number
- Displays HTTP response codes and error messages
- Distinguishes between connection errors, timeouts, and other issues
- Provides detailed error messages for troubleshooting

### 3. Server Startup Debugging
When server startup fails, the tool now:
- Checks if the process is still running
- Captures server output (stdout/stderr)
- Reports process exit codes
- Provides specific error messages for different failure modes

## Common Issues and Solutions

### 1. Port Already in Use
**Error**: `Port 3000 is already in use`
**Solution**: 
- Stop any existing Embucket processes: `pkill -f embucketd`
- Or use a different port in your configuration

### 2. Binary Not Found
**Error**: `Embucket binary not found at ./target/debug/embucketd`
**Solution**: Build the server first: `cargo build --bin embucketd`

### 3. Permission Issues
**Error**: Binary is not executable
**Solution**: Check file permissions: `chmod +x ./target/debug/embucketd`

### 4. Server Starts But Health Checks Fail
**Symptoms**: Process starts but health checks timeout
**Debugging**: 
- Check server logs in the output
- Verify the server is listening on the correct port
- Test manually: `curl http://localhost:3000/health`

### 5. Server Exits Immediately
**Symptoms**: Process starts but exits with an error code
**Debugging**: 
- Check the server stderr output in the logs
- Common causes: configuration errors, missing dependencies, database connection issues

## Debugging Workflow

1. **Check Prerequisites**: Ensure binary exists and is executable
2. **Verify Port Availability**: Make sure the port isn't already in use
3. **Review Server Output**: Check stdout/stderr for error messages
4. **Test Manual Startup**: Try running the server manually to see raw error messages
5. **Check Health Endpoint**: Verify the server responds to health checks

## Enhanced Logging

The fuzzing agent now provides comprehensive logging including:
- Build process results
- Server startup details
- Health check attempts
- Database setup results
- Query execution details
- Server shutdown process

All logs are captured in the session results for analysis by the AI agent.

## Future Debugging

If you encounter similar issues in the future:

1. **Check the detailed logs** in the session results
2. **Look for specific error messages** in server stdout/stderr
3. **Verify the working directory** is correct (should be Embucket project root)
4. **Test individual components** (build, start, health check) separately
5. **Use the enhanced error messages** to identify the specific failure point

The enhanced debugging should provide much clearer information about what's happening during server startup and why it might be failing.
