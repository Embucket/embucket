"""
Embucket server build tool.
"""

import subprocess
import os

def build_embucket_server(force_rebuild: bool = False) -> str:
    """
    Build the Embucket server binary using cargo in debug mode.

    Args:
        force_rebuild: If True, always rebuild. If False, use existing binary if available.

    Returns:
        String describing the build result
    """
    print("Building Embucket server...")

    original_cwd = os.getcwd()  # Initialize outside try block

    try:
        # Find the workspace root directory (where Cargo.toml is located)
        workspace_root = _find_workspace_root()
        if not workspace_root:
            return "Error: Could not find Embucket workspace root (Cargo.toml not found in current or parent directories)"

        print(f"Using workspace root: {workspace_root}")

        # Change to workspace root for building
        os.chdir(workspace_root)

        # Determine binary path (debug build only)
        binary_path = "target/debug/embucketd"

        # Check if binary already exists
        if not force_rebuild and os.path.exists(binary_path):
            return f"Build completed successfully - existing binary found at: {binary_path}"

        # Build command for debug mode
        build_cmd = ["cargo", "build", "--bin", "embucketd"]
        timeout = 300  # 5 minutes for debug builds
        print("Running: cargo build --bin embucketd")
        print("Note: Debug build (faster compilation, slower runtime)")

        # Run cargo build command
        result = subprocess.run(
            build_cmd,
            capture_output=True,
            text=True,
            timeout=timeout
        )

        if result.returncode == 0 and os.path.exists(binary_path):
            return f"Build completed successfully - binary created at: {os.path.join(workspace_root, binary_path)}"
        else:
            return f"Build failed: {result.stderr}"

    except subprocess.TimeoutExpired:
        return "Build timed out after 5 minutes"
    except FileNotFoundError:
        return "Cargo not found. Make sure Rust and Cargo are installed."
    except Exception as e:
        return f"Build error: {str(e)}"
    finally:
        # Always restore original working directory
        try:
            os.chdir(original_cwd)
        except:
            pass


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
