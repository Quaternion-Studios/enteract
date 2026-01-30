# Whisper vs Faster-Whisper Benchmark Study

Research study comparing speech-to-text engine performance for Enteract.

## Purpose

Evaluate whether to port from the current whisper-rs (whisper.cpp) implementation
to faster-whisper (Python CTranslate2) based on:
- Transcription latency
- Accuracy (Word Error Rate)
- Memory usage
- CPU/GPU utilization

## Quick Start

```bash
# Setup
./setup.sh

# Or manually:
python3 -m venv venv
source venv/bin/activate
pip install -r requirements.txt
cd whisper_rs_benchmark && cargo build --release && cd ..
python3 generate_samples.py

# Run benchmark
python3 benchmark.py
```

## Usage

### Full Benchmark
```bash
python benchmark.py
```

### Specific Models
```bash
python benchmark.py --models tiny base small
```

### Single Audio File
```bash
python benchmark.py --audio /path/to/audio.wav
```

### Record from Microphone
```bash
python benchmark.py --record 10 --name "my_sample"
```

### Use GPU (if available)
```bash
python benchmark.py --device cuda
```

## Test Audio Categories

- **clear_speech**: Normal, well-enunciated speech
- **technical_jargon**: Programming terms, acronyms, technical vocabulary
- **fast_speech**: Rapid speaking pace
- **noisy_background**: Speech with ambient noise
- **numbers_dates**: Numeric content, dates, currency

## Output

- `results/benchmark_report.md` - Summary report with recommendations
- `results/benchmark_results.json` - Raw benchmark data

## Engines Compared

| Engine | Language | Backend | GPU Support |
|--------|----------|---------|-------------|
| whisper-rs | Rust | whisper.cpp | Optional |
| faster-whisper | Python | CTranslate2 | Yes (CUDA) |

## Models Tested

| Model | Size | Parameters |
|-------|------|------------|
| tiny | ~75 MB | 39M |
| base | ~142 MB | 74M |
| small | ~466 MB | 244M |
| medium | ~1.5 GB | 769M |
| large-v3 | ~3 GB | 1550M |

## Adding Google STT Baseline

For accuracy comparison with ground truth:

```bash
pip install google-cloud-speech
export GOOGLE_APPLICATION_CREDENTIALS="/path/to/credentials.json"
```

The benchmark will automatically use Google STT to establish ground truth WER.

## File Structure

```
benchmark/whisper-study/
├── benchmark.py          # Main benchmark script
├── generate_samples.py   # Test audio generator
├── requirements.txt      # Python dependencies
├── setup.sh              # Setup script
├── whisper_rs_benchmark/ # Rust benchmark binary source
├── audio-samples/        # Test audio files
└── results/              # Benchmark output
```
