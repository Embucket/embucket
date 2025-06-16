"""
Embucket server lifecycle management tools.
"""

import time
import subprocess
import requests
import os
import signal
from typing import Optional, Tuple
from datetime import datetime
# Global variable to track server process
_embucket_process: Optional[subprocess.Popen] = None
# Global variables to track log files
_log_stdout_file: Optional[str] = None
_log_stderr_file: Optional[str] = None


def _check_server_health(host: str, port: int, timeout: int = 30) -> bool:
    """
    Check if the Embucket server is responding to health checks.

    Args:
        host: Server host
        port: Server port
        timeout: Maximum time to wait for server to be ready

    Returns:
        True if server is healthy, False otherwise
    """
    health_url = f"http://{host}:{port}/health"
    start_time = time.time()
    attempt = 0

    while time.time() - start_time < timeout:
        attempt += 1
        try:
            print(f"Health check attempt {attempt}...")
            response = requests.get(health_url, timeout=5)
            print(f"Health check response: {response.status_code}")
            if response.status_code == 200:
                return True
            else:
                print(f"Health check failed with status {response.status_code}: {response.text}")
        except requests.exceptions.ConnectionError as e:
            print(f"Health check connection error: {e}")
        except requests.exceptions.Timeout as e:
            print(f"Health check timeout: {e}")
        except requests.exceptions.RequestException as e:
            print(f"Health check request error: {e}")
        time.sleep(2)

    print(f"Health checks failed after {timeout} seconds ({attempt} attempts)")
    return False


def _create_log_files() -> Tuple[str, str]:
    """
    Create timestamped log files for Embucket server output.

    Returns:
        Tuple of (stdout_log_path, stderr_log_path)
    """
    # Create logs directory if it doesn't exist
    logs_dir = "test/logs"
    os.makedirs(logs_dir, exist_ok=True)

    # Create timestamped log files
    timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
    stdout_log = os.path.join(logs_dir, f"embucket_stdout_{timestamp}.log")
    stderr_log = os.path.join(logs_dir, f"embucket_stderr_{timestamp}.log")

    return stdout_log, stderr_log


def get_current_log_files() -> Tuple[Optional[str], Optional[str]]:
    """
    Get the current log file paths for the running server.

    Returns:
        Tuple of (stdout_log_path, stderr_log_path) or (None, None) if no server running
    """
    global _log_stdout_file, _log_stderr_file
    return _log_stdout_file, _log_stderr_file


def _kill_processes_on_port(port: int) -> bool:
    """
    Kill any processes that might be using the specified port.

    Args:
        port: Port number to check

    Returns:
        True if any processes were killed, False otherwise
    """
    try:
        import psutil
        killed_any = False

        for proc in psutil.process_iter(['pid', 'name', 'connections']):
            try:
                connections = proc.info['connections']
                if connections:
                    for conn in connections:
                        if hasattr(conn, 'laddr') and conn.laddr and conn.laddr.port == port:
                            print(f"🔪 Killing process {proc.info['pid']} ({proc.info['name']}) using port {port}")
                            proc.kill()
                            killed_any = True
                            break
            except (psutil.NoSuchProcess, psutil.AccessDenied, psutil.ZombieProcess):
                continue

        if killed_any:
            time.sleep(2)  # Give processes time to die

        return killed_any
    except ImportError:
        print("⚠️ psutil not available - cannot automatically kill processes on port")
        return False
    except Exception as e:
        print(f"⚠️ Error killing processes on port {port}: {e}")
        return False


def start_embucket_server(host: str = "localhost", port: int = 3000, kill_existing: bool = True) -> bool:
    """
    Start the Embucket server process.

    Args:
        host: Host to bind the server to
        port: Port to bind the server to
        kill_existing: Whether to kill existing processes on the port

    Returns:
        True if server started successfully, False otherwise
    """
    global _embucket_process, _log_stdout_file, _log_stderr_file

    # Check if server is already running
    if _embucket_process and _embucket_process.poll() is None:
        print("✓ Embucket server is already running")
        return True

    # Find workspace root and check if binary exists
    workspace_root = _find_workspace_root()
    if not workspace_root:
        print("✗ Could not find Embucket workspace root (Cargo.toml not found)")
        return False

    binary_path = os.path.join(workspace_root, "target/debug/embucketd")
    if not os.path.exists(binary_path):
        print(f"✗ Embucket binary not found at {binary_path}")
        print("Please build the server first using: cargo build --bin embucketd")
        return False

    # Check if port is already in use (prioritize IPv4, IPv6 is optional)
    import socket
    ipv4_available = False
    ipv6_available = False

    # Test IPv4 (required)
    try:
        with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
            s.bind((host, port))
        print(f"✓ Port {port} is available on IPv4")
        ipv4_available = True
    except OSError as e:
        print(f"✗ Port {port} is already in use on IPv4: {e}")

    # Test IPv6 (optional - many systems may not have IPv6 or it may be in use)
    try:
        with socket.socket(socket.AF_INET6, socket.SOCK_STREAM) as s:
            s.bind((host, port))
        print(f"✓ Port {port} is available on IPv6")
        ipv6_available = True
    except OSError as e:
        print(f"⚠️ Port {port} is not available on IPv6: {e}")
        print("   (This is often normal and won't prevent server startup)")

    # Only fail if IPv4 is not available (IPv6 is optional)
    if not ipv4_available:
        if kill_existing:
            print("🔄 Attempting to kill existing processes on port...")
            if _kill_processes_on_port(port):
                print("✓ Killed existing processes, retrying port check...")
                # Retry IPv4 check after killing processes
                try:
                    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
                        s.bind((host, port))
                    print(f"✓ Port {port} is now available on IPv4")
                    ipv4_available = True
                except OSError as e:
                    print(f"✗ Port {port} is still in use on IPv4 after cleanup: {e}")

        if not ipv4_available:
            print("✗ IPv4 port is required but not available")
            print("Please choose a different port or manually stop the process using that port")
            return False

    if ipv4_available and not ipv6_available:
        print("ℹ️ Server will start on IPv4 only (IPv6 not available)")
    elif ipv4_available and ipv6_available:
        print("ℹ️ Server will start on both IPv4 and IPv6")

    # Create log files for this session
    stdout_log_path, stderr_log_path = _create_log_files()
    _log_stdout_file = stdout_log_path
    _log_stderr_file = stderr_log_path

    print("Starting Embucket server...")
    print(f"Running: {binary_path} --host {host} --port {port}")
    print(f"Working directory: {os.getcwd()}")
    print(f"Logging stdout to: {stdout_log_path}")
    print(f"Logging stderr to: {stderr_log_path}")

    try:
        # Open log files for writing
        stdout_file = open(stdout_log_path, 'w')
        stderr_file = open(stderr_log_path, 'w')

        # Set environment variables to disable colored output in logs
        env = os.environ.copy()
        env['NO_COLOR'] = '1'  # Standard environment variable to disable colors
        env['RUST_LOG_STYLE'] = 'never'  # Disable tracing-subscriber colors

        # Start the server process with log file redirection
        _embucket_process = subprocess.Popen(
            [binary_path, "--host", host, "--port", str(port)],
            stdout=stdout_file,
            stderr=stderr_file,
            text=True,
            env=env
        )

        print(f"✓ Server process started (PID: {_embucket_process.pid})")
        print(f"✓ Server starting on http://{host}:{port}")
        print("Waiting for server to be ready...")

        # Wait for server to be ready
        if _check_server_health(host, port, timeout=30):
            print("✓ Server is ready to accept connections")
            return True
        else:
            print("✗ Server failed to become ready within 30 seconds")

            # Show log file locations for debugging
            print(f"Check server logs at:")
            print(f"  STDOUT: {stdout_log_path}")
            print(f"  STDERR: {stderr_log_path}")

            # Show recent log content for immediate debugging
            try:
                # Flush and close files to ensure content is written
                stdout_file.flush()
                stderr_file.flush()
                stdout_file.close()
                stderr_file.close()

                # Read recent content from log files
                if os.path.exists(stdout_log_path):
                    with open(stdout_log_path, 'r') as f:
                        stdout_content = f.read()
                        if stdout_content.strip():
                            print(f"Recent STDOUT:\n{stdout_content[-1000:]}")  # Last 1000 chars
                        else:
                            print("STDOUT log is empty")

                if os.path.exists(stderr_log_path):
                    with open(stderr_log_path, 'r') as f:
                        stderr_content = f.read()
                        if stderr_content.strip():
                            print(f"Recent STDERR:\n{stderr_content[-1000:]}")  # Last 1000 chars
                        else:
                            print("STDERR log is empty")

            except Exception as e:
                print(f"Error reading log files: {e}")

            # Clean up process
            if _embucket_process:
                if _embucket_process.poll() is None:
                    _embucket_process.kill()
                    _embucket_process.wait()
                _embucket_process = None

            # Reset log file tracking
            _log_stdout_file = None
            _log_stderr_file = None
            return False

    except Exception as e:
        print(f"✗ Failed to start server: {e}")
        _embucket_process = None
        _log_stdout_file = None
        _log_stderr_file = None
        # Close log files if they were opened
        try:
            stdout_file.close()
            stderr_file.close()
        except:
            pass
        return False


def stop_embucket_server() -> str:
    """
    Stop the Embucket server process and finalize log files.

    Returns:
        String describing the stop result
    """
    global _embucket_process, _log_stdout_file, _log_stderr_file

    if not _embucket_process:
        return "No server process found"

    try:
        # Check if process is still running
        if _embucket_process.poll() is None:
            # Send SIGTERM for graceful shutdown
            _embucket_process.terminate()

            # Wait for graceful shutdown (up to 10 seconds)
            try:
                _embucket_process.wait(timeout=10)
                result = "Server stopped gracefully"
            except subprocess.TimeoutExpired:
                _embucket_process.kill()
                _embucket_process.wait()
                result = "Server forcefully terminated"
        else:
            result = "Server process was already stopped"

        # Log file information
        if _log_stdout_file and _log_stderr_file:
            result += f"\nLogs saved to:\n  STDOUT: {_log_stdout_file}\n  STDERR: {_log_stderr_file}"

        # Clean up
        _embucket_process = None
        _log_stdout_file = None
        _log_stderr_file = None

        return result

    except Exception as e:
        _embucket_process = None
        _log_stdout_file = None
        _log_stderr_file = None
        return f"Error stopping server: {str(e)}"


def _find_workspace_root() -> str:
    """
    Find the workspace root directory by looking for Cargo.toml.

    Returns:
        Path to workspace root, or None if not found
    """
    current_dir = os.getcwd()

    # Check current directory and parent directories
    while current_dir != os.path.dirname(current_dir):  # Stop at filesystem root
        cargo_toml_path = os.path.join(current_dir, "Cargo.toml")
        if os.path.exists(cargo_toml_path):
            return current_dir
        current_dir = os.path.dirname(current_dir)

    # Check if Cargo.toml is in the current directory
    if os.path.exists("Cargo.toml"):
        return os.getcwd()

    return None
