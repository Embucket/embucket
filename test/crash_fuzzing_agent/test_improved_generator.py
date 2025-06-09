#!/usr/bin/env python3
"""
Test the improved query generator to verify better success/error distribution.
"""

import sys
import os
sys.path.append(os.path.dirname(os.path.abspath(__file__)))

from tools.random_query_generator_tool import random_query_generator
from analyze_query_generation import analyze_query_validity
from collections import defaultdict

def test_improved_generator():
    print("🔧 TESTING IMPROVED QUERY GENERATOR")
    print("=" * 60)
    
    # Test different safety levels
    safety_levels = [
        (0.1, "Very Fuzzy (10% safe)"),
        (0.4, "Balanced (40% safe)"),
        (0.7, "Mostly Safe (70% safe)"),
        (0.9, "Very Safe (90% safe)")
    ]
    
    for safe_prob, description in safety_levels:
        print(f"\n📊 Testing {description}")
        print("-" * 40)
        
        results = defaultdict(int)
        sample_queries = defaultdict(list)
        
        # Generate sample queries
        for i in range(20):
            try:
                query = random_query_generator(
                    safe_query_probability=safe_prob
                )
                
                category, reason, severity = analyze_query_validity(query)
                results[severity] += 1
                
                # Store examples
                if len(sample_queries[severity]) < 2:
                    sample_queries[severity].append(query)
                    
            except Exception as e:
                results["GENERATION_ERROR"] += 1
                print(f"  Error generating query {i+1}: {e}")
        
        # Print results
        total = sum(results.values())
        for severity in ["LIKELY_SUCCESS", "MAYBE_SUCCESS", "EXPECTED_ERROR", "UNKNOWN", "GENERATION_ERROR"]:
            count = results[severity]
            if count > 0:
                percentage = (count / total) * 100
                print(f"  {severity}: {count}/{total} ({percentage:.1f}%)")
        
        # Show sample queries
        print(f"\n  Sample queries:")
        for severity, queries in sample_queries.items():
            if queries:
                print(f"    {severity}: {queries[0][:80]}...")
    
    print(f"\n🎯 COMPARISON WITH ORIGINAL")
    print("=" * 60)
    print("Original generator (from previous analysis):")
    print("  - Simple: 35% likely success, 5% expected errors")
    print("  - Medium: 25% maybe success, 55% expected errors")
    print("  - Complex: 30% maybe success, 50% expected errors")
    print()
    print("Improved generator should show:")
    print("  - Higher success rates with higher safe_query_probability")
    print("  - Better type awareness and fewer obvious errors")
    print("  - Configurable balance between safety and fuzzing")

if __name__ == "__main__":
    test_improved_generator()
