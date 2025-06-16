#!/usr/bin/env python3
"""
Iterative Improvement Agent for Embucket Fuzzing

This agent runs fuzzing sessions in a loop, analyzing results and improving
the query generator when no bugs are found, until either a crash is detected
or 10 iterations are completed.
"""

import os
import time
import json
from dotenv import load_dotenv

from agents import Agent, Runner

# Load environment variables from .env file
load_dotenv()

# Import the comprehensive fuzzing tool
from tools.comprehensive_fuzzing_tool import run_comprehensive_fuzzing_session

# Import improvement tools
from tools.query_generator_improvement_tool import analyze_and_improve_query_generator
from tools.code_modification_tool import apply_query_generator_improvements, backup_and_restore_query_generator, read_query_generator_code


class IterativeImprovementAgent(Agent):
    """AI agent that iteratively improves query generation to find bugs."""

    def __init__(self):
        # Load configuration from environment variables
        self.embucket_host = os.getenv("EMBUCKET_HOST", "localhost")
        self.embucket_port = int(os.getenv("EMBUCKET_PORT", "3000"))
        self.embucket_url = f"http://{self.embucket_host}:{self.embucket_port}"
        self.slt_output_dir = os.getenv("SLT_OUTPUT_DIR", "test/sql/fuzz_regressions")
        self.max_iterations = 10
        self.queries_per_iteration = int(os.getenv("DEFAULT_NUM_QUERIES", "5"))

        # Verify OpenAI API key is set
        openai_api_key = os.getenv("OPENAI_API_KEY")
        if not openai_api_key:
            raise ValueError(
                "OPENAI_API_KEY environment variable is required. "
                "Please set it in the .env file or as an environment variable."
            )

        super().__init__(
            name="IterativeImprovementAgent",
            instructions="""You are an iterative fuzzing improvement agent. Your goal is to find bugs by running multiple fuzzing sessions and improving the query generator when no bugs are found.

CRITICAL BEHAVIOR: You MUST continue running iterations until you either find bugs OR complete the maximum number of iterations specified. DO NOT STOP after just one iteration.

Your workflow for EACH iteration:
1. Run a fuzzing session using run_comprehensive_fuzzing_session
2. Analyze the results to check if any actual bugs were found
3. If bugs were found, report success and stop
4. If no bugs were found, improve the query generator and CONTINUE to the next iteration
5. Always state when you're continuing to the next iteration

MANDATORY: After each improvement step, you MUST:
- Provide detailed output about changes made
- Explicitly state you're continuing to the next iteration
- Actually continue without waiting for further instructions

You must be persistent and continue iterating until bugs are found or max iterations reached.""",
            tools=[
                run_comprehensive_fuzzing_session,
                analyze_and_improve_query_generator,
                apply_query_generator_improvements,
                backup_and_restore_query_generator,
                read_query_generator_code
            ]
        )

        # Ensure output directory exists
        os.makedirs(self.slt_output_dir, exist_ok=True)

    async def run_iterative_improvement(self):
        """
        Run the iterative improvement process.
        """
        # Check if verbose mode is enabled
        verbose_mode = os.getenv("FUZZING_VERBOSE", "false").lower() == "true"

        print(f"🚀 Starting iterative improvement process...")
        if verbose_mode:
            print(f"📊 Configuration:")
            print(f"   Max iterations: {self.max_iterations}")
            print(f"   Queries per iteration: {self.queries_per_iteration}")
            print(f"   Embucket: {self.embucket_url}")
            print(f"   Output directory: {self.slt_output_dir}")
            print()

        # Create initial backup of query generator directly
        if verbose_mode:
            print("📁 Creating backup of query generator...")
        try:
            import time
            query_generator_path = "tools/random_query_generator_tool.py"
            backup_path = f"{query_generator_path}.backup.initial.{int(time.time())}"

            with open(query_generator_path, 'r') as source:
                content = source.read()
            with open(backup_path, 'w') as backup:
                backup.write(content)

            if verbose_mode:
                print(f"✅ Initial backup created: {backup_path}")
        except Exception as e:
            if verbose_mode:
                print(f"⚠️ Failed to create backup: {e}")
                print("Continuing without backup...")

        # Main iterative improvement instruction
        improvement_instruction = f"""
        You are running an iterative improvement process to find bugs in Embucket. You MUST complete ALL {self.max_iterations} iterations unless bugs are found.

        **CRITICAL**: After completing each iteration, you MUST explicitly state "This concludes Iteration X. Continuing to the next iteration to further explore for potential bugs." and then IMMEDIATELY start the next iteration. DO NOT STOP until you have completed all {self.max_iterations} iterations or found actual bugs.

        **ITERATION WORKFLOW** (repeat for iterations 1 through {self.max_iterations}):

        **Start each iteration by stating**: "=== STARTING ITERATION X of {self.max_iterations} ==="

        1. **Run Fuzzing Session**:
           Use run_comprehensive_fuzzing_session with:
           - num_queries: {self.queries_per_iteration}
           - complexity: "complex"
           - host: "{self.embucket_host}"
           - port: {self.embucket_port}
           - output_dir: "{self.slt_output_dir}"

        2. **Analyze Results**:
           Parse the JSON results and check: actual_bugs > 0 OR crashes_found > 0

        3. **If bugs found**:
           - Report "🚨 BUGS FOUND! Stopping iterative process."
           - Provide detailed analysis of the bugs found
           - STOP the process immediately

        4. **If no bugs found**:
           - State "No bugs found in this iteration. Proceeding with improvements."
           - Use read_query_generator_code to get current code
           - Use analyze_and_improve_query_generator to get improvement suggestions
           - Use apply_query_generator_improvements to apply the suggestions
           - **Report detailed changes made**:
             * Parse the apply_query_generator_improvements result JSON
             * Display the "analysis" field explaining why changes were needed
             * List each modification from "details" field with descriptions
             * Show the "expected_impact" of the changes
           - State "This concludes Iteration X. Continuing to the next iteration to further explore for potential bugs."
           - **IMMEDIATELY continue to the next iteration** (do not wait for further instructions)

        **MANDATORY CONTINUATION**: Unless bugs are found, you MUST continue to the next iteration immediately. Do not stop after any single iteration.

        **FINAL REPORT** (only after completing all {self.max_iterations} iterations or finding bugs):
        - Total iterations completed
        - Whether bugs were found
        - Detailed summary of all improvements made across all iterations
        - Success rate progression
        - Recommendations for next steps
        """

        # Run the iterative improvement process
        result = await Runner.run(self, improvement_instruction, max_turns=100)

        print(f"\n🏁 Iterative improvement process completed!")
        print(f"Final result: {result.final_output}")

        return result

    async def run_iterative_improvement_with_python_loop(self):
        """
        Alternative implementation using Python loop for more reliable iteration control.
        """
        # Check if verbose mode is enabled
        verbose_mode = os.getenv("FUZZING_VERBOSE", "false").lower() == "true"

        print(f"🚀 Starting iterative improvement process (Python loop version)...")
        if verbose_mode:
            print(f"📊 Configuration:")
            print(f"   Max iterations: {self.max_iterations}")
            print(f"   Queries per iteration: {self.queries_per_iteration}")
            print(f"   Embucket: {self.embucket_url}")
            print(f"   Output directory: {self.slt_output_dir}")
            print()

        # Create initial backup
        if verbose_mode:
            print("📁 Creating backup of query generator...")
        try:
            query_generator_path = "tools/random_query_generator_tool.py"
            backup_path = f"{query_generator_path}.backup.initial.{int(time.time())}"

            with open(query_generator_path, 'r') as source:
                content = source.read()
            with open(backup_path, 'w') as backup:
                backup.write(content)

            if verbose_mode:
                print(f"✅ Initial backup created: {backup_path}")
        except Exception as e:
            if verbose_mode:
                print(f"⚠️ Failed to create backup: {e}")
                print("Continuing without backup...")

        bugs_found = False
        total_improvements = []

        for iteration in range(1, self.max_iterations + 1):
            print(f"\n{'='*60}")
            print(f"🔄 ITERATION {iteration} of {self.max_iterations}")
            print(f"{'='*60}")

            # Single iteration instruction
            iteration_instruction = f"""
            Run a single iteration of the fuzzing improvement process:

            1. **Run Fuzzing Session**:
               Use run_comprehensive_fuzzing_session with:
               - num_queries: {self.queries_per_iteration}
               - complexity: "complex"
               - host: "{self.embucket_host}"
               - port: {self.embucket_port}
               - output_dir: "{self.slt_output_dir}"

            2. **Analyze Results**:
               Parse the JSON results and check if actual_bugs > 0 OR crashes_found > 0

            3. **If bugs found**:
               - Report "🚨 BUGS FOUND! Iteration {iteration} successful."
               - Provide detailed analysis of the bugs found
               - Return "BUGS_FOUND" to indicate success

            4. **If no bugs found**:
               - Report "No bugs found in iteration {iteration}. Applying improvements."
               - Use read_query_generator_code to get current code
               - Use analyze_and_improve_query_generator with iteration_number: {iteration}
               - **CRITICAL**: Immediately after calling analyze_and_improve_query_generator, you MUST say:
                 "ANALYSIS_RESULT: [paste the complete JSON result here]"
               - Use apply_query_generator_improvements to apply suggestions
               - **CRITICAL**: Immediately after calling apply_query_generator_improvements, you MUST say:
                 "APPLY_RESULT: [paste the complete JSON result here]"
               - **CRITICAL**: You MUST then summarize the changes in this exact format:

                 ITERATION_SUMMARY_START
                 Iteration {iteration} Changes:
                 - Analysis: [brief summary of what was analyzed]
                 - Changes Made: [list the specific modifications that were applied]
                 - Reasoning: [why these changes were needed]
                 - Expected Impact: [what these changes should achieve]
                 ITERATION_SUMMARY_END

               - Return "IMPROVEMENTS_APPLIED" to continue

            Be concise but thorough in your analysis.
            """

            # Run single iteration
            result = await Runner.run(self, iteration_instruction, max_turns=10)

            # Check if debug mode is enabled
            debug_mode = os.getenv("FUZZING_DEBUG", "false").lower() == "true"

            if debug_mode:
                print(f"\n📋 DEBUG - Full Iteration {iteration} Result:")
                print("="*80)
                print(result.final_output)
                print("="*80)
            else:
                # Always show a snippet of the result for debugging (first 500 chars)
                print(f"\n📋 Iteration {iteration} Result Preview:")
                preview = result.final_output[:500] + "..." if len(result.final_output) > 500 else result.final_output
                print(f"   {preview}")

            if verbose_mode and not debug_mode:
                print(f"\n📋 Full Iteration {iteration} Result:")
                print(f"   {result.final_output}")

            # Extract and display detailed changes if they were applied
            self._extract_and_display_changes(result.final_output, iteration, verbose_mode or debug_mode)

            # Check if bugs were found
            if "BUGS_FOUND" in result.final_output or "🚨" in result.final_output:
                print(f"\n🎉 SUCCESS! Bugs found in iteration {iteration}")
                bugs_found = True
                break
            elif "IMPROVEMENTS_APPLIED" in result.final_output:
                print(f"✅ Iteration {iteration} completed - improvements applied")
                total_improvements.append(f"Iteration {iteration}: {result.final_output[:100]}...")
            else:
                if verbose_mode:
                    print(f"⚠️ Iteration {iteration} completed with unclear result")
                total_improvements.append(f"Iteration {iteration}: {result.final_output[:100]}...")

            # Small delay between iterations
            time.sleep(1)

        # Final report
        print(f"\n{'='*60}")
        print(f"🏁 FINAL REPORT")
        print(f"{'='*60}")
        print(f"Total iterations completed: {iteration}")
        print(f"Bugs found: {'Yes' if bugs_found else 'No'}")
        print(f"Total improvements applied: {len(total_improvements)}")

        if total_improvements:
            print(f"\nImprovement summary:")
            for improvement in total_improvements:
                print(f"  - {improvement}")

        return {"bugs_found": bugs_found, "iterations_completed": iteration, "improvements": total_improvements}

    def _extract_and_display_changes(self, agent_output: str, iteration: int, verbose_mode: bool = False):
        """
        Extract and display detailed changes from agent output.
        """
        try:
            # Look for the new iteration summary format first
            import re
            summary_pattern = r'ITERATION_SUMMARY_START\s*(.*?)\s*ITERATION_SUMMARY_END'
            summary_match = re.search(summary_pattern, agent_output, re.DOTALL)

            if summary_match:
                summary_content = summary_match.group(1).strip()
                print(f"\n🔧 ITERATION {iteration} SUMMARY:")
                for line in summary_content.split('\n'):
                    line = line.strip()
                    if line:
                        print(f"   {line}")
                return

            # Look for the old structured changes output
            pattern = r'CHANGES_APPLIED_START\s*(\{.*?\})\s*CHANGES_APPLIED_END'
            match = re.search(pattern, agent_output, re.DOTALL)

            if match:
                try:
                    changes_json = json.loads(match.group(1))
                    # Always show iteration changes (this is important for monitoring progress)
                    print(f"\n🔧 CHANGES FOR ITERATION {iteration}:")
                    print(f"📝 Changes: {changes_json.get('changes_made', 'Not specified')}")
                    print(f"🤔 Reasoning: {changes_json.get('reasoning', 'Not specified')}")
                    print(f"🎯 Expected Impact: {changes_json.get('expected_impact', 'Not specified')}")
                    return
                except json.JSONDecodeError:
                    if verbose_mode:
                        print(f"\n⚠️ Could not parse changes JSON for iteration {iteration}")

            # Look for explicit tool results
            analysis_pattern = r'ANALYSIS_RESULT:\s*(.*?)(?=APPLY_RESULT:|$)'
            apply_pattern = r'APPLY_RESULT:\s*(.*?)(?=ITERATION_SUMMARY_START|$)'

            analysis_match = re.search(analysis_pattern, agent_output, re.DOTALL)
            apply_match = re.search(apply_pattern, agent_output, re.DOTALL)

            if analysis_match or apply_match:
                print(f"\n🔧 TOOL RESULTS FOR ITERATION {iteration}:")

                if analysis_match:
                    analysis_text = analysis_match.group(1).strip()
                    print(f"📊 Analysis Result:")
                    print(f"   {analysis_text[:200]}..." if len(analysis_text) > 200 else f"   {analysis_text}")

                if apply_match:
                    apply_text = apply_match.group(1).strip()
                    print(f"🔧 Apply Result:")
                    print(f"   {apply_text[:200]}..." if len(apply_text) > 200 else f"   {apply_text}")
                return

            # Fallback 1: Look for JSON blocks in the output that might contain tool results
            # Use a more robust approach to find JSON blocks
            json_blocks = self._extract_json_blocks(agent_output)

            if verbose_mode:
                print(f"\n🔍 DEBUG: Found {len(json_blocks)} JSON blocks in agent output")
                for i, block in enumerate(json_blocks):
                    print(f"   Block {i+1}: {block[:100]}...")

            for json_block in json_blocks:
                try:
                    tool_result = json.loads(json_block)

                    # Check for apply_query_generator_improvements result
                    if tool_result.get("modifications_applied", 0) > 0:
                        # Always show iteration changes (this is important for monitoring progress)
                        print(f"\n🔧 CHANGES APPLIED IN ITERATION {iteration}:")
                        print(f"📊 Modifications Applied: {tool_result.get('modifications_applied', 0)}")
                        print(f"📝 Analysis: {tool_result.get('analysis', 'Not provided')}")
                        print(f"🎯 Expected Impact: {tool_result.get('expected_impact', 'Not provided')}")

                        # Show key changes in compact format
                        details = tool_result.get('details', [])
                        if details:
                            print(f"🔍 Key Changes:")
                            for detail in details[:3]:  # Show first 3 changes in quiet mode
                                if isinstance(detail, str) and not detail.startswith("DEBUG:"):
                                    print(f"   • {detail}")
                            if len(details) > 3 and not verbose_mode:
                                print(f"   • ... and {len(details) - 3} more changes")
                            elif verbose_mode and len(details) > 3:
                                for detail in details[3:]:
                                    if isinstance(detail, str) and not detail.startswith("DEBUG:"):
                                        print(f"   • {detail}")
                        return

                    # Check for analyze_and_improve_query_generator result
                    elif "suggested_modifications" in tool_result and tool_result.get("suggested_modifications"):
                        # Always show iteration analysis (this is important for monitoring progress)
                        print(f"\n🔧 IMPROVEMENT ANALYSIS FOR ITERATION {iteration}:")
                        print(f"📝 Analysis: {tool_result.get('analysis', 'Not provided')}")
                        print(f"🎯 Expected Impact: {tool_result.get('expected_impact', 'Not provided')}")

                        # Show suggested modifications
                        modifications = tool_result.get('suggested_modifications', [])
                        if modifications:
                            print(f"🔍 Suggested Changes:")
                            for mod in modifications[:3]:  # Show first 3 in quiet mode
                                if isinstance(mod, dict):
                                    desc = mod.get('description', 'No description')
                                    print(f"   • {desc}")
                            if len(modifications) > 3 and not verbose_mode:
                                print(f"   • ... and {len(modifications) - 3} more suggestions")
                            elif verbose_mode and len(modifications) > 3:
                                for mod in modifications[3:]:
                                    if isinstance(mod, dict):
                                        desc = mod.get('description', 'No description')
                                        print(f"   • {desc}")
                        return

                except json.JSONDecodeError:
                    continue

            # Fallback 1.5: Look for tool function results in a different format
            if self._extract_tool_results_from_text(agent_output, iteration, verbose_mode):
                return  # Found and displayed tool results

            # Fallback 2: Look for any mentions of changes in the output
            if any(keyword in agent_output.lower() for keyword in ['modification', 'change', 'applied', 'improvement']):
                # Always show some change information (this is important for monitoring progress)
                print(f"\n🔧 CHANGES DETECTED FOR ITERATION {iteration}:")
                # Extract relevant lines that mention changes
                lines = agent_output.split('\n')
                relevant_lines = []
                for line in lines:
                    if any(keyword in line.lower() for keyword in ['change', 'modif', 'improv', 'applied', 'success']):
                        clean_line = line.strip()
                        if clean_line and not clean_line.startswith('DEBUG:'):
                            relevant_lines.append(clean_line)

                if relevant_lines:
                    # Show first 3 lines in quiet mode, more in verbose mode
                    lines_to_show = relevant_lines[:10] if verbose_mode else relevant_lines[:3]
                    for line in lines_to_show:
                        print(f"   {line}")
                    if len(relevant_lines) > 3 and not verbose_mode:
                        print(f"   ... and {len(relevant_lines) - 3} more changes")
                else:
                    print("   No specific change details found in output")
            else:
                # Always indicate when no changes were detected
                print(f"\n📋 No changes detected for iteration {iteration}")

        except Exception as e:
            # Always show errors in change extraction (this could indicate issues)
            print(f"\n⚠️ Error extracting changes for iteration {iteration}: {e}")

    def _extract_json_blocks(self, text: str) -> list:
        """
        Extract JSON blocks from text more robustly than regex.
        """
        json_blocks = []
        lines = text.split('\n')

        i = 0
        while i < len(lines):
            line = lines[i].strip()
            if line.startswith('{'):
                # Found potential start of JSON block
                json_lines = [line]
                brace_count = line.count('{') - line.count('}')
                i += 1

                # Continue collecting lines until braces are balanced
                while i < len(lines) and brace_count > 0:
                    line = lines[i].strip()
                    if line:  # Skip empty lines
                        json_lines.append(line)
                        brace_count += line.count('{') - line.count('}')
                    i += 1

                # Try to parse the collected JSON
                json_text = '\n'.join(json_lines)
                try:
                    json.loads(json_text)  # Validate it's valid JSON
                    if 'modifications_applied' in json_text or 'suggested_modifications' in json_text:
                        json_blocks.append(json_text)
                except json.JSONDecodeError:
                    pass  # Not valid JSON, skip
            else:
                i += 1

        return json_blocks

    def _extract_tool_results_from_text(self, text: str, iteration: int, verbose_mode: bool = False):
        """
        Extract tool results from text by looking for specific patterns.
        """
        lines = text.split('\n')

        # Look for patterns that indicate tool results
        analysis_found = False
        modifications_found = False

        # First, do a comprehensive search for any tool-related keywords
        tool_keywords = [
            'analyze_and_improve_query_generator',
            'apply_query_generator_improvements',
            'read_query_generator_code',
            'modifications_applied',
            'suggested_modifications',
            'success_rate',
            'error_rate',
            'expected_impact',
            'analysis',
            'improvement',
            'modification'
        ]

        found_keywords = []
        for keyword in tool_keywords:
            if keyword in text.lower():
                found_keywords.append(keyword)

        if found_keywords and verbose_mode:
            print(f"\n🔍 DEBUG: Found tool keywords in iteration {iteration}: {found_keywords}")

        # If we found tool keywords but no structured output, show raw relevant sections
        if found_keywords and not analysis_found and not modifications_found:
            print(f"\n🔧 TOOL ACTIVITY DETECTED FOR ITERATION {iteration}:")
            relevant_lines = []
            for line in lines:
                line_clean = line.strip()
                if line_clean and any(keyword in line_clean.lower() for keyword in found_keywords):
                    relevant_lines.append(line_clean)

            if relevant_lines:
                for line in relevant_lines[:10]:  # Show first 10 relevant lines
                    print(f"   {line}")
                if len(relevant_lines) > 10:
                    print(f"   ... and {len(relevant_lines) - 10} more tool-related lines")
                return True

        for i, line in enumerate(lines):
            line_lower = line.lower()

            # Look for tool function calls and results
            if 'analyze_and_improve_query_generator' in line_lower:
                analysis_found = True
                print(f"\n🔧 ANALYSIS TOOL CALLED FOR ITERATION {iteration}:")
                # Extract surrounding context
                start = max(0, i-1)
                end = min(len(lines), i+15)
                for j in range(start, end):
                    context_line = lines[j].strip()
                    if context_line and not context_line.startswith('DEBUG:'):
                        print(f"   {context_line}")
                        if 'expected_impact' in context_line.lower():
                            break
                break

            elif 'apply_query_generator_improvements' in line_lower:
                modifications_found = True
                print(f"\n🔧 MODIFICATION TOOL CALLED FOR ITERATION {iteration}:")
                # Extract surrounding context
                start = max(0, i-1)
                end = min(len(lines), i+15)
                for j in range(start, end):
                    context_line = lines[j].strip()
                    if context_line and not context_line.startswith('DEBUG:'):
                        print(f"   {context_line}")
                        if 'summary' in context_line.lower() or j > i + 10:
                            break
                break

            # Look for analysis results by content
            elif 'analysis' in line_lower and ('success_rate' in line_lower or 'error_rate' in line_lower):
                analysis_found = True
                print(f"\n🔧 ANALYSIS DETECTED FOR ITERATION {iteration}:")
                # Extract surrounding context
                start = max(0, i-2)
                end = min(len(lines), i+10)
                for j in range(start, end):
                    context_line = lines[j].strip()
                    if context_line and not context_line.startswith('DEBUG:'):
                        print(f"   {context_line}")
                break

            # Look for modification results
            elif 'modifications_applied' in line_lower or ('applied' in line_lower and 'modification' in line_lower):
                modifications_found = True
                print(f"\n🔧 MODIFICATIONS DETECTED FOR ITERATION {iteration}:")
                # Extract surrounding context
                start = max(0, i-2)
                end = min(len(lines), i+8)
                for j in range(start, end):
                    context_line = lines[j].strip()
                    if context_line and not context_line.startswith('DEBUG:'):
                        print(f"   {context_line}")
                break

        # If we found something, return True to indicate we handled it
        return analysis_found or modifications_found


async def main():
    """Main entry point for the iterative improvement agent."""
    try:
        # Ensure we're running from the correct directory
        script_dir = os.path.dirname(os.path.abspath(__file__))
        expected_dir = os.path.join(script_dir)  # Should be test/crash_fuzzing_agent

        print(f"Script directory: {script_dir}")
        print(f"Current working directory: {os.getcwd()}")

        # Change to the script directory if we're not already there
        if os.getcwd() != script_dir:
            print(f"Changing working directory to: {script_dir}")
            os.chdir(script_dir)
            print(f"New working directory: {os.getcwd()}")

        agent = IterativeImprovementAgent()
        
        print("=" * 80)
        print("EMBUCKET ITERATIVE IMPROVEMENT FUZZING AGENT")
        print("=" * 80)
        print()
        print("This agent will:")
        print("1. Run fuzzing sessions to test Embucket")
        print("2. Analyze results and improve query generation when no bugs found")
        print("3. Loop until bugs are found or 10 iterations completed")
        print()

        # Check if user wants to use the Python loop version (more reliable)
        use_python_loop = os.getenv("USE_PYTHON_LOOP", "true").lower() == "true"

        if use_python_loop:
            print("🔧 Using Python loop version for more reliable iteration control...")
            await agent.run_iterative_improvement_with_python_loop()
        else:
            print("🔧 Using agent-based iteration control...")
            await agent.run_iterative_improvement()

    except ValueError as e:
        print(f"Configuration Error: {e}")
        print("\nPlease check your .env file and ensure OPENAI_API_KEY is set.")
        return 1
    except Exception as e:
        print(f"Error: {e}")
        return 1

    return 0


if __name__ == "__main__":
    import asyncio
    asyncio.run(main())
