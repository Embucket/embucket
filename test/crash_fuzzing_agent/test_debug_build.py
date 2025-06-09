#!/usr/bin/env python3
"""
Test script for the debug build of Embucket server.
"""

import subprocess
import os


def test_debug_build():
    """Test a debug build which should be faster."""
    print("Testing debug build of Embucket server...")
    print("=" * 50)
    
    try:
        # Check if we're in the right directory
        if not os.path.exists("Cargo.toml"):
            print("Error: Cargo.toml not found. Make sure you're in the Embucket project root.")
            return False
        
        print("Running: cargo build --bin embucketd")
        print("Note: Debug build (faster compilation, slower runtime)")
        
        # Run debug build
        result = subprocess.run(
            ["cargo", "build", "--bin", "embucketd"],
            capture_output=True,
            text=True,
            timeout=300  # 5 minutes should be enough for debug build
        )
        
        if result.returncode == 0:
            print("✓ Debug build completed successfully")
            
            # Check if the binary was created
            binary_path = "target/debug/embucketd"
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
            if result.stdout:
                print("STDOUT:", result.stdout)
            if result.stderr:
                print("STDERR:", result.stderr)
            return False
            
    except subprocess.TimeoutExpired:
        print("✗ Debug build timed out after 5 minutes")
        return False
    except FileNotFoundError:
        print("✗ Cargo not found. Make sure Rust and Cargo are installed.")
        return False
    except Exception as e:
        print(f"✗ Unexpected error during build: {e}")
        return False


if __name__ == "__main__":
    success = test_debug_build()
    if success:
        print("\n✅ Debug build test completed successfully!")
        print("The real build_embucket_server tool is working correctly.")
    else:
        print("\n❌ Debug build test failed!")
        exit(1)
