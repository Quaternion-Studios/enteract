# Whisper vs Faster-Whisper Benchmark Report

**Generated**: 2026-01-29T22:32:39.240346

## System Information

- **Platform**: darwin
- **Python**: 3.13.5
- **CPU Cores**: 11
- **RAM**: 18.0 GB
- **faster-whisper available**: True
- **Google STT available**: False

## Summary: Average Metrics by Engine and Model

| Engine         | Model   | First Token   | RTF   | WER   | Memory   |
|:---------------|:--------|:--------------|:------|:------|:---------|
| faster-whisper | tiny    | 0.532s        | 0.10x | N/A   | 1335 MB  |
| faster-whisper | base    | 1.019s        | 0.18x | N/A   | 1414 MB  |
| faster-whisper | small   | 2.920s        | 0.53x | N/A   | 1577 MB  |
| whisper-rs     | tiny    | 0.012s        | 0.03x | N/A   | 12700 MB |
| whisper-rs     | base    | 0.020s        | 0.06x | N/A   | 12789 MB |
| whisper-rs     | small   | 0.056s        | 0.18x | N/A   | 12968 MB |

## Detailed Results by Audio Sample

### noisy_background_with_noise.wav

| Engine         | Model   | Time   | RTF   | WER   | Memory   | Transcription                                         |
|:---------------|:--------|:-------|:------|:------|:---------|:------------------------------------------------------|
| faster-whisper | tiny    | 0.46s  | 0.08x | N/A   | 491 MB   | Please listen carefully. I'm speaking with backgro... |
| whisper-rs     | tiny    | 0.14s  | 0.03x | N/A   | 13043 MB | Please listen carefully. I'm speaking with backgro... |
| faster-whisper | base    | 0.87s  | 0.16x | N/A   | 721 MB   | Please listen carefully, I'm speaking with backgro... |
| whisper-rs     | base    | 0.31s  | 0.06x | N/A   | 12933 MB | Please listen carefully, I'm speaking with backgro... |
| faster-whisper | small   | 2.75s  | 0.51x | N/A   | 1373 MB  | Please listen carefully, I'm speaking with backgro... |
| whisper-rs     | small   | 0.91s  | 0.17x | N/A   | 12996 MB | Please listen carefully, I'm speaking with backgro... |

### technical_jargon.wav

| Engine         | Model   | Time   | RTF   | WER   | Memory   | Transcription                                         |
|:---------------|:--------|:-------|:------|:------|:---------|:------------------------------------------------------|
| faster-whisper | tiny    | 0.91s  | 0.09x | N/A   | 1380 MB  | the API endpoint uses OAuth 2.0 authentication wit... |
| whisper-rs     | tiny    | 0.16s  | 0.02x | N/A   | 12579 MB | The API endpoint uses OAuth 2.0 authentication wit... |
| faster-whisper | base    | 1.17s  | 0.11x | N/A   | 1411 MB  | the API end point uses OAuth 2.0 authentication wi... |
| whisper-rs     | base    | 0.32s  | 0.03x | N/A   | 12706 MB | the API end point uses OAuth 2.0 authentication wi... |
| faster-whisper | small   | 3.06s  | 0.29x | N/A   | 1478 MB  | The API endpoint uses OAuth 2.0 authentication wit... |
| whisper-rs     | small   | 1.02s  | 0.10x | N/A   | 13021 MB | The API endpoint uses OAuth 2.0 authentication wit... |

### noisy_background.wav

| Engine         | Model   | Time   | RTF   | WER   | Memory   | Transcription                                         |
|:---------------|:--------|:-------|:------|:------|:---------|:------------------------------------------------------|
| faster-whisper | tiny    | 0.54s  | 0.10x | N/A   | 1478 MB  | Please listen carefully, I'm speaking with backgro... |
| whisper-rs     | tiny    | 0.15s  | 0.03x | N/A   | 12693 MB | Please listen carefully. I'm speaking with backgro... |
| faster-whisper | base    | 1.03s  | 0.19x | N/A   | 1513 MB  | Please listen carefully, I'm speaking with backgro... |
| whisper-rs     | base    | 0.30s  | 0.06x | N/A   | 12918 MB | Please listen carefully, I'm speaking with backgro... |
| faster-whisper | small   | 2.89s  | 0.53x | N/A   | 1586 MB  | Please listen carefully, I'm speaking with backgro... |
| whisper-rs     | small   | 0.99s  | 0.18x | N/A   | 13021 MB | Please listen carefully. I'm speaking with backgro... |

### fast_speech.wav

| Engine         | Model   | Time   | RTF   | WER   | Memory   | Transcription                                         |
|:---------------|:--------|:-------|:------|:------|:---------|:------------------------------------------------------|
| faster-whisper | tiny    | 0.52s  | 0.20x | N/A   | 1586 MB  | I need to quickly explain that we are running out ... |
| whisper-rs     | tiny    | 0.16s  | 0.06x | N/A   | 12685 MB | I need to quickly explain that we're running out o... |
| faster-whisper | base    | 0.99s  | 0.37x | N/A   | 1636 MB  | I need to quickly explain that we're running out o... |
| whisper-rs     | base    | 0.31s  | 0.12x | N/A   | 12757 MB | I need to quickly explain that we're running out o... |
| faster-whisper | small   | 2.90s  | 1.10x | N/A   | 1670 MB  | I need to quickly explain that we're running out o... |
| whisper-rs     | small   | 0.97s  | 0.37x | N/A   | 12919 MB | I need to quickly explain that we're running out o... |

### clear_speech.wav

| Engine         | Model   | Time   | RTF   | WER   | Memory   | Transcription                                         |
|:---------------|:--------|:-------|:------|:------|:---------|:------------------------------------------------------|
| faster-whisper | tiny    | 0.57s  | 0.08x | N/A   | 1629 MB  | The quick brown fox jumps over the lazy dog.  This... |
| whisper-rs     | tiny    | 0.17s  | 0.02x | N/A   | 12605 MB | The quick brown fox jumps over the lazy dog, this ... |
| faster-whisper | base    | 1.08s  | 0.15x | N/A   | 1639 MB  | The quick brown fox jumps over the lazy dog, this ... |
| whisper-rs     | base    | 0.31s  | 0.04x | N/A   | 12737 MB | The quick brown fox jumps over the lazy dog. This ... |
| faster-whisper | small   | 2.97s  | 0.42x | N/A   | 1690 MB  | The quick brown fox jumps over the lazy dog.  This... |
| whisper-rs     | small   | 0.97s  | 0.14x | N/A   | 12913 MB | The quick brown fox jumps over the lazy dog. This ... |

### numbers_dates.wav

| Engine         | Model   | Time   | RTF   | WER   | Memory   | Transcription                                         |
|:---------------|:--------|:-------|:------|:------|:---------|:------------------------------------------------------|
| faster-whisper | tiny    | 0.55s  | 0.05x | N/A   | 1444 MB  | The meeting is scheduled for January 15, 2025 at 3... |
| whisper-rs     | tiny    | 0.15s  | 0.01x | N/A   | 12593 MB | The meeting is scheduled for January 15, 2025 at 3... |
| faster-whisper | base    | 1.08s  | 0.11x | N/A   | 1565 MB  | The meeting is scheduled for January 15, 2025 at 3... |
| whisper-rs     | base    | 0.31s  | 0.03x | N/A   | 12681 MB | The meeting is scheduled for January 15, 2025 at 3... |
| faster-whisper | small   | 3.07s  | 0.30x | N/A   | 1665 MB  | The meeting is scheduled for January 15, 2025 at 3... |
| whisper-rs     | small   | 1.01s  | 0.10x | N/A   | 12936 MB | The meeting is scheduled for January 15, 2025 at 3... |

## Analysis & Recommendations

### Speed
- **Fastest**: whisper-rs/tiny (RTF: 0.01x)

### Memory Efficiency
- **Lowest Memory**: faster-whisper/tiny (491 MB)

---

*Report generated by whisper-study benchmark harness*
