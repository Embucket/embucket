# Embucket Crash Fuzzing Agent

An AI-powered fuzzing agent built with the [openai-agents-python](https://github.com/openai/openai-agents-python) framework that runs comprehensive SQL fuzzing sessions against Embucket to find crashes and bugs, then analyzes the results.

## Features

- **AI-Powered Analysis**: Uses OpenAI's LLM to analyze fuzzing results and provide insights
- **Comprehensive Fuzzing Tool**: Single tool handles entire workflow (build → start → fuzz → analyze → stop)
- **Early Termination**: Stops immediately when crashes are detected to preserve evidence
- **Detailed Logging**: Comprehensive logs of all operations and results
- **Random Query Generation**: Generates random SQL queries with configurable complexity using SQLGlot
- **Crash Detection**: Identifies crashes, server errors, timeouts, and distinguishes from expected errors
- **SLT File Generation**: Saves problematic queries as `.slt` regression test files
- **Efficient Turn Usage**: Minimal AI turns needed (1-2 turns total)

## Setup

### 1. Install Dependencies

Make sure you have the openai-agents-python package installed:

```bash
pip install openai-agents python-dotenv
```

### 2. Configure Environment Variables

Copy the example environment file and configure it:

```bash
cp .env.example .env
```

Edit the `.env` file and set your OpenAI API key:

```env
OPENAI_API_KEY=sk-your_actual_openai_api_key_here
```

You can get an API key from: https://platform.openai.com/api-keys

### 3. Optional Configuration

You can customize other settings in the `.env` file:

- `EMBUCKET_HOST`: Host for Embucket server (default: localhost)
- `EMBUCKET_PORT`: Port for Embucket server (default: 3000)
- `SLT_OUTPUT_DIR`: Directory to save SLT files (default: test/sql/fuzz_regressions)
- `DEFAULT_COMPLEXITY`: SQL query complexity (default: medium)
- `DEFAULT_NUM_QUERIES`: Number of queries to test (default: 10)

## Usage

### Run the Fuzzing Agent

```bash
# Activate virtual environment
source venv/bin/activate

# Run with example script
python example_usage.py

# Or run directly
python fuzzing_agent.py
```

This will run a comprehensive fuzzing session with the default configuration.

### Test the Architecture (No API Key Required)

```bash
# Activate virtual environment
source venv/bin/activate

# Test the comprehensive tool architecture
python test_comprehensive_tool.py
```

## Architecture

### Main Components

- **`fuzzing_agent.py`**: Main agent class that analyzes fuzzing results
- **`tools/comprehensive_fuzzing_tool.py`**: Single tool that handles entire fuzzing workflow
- **`tools/`**: Directory containing individual tool implementations (used by comprehensive tool)

### Streamlined Architecture

The agent now uses a **two-phase approach**:

1. **Comprehensive Fuzzing Tool**: Handles entire workflow in one tool call
   - Imports and orchestrates individual tool functions (without decorators)
   - Builds and starts Embucket server
   - Generates and executes SQL queries
   - Monitors for crashes and collects logs
   - Saves SLT files for bugs
   - Stops server when done
   - Returns detailed results

2. **AI Analysis Phase**: Agent analyzes results and provides insights
   - Classifies bugs vs expected errors
   - Provides recommendations
   - Assesses Embucket stability

### Individual Tool Functions

The individual tools are kept in separate files for maintainability:
- **`server_build_tool.py`**: Builds Embucket server binary
- **`server_lifecycle_tool.py`**: Starts/stops Embucket server
- **`database_setup_tool.py`**: Sets up test database with tables
- **`random_query_generator_tool.py`**: Generates SQL queries using SQLGlot
- **`query_execution_tool.py`**: Executes queries against Embucket
- **`slt_file_tool.py`**: Saves crash queries as SLT files

These functions are imported by the comprehensive tool (no longer decorated as individual agent tools).

### Workflow

**Phase 1: Comprehensive Fuzzing (Single Tool Call)**
1. Build Embucket server using cargo
2. Start the Embucket server
3. Set up test database with tables
4. Generate and execute SQL queries
5. Monitor for crashes (early termination if found)
6. Collect detailed logs of all operations
7. Save SLT files for any bugs found
8. Stop server and return comprehensive results

**Phase 2: AI Analysis (Agent)**
9. Analyze fuzzing results
10. Classify bugs vs expected errors
11. Provide insights and recommendations
12. Assess overall Embucket stability

## Output

The agent saves crash queries as `.slt` files with descriptive names like:
- `crash_2024_01_15_143022.slt`
- `server_error_2024_01_15_143045.slt`
- `timeout_2024_01_15_143102.slt`

Each SLT file includes:
- Metadata about the error (type, status code, execution time)
- The original error response
- The SQL query that caused the issue

## Development

### Adding New Tools

1. Create a new file in the `tools/` directory
2. Use the `@function_tool` decorator from `agents`
3. Add the tool to the agent's tools list in `fuzzing_agent.py`

### Testing

Run the test script to verify the architecture is working:

```bash
source venv/bin/activate
python test_comprehensive_tool.py
```

This will test the comprehensive tool architecture without requiring an API key.

## Troubleshooting

### Agent Architecture

The agent now uses a streamlined architecture:
1. **Single Comprehensive Tool**: All fuzzing logic is in one tool call
2. **AI Analysis**: The agent analyzes results and provides insights
3. **Minimal Turns**: Only 1-2 AI turns needed (no more max_turns issues)

### Server Startup Issues

If the Embucket server fails to start, the agent will immediately stop with a clear error message. Check:
- Port 3000 is not already in use
- Embucket binary is built (`cargo build`)
- No firewall blocking localhost connections

## Notes

- **Comprehensive Tool**: Single tool handles entire fuzzing workflow for efficiency
- **Modular Design**: Individual functions kept in separate files for maintainability
- **Early Termination**: Stops immediately on crashes to preserve debugging information
- **Detailed Logging**: All operations and results are logged for analysis
- **AI Analysis**: The agent analyzes results and provides actionable insights
- **Real Implementation**: Uses actual SQLGlot, cargo build, and HTTP requests
- **SLT Files**: Automatically saves crash queries as regression test files
- **No Turn Limits**: Only 1-2 AI turns needed (solved max_turns issue)
