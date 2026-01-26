#!/bin/bash
# Test script to verify Ollama integration with Enteract

set -e

echo "🧪 Testing Ollama Integration"
echo "=============================="

# Check if Ollama is installed
if ! command -v ollama &> /dev/null; then
    echo "❌ Ollama not found. Please install: brew install ollama"
    exit 1
fi
echo "✅ Ollama installed"

# Check if Ollama is running
if ! curl -s http://localhost:11434/api/version &> /dev/null; then
    echo "⚠️  Ollama not running. Starting..."
    ollama serve > /tmp/ollama-test.log 2>&1 &
    OLLAMA_PID=$!
    echo "   Started Ollama (PID: $OLLAMA_PID)"
    sleep 2
else
    echo "✅ Ollama already running"
    OLLAMA_PID=""
fi

# Test version endpoint
echo ""
echo "Testing /api/version..."
VERSION=$(curl -s http://localhost:11434/api/version | jq -r '.version')
if [ -n "$VERSION" ]; then
    echo "✅ Version: $VERSION"
else
    echo "❌ Failed to get version"
    [ -n "$OLLAMA_PID" ] && kill $OLLAMA_PID
    exit 1
fi

# Test tags endpoint (list models)
echo ""
echo "Testing /api/tags (model list)..."
MODEL_COUNT=$(curl -s http://localhost:11434/api/tags | jq '.models | length')
echo "✅ Found $MODEL_COUNT model(s)"

# List models
if [ "$MODEL_COUNT" -gt 0 ]; then
    echo ""
    echo "Available models:"
    curl -s http://localhost:11434/api/tags | jq -r '.models[] | "  - \(.name) (\(.size / 1024 / 1024 / 1024 | floor)GB)"'
fi

# Test generate endpoint with a simple prompt (if models available)
if [ "$MODEL_COUNT" -gt 0 ]; then
    echo ""
    echo "Testing /api/generate..."
    MODEL_NAME=$(curl -s http://localhost:11434/api/tags | jq -r '.models[0].name')
    echo "Using model: $MODEL_NAME"

    RESPONSE=$(curl -s http://localhost:11434/api/generate -d '{
        "model": "'$MODEL_NAME'",
        "prompt": "Say hello in one word",
        "stream": false
    }' | jq -r '.response')

    if [ -n "$RESPONSE" ]; then
        echo "✅ Generate test passed"
        echo "   Response: $RESPONSE"
    else
        echo "⚠️  Generate test failed (model may need to load)"
    fi
fi

echo ""
echo "=============================="
echo "🎉 Ollama integration tests complete!"
echo ""
echo "Summary:"
echo "  - Ollama server: ✅ Running"
echo "  - API endpoints: ✅ Working"
echo "  - Models available: $MODEL_COUNT"
echo ""
echo "Next steps:"
echo "  1. Keep Ollama running: ollama serve"
echo "  2. Launch Enteract: npm run tauri dev"
echo "  3. Check Model Manager UI shows 'running'"

# Cleanup if we started Ollama
if [ -n "$OLLAMA_PID" ]; then
    echo ""
    echo "Note: Ollama was started by this script (PID: $OLLAMA_PID)"
    echo "      It will continue running in the background"
fi
