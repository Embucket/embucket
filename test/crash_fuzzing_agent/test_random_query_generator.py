#!/usr/bin/env python3
"""
Test script for the SQLGlot-based random query generator tool.
"""

import sys
import os
sys.path.append(os.path.dirname(__file__))

from tools.random_query_generator_tool import RandomQueryGenerator


def test_random_query_generator():
    """Test the RandomQueryGenerator directly."""
    print("Testing SQLGlot-based SQL query generator...")
    print("=" * 60)

    generator = RandomQueryGenerator()
    
    # Test simple queries
    print("\n1. SIMPLE QUERIES:")
    print("-" * 30)
    for i in range(3):
        try:
            query = generator.generate_simple_query()
            sql = query.sql(dialect="snowflake", pretty=True)
            print(f"Simple Query {i+1}:")
            print(sql)
            print()
        except Exception as e:
            print(f"Error generating simple query {i+1}: {e}")
    
    # Test medium queries
    print("\n2. MEDIUM QUERIES:")
    print("-" * 30)
    for i in range(3):
        try:
            query = generator.generate_medium_query()
            sql = query.sql(dialect="snowflake", pretty=True)
            print(f"Medium Query {i+1}:")
            print(sql)
            print()
        except Exception as e:
            print(f"Error generating medium query {i+1}: {e}")
    
    # Test complex queries
    print("\n3. COMPLEX QUERIES:")
    print("-" * 30)
    for i in range(2):
        try:
            query = generator.generate_complex_query()
            sql = query.sql(dialect="snowflake", pretty=True)
            print(f"Complex Query {i+1}:")
            print(sql)
            print()
        except Exception as e:
            print(f"Error generating complex query {i+1}: {e}")
    
    print("✓ SQLGlot-based SQL generator test completed!")


if __name__ == "__main__":
    test_random_query_generator()
