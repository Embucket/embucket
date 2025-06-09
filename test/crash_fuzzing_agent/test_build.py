#!/usr/bin/env python3
"""
Test script for the real build_embucket_server implementation.
"""

import subprocess
import os


def test_build_embucket_server():
    """Test the real build implementation directly."""
    print("Testing real Embucket server build...")
    print("=" * 50)
    
    try:
        # Check if we're in the right directory (should have Cargo.toml)
        if not os.path.exists("Cargo.toml"):
            print("Error: Cargo.toml not found. Make sure you're in the Embucket project root.")
            return False
        
        print("Running: cargo build --release --bin embucketd")

        # Run cargo build command
        result = subprocess.run(
            ["cargo", "build", "--release", "--bin", "embucketd"],
            capture_output=True,
            text=True,
            timeout=300  # 5 minute timeout
        )

        if result.returncode == 0:
            print("✓ Build completed successfully")

            # Check if the binary was created
            binary_path = "target/release/embucketd"
            if os.path.exists(binary_path):
                print(f"✓ Binary created at: {binary_path}")
                
                # Get binary info
                stat_info = os.stat(binary_path)
                print(f"✓ Binary size: {stat_info.st_size:,} bytes")
                return True
            else:
                print(f"✗ Binary not found at expected location: {binary_path}")
                return False
        else:
            print("✗ Build failed")
            print("STDOUT:", result.stdout)
            print("STDERR:", result.stderr)
            return False
            
    except subprocess.TimeoutExpired:
        print("✗ Build timed out after 5 minutes")
        return False
    except FileNotFoundError:
        print("✗ Cargo not found. Make sure Rust and Cargo are installed.")
        return False
    except Exception as e:
        print(f"✗ Unexpected error during build: {e}")
        return False


if __name__ == "__main__":
    success = test_build_embucket_server()
    if success:
        print("\n✅ Build test completed successfully!")
    else:
        print("\n❌ Build test failed!")
        exit(1)
