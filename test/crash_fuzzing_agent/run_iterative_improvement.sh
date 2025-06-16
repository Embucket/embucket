#!/bin/bash

# Script to run the iterative improvement agent with the correct environment

# Get the directory where this script is located
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "🚀 Starting Iterative Improvement Agent"
echo "📁 Script directory: $SCRIPT_DIR"

# Change to the script directory
cd "$SCRIPT_DIR"
echo "📁 Current directory: $(pwd)"

# Check if virtual environment exists
if [ ! -d "venv" ]; then
    echo "❌ Virtual environment not found at $SCRIPT_DIR/venv"
    echo "Please create the virtual environment first:"
    echo "  cd $SCRIPT_DIR"
    echo "  python3 -m venv venv"
    echo "  source venv/bin/activate"
    echo "  pip install -r requirements.txt"
    exit 1
fi

# Activate virtual environment
echo "🔧 Activating virtual environment..."
source venv/bin/activate

# Check if required packages are installed
echo "📦 Checking dependencies..."
python3 -c "
import sys
try:
    from agents import Agent, Runner, function_tool
    print('✅ openai_agents package available')
except ImportError as e:
    print(f'❌ openai_agents package not available: {e}')
    print('Please install dependencies: pip install -r requirements.txt')
    sys.exit(1)

try:
    from dotenv import load_dotenv
    print('✅ python-dotenv package available')
except ImportError as e:
    print(f'❌ python-dotenv package not available: {e}')
    sys.exit(1)
"

if [ $? -ne 0 ]; then
    echo "❌ Dependency check failed"
    exit 1
fi

# Check if .env file exists
if [ ! -f ".env" ]; then
    echo "⚠️ .env file not found. Creating template..."
    cat > .env << EOF
# OpenAI API Configuration
OPENAI_API_KEY=your_openai_api_key_here

# Embucket Configuration
EMBUCKET_HOST=localhost
EMBUCKET_PORT=3000

# Fuzzing Configuration
DEFAULT_NUM_QUERIES=5
DEFAULT_SAFE_QUERY_PROBABILITY=0.3

# Output Configuration
SLT_OUTPUT_DIR=test/sql/fuzz_regressions
EOF
    echo "📝 Created .env template. Please edit it with your configuration."
    echo "❌ Cannot continue without proper .env configuration"
    exit 1
fi

# Run the iterative improvement agent
echo "🎯 Running iterative improvement agent..."
python3 iterative_improvement_agent.py

echo "🏁 Iterative improvement agent completed"
