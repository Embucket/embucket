#!/usr/bin/env python3
"""
Analyze the query generation patterns to understand why most queries fail.
This script generates sample queries and categorizes them by likely success/failure.
"""

import sys
import os
sys.path.append(os.path.dirname(os.path.abspath(__file__)))

from tools.random_query_generator_tool import random_query_generator
import sqlglot
import re
from collections import defaultdict

def analyze_query_validity(query):
    """
    Analyze a query to predict if it's likely to succeed or fail.
    Returns: (category, reason, severity)
    """
    query_lower = query.lower()
    
    # Check for obvious type mismatches
    type_mismatch_patterns = [
        r"abs\s*\(\s*\w*(?:date|time|created_at|updated_at)",  # ABS on date columns
        r"round\s*\(\s*\w*(?:id|name|email|status)",  # ROUND on non-numeric
        r"length\s*\(\s*\w*(?:id|amount|quantity|age)",  # LENGTH on numeric
        r"upper\s*\(\s*\w*(?:id|amount|quantity|age)",  # UPPER on numeric
        r"dateadd\s*\([^)]*(?:null|true|false)",  # DATEADD with wrong types
        r"least\s*\([^)]*(?:true|false)",  # LEAST with booleans
        r"greatest\s*\([^)]*(?:true|false)",  # GREATEST with booleans
    ]
    
    for pattern in type_mismatch_patterns:
        if re.search(pattern, query_lower):
            return ("TYPE_MISMATCH", f"Pattern: {pattern}", "EXPECTED_ERROR")
    
    # Check for mixed types in IN clauses
    if re.search(r"in\s*\([^)]*(?:true|false)[^)]*(?:'[^']*'|\"[^\"]*\")", query_lower):
        return ("MIXED_TYPES_IN", "Boolean and string in IN clause", "EXPECTED_ERROR")
    
    if re.search(r"in\s*\([^)]*(?:'[^']*'|\"[^\"]*\")[^)]*(?:\d+)", query_lower):
        return ("MIXED_TYPES_IN", "String and number in IN clause", "EXPECTED_ERROR")
    
    # Check for NULL in arithmetic
    if re.search(r"(?:<=|>=|<|>|=)\s*null", query_lower):
        return ("NULL_COMPARISON", "Direct NULL comparison", "EXPECTED_ERROR")
    
    # Check for function calls with NULL
    if re.search(r"(?:abs|round|length|upper|lower)\s*\(\s*null", query_lower):
        return ("NULL_FUNCTION", "Function called with NULL", "EXPECTED_ERROR")
    
    # Check for potentially valid queries
    simple_patterns = [
        r"^select\s+\*\s+from\s+\w+\s*$",  # Simple SELECT *
        r"^select\s+\w+\s+from\s+\w+\s*$",  # Simple SELECT column
        r"^select\s+count\s*\(\s*\*\s*\)\s+from\s+\w+\s*$",  # Simple COUNT
    ]
    
    for pattern in simple_patterns:
        if re.search(pattern, query_lower):
            return ("SIMPLE_VALID", "Simple query pattern", "LIKELY_SUCCESS")
    
    # Check for complex but potentially valid queries
    if "join" in query_lower and not any(re.search(p, query_lower) for p in type_mismatch_patterns):
        return ("COMPLEX_VALID", "JOIN query without obvious errors", "MAYBE_SUCCESS")
    
    return ("UNKNOWN", "No specific pattern detected", "UNKNOWN")

def main():
    print("🔍 ANALYZING QUERY GENERATION PATTERNS")
    print("=" * 60)
    
    # Generate sample queries for analysis
    complexities = ["simple", "medium", "complex"]
    samples_per_complexity = 20
    
    results = defaultdict(lambda: defaultdict(int))
    query_examples = defaultdict(list)
    
    for complexity in complexities:
        print(f"\n📊 Analyzing {complexity} complexity queries...")
        
        for i in range(samples_per_complexity):
            try:
                query = random_query_generator(complexity)
                category, reason, severity = analyze_query_validity(query)
                
                results[complexity][severity] += 1
                
                # Store examples for each category
                if len(query_examples[f"{complexity}_{severity}"]) < 3:
                    query_examples[f"{complexity}_{severity}"].append({
                        "query": query,
                        "category": category,
                        "reason": reason
                    })
                    
            except Exception as e:
                results[complexity]["GENERATION_ERROR"] += 1
                print(f"  Error generating query {i+1}: {e}")
    
    # Print analysis results
    print("\n" + "=" * 60)
    print("📈 ANALYSIS RESULTS")
    print("=" * 60)
    
    total_queries = 0
    total_likely_success = 0
    total_expected_errors = 0
    
    for complexity in complexities:
        print(f"\n{complexity.upper()} COMPLEXITY:")
        print("-" * 30)
        
        complexity_total = sum(results[complexity].values())
        total_queries += complexity_total
        
        for severity in ["LIKELY_SUCCESS", "MAYBE_SUCCESS", "EXPECTED_ERROR", "UNKNOWN", "GENERATION_ERROR"]:
            count = results[complexity][severity]
            if count > 0:
                percentage = (count / complexity_total) * 100
                print(f"  {severity}: {count}/{complexity_total} ({percentage:.1f}%)")
                
                if severity in ["LIKELY_SUCCESS", "MAYBE_SUCCESS"]:
                    total_likely_success += count
                elif severity == "EXPECTED_ERROR":
                    total_expected_errors += count
    
    # Overall statistics
    print(f"\n🎯 OVERALL STATISTICS:")
    print("-" * 30)
    success_rate = (total_likely_success / total_queries) * 100
    error_rate = (total_expected_errors / total_queries) * 100
    print(f"Total queries analyzed: {total_queries}")
    print(f"Likely to succeed: {total_likely_success} ({success_rate:.1f}%)")
    print(f"Expected to fail: {total_expected_errors} ({error_rate:.1f}%)")
    
    # Show examples
    print(f"\n📝 QUERY EXAMPLES:")
    print("=" * 60)
    
    for key, examples in query_examples.items():
        if examples:
            complexity, severity = key.split("_", 1)
            print(f"\n{complexity.upper()} - {severity}:")
            print("-" * 40)
            for i, example in enumerate(examples, 1):
                print(f"{i}. Category: {example['category']}")
                print(f"   Reason: {example['reason']}")
                print(f"   Query: {example['query']}")
                print()
    
    # Recommendations
    print("🔧 RECOMMENDATIONS:")
    print("=" * 60)
    
    if error_rate > 70:
        print("❌ HIGH ERROR RATE DETECTED:")
        print("  - Query generator produces too many obviously invalid queries")
        print("  - Consider improving type awareness in query generation")
        print("  - Add basic semantic validation before query execution")
    
    if success_rate < 20:
        print("⚠️  LOW SUCCESS RATE:")
        print("  - Very few queries likely to succeed")
        print("  - Consider adding more simple, valid query patterns")
        print("  - Balance fuzzing with some known-good queries")
    
    print("\n✅ This analysis explains why you see mostly failing queries!")
    print("   The fuzzing agent is working as designed, but could be improved.")

if __name__ == "__main__":
    main()
