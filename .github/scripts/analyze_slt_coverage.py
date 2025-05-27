#!/usr/bin/env python3
import os
import sys
import json
import pandas as pd
import openai
import argparse


def main():
    parser = argparse.ArgumentParser(description='Analyze PR changes and SLT coverage')
    parser.add_argument('--pr-diff-file', required=True, help='File containing PR diff')
    parser.add_argument('--slt-results-file', required=True, help='CSV file with SLT results')
    parser.add_argument('--output-file', required=True, help='File to write analysis output')
    args = parser.parse_args()

    # Read PR diff and SLT results
    try:
        with open(args.pr_diff_file, 'r') as f:
            pr_diff = f.read()

        # Read SLT results CSV file
        slt_results = pd.read_csv(args.slt_results_file)

        # Check if API key is present
        api_key = os.environ.get('OPENAI_API_KEY')
        if not api_key:
            print("Error: OPENAI_API_KEY environment variable is not set")
            sys.exit(1)

        # Format data for OpenAI
        prompt = f'''
Analyze the following PR changes and SQL Logic Test (SLT) results.

PR Changes:
{pr_diff}

SLT Results:
{slt_results.to_string()}

Task:
1. Identify what functionality is being modified in the PR.
2. Find any SQL Logic Tests that cover this functionality.
3. Check if these tests are passing or failing.
4. Generate a summary with the following structure:
   - If PR changes match SLT tests and tests are failing: Explain which tests failed and why they are relevant to the PR.
   - If PR changes match SLT tests and tests are passing: Confirm all relevant tests pass.
   - If no matching SLTs found: Output an empty string.
'''

        # Call OpenAI API
        client = openai.OpenAI(api_key=api_key)
        response = client.chat.completions.create(
            model='gpt-4',
            messages=[
                {'role': 'system', 'content': 'You are an AI assistant analyzing code changes and test results.'},
                {'role': 'user', 'content': prompt}
            ],
            temperature=0.1
        )

        analysis = response.choices[0].message.content.strip()

        # Write result to output file
        with open(args.output_file, 'w') as f:
            f.write(analysis)

        # If the analysis is empty, indicate no matches found
        if not analysis:
            print('No matching SLTs found for the PR changes. Skipping comment creation.')
            sys.exit(1)
        else:
            print('Analysis complete. Comment will be created.')

    except Exception as e:
        print(f"Error in analysis: {str(e)}")
        sys.exit(1)


if __name__ == "__main__":
    main()
