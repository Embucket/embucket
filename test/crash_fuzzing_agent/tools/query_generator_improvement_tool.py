"""
Query Generator Improvement Tool for analyzing fuzzing results and suggesting improvements.
"""

import json
import os
from typing import Dict, List, Any
from agents import function_tool
from dotenv import load_dotenv

# Load environment variables
load_dotenv()

@function_tool
def analyze_and_improve_query_generator(
    fuzzing_results: str,
    current_generator_code: str,
    iteration_number: int
) -> str:
    """
    Analyze fuzzing session results and suggest improvements to the query generator.
    
    This tool uses an LLM to analyze the current query generation patterns and suggest
    modifications to make queries more complex and likely to find bugs.
    
    Args:
        fuzzing_results: JSON string containing the fuzzing session results
        current_generator_code: Current source code of the query generator
        iteration_number: Current iteration number (1-10)
    
    Returns:
        JSON string containing analysis and suggested code improvements
    """
    try:
        # Parse the fuzzing results
        results = json.loads(fuzzing_results)
        
        # Extract key metrics
        execution_summary = results.get("execution_summary", {})
        queries_executed = execution_summary.get("queries_executed", 0)
        successful_queries = execution_summary.get("successful_queries", 0)
        expected_errors = execution_summary.get("expected_errors", 0)
        actual_bugs = execution_summary.get("actual_bugs", 0)
        
        # Calculate success/error rates
        success_rate = (successful_queries / queries_executed * 100) if queries_executed > 0 else 0
        error_rate = (expected_errors / queries_executed * 100) if queries_executed > 0 else 0
        bug_rate = (actual_bugs / queries_executed * 100) if queries_executed > 0 else 0
        
        # Extract sample queries from detailed logs
        sample_queries = []
        detailed_logs = results.get("detailed_logs", [])
        for log in detailed_logs:
            if "query" in log and log["query"]:
                sample_queries.append({
                    "query": log["query"],
                    "success": log.get("success", False),
                    "error_type": log.get("error_type", "unknown")
                })
        
        # Prepare analysis prompt for LLM
        analysis_prompt = f"""
You are an expert SQL fuzzing engineer. Analyze the following fuzzing session results and suggest improvements to make the query generator more aggressive and likely to find bugs.

ITERATION: {iteration_number}/10

CURRENT RESULTS:
- Queries executed: {queries_executed}
- Success rate: {success_rate:.1f}%
- Expected error rate: {error_rate:.1f}%
- Bug detection rate: {bug_rate:.1f}%
- Actual bugs found: {actual_bugs}

SAMPLE QUERIES GENERATED:
{json.dumps(sample_queries[:5], indent=2)}

CURRENT QUERY GENERATOR CODE:
```python
{current_generator_code}
```

ANALYSIS REQUIREMENTS:
1. The current success rate is {success_rate:.1f}%. For aggressive fuzzing, we want 20-40% success rate.
2. We found {actual_bugs} bugs. If this is 0, we need more aggressive queries.
3. Focus on making queries more complex and edge-case prone.

IMPROVEMENT SUGGESTIONS:
Please provide specific code modifications to:
1. Add more complex SQL constructs (window functions, recursive CTEs, complex joins)
2. Increase probability of edge cases (NULL handling, type mismatches, overflow conditions)
3. Add more aggressive fuzzing patterns
4. Reduce safe_query_probability if success rate is too high
5. Add new SQL features that might trigger bugs

Respond with a JSON object containing:
{{
    "analysis": "Your analysis of current patterns and why they're not finding bugs",
    "success_rate_assessment": "Assessment of whether success rate is appropriate",
    "suggested_modifications": [
        {{
            "location": "method_name or line_range",
            "change_type": "add|modify|replace",
            "description": "What to change and why",
            "code_snippet": "The actual code to add/modify"
        }}
    ],
    "expected_impact": "What you expect these changes to achieve"
}}
"""

        # For now, return a structured response that simulates LLM analysis
        # In a real implementation, this would call an LLM API

        # Generate detailed analysis based on current metrics
        detailed_analysis = _generate_detailed_analysis(success_rate, error_rate, bug_rate, actual_bugs, queries_executed, iteration_number)

        analysis_result = {
            "iteration": iteration_number,
            "current_metrics": {
                "success_rate": success_rate,
                "error_rate": error_rate,
                "bug_rate": bug_rate,
                "queries_executed": queries_executed,
                "actual_bugs": actual_bugs
            },
            "analysis": detailed_analysis,
            "success_rate_assessment": "too_high" if success_rate > 50 else "appropriate" if success_rate > 20 else "too_low",
            "suggested_modifications": _generate_improvement_suggestions(success_rate, error_rate, bug_rate, iteration_number),
            "expected_impact": f"These changes should reduce success rate to 20-40% and increase bug detection probability by making queries more complex and edge-case prone."
        }
        
        return json.dumps(analysis_result, indent=2)
        
    except Exception as e:
        error_result = {
            "error": f"Failed to analyze fuzzing results: {str(e)}",
            "analysis": "Error occurred during analysis",
            "suggested_modifications": [],
            "expected_impact": "No improvements can be suggested due to analysis error"
        }
        return json.dumps(error_result, indent=2)


def _generate_detailed_analysis(success_rate: float, error_rate: float, bug_rate: float, actual_bugs: int, queries_executed: int, iteration: int) -> str:
    """Generate detailed analysis of current fuzzing results."""
    analysis_parts = []

    # Iteration context
    analysis_parts.append(f"ITERATION {iteration}/10 ANALYSIS:")

    # Success rate analysis
    if success_rate > 70:
        analysis_parts.append(f"• SUCCESS RATE ({success_rate:.1f}%) is TOO HIGH - queries are not aggressive enough")
        analysis_parts.append("  → Need to reduce safe_query_probability and add more complex constructs")
    elif success_rate > 50:
        analysis_parts.append(f"• SUCCESS RATE ({success_rate:.1f}%) is MODERATELY HIGH - some room for more aggressive queries")
        analysis_parts.append("  → Should add edge cases and complex SQL features")
    elif success_rate > 20:
        analysis_parts.append(f"• SUCCESS RATE ({success_rate:.1f}%) is in TARGET RANGE - good balance for fuzzing")
        analysis_parts.append("  → Focus on adding specific bug-triggering patterns")
    else:
        analysis_parts.append(f"• SUCCESS RATE ({success_rate:.1f}%) is LOW - queries may be too aggressive or malformed")
        analysis_parts.append("  → Need to balance complexity with validity")

    # Bug detection analysis
    if actual_bugs == 0:
        analysis_parts.append(f"• BUG DETECTION: No bugs found in {queries_executed} queries")
        analysis_parts.append("  → Need more aggressive patterns: window functions, CTEs, edge cases")
    else:
        analysis_parts.append(f"• BUG DETECTION: Found {actual_bugs} bugs - SUCCESS!")
        analysis_parts.append("  → Current approach is working, continue with similar patterns")

    # Iteration-specific recommendations
    if iteration <= 2:
        analysis_parts.append("• EARLY ITERATION: Focus on reducing success rate and adding basic complexity")
    elif iteration <= 5:
        analysis_parts.append("• MID ITERATION: Add advanced SQL features and edge cases")
    else:
        analysis_parts.append("• LATE ITERATION: Use most aggressive patterns and complex constructs")

    return "\n".join(analysis_parts)


def _generate_improvement_suggestions(success_rate: float, error_rate: float, bug_rate: float, iteration: int) -> List[Dict[str, Any]]:
    """Generate specific improvement suggestions based on current metrics."""
    suggestions = []

    # Always try to make queries more aggressive if no bugs found (regardless of success rate)
    if bug_rate == 0.0:  # No bugs found - need more aggressive queries
        if success_rate > 30:  # Lowered threshold from 50 to 30
            suggestions.append({
                "location": "__init__ method",
                "change_type": "modify",
                "description": "Reduce safe_query_probability to make queries more aggressive",
                "code_snippet": f"self.safe_query_probability = {max(0.1, 0.4 - iteration * 0.05):.2f}  # Iteration {iteration}: More aggressive"
            })
        else:
            # Even if success rate is low, still try to make queries more complex
            suggestions.append({
                "location": "__init__ method",
                "change_type": "modify",
                "description": "Slightly reduce safe_query_probability for more edge cases",
                "code_snippet": f"self.safe_query_probability = {max(0.05, 0.3 - iteration * 0.03):.2f}  # Iteration {iteration}: More edge cases"
            })
    
    # Add more complex SQL constructs based on iteration
    if iteration >= 2:
        suggestions.append({
            "location": "_generate_aggressive_complex_query method",
            "change_type": "add",
            "description": "Add window functions for more complex queries",
            "code_snippet": """
        # Add window functions for complexity
        if random.choice([True, False]):
            window_func = random.choice(["ROW_NUMBER", "RANK", "DENSE_RANK", "LAG", "LEAD"])
            if window_func in ["LAG", "LEAD"]:
                col = func(window_func, self._random_column(table1_alias, table1_name))
            else:
                col = func(window_func)
            # Add OVER clause
            over_clause = f"OVER (ORDER BY {self._random_column(table1_alias, table1_name).sql(dialect='snowflake')})"
            columns.append(exp.Anonymous(this=f"{window_func}() {over_clause}"))
            """
        })
    
    if iteration >= 3:
        suggestions.append({
            "location": "_random_condition method",
            "change_type": "add",
            "description": "Add more edge case conditions (division by zero, overflow)",
            "code_snippet": """
        elif condition_type == "division_by_zero":
            col = self._random_column(table_alias, table_name, column_type="integer")
            # Create potential division by zero
            divisor = random.choice([exp.Literal.number(0), exp.Literal.number(random.randint(-1, 1))])
            return exp.EQ(this=exp.Div(this=col, expression=divisor), expression=exp.Literal.number(1))
            """
        })
    
    if iteration >= 4:
        suggestions.append({
            "location": "_random_function method",
            "change_type": "add",
            "description": "Add recursive CTE generation for maximum complexity",
            "code_snippet": """
    def _generate_recursive_cte(self):
        \"\"\"Generate a recursive CTE that might cause stack overflow or infinite loops.\"\"\"
        base_case = select(exp.Literal.number(1).as_("n"), exp.Literal.string("start").as_("path"))
        recursive_case = select(
            (column("n") + exp.Literal.number(1)).as_("n"),
            func("CONCAT", column("path"), exp.Literal.string("->next")).as_("path")
        ).from_(table("recursive_data")).where(column("n") < exp.Literal.number(1000))
        
        return base_case.union(recursive_case, distinct=False)
            """
        })
    
    if iteration >= 5:
        suggestions.append({
            "location": "generate_complex_query method",
            "change_type": "modify",
            "description": "Always use aggressive mode after iteration 5",
            "code_snippet": """
        # After iteration 5, always use aggressive complex queries
        return self._generate_aggressive_complex_query()
            """
        })

    # Ensure we always have at least one suggestion if no bugs found
    if not suggestions and bug_rate == 0.0:
        suggestions.append({
            "location": "__init__ method",
            "change_type": "modify",
            "description": f"Iteration {iteration}: Reduce safe_query_probability for more aggressive fuzzing",
            "code_snippet": f"self.safe_query_probability = {max(0.05, 0.35 - iteration * 0.03):.2f}  # Iteration {iteration}: Fallback aggressive mode"
        })

    return suggestions
