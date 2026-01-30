#!/bin/bash
# Setup script for Whisper benchmark study

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

echo "=== Whisper Benchmark Study Setup ==="
echo

# Check Python
if ! command -v python3 &> /dev/null; then
    echo "Error: Python 3 not found"
    exit 1
fi

PYTHON_VERSION=$(python3 --version)
echo "Python: $PYTHON_VERSION"

# Create virtual environment
if [ ! -d "venv" ]; then
    echo "Creating virtual environment..."
    python3 -m venv venv
fi

# Activate venv
source venv/bin/activate

# Install dependencies
echo "Installing Python dependencies..."
pip install --upgrade pip
pip install -r requirements.txt

# Check for CUDA (optional)
if python3 -c "import torch; print(torch.cuda.is_available())" 2>/dev/null | grep -q "True"; then
    echo "CUDA available - GPU acceleration enabled"
    DEVICE="cuda"
else
    echo "CUDA not available - using CPU"
    DEVICE="cpu"
fi

# Build Rust benchmark binary
echo
echo "Building whisper-rs benchmark binary..."
cd whisper_rs_benchmark

if command -v cargo &> /dev/null; then
    cargo build --release
    cp target/release/whisper_rs_benchmark ../
    echo "Rust binary built: whisper_rs_benchmark"
else
    echo "Warning: Cargo not found. whisper-rs benchmark will be skipped."
    echo "Install Rust from https://rustup.rs/"
fi

cd "$SCRIPT_DIR"

# Generate sample audio
echo
echo "Generating test audio samples..."
python3 generate_samples.py

echo
echo "=== Setup Complete ==="
echo
echo "To run the benchmark:"
echo "  source venv/bin/activate"
echo "  python benchmark.py"
echo
echo "Options:"
echo "  --models tiny base small    # Specific models"
echo "  --device cuda               # Use GPU (if available)"
echo "  --audio /path/to/file.wav   # Single file"
echo "  --record 10                 # Record from mic"
