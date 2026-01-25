# macOS Audio Loopback Test Fixtures

**Created:** January 25, 2026
**Author:** jack (enteract crew)
**Purpose:** Test fixtures for Phase 1 macOS audio loopback implementation

---

## Overview

This document describes all test fixtures prepared for validating the macOS audio loopback migration from Windows WASAPI to CoreAudio/CPAL.

These fixtures enable immediate testing when Evan's Phase 1 implementation lands.

---

## 📁 Test Audio Files

**Location:** `test_assets/audio/`

| File | Duration | Sample Rate | Format | Purpose |
|------|----------|-------------|--------|---------|
| `speech_male_48k.wav` | 30s | 48,000 Hz | Stereo PCM16 | Male speech simulation for transcription testing |
| `speech_female_44k.wav` | 30s | 44,100 Hz | Stereo PCM16 | Female speech simulation for 44.1kHz testing |
| `music_classical.wav` | 30s | 48,000 Hz | Stereo PCM16 | Classical music for filtering tests |
| `silence.wav` | 10s | 48,000 Hz | Stereo PCM16 | Digital silence detection |
| `noise_white.wav` | 10s | 48,000 Hz | Stereo PCM16 | White noise rejection |
| `tone_440hz.wav` | 10s | 48,000 Hz | Stereo PCM16 | Signal integrity (440 Hz sine) |

**Total size:** ~45 MB

### Regeneration

```bash
./scripts/audio_test_utils.sh generate
```

See `test_assets/audio/README.md` for detailed file specifications.

---

## 🔧 Test Scripts

**Location:** `scripts/`

### 1. audio_test_utils.sh

Multi-purpose audio testing utility.

**Commands:**
```bash
./scripts/audio_test_utils.sh generate    # Generate all test audio files
./scripts/audio_test_utils.sh play <file> # Play audio file
./scripts/audio_test_utils.sh list        # List audio devices
./scripts/audio_test_utils.sh check       # Check for BlackHole/Loopback
./scripts/audio_test_utils.sh reset       # Reset audio permissions
./scripts/audio_test_utils.sh verify      # Verify test environment
./scripts/audio_test_utils.sh info <file> # Show file details
```

**Features:**
- Colored output (green/red/yellow indicators)
- macOS version detection
- Virtual device detection (BlackHole, Loopback)
- Permission reset (tccutil)
- Audio file generation
- Environment verification

### 2. sanity_check.sh

Quick pre-flight validation before testing.

**Usage:**
```bash
./scripts/sanity_check.sh
```

**Checks:**
- macOS version compatibility (13.x, 14.2+, 14.6+, 15.x)
- Development tools (Rust, cargo, sox, ffmpeg)
- Project structure
- Build compilation
- Test audio fixtures
- Virtual audio devices
- Unit tests (stubs)

**Exit codes:**
- 0: All checks passed
- 1: Critical failures detected

### 3. measure_cpu_usage.sh

Performance benchmarking for CPU and memory usage.

**Usage:**
```bash
./scripts/measure_cpu_usage.sh                    # Default: 60s measurement
./scripts/measure_cpu_usage.sh -d 120             # 2 minutes
./scripts/measure_cpu_usage.sh -i 2               # Sample every 2 seconds
```

**Output:**
- CSV file with timestamp, CPU%, memory MB, thread count
- Statistics summary (average, peak, minimum)
- Evaluation against targets (<5% CPU, <100MB memory)
- Memory growth detection (potential leaks)

**Example output:**
```
CPU Usage:
  Average: 3.24%
  Peak:    15.12%
  Minimum: 0.80%
  Target: <5% (PASS)

Memory Usage:
  Average: 87.45 MB
  Peak:    102.13 MB
  Minimum: 84.21 MB
  Growth:  2.34 MB
  Stability: Good (minimal growth)
```

---

## 🧪 Rust Test Stubs

**Location:** `src-tauri/src/audio_loopback/tests/`

### Test Modules

All tests are marked `#[ignore]` until Phase 1 implementation provides the necessary traits.

#### 1. device_enumeration_tests.rs

Tests for audio device discovery and enumeration.

**Test cases (8 total):**
- `test_enumerate_all_devices` - List all available devices
- `test_detect_virtual_devices` - Detect BlackHole/Loopback
- `test_default_device_exists` - Verify default device
- `test_device_supported_configs` - Query supported formats
- `test_enumerate_loopback_devices_14_6_plus` - Native loopback on 14.6+
- `test_enumerate_no_virtual_devices` - Handle missing virtual devices
- `test_device_hot_plug_detection` - Device connect/disconnect
- `test_macos_version_detection` - OS version for API selection

#### 2. capture_lifecycle_tests.rs

Tests for capture start/stop/pause lifecycle.

**Test cases (12 total):**
- `test_basic_capture_start_stop` - Basic lifecycle
- `test_multiple_start_stop_cycles` - Repeated start/stop
- `test_capture_already_started` - Double-start handling
- `test_stop_not_started` - Stop without start
- `test_capture_with_callback_error` - Error handling in callbacks
- `test_capture_cleanup_on_drop` - Resource cleanup
- `test_capture_state_transitions` - State machine validation
- `test_capture_buffer_overflow` - Buffer overflow handling
- `test_device_switch_during_capture` - Device switching
- `test_system_sleep_wake_cycle` - Sleep/wake handling
- `test_concurrent_capture_instances` - Multiple instances
- `test_capture_latency_measurement` - Latency benchmarking

#### 3. sample_rate_tests.rs

Tests for audio processing pipeline (resampling, conversion).

**Test cases (12 total):**
- `test_48khz_to_16khz_conversion` - 48kHz → 16kHz resampling
- `test_44khz_to_16khz_conversion` - 44.1kHz → 16kHz resampling
- `test_stereo_to_mono_conversion` - Channel reduction
- `test_dc_offset_removal` - DC bias removal
- `test_16bit_pcm_to_f32_conversion` - Format conversion
- `test_32bit_float_to_i16_conversion` - Format conversion
- `test_audio_quality_preservation` - Quality validation
- `test_silence_detection` - Silence handling
- `test_white_noise_handling` - Noise filtering
- `test_buffer_overlap_management` - Transcription buffers
- `test_channel_selection_logic` - Intelligent channel selection
- `test_invalid_audio_format` - Error handling

### Running Tests

```bash
# Run all tests (only non-ignored)
cargo test --lib

# Run ignored tests (when implementation ready)
cargo test --lib -- --ignored

# Run specific module
cargo test device_enumeration_tests

# Run specific test
cargo test test_48khz_to_16khz_conversion -- --ignored
```

---

## 📊 Test Coverage Summary

| Category | Test Count | Status |
|----------|------------|--------|
| Device Enumeration | 8 | Stubs ready |
| Capture Lifecycle | 12 | Stubs ready |
| Sample Rate Processing | 12 | Stubs ready |
| **Total** | **32** | **All #[ignore]** |

Additional manual testing scenarios documented in:
- `resources/MACOS_AUDIO_TESTING_STRATEGY.md` (78 test cases total)

---

## 🚀 Quick Start

### 1. Verify Environment

```bash
./scripts/sanity_check.sh
```

Expected output: All checks passed (some warnings OK)

### 2. Generate Test Audio

```bash
./scripts/audio_test_utils.sh generate
```

Expected: 6 WAV files in `test_assets/audio/`

### 3. Verify Test Stubs Compile

```bash
cd src-tauri
cargo test --lib audio_loopback::tests --no-run
```

Expected: Compilation succeeds (tests won't run, all ignored)

### 4. When Phase 1 Lands

```bash
# Remove #[ignore] from tests in:
# - src-tauri/src/audio_loopback/tests/*.rs

# Run tests
cargo test --lib audio_loopback::tests

# Run with output
cargo test --lib audio_loopback::tests -- --nocapture
```

---

## 📋 Integration with Phase 1

**When Evan implements Phase 1**, these fixtures enable immediate validation:

1. **Device Enumeration**
   - Un-ignore `device_enumeration_tests.rs`
   - Implement `MacOSLoopbackEnumerator` trait
   - Run: `cargo test device_enumeration`

2. **Capture Engine**
   - Un-ignore `capture_lifecycle_tests.rs`
   - Implement `MacOSCaptureEngine` trait
   - Run: `cargo test capture_lifecycle`

3. **Audio Processing**
   - Un-ignore `sample_rate_tests.rs`
   - Reuse existing `audio_processor::process_audio_chunk`
   - Run: `cargo test sample_rate`

4. **Performance Validation**
   - Build release: `cargo build --release`
   - Start Enteract
   - Run: `./scripts/measure_cpu_usage.sh -d 120`
   - Verify: CPU <5%, Memory <100MB

---

## 🔍 Additional Resources

- **Research:** `resources/MACOS_AUDIO_LOOPBACK_RESEARCH.md`
- **Testing Strategy:** `resources/MACOS_AUDIO_TESTING_STRATEGY.md`
- **Audio Fixtures:** `test_assets/audio/README.md`

---

## ✅ Deliverables Checklist

- [x] 6 test audio files generated
- [x] 3 test scripts created and executable
- [x] 32 Rust test stubs created (all #[ignore])
- [x] Test module structure integrated
- [x] Sanity check script validates environment
- [x] Performance measurement script ready
- [x] Documentation complete

**Status:** Ready for Phase 1 implementation

**Next step:** Evan implements Phase 1, then un-ignore tests and validate

---

**Prepared by:** jack (enteract crew)
**Date:** January 25, 2026
**Related:** Mayor assignment hq-dgc
