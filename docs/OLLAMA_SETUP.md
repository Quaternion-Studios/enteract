# Ollama Integration Setup

This guide explains how to set up and use Ollama AI models with Enteract.

## Prerequisites

- Ollama installed on your system
- macOS: `brew install ollama`
- Linux/Windows: https://ollama.ai/download

## Setup Steps

### 1. Start Ollama Service

Ollama must be running before launching Enteract:

```bash
ollama serve
```

This starts the Ollama server on `http://localhost:11434` (default port).

**Note**: Keep this terminal window open or run as a background service.

### 2. Install Models

Enteract works with any Ollama-compatible models. Recommended lightweight models for resource-constrained systems:

```bash
# Ultra-lightweight (500MB) - Good for testing
ollama pull qwen2.5:0.5b

# Lightweight vision model (1.7GB) - Image analysis
ollama pull moondream

# Small general-purpose (2GB)
ollama pull gemma:2b

# Coding assistant (3.8GB)
ollama pull codellama:7b-instruct
```

### 3. Verify Installation

Check that models are available:

```bash
ollama list
```

Test a model:

```bash
ollama run moondream "Hello, how are you?"
```

### 4. Launch Enteract

With Ollama running, launch Enteract:

```bash
npm run tauri dev
```

## Using Model Manager

1. Navigate to AI Models section in Enteract
2. Model manager should show "running" status
3. Available models will be listed
4. Select a model to use for analysis

## Troubleshooting

### "Model manager is not running"

**Cause**: Ollama service is not running

**Solution**:
```bash
# Start Ollama in a terminal
ollama serve

# Verify it's running
curl http://localhost:11434/api/version
# Should return: {"version":"0.14.1"} or similar
```

### Cannot connect to Ollama

**Check**:
1. Ollama is running: `ps aux | grep ollama`
2. Port 11434 is accessible: `lsof -i :11434`
3. Firewall settings allow localhost connections

### No models available

**Solution**:
```bash
# Pull at least one model
ollama pull moondream

# Verify
ollama list
```

## Architecture

- **Ollama Base URL**: `http://localhost:11434`
- **Backend**: `src-tauri/src/ollama.rs`
- **Tauri Commands**:
  - `get_ollama_status()` - Check if Ollama is running
  - `get_ollama_models()` - List available models
  - `pull_ollama_model()` - Download new models
  - `delete_ollama_model()` - Remove models
  - `chat_with_ollama()` - Send chat requests
  - `generate_with_ollama()` - Generate completions

## Performance Notes

- **Memory**: Models load into RAM when used
  - moondream: ~2GB RAM
  - codellama:7b: ~5GB RAM
  - qwen3-vl:8b: ~10GB RAM

- **GPU**: Ollama automatically uses Metal (macOS GPU) if available

- **Disk**: Models are stored in `~/.ollama/models/`

## Recommended Models by Use Case

| Use Case | Model | Size | Notes |
|----------|-------|------|-------|
| Testing | qwen2.5:0.5b | 500MB | Fastest, minimal quality |
| Vision | moondream | 1.7GB | Image description, OCR |
| Coding | codellama:7b-instruct | 3.8GB | Code generation, explanation |
| General | gemma:2b | 2GB | Good balance |
| Advanced | qwen3-vl:8b | 6.1GB | High quality, slower |

## Auto-Start Ollama (Optional)

### macOS (launchd)

Create `~/Library/LaunchAgents/ai.ollama.plist`:

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>ai.ollama</string>
    <key>ProgramArguments</key>
    <array>
        <string>/opt/homebrew/bin/ollama</string>
        <string>serve</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
</dict>
</plist>
```

Then:
```bash
launchctl load ~/Library/LaunchAgents/ai.ollama.plist
```

## Integration Status

✅ Ollama connection (`ollama.rs`)
✅ Model listing
✅ Model download/deletion
✅ Chat API integration
✅ Streaming responses
✅ GPU acceleration (Metal on macOS)

## Future Enhancements

- [ ] Auto-start Ollama with Enteract
- [ ] Model recommendations based on system resources
- [ ] Download progress indicators in UI
- [ ] Model performance benchmarking
- [ ] Automatic model updates
