# macOS Audio Loopback Testing Strategy

**Date:** January 25, 2026
**Author:** jack (enteract crew)
**Purpose:** Testing strategy for Phase 3 macOS audio capture implementation
**Related:** MACOS_AUDIO_LOOPBACK_RESEARCH.md

---

## Overview

This document provides a comprehensive testing strategy for validating the macOS audio loopback implementation across multiple macOS versions, ensuring feature parity with the Windows WASAPI implementation.

**Testing Phases:**
1. Environment Setup & Tooling
2. Unit Testing (Components)
3. Integration Testing (End-to-End)
4. Performance Benchmarking
5. Compatibility Validation
6. User Acceptance Testing

---

## 1. Test Environment Setup

### Required macOS Versions

| Version | Release Date | Testing Priority | Reason |
|---------|--------------|------------------|--------|
| macOS 13.6 (Ventura) | September 2023 | HIGH | No CoreAudio Taps, ScreenCaptureKit baseline |
| macOS 14.2 (Sonoma) | December 2023 | HIGH | CoreAudio Taps introduction |
| macOS 14.6 (Sonoma) | July 2024 | CRITICAL | CPAL native loopback support |
| macOS 15.2 (Sequoia) | Current | CRITICAL | Latest release, known SCK issues |

### Option A: Physical Hardware Setup

**Recommended Configuration:**

| Machine | macOS Version | Purpose |
|---------|---------------|---------|
| Primary Dev Mac | macOS 15.2 | Primary development, latest testing |
| Secondary Mac | macOS 14.6 | CPAL loopback validation |
| Older Mac | macOS 13.6 | Legacy fallback testing |

**Pros:**
- Real hardware performance metrics
- True audio device behavior
- No virtualization overhead

**Cons:**
- Expensive (multiple Macs required)
- Time-consuming to switch between versions
- Limited to available hardware

### Option B: Virtual Machines (UTM) ⭐ RECOMMENDED

**Setup Instructions:**

1. **Install UTM** (free, Apple Silicon native)
   ```bash
   brew install --cask utm
   ```

2. **Download macOS Installers**
   - macOS 13.6: https://support.apple.com/en-us/108382
   - macOS 14.2: https://support.apple.com/en-us/108382
   - macOS 14.6: https://support.apple.com/en-us/108382
   - macOS 15.2: Download from Mac App Store

3. **Create VMs for Each Version**

   **VM Configuration (per version):**
   - **Architecture:** Apple Virtualization
   - **CPU Cores:** 4 cores
   - **RAM:** 8 GB
   - **Storage:** 60 GB
   - **Display:** Default
   - **Audio:** Enabled ✅ (critical for testing)
   - **Shared Directories:** Enable for transferring builds

4. **Install macOS**
   ```
   1. Create new VM in UTM
   2. Select "Virtualize" → "macOS 12+"
   3. Point to IPSW file
   4. Follow installation wizard
   5. Set up test user account
   6. Disable sleep/screen saver
   7. Enable SSH for remote access (optional)
   ```

5. **Snapshot Each VM**
   - Create snapshot after clean install
   - Name: "Clean Install - macOS X.Y"
   - Allows quick reset between tests

**VM Performance Notes:**
- Audio I/O works in UTM VMs (uses host audio devices)
- Performance sufficient for functional testing
- Not suitable for micro-benchmarking (use physical hardware)

**Pros:**
- Cost-effective (one Mac)
- Quick version switching
- Snapshot/restore for clean state
- Parallel testing possible

**Cons:**
- Audio may behave slightly differently than hardware
- Some performance metrics less accurate
- Requires powerful host machine

### Option C: Hybrid Approach

- **Primary development:** macOS 15.2 physical
- **Compatibility testing:** UTM VMs for 13.6, 14.2, 14.6
- **Performance benchmarking:** macOS 14.6+ physical

### Required Software (All Environments)

**Development Tools:**
```bash
# Install Xcode Command Line Tools
xcode-select --install

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install Tauri CLI
cargo install tauri-cli

# Install audio testing tools
brew install sox          # Audio file manipulation
brew install ffmpeg       # Audio conversion/analysis
brew install blackhole-2ch # Virtual audio device
```

**Virtual Audio Devices:**

1. **BlackHole** (Required for legacy testing)
   ```bash
   brew install blackhole-2ch
   # OR download from: https://existential.audio/blackhole/
   ```

   Verify installation:
   ```bash
   # List audio devices
   system_profiler SPAudioDataType
   # Should show "BlackHole 2ch"
   ```

2. **Loopback** (Optional, commercial)
   - Download from: https://rogueamoeba.com/loopback/
   - Useful for advanced routing scenarios
   - 20-minute trial mode available

**Audio Testing Utilities:**

Create test script: `scripts/audio_test_utils.sh`
```bash
#!/bin/bash

# Generate test audio file (sine wave at 440 Hz, 10 seconds)
generate_test_audio() {
    sox -n -r 48000 -c 2 test_audio_48k.wav synth 10 sine 440
    sox -n -r 44100 -c 2 test_audio_44k.wav synth 10 sine 440
    sox -n -r 16000 -c 1 test_audio_16k.wav synth 10 sine 440
}

# Play audio through default output
play_test_audio() {
    afplay test_audio_48k.wav &
    echo "Playing test audio (PID: $!)"
}

# List all audio devices
list_audio_devices() {
    system_profiler SPAudioDataType
}

# Check BlackHole installation
check_blackhole() {
    if system_profiler SPAudioDataType | grep -q "BlackHole"; then
        echo "✅ BlackHole installed"
    else
        echo "❌ BlackHole not found"
    fi
}

# Reset audio permissions (requires SIP disable on some macOS versions)
reset_audio_permissions() {
    tccutil reset Microphone com.quaternionstudios.enteract
    tccutil reset ScreenCapture com.quaternionstudios.enteract
    echo "Permissions reset. Restart Enteract."
}

# Main menu
case "$1" in
    generate) generate_test_audio ;;
    play) play_test_audio ;;
    list) list_audio_devices ;;
    check) check_blackhole ;;
    reset) reset_audio_permissions ;;
    *)
        echo "Usage: $0 {generate|play|list|check|reset}"
        exit 1
        ;;
esac
```

### Test Data Preparation

**Audio Test Files:**

Create `test_assets/audio/` directory with:

1. **speech_male_48k.wav** - 30s male speech at 48kHz stereo
2. **speech_female_44k.wav** - 30s female speech at 44.1kHz stereo
3. **music_classical.wav** - 30s classical music (multi-frequency)
4. **silence.wav** - 10s digital silence
5. **noise_white.wav** - 10s white noise
6. **tone_440hz.wav** - 10s 440 Hz sine wave

Generate with:
```bash
cd test_assets/audio/

# Male speech (use any speech sample)
# Download from: https://www.voiptroubleshooter.com/open_speech/
# Or record manually

# Generate tones and noise
sox -n -r 48000 -c 2 tone_440hz.wav synth 10 sine 440
sox -n -r 48000 -c 2 noise_white.wav synth 10 whitenoise
sox -n -r 48000 -c 2 silence.wav synth 10 sine 0
```

---

## 2. Test Cases for CPAL Integration

### 2.1 Device Enumeration Tests

**Test Suite:** `tests/macos/device_enumeration_tests.rs`

```rust
#[cfg(test)]
mod device_enumeration_tests {
    use cpal::traits::HostTrait;

    #[test]
    fn test_enumerate_all_devices() {
        let host = cpal::default_host();
        let devices: Vec<_> = host.input_devices()
            .expect("Failed to enumerate devices")
            .collect();

        assert!(!devices.is_empty(), "No input devices found");

        for device in devices {
            println!("Device: {:?}", device.name());
        }
    }

    #[test]
    fn test_detect_virtual_devices() {
        let host = cpal::default_host();
        let devices: Vec<_> = host.input_devices()
            .expect("Failed to enumerate devices")
            .collect();

        let blackhole_found = devices.iter().any(|d| {
            d.name().unwrap_or_default().contains("BlackHole")
        });

        if blackhole_found {
            println!("✅ BlackHole detected");
        } else {
            println!("⚠️ BlackHole not installed (expected for virtual device tests)");
        }
    }

    #[test]
    fn test_default_device_exists() {
        let host = cpal::default_host();
        let default_input = host.default_input_device();

        assert!(default_input.is_some(), "No default input device");
        println!("Default device: {:?}", default_input.unwrap().name());
    }

    #[test]
    fn test_device_supported_configs() {
        let host = cpal::default_host();
        let device = host.default_input_device()
            .expect("No default input device");

        let configs: Vec<_> = device.supported_input_configs()
            .expect("Failed to get configs")
            .collect();

        assert!(!configs.is_empty(), "Device has no supported configs");

        for config in configs {
            println!("Config: {:?}", config);
            assert!(config.min_sample_rate().0 > 0);
            assert!(config.max_sample_rate().0 >= config.min_sample_rate().0);
        }
    }
}
```

**Manual Test Checklist:**

- [ ] Enumerate devices with no virtual devices installed
- [ ] Enumerate devices with BlackHole installed
- [ ] Enumerate devices with Loopback installed
- [ ] Hot-plug USB audio interface, verify it appears
- [ ] Hot-unplug USB audio interface, verify it disappears
- [ ] Switch default audio device in System Settings, verify detection
- [ ] Test on macOS 13.6 (should show built-in + virtual devices)
- [ ] Test on macOS 14.6+ (should show loopback devices if supported)

**Expected Results:**

| macOS Version | Built-in Mic | Built-in Speakers (Loopback) | BlackHole | Notes |
|---------------|--------------|------------------------------|-----------|-------|
| 13.6 | ✅ | ❌ | ✅ (if installed) | No loopback support |
| 14.2 | ✅ | ⚠️ (via CoreAudio Taps) | ✅ (if installed) | Taps available but CPAL may not use |
| 14.6+ | ✅ | ✅ (native loopback) | ✅ (if installed) | Full CPAL support |
| 15.2 | ✅ | ✅ (native loopback) | ✅ (if installed) | Same as 14.6+ |

### 2.2 Capture Start/Stop Tests

**Test Suite:** `tests/macos/capture_lifecycle_tests.rs`

```rust
#[cfg(test)]
mod capture_lifecycle_tests {
    use std::sync::{Arc, Mutex};
    use std::time::Duration;
    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

    #[test]
    fn test_basic_capture_start_stop() {
        let host = cpal::default_host();
        let device = host.default_input_device()
            .expect("No input device");

        let config = device.default_input_config()
            .expect("Failed to get config");

        let samples_captured = Arc::new(Mutex::new(0));
        let samples_captured_clone = samples_captured.clone();

        let stream = device.build_input_stream(
            &config.into(),
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                let mut count = samples_captured_clone.lock().unwrap();
                *count += data.len();
            },
            |err| eprintln!("Error: {}", err),
            None
        ).expect("Failed to build stream");

        // Start capture
        stream.play().expect("Failed to start stream");
        std::thread::sleep(Duration::from_secs(2));

        // Stop capture
        stream.pause().expect("Failed to stop stream");

        let final_count = *samples_captured.lock().unwrap();
        assert!(final_count > 0, "No samples captured");

        println!("Captured {} samples in 2 seconds", final_count);
    }

    #[test]
    fn test_multiple_start_stop_cycles() {
        let host = cpal::default_host();
        let device = host.default_input_device()
            .expect("No input device");

        let config = device.default_input_config()
            .expect("Failed to get config");

        for cycle in 1..=5 {
            println!("Cycle {}/5", cycle);

            let stream = device.build_input_stream(
                &config.into(),
                |_data: &[f32], _: &cpal::InputCallbackInfo| {},
                |err| eprintln!("Error: {}", err),
                None
            ).expect("Failed to build stream");

            stream.play().expect("Failed to start");
            std::thread::sleep(Duration::from_millis(500));
            stream.pause().expect("Failed to stop");

            // Small delay between cycles
            std::thread::sleep(Duration::from_millis(100));
        }

        println!("✅ All cycles completed");
    }

    #[test]
    fn test_capture_with_device_busy() {
        // This test verifies graceful handling when device is in use
        // May need to manually trigger (e.g., FaceTime call in progress)

        // TODO: Implement device busy simulation
    }
}
```

**Manual Test Checklist:**

- [ ] Start capture, verify audio callback fires
- [ ] Stop capture, verify callbacks cease
- [ ] Start/stop 10 times rapidly, check for crashes
- [ ] Start capture, close app, verify cleanup
- [ ] Start capture, sleep Mac, wake, verify recovery
- [ ] Start capture on device A, switch to device B, verify switch
- [ ] Start capture while FaceTime active (device busy scenario)

### 2.3 Sample Rate Conversion Tests

**Test Suite:** `tests/macos/sample_rate_tests.rs`

```rust
#[cfg(test)]
mod sample_rate_tests {
    use crate::audio_loopback::audio_processor::process_audio_chunk;

    #[test]
    fn test_48khz_to_16khz_conversion() {
        // Generate 1 second of 48kHz audio (440 Hz sine wave)
        let sample_rate_in = 48000;
        let sample_rate_out = 16000;
        let duration = 1.0; // seconds

        let samples_in = (sample_rate_in as f32 * duration) as usize;
        let mut audio_48k: Vec<i16> = Vec::with_capacity(samples_in * 2); // stereo

        for i in 0..samples_in {
            let t = i as f32 / sample_rate_in as f32;
            let sample = (2.0 * std::f32::consts::PI * 440.0 * t).sin();
            let sample_i16 = (sample * 32767.0) as i16;
            audio_48k.push(sample_i16); // left
            audio_48k.push(sample_i16); // right
        }

        // Convert to bytes
        let audio_bytes: Vec<u8> = audio_48k.iter()
            .flat_map(|&s| s.to_le_bytes())
            .collect();

        // Process through pipeline
        let processed = process_audio_chunk(
            &audio_bytes,
            16,  // bits per sample
            2,   // channels
            sample_rate_in,
            sample_rate_out
        );

        // Verify output length (approximately)
        let expected_samples = (sample_rate_out as f32 * duration) as usize;
        let tolerance = (expected_samples as f32 * 0.05) as usize; // 5% tolerance

        assert!(
            processed.len() >= expected_samples - tolerance &&
            processed.len() <= expected_samples + tolerance,
            "Expected ~{} samples, got {}",
            expected_samples, processed.len()
        );

        // Verify mono output
        assert_eq!(processed.len(), expected_samples);

        println!("✅ 48kHz → 16kHz: {} samples → {} samples",
                 samples_in, processed.len());
    }

    #[test]
    fn test_44khz_to_16khz_conversion() {
        // Similar test for 44.1kHz
        let sample_rate_in = 44100;
        let sample_rate_out = 16000;
        let duration = 1.0;

        // Generate audio...
        // Process...
        // Verify...

        // TODO: Implement
    }

    #[test]
    fn test_dc_offset_removal() {
        // Generate audio with DC offset
        let mut audio: Vec<i16> = vec![1000; 16000]; // 1 second at 16kHz with DC offset

        let audio_bytes: Vec<u8> = audio.iter()
            .flat_map(|&s| s.to_le_bytes())
            .collect();

        let processed = process_audio_chunk(
            &audio_bytes,
            16,
            1, // mono
            16000,
            16000 // no resampling
        );

        // Calculate mean (should be near zero after DC removal)
        let mean: f32 = processed.iter().sum::<f32>() / processed.len() as f32;

        assert!(
            mean.abs() < 0.01,
            "DC offset not removed: mean = {}",
            mean
        );

        println!("✅ DC offset removed: mean = {}", mean);
    }
}
```

**Manual Test Checklist:**

- [ ] Play 48kHz audio, verify output is 16kHz
- [ ] Play 44.1kHz audio, verify output is 16kHz
- [ ] Compare output quality to Windows version (A/B test)
- [ ] Verify stereo → mono conversion (check channel selection logic)
- [ ] Test with DC offset audio file, verify removal
- [ ] Measure audio latency (input to callback)

### 2.4 Audio Quality Comparison Tests

**Test Procedure:**

1. **Setup:**
   - Windows machine with WASAPI implementation
   - Mac with CPAL implementation
   - Same test audio files
   - Same Whisper model

2. **Capture Both:**
   ```bash
   # Windows
   cargo run --release
   # Start capture, play test_audio_48k.wav
   # Save transcription output to windows_output.txt

   # macOS
   cargo run --release
   # Start capture, play test_audio_48k.wav
   # Save transcription output to macos_output.txt
   ```

3. **Compare:**
   ```bash
   # Compare transcription accuracy
   diff -u windows_output.txt macos_output.txt

   # Calculate Word Error Rate (WER)
   python scripts/calculate_wer.py windows_output.txt macos_output.txt
   ```

4. **Audio Analysis:**
   ```bash
   # Capture raw audio to file on both platforms
   # Compare spectrograms
   sox windows_capture.wav -n spectrogram -o windows_spec.png
   sox macos_capture.wav -n spectrogram -o macos_spec.png

   # Compare waveforms
   sox windows_capture.wav -n stat
   sox macos_capture.wav -n stat
   ```

**Expected Results:**
- Transcription accuracy within 5% (WER)
- Spectrograms visually similar
- RMS levels within 3dB
- Frequency content preserved

**Test Matrix:**

| Test Audio | Windows Transcription | macOS Transcription | Match? |
|------------|----------------------|---------------------|--------|
| speech_male_48k.wav | | | ✅/❌ |
| speech_female_44k.wav | | | ✅/❌ |
| music_classical.wav | (music) | (music) | ✅/❌ |
| tone_440hz.wav | (sound) | (sound) | ✅/❌ |

---

## 3. Permission Testing Matrix

### 3.1 Permission States

| Permission | State | Test Action | Expected Behavior |
|------------|-------|-------------|-------------------|
| Audio Capture | Not Determined | Start capture | System prompt appears |
| Audio Capture | Denied | Start capture | Error message, guide to Settings |
| Audio Capture | Granted | Start capture | Capture starts successfully |
| Screen Recording | Not Determined | Start SCK capture | System prompt appears |
| Screen Recording | Denied | Start SCK capture | Error message |
| Screen Recording | Granted | Start SCK capture | Capture starts |

### 3.2 Permission Reset Procedure

**Method 1: Using tccutil (macOS 13+)**

```bash
#!/bin/bash
# scripts/reset_permissions.sh

APP_ID="com.quaternionstudios.enteract"

echo "Resetting permissions for $APP_ID..."

# Reset microphone (for virtual devices)
tccutil reset Microphone "$APP_ID"

# Reset screen recording (for ScreenCaptureKit)
tccutil reset ScreenCapture "$APP_ID"

# Note: Audio capture permission doesn't have a tccutil category yet
# Full reset requires manual deletion from TCC database

echo "✅ Permissions reset. Restart Enteract to test permission prompts."
```

**Method 2: Manual TCC Database Edit (Advanced)**

```bash
# REQUIRES SIP DISABLED - NOT RECOMMENDED FOR ROUTINE TESTING

# Backup TCC database
cp ~/Library/Application\ Support/com.apple.TCC/TCC.db ~/Desktop/TCC.db.backup

# Open database
sqlite3 ~/Library/Application\ Support/com.apple.TCC/TCC.db

# List Enteract entries
SELECT * FROM access WHERE client LIKE '%enteract%';

# Delete entries
DELETE FROM access WHERE client LIKE '%enteract%';

# Quit
.quit

# Restart Mac for changes to take effect
sudo shutdown -r now
```

**Method 3: Clean VM Snapshot (Recommended)**

```bash
# In UTM, restore to "Clean Install" snapshot
# Reinstall Enteract
# Test permission flow from scratch
```

### 3.3 Permission Test Cases

**Test Suite:** `tests/macos/permission_tests.rs`

```rust
#[cfg(test)]
mod permission_tests {
    #[test]
    fn test_permission_not_granted_error() {
        // Start capture when permission not granted
        // Expected: Error with user-friendly message

        let result = start_audio_loopback_capture(device_id, app_handle).await;

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.contains("permission"));
        assert!(error.contains("Settings"));
    }

    #[test]
    fn test_permission_granted_success() {
        // Assumes permission already granted (manual setup)

        let result = start_audio_loopback_capture(device_id, app_handle).await;

        assert!(result.is_ok());
    }
}
```

**Manual Test Checklist:**

- [ ] Fresh install, no permissions → Start capture → Verify prompt appears
- [ ] Click "Don't Allow" → Verify error message with Settings link
- [ ] Grant permission manually → Restart app → Verify capture works
- [ ] Revoke permission → Start capture → Verify error message
- [ ] Test on macOS 13.6 (ScreenCaptureKit permission)
- [ ] Test on macOS 14.6+ (Audio Capture permission)
- [ ] Verify Info.plist contains NSAudioCaptureUsageDescription
- [ ] Verify permission description is user-friendly

### 3.4 Permission UX Recommendations

**Error Message Template:**

```rust
// When permission denied
const PERMISSION_DENIED_MESSAGE: &str =
    "Enteract needs permission to capture system audio.\n\n\
     To grant permission:\n\
     1. Open System Settings\n\
     2. Go to Privacy & Security → Screen & System Audio Recording\n\
     3. Enable Enteract\n\
     4. Restart Enteract\n\n\
     [Open System Settings]  [Cancel]";
```

**UI Flow:**
1. User clicks "Start Capture"
2. If permission not granted, show dialog with above message
3. "Open System Settings" button opens: `x-apple.systempreferences:com.apple.preference.security?Privacy_SystemAudioRecording`
4. Poll for permission grant (or require app restart)

---

## 4. Performance Benchmarks

### 4.1 CPU Usage Measurement

**Test Script:** `scripts/measure_cpu_usage.sh`

```bash
#!/bin/bash

APP_NAME="Enteract"
DURATION=60  # seconds
SAMPLE_INTERVAL=1  # seconds

echo "Measuring CPU usage for $APP_NAME over $DURATION seconds..."

# Start Enteract (assumes already running)
PID=$(pgrep -f "$APP_NAME")

if [ -z "$PID" ]; then
    echo "Error: $APP_NAME not running"
    exit 1
fi

echo "Found $APP_NAME (PID: $PID)"

# Measure CPU usage
OUTPUT_FILE="cpu_usage_$(date +%Y%m%d_%H%M%S).csv"
echo "timestamp,cpu_percent,mem_mb" > "$OUTPUT_FILE"

for i in $(seq 1 $DURATION); do
    CPU=$(ps -p $PID -o %cpu | tail -1 | xargs)
    MEM=$(ps -p $PID -o rss | tail -1 | xargs)
    MEM_MB=$(echo "scale=2; $MEM / 1024" | bc)

    echo "$i,$CPU,$MEM_MB" >> "$OUTPUT_FILE"

    sleep $SAMPLE_INTERVAL
done

echo "✅ Measurement complete: $OUTPUT_FILE"

# Calculate statistics
echo ""
echo "CPU Usage Statistics:"
awk -F',' 'NR>1 {sum+=$2; count++} END {print "Average: " sum/count "%"}' "$OUTPUT_FILE"
awk -F',' 'NR>1 {if($2>max) max=$2} END {print "Peak: " max "%"}' "$OUTPUT_FILE"

echo ""
echo "Memory Usage Statistics:"
awk -F',' 'NR>1 {sum+=$3; count++} END {print "Average: " sum/count " MB"}' "$OUTPUT_FILE"
awk -F',' 'NR>1 {if($3>max) max=$3} END {print "Peak: " max " MB"}' "$OUTPUT_FILE"
```

**Test Scenarios:**

| Scenario | Expected CPU | Expected Memory |
|----------|-------------|-----------------|
| Idle (no capture) | <1% | ~50 MB |
| Capture only (no audio) | <2% | ~50 MB |
| Capture with speech | 3-8% | ~100 MB |
| Capture + transcription | 10-20% | ~150 MB |

**Benchmark on:**
- macOS 14.6 (M1 MacBook Pro)
- macOS 15.2 (M2 MacBook Air)
- Compare to Windows baseline

### 4.2 Memory Leak Detection

**Test with Xcode Instruments:**

```bash
# Build in release mode with debug symbols
cargo build --release

# Run with Instruments
instruments -t Leaks -D leak_report.trace \
    target/release/enteract

# Let run for 30 minutes with continuous capture
# Check report for leaks
```

**Automated Memory Test:**

```rust
#[test]
#[ignore] // Long-running test
fn test_no_memory_leak_over_time() {
    use std::time::{Duration, Instant};

    let start_mem = get_memory_usage();
    let start_time = Instant::now();

    // Run capture for 10 minutes
    while start_time.elapsed() < Duration::from_secs(600) {
        // Simulate capture cycle
        start_capture();
        std::thread::sleep(Duration::from_secs(5));
        stop_capture();
        std::thread::sleep(Duration::from_millis(500));
    }

    let end_mem = get_memory_usage();
    let mem_increase = end_mem - start_mem;

    // Allow 10% memory growth
    assert!(
        mem_increase < start_mem / 10,
        "Memory leaked: {} MB → {} MB (+{} MB)",
        start_mem, end_mem, mem_increase
    );
}

fn get_memory_usage() -> usize {
    // Use process memory info API
    // Return RSS in MB
    todo!()
}
```

**Test Cases:**
- [ ] Capture for 1 hour continuously, verify memory stable
- [ ] Start/stop capture 100 times, verify memory returns to baseline
- [ ] Switch devices 50 times, verify no accumulation
- [ ] Run overnight (8 hours), verify no growth

### 4.3 Latency Measurement

**Test Setup:**

1. Generate test tone (beep) at known timestamp
2. Measure time until audio appears in transcription callback
3. Calculate latency: `callback_time - generation_time`

**Test Script:**

```rust
#[test]
fn test_capture_latency() {
    use std::time::Instant;
    use std::sync::{Arc, Mutex};

    let latencies = Arc::new(Mutex::new(Vec::new()));
    let latencies_clone = latencies.clone();

    let generation_time = Instant::now();

    // Start capture with timestamp tracking
    let stream = device.build_input_stream(
        &config.into(),
        move |data: &[f32], info: &cpal::InputCallbackInfo| {
            let callback_time = Instant::now();
            let latency = callback_time.duration_since(generation_time);

            latencies_clone.lock().unwrap().push(latency.as_millis());
        },
        |err| eprintln!("Error: {}", err),
        None
    ).unwrap();

    stream.play().unwrap();

    // Play test tone
    std::process::Command::new("afplay")
        .arg("test_assets/audio/tone_440hz.wav")
        .spawn()
        .unwrap();

    std::thread::sleep(Duration::from_secs(5));

    let latencies = latencies.lock().unwrap();
    let avg_latency: u128 = latencies.iter().sum::<u128>() / latencies.len() as u128;

    println!("Average latency: {} ms", avg_latency);
    assert!(avg_latency < 100, "Latency too high: {} ms", avg_latency);
}
```

**Target Latency:**
- Capture callback: <50ms
- Audio → transcription: <1000ms (800ms interval + processing)
- Total (audio → UI): <1500ms

---

## 5. Compatibility Validation

### 5.1 macOS Version Compatibility Matrix

| Feature | 13.6 | 14.2 | 14.6 | 15.2 |
|---------|------|------|------|------|
| Device enumeration | ✅ | ✅ | ✅ | ✅ |
| Virtual device capture | ✅ | ✅ | ✅ | ✅ |
| Native loopback (CPAL) | ❌ | ❌ | ✅ | ✅ |
| CoreAudio Taps (manual) | ❌ | ✅ | ✅ | ✅ |
| ScreenCaptureKit | ✅ | ✅ | ✅ | ⚠️ |
| Permission prompts | SCK | Audio | Audio | Audio |

**Test Checklist (per version):**

- [ ] Install Enteract from .dmg
- [ ] Launch application
- [ ] Open audio settings
- [ ] List available devices
- [ ] Start capture on default device
- [ ] Verify audio callback fires
- [ ] Stop capture
- [ ] Play test audio, verify transcription
- [ ] Check console for errors
- [ ] Monitor CPU/memory

### 5.2 Hardware Compatibility

**Test on Different Macs:**

| Mac Model | Processor | Audio Chip | Test Status |
|-----------|-----------|------------|-------------|
| MacBook Air M1 | Apple M1 | Apple | [ ] |
| MacBook Pro M2 | Apple M2 | Apple | [ ] |
| Mac Mini M1 | Apple M1 | Apple | [ ] |
| iMac Intel | Intel i5 | Realtek | [ ] |

**External Audio Interfaces:**

| Interface | Connection | Test Status |
|-----------|------------|-------------|
| Built-in | N/A | [ ] |
| USB Microphone | USB-A | [ ] |
| USB-C Audio Adapter | USB-C | [ ] |
| Bluetooth Headphones | Bluetooth | [ ] |
| Thunderbolt Audio Interface | TB3 | [ ] |

### 5.3 Regression Testing

**Before Each Release:**

```bash
# Run full test suite
cargo test --release

# Run manual test plan
./scripts/run_manual_tests.sh

# Generate test report
./scripts/generate_test_report.sh > test_report_$(date +%Y%m%d).md
```

---

## 6. User Acceptance Testing

### 6.1 UAT Test Plan

**Participants:**
- 3-5 macOS users (mix of technical and non-technical)
- Different macOS versions (13.6, 14.6, 15.2)
- Different use cases (meetings, YouTube, music)

**Test Scenarios:**

**Scenario 1: First-Time Setup**
1. Download and install Enteract
2. Launch application
3. Navigate to audio settings
4. Click "Start Audio Capture"
5. **Expected:** Permission prompt appears (or clear guidance to Settings)
6. Grant permission
7. **Expected:** Capture starts successfully
8. **Metrics:** Time to first successful capture, # of errors

**Scenario 2: Daily Use - Meeting Transcription**
1. Join Zoom/Teams/FaceTime call
2. Start Enteract capture
3. **Expected:** Meeting audio is transcribed in real-time
4. Review transcription accuracy
5. Stop capture
6. **Metrics:** Transcription accuracy (WER), user satisfaction

**Scenario 3: Music Filtering**
1. Play music on Spotify/Apple Music
2. Start Enteract capture
3. **Expected:** Music is detected and filtered (not transcribed)
4. Play speech (podcast, video)
5. **Expected:** Speech is transcribed
6. **Metrics:** False positive rate (music transcribed as speech)

**Scenario 4: Device Switching**
1. Start capture on built-in speakers
2. Play audio
3. Switch to headphones
4. **Expected:** Capture continues on headphones
5. Switch back to speakers
6. **Expected:** Capture switches back
7. **Metrics:** Switch success rate, user confusion

**Feedback Form:**

```
Enteract macOS Audio Capture - User Acceptance Test

User ID: _______
macOS Version: _______
Date: _______

Setup Experience (1-5):
- Installation: [ ]
- Permission granting: [ ]
- First capture: [ ]

Daily Use (1-5):
- Ease of starting capture: [ ]
- Transcription accuracy: [ ]
- Performance (speed, CPU): [ ]

Issues Encountered:
- [ ] Permission issues
- [ ] Capture failed to start
- [ ] Audio quality poor
- [ ] App crashed
- [ ] Other: __________

Overall Satisfaction (1-5): [ ]

Would you use this feature? [ ] Yes [ ] No

Comments:
______________________________________
______________________________________
```

### 6.2 Beta Testing Plan

**Phase 1: Internal Testing (1 week)**
- Enteract team members
- Test all macOS versions
- Fix critical bugs

**Phase 2: Limited Beta (2 weeks)**
- 10-20 external users
- Diverse macOS versions
- Feedback via Discord/email
- Focus on edge cases

**Phase 3: Public Beta (4 weeks)**
- Announce on website/social
- Telemetry enabled (opt-in)
- Monitor crash reports
- Gather feature requests

**Telemetry to Collect (opt-in):**
- macOS version
- Capture success/failure rate
- Average CPU usage
- Crash logs
- Permission grant/deny rate

---

## 7. Continuous Integration Testing

### 7.1 GitHub Actions Workflow

**File:** `.github/workflows/macos_audio_tests.yml`

```yaml
name: macOS Audio Tests

on:
  push:
    branches: [ main, develop ]
  pull_request:
    branches: [ main, develop ]

jobs:
  test-macos:
    strategy:
      matrix:
        os: [macos-13, macos-14, macos-latest]
    runs-on: ${{ matrix.os }}

    steps:
    - uses: actions/checkout@v3

    - name: Install Rust
      uses: actions-rs/toolchain@v1
      with:
        toolchain: stable
        profile: minimal

    - name: Install dependencies
      run: |
        brew install sox ffmpeg

    - name: Run unit tests
      run: cargo test --lib --release

    - name: Run integration tests (no hardware)
      run: cargo test --test macos_audio_tests --release -- --skip hardware

    - name: Build release
      run: cargo build --release

    - name: Upload artifacts
      uses: actions/upload-artifact@v3
      with:
        name: enteract-macos-${{ matrix.os }}
        path: target/release/enteract
```

### 7.2 Automated Test Schedule

**Daily (Nightly Build):**
- Run full unit test suite
- Run integration tests (mocked hardware)
- Build release artifacts
- Report to Slack/Discord

**Weekly:**
- Run manual test checklist on physical hardware
- Review CPU/memory benchmarks
- Check for new macOS updates

**Pre-Release:**
- Full compatibility matrix validation
- Performance regression testing
- UAT with beta testers
- Review all open issues

---

## 8. Test Reporting

### 8.1 Test Report Template

**File:** `test_reports/YYYYMMDD_test_report.md`

```markdown
# macOS Audio Loopback Test Report

**Date:** YYYY-MM-DD
**Tester:** Name
**Build:** v0.1.0-beta
**macOS Version:** 15.2 (Sequoia)
**Hardware:** MacBook Pro M2

## Test Summary

| Category | Pass | Fail | Skip | Total |
|----------|------|------|------|-------|
| Unit Tests | 45 | 0 | 0 | 45 |
| Integration Tests | 12 | 1 | 2 | 15 |
| Manual Tests | 18 | 0 | 0 | 18 |
| **Total** | **75** | **1** | **2** | **78** |

**Pass Rate:** 96.2%

## Issues Found

### #1: Capture fails on macOS 13.6 without BlackHole

**Severity:** High
**Status:** Open
**Description:** On macOS 13.6, if BlackHole is not installed, capture fails with error "No loopback device found"
**Expected:** Should fall back to ScreenCaptureKit or show user guidance
**Actual:** Silent failure
**Reproduction:**
1. macOS 13.6 VM
2. No virtual devices installed
3. Start capture
4. Error in console

**Fix:** Add fallback logic for macOS 13.x

## Performance Metrics

| Metric | Target | Actual | Status |
|--------|--------|--------|--------|
| CPU (idle) | <1% | 0.8% | ✅ |
| CPU (capture) | <5% | 3.2% | ✅ |
| CPU (transcribe) | <20% | 15.1% | ✅ |
| Memory | <100 MB | 87 MB | ✅ |
| Latency | <100ms | 62ms | ✅ |

## Recommendations

1. Fix issue #1 before release
2. Add user guidance for BlackHole installation
3. Consider bundling BlackHole installer
4. Improve error messages for permission denied

## Sign-off

- [ ] All critical issues resolved
- [ ] Performance meets targets
- [ ] Ready for beta release

Tester Signature: _____________
Date: __________
```

### 8.2 Bug Report Template

**File:** `.github/ISSUE_TEMPLATE/macos_audio_bug.md`

```markdown
---
name: macOS Audio Bug Report
about: Report a bug in macOS audio loopback
title: '[macOS Audio] '
labels: bug, macos, audio
assignees: ''
---

**macOS Version:**
- [ ] macOS 13.6 (Ventura)
- [ ] macOS 14.2 (Sonoma)
- [ ] macOS 14.6 (Sonoma)
- [ ] macOS 15.2 (Sequoia)
- [ ] Other: __________

**Hardware:**
- Mac Model:
- Processor:
- Audio Devices:

**Enteract Version:**
v0.1.0-beta

**Description:**
A clear description of the bug.

**Steps to Reproduce:**
1.
2.
3.

**Expected Behavior:**
What should happen.

**Actual Behavior:**
What actually happens.

**Console Logs:**
```
Paste relevant console output here
```

**Screenshots:**
If applicable, add screenshots.

**Permissions:**
- [ ] Audio Capture permission granted
- [ ] Screen Recording permission granted (for macOS 13.x)
- [ ] Microphone permission granted

**Additional Context:**
Any other context about the problem.
```

---

## 9. Test Automation Scripts

### 9.1 Master Test Runner

**File:** `scripts/run_all_tests.sh`

```bash
#!/bin/bash

set -e  # Exit on error

echo "================================================"
echo "  Enteract macOS Audio Loopback Test Suite"
echo "================================================"
echo ""

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Test results
TOTAL_TESTS=0
PASSED_TESTS=0
FAILED_TESTS=0

run_test() {
    local test_name="$1"
    local test_command="$2"

    echo -n "Running: $test_name... "
    TOTAL_TESTS=$((TOTAL_TESTS + 1))

    if eval "$test_command" > /tmp/test_output.log 2>&1; then
        echo -e "${GREEN}PASS${NC}"
        PASSED_TESTS=$((PASSED_TESTS + 1))
    else
        echo -e "${RED}FAIL${NC}"
        FAILED_TESTS=$((FAILED_TESTS + 1))
        echo "Error log:"
        cat /tmp/test_output.log
    fi
}

echo "1. Unit Tests"
echo "-------------"
run_test "Device enumeration tests" "cargo test device_enumeration_tests --release"
run_test "Capture lifecycle tests" "cargo test capture_lifecycle_tests --release"
run_test "Sample rate tests" "cargo test sample_rate_tests --release"

echo ""
echo "2. Integration Tests"
echo "--------------------"
run_test "End-to-end capture test" "cargo test --test integration_tests --release"

echo ""
echo "3. Manual Tests (interactive)"
echo "-----------------------------"
echo -e "${YELLOW}Note: Manual tests require user interaction${NC}"

./scripts/manual_test_guide.sh

echo ""
echo "4. Performance Tests"
echo "--------------------"
echo "Starting 60-second CPU measurement..."
./scripts/measure_cpu_usage.sh

echo ""
echo "================================================"
echo "  Test Results Summary"
echo "================================================"
echo "Total tests: $TOTAL_TESTS"
echo -e "Passed: ${GREEN}$PASSED_TESTS${NC}"
echo -e "Failed: ${RED}$FAILED_TESTS${NC}"
echo ""

if [ $FAILED_TESTS -eq 0 ]; then
    echo -e "${GREEN}✅ All tests passed!${NC}"
    exit 0
else
    echo -e "${RED}❌ Some tests failed${NC}"
    exit 1
fi
```

### 9.2 Quick Sanity Check

**File:** `scripts/sanity_check.sh`

```bash
#!/bin/bash

echo "Quick Sanity Check for macOS Audio"

# Check macOS version
MACOS_VERSION=$(sw_vers -productVersion)
echo "macOS Version: $MACOS_VERSION"

# Check architecture
ARCH=$(uname -m)
echo "Architecture: $ARCH"

# Check if BlackHole installed
if system_profiler SPAudioDataType | grep -q "BlackHole"; then
    echo "✅ BlackHole: Installed"
else
    echo "⚠️  BlackHole: Not installed (needed for macOS 13.x testing)"
fi

# Check Rust
if command -v cargo &> /dev/null; then
    RUST_VERSION=$(cargo --version)
    echo "✅ Rust: $RUST_VERSION"
else
    echo "❌ Rust: Not installed"
    exit 1
fi

# Build project
echo "Building project..."
if cargo build --release; then
    echo "✅ Build: Success"
else
    echo "❌ Build: Failed"
    exit 1
fi

# Run basic device enum test
echo "Testing device enumeration..."
if cargo test test_enumerate_all_devices --release -- --nocapture; then
    echo "✅ Device Enumeration: Working"
else
    echo "❌ Device Enumeration: Failed"
    exit 1
fi

echo ""
echo "✅ Sanity check complete!"
```

---

## 10. Troubleshooting Guide

### Common Issues

| Issue | Cause | Solution |
|-------|-------|----------|
| "No input devices found" | macOS virtualization issue | Verify UTM audio enabled |
| Permission prompt doesn't appear | Info.plist missing key | Add NSAudioCaptureUsageDescription |
| Capture starts but no audio | Wrong device selected | Check device enumeration |
| High CPU usage | Inefficient resampling | Profile with Instruments |
| Memory leak | Buffer not freed | Run leak detection |
| Crash on start | Missing dependency | Check linked libraries |

### Debug Commands

```bash
# List all audio devices
system_profiler SPAudioDataType

# Check app permissions
tccutil list Microphone
tccutil list ScreenCapture

# Monitor real-time audio
sudo fs_usage -w -f filesys AudioToolbox

# Check for audio glitches
sudo dtruss -n AudioToolbox

# Profile CPU
sample Enteract 10 -file cpu_profile.txt
```

---

## Appendix: Test Checklists

### Pre-Implementation Checklist

- [ ] Set up macOS 13.6 VM
- [ ] Set up macOS 14.2 VM
- [ ] Set up macOS 14.6 VM
- [ ] Set up macOS 15.2 VM
- [ ] Install BlackHole on all VMs
- [ ] Install test audio files
- [ ] Install test scripts
- [ ] Create snapshot for each VM
- [ ] Document baseline performance (Windows)

### Post-Implementation Checklist

- [ ] All unit tests pass
- [ ] All integration tests pass
- [ ] Manual test plan executed
- [ ] Performance benchmarks meet targets
- [ ] No memory leaks detected
- [ ] Compatibility matrix validated
- [ ] UAT completed with positive feedback
- [ ] Documentation updated
- [ ] Release notes drafted

---

**Document Version:** 1.0
**Last Updated:** January 25, 2026
**Author:** jack (enteract crew)
**Status:** Ready for Phase 3 Implementation
