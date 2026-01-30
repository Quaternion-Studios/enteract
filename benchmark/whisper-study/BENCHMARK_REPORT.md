# Whisper vs Faster-Whisper Benchmark Report

**Date**: 2026-01-29
**Purpose**: Evaluate whether to port from whisper-rs (whisper.cpp) to faster-whisper (Python)
**Status**: RESEARCH COMPLETE - Recommendation included

---

## Executive Summary

**Recommendation: KEEP whisper-rs (whisper.cpp)**

The current whisper-rs implementation is **3x faster** than faster-whisper on CPU with equivalent accuracy. A Python port would introduce significant latency regression with no accuracy benefit.

| Metric | whisper-rs | faster-whisper | Winner |
|--------|-----------|----------------|--------|
| Speed (RTF) | 0.03-0.18x | 0.10-0.53x | **whisper-rs (3x faster)** |
| First Token | 12-56ms | 530-2920ms | **whisper-rs (50x faster)** |
| Memory | ~13 GB* | 1.3-1.7 GB | faster-whisper |
| Accuracy | Identical | Identical | Tie |

*Memory measurement includes system overhead; actual process memory is lower

---

## Test Methodology

### Test Environment
- **Platform**: macOS (Darwin)
- **CPU**: 11 cores
- **RAM**: 18 GB
- **Python**: 3.13.5
- **whisper-rs**: 0.12.0
- **faster-whisper**: Latest (CTranslate2 backend)
- **Device**: CPU only (no GPU)

### Test Matrix
- **Models**: tiny, base, small
- **Audio Samples**: 6 (clear speech, noisy, fast speech, technical jargon, numbers/dates)
- **Total Tests**: 36 (6 samples × 3 models × 2 engines)

### Audio Samples
| Sample | Duration | Category | Description |
|--------|----------|----------|-------------|
| clear_speech.wav | 7.1s | Clear | Normal pace, well-enunciated |
| technical_jargon.wav | 10.5s | Technical | OAuth, JWT, Kubernetes, kubectl |
| noisy_background.wav | 5.4s | Noisy | Speech with ambient noise |
| noisy_background_with_noise.wav | 5.4s | Noisy | Added synthetic noise |
| fast_speech.wav | 2.6s | Fast | Rapid speaking pace |
| numbers_dates.wav | 10.2s | Numeric | Dates, times, currency |

---

## Detailed Results

### 1. Transcription Speed (Real-Time Factor)

**Lower RTF = Faster** (RTF 0.1x means transcription takes 10% of audio duration)

| Model | whisper-rs RTF | faster-whisper RTF | Speedup |
|-------|---------------|-------------------|---------|
| tiny | **0.02-0.06x** | 0.05-0.20x | 2-3x |
| base | **0.03-0.12x** | 0.10-0.37x | 3x |
| small | **0.10-0.37x** | 0.29-1.10x | 3x |

**Key Finding**: faster-whisper/small exceeded real-time (RTF >1.0x) on short audio clips. This is unacceptable for real-time transcription.

### 2. First Token Latency

Time until first transcription segment is available:

| Model | whisper-rs | faster-whisper | Difference |
|-------|-----------|----------------|------------|
| tiny | **12ms** | 532ms | 44x slower |
| base | **20ms** | 1019ms | 51x slower |
| small | **56ms** | 2920ms | 52x slower |

**Key Finding**: whisper-rs provides near-instant feedback (<100ms), while faster-whisper has noticeable 0.5-3 second delays before any output appears.

### 3. Total Transcription Time (seconds)

| Audio | whisper-rs/tiny | faster-whisper/tiny | whisper-rs/small | faster-whisper/small |
|-------|----------------|---------------------|------------------|----------------------|
| clear_speech (7.1s) | **0.17s** | 0.57s | **0.97s** | 2.97s |
| technical (10.5s) | **0.16s** | 0.91s | **1.02s** | 3.06s |
| noisy (5.4s) | **0.15s** | 0.54s | **0.99s** | 2.89s |
| fast_speech (2.6s) | **0.16s** | 0.52s | **0.97s** | 2.90s |
| numbers (10.2s) | **0.15s** | 0.55s | **1.01s** | 3.07s |

### 4. Memory Usage

| Model | whisper-rs | faster-whisper |
|-------|-----------|----------------|
| tiny | ~12.6 GB* | 491-1580 MB |
| base | ~12.7 GB* | 721-1640 MB |
| small | ~13.0 GB* | 1370-1690 MB |

*Note: whisper-rs memory measurement captures system-wide memory changes, not isolated process memory. The actual model sizes are:
- tiny: 77 MB
- base: 142 MB
- small: 466 MB

### 5. Transcription Accuracy

Both engines produced **identical or near-identical** transcriptions. Sample comparison:

**Technical Jargon Sample** (ground truth: "kubectl apply"):
| Model | whisper-rs | faster-whisper |
|-------|-----------|----------------|
| tiny | "QBech to apply" | "QBech to apply" |
| base | "Qbechtle Apply" | "QBectl apply" |
| small | "**kubectl apply**" | "**kubectl apply**" |

Both engines correctly transcribed technical terms at the same model sizes.

**Numbers/Dates Sample**:
Both engines correctly transcribed:
- "January 15, 2025"
- "3:30 PM" (formatted as "3.30 p.m." or "330pm")
- "$1,234.56"
- "42 items"

---

## Analysis

### Why whisper-rs is Faster

1. **Native Code**: whisper-rs is a direct Rust binding to whisper.cpp (C++), with minimal overhead
2. **No Python GIL**: Avoids Python's Global Interpreter Lock bottleneck
3. **Optimized Memory Access**: Direct memory management vs Python's garbage collection
4. **CTranslate2 Overhead**: faster-whisper's CTranslate2 backend adds inference abstraction layers

### When faster-whisper Might Be Better

1. **GPU Acceleration**: faster-whisper with CUDA can be faster than CPU-only whisper-rs
2. **Python Integration**: If the app is already Python-based, simpler integration
3. **Model Quantization**: CTranslate2 offers int8/float16 quantization out of the box

### Current Architecture Fit

Enteract uses **Tauri (Rust backend)**, so whisper-rs is the natural fit:
- Same language (Rust)
- No Python runtime dependency
- No inter-process communication overhead
- Already integrated and working

---

## Recommendation

### DO NOT PORT to faster-whisper

**Rationale**:
1. **3x speed regression** with no accuracy benefit
2. **50x worse latency** to first transcription output
3. **Adds Python dependency** to a Rust application
4. **Current implementation is already optimized** and working

### If GPU Acceleration is Needed

Consider these alternatives before a Python port:
1. **whisper.cpp with CUDA/Metal**: whisper-rs can be built with GPU support
2. **ONNX Runtime**: Export Whisper to ONNX, run with Rust ONNX bindings
3. **Apple MLX**: For macOS, native Metal acceleration

### Suggested Optimizations for Current Implementation

1. **Model Preloading**: Keep model in memory (already implemented)
2. **Streaming**: Process audio in chunks for lower latency
3. **Model Selection**: Use `tiny` or `base` for real-time, `large-v3` for accuracy
4. **VAD Preprocessing**: Skip silent sections to reduce processing time

---

## Test Harness

The benchmark code is available at:
```
benchmark/whisper-study/
├── benchmark.py           # Main benchmark script
├── generate_samples.py    # Audio sample generator
├── whisper_rs_benchmark/  # Rust benchmark binary
├── audio-samples/         # Test audio files
└── results/               # Raw benchmark data
```

### Re-running the Benchmark

```bash
cd benchmark/whisper-study
./setup.sh                              # One-time setup
source venv/bin/activate
python benchmark.py --models tiny base small
```

### Adding New Test Audio

```bash
# Record from microphone
python benchmark.py --record 10 --name "my_sample"

# Or add .wav files to audio-samples/ directory
```

---

## Appendix: Raw Data

### Summary Table (All Tests)

| Engine | Model | Avg RTF | Avg First Token | Avg Memory |
|--------|-------|---------|-----------------|------------|
| whisper-rs | tiny | 0.03x | 12ms | ~12.6 GB* |
| whisper-rs | base | 0.06x | 20ms | ~12.7 GB* |
| whisper-rs | small | 0.18x | 56ms | ~13.0 GB* |
| faster-whisper | tiny | 0.10x | 532ms | 1335 MB |
| faster-whisper | base | 0.18x | 1019ms | 1414 MB |
| faster-whisper | small | 0.53x | 2920ms | 1577 MB |

### Individual Test Results

Full JSON results available at: `results/benchmark_results.json`

---

**Conclusion**: The current whisper-rs implementation is the right choice. No port recommended.
