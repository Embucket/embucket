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

    try:
        # Check if we're in the right directory (should have Cargo.toml)
        if not os.path.exists("Cargo.toml"):
            return "Error: Cargo.toml not found. Make sure you're in the Embucket project root."

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
            return f"Build completed successfully - binary created at: {binary_path}"
        else:
            return f"Build failed: {result.stderr}"

    except subprocess.TimeoutExpired:
        return "Build timed out after 5 minutes"
    except FileNotFoundError:
        return "Cargo not found. Make sure Rust and Cargo are installed."
    except Exception as e:
        return f"Build error: {str(e)}"
