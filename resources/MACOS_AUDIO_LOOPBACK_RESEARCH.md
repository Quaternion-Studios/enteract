# macOS Audio Loopback Migration Research

**Date:** January 25, 2026
**Author:** jack (enteract crew)
**Purpose:** Research CoreAudio approaches for migrating Windows WASAPI loopback to macOS

---

## Executive Summary

This document provides comprehensive research for migrating Enteract's audio loopback system from Windows (WASAPI) to macOS (CoreAudio). The research covers available Rust crates, CoreAudio capture methods, compatibility requirements, and implementation recommendations.

### Key Findings

1. **CoreAudio Taps API** (macOS 14.2+) is the modern, official solution for system audio capture
2. **CPAL 0.17.1** provides native loopback support for macOS 14.6+ with cross-platform abstraction
3. **Virtual audio devices** (BlackHole) remain necessary for macOS 13.x support
4. Permission requirements are more restrictive than Windows (NSAudioCaptureUsageDescription needed)

---

## Current Windows Implementation

### Architecture Overview

The Windows implementation uses **WASAPI (Windows Audio Session API)** with loopback capture:

**Key Components:**
- `device_enumerator.rs` - Enumerate audio devices via WASAPI
- `capture_engine.rs` - Main capture loop using WASAPI loopback
- `audio_processor.rs` - Audio processing pipeline (resampling, DC removal, quality filtering)
- `quality_filter.rs` - Transcription quality estimation
- `types.rs` - Shared data structures

**Dependencies:**
```toml
wasapi = "0.13"
rubato = "0.15"  # Audio resampling
hound = "3.5"    # WAV file handling
```

### Capture Flow

1. **Device Enumeration**: Scan render devices and test loopback capability
2. **Capture Initialization**:
   - Initialize WASAPI with `Direction::Capture` + `use_loopback=true`
   - Configure 16kHz target sample rate for Whisper
3. **Audio Processing Pipeline**:
   - Capture raw audio (16/32-bit, stereo)
   - Convert to mono (intelligently select channel)
   - DC offset removal
   - Resample to 16kHz for Whisper
   - Buffer 4 seconds of audio
   - Transcribe every 800ms (with 1s overlap)
4. **Quality Filtering**: Filter out music, silence markers, repetitive text

### Key Requirements to Replicate

| Feature | Windows Implementation | macOS Equivalent |
|---------|----------------------|------------------|
| System audio capture | WASAPI render loopback | CoreAudio Taps / ScreenCaptureKit |
| Device enumeration | `DeviceCollection::new(&Direction::Render)` | `AudioObjectGetPropertyData(kAudioHardwarePropertyDevices)` |
| Sample format | 16/32-bit PCM, 44.1kHz/48kHz | Same (via AudioStreamBasicDescription) |
| Target output | 16kHz mono for Whisper | Same |
| Real-time processing | Event-driven callbacks | IOProc callbacks / SCStream handlers |
| Permissions | None required | NSAudioCaptureUsageDescription + user grant |

---

## macOS CoreAudio Capture Approaches

### 1. CoreAudio Taps API (macOS 14.2+) ⭐ RECOMMENDED

**Official Apple solution introduced December 2023**

#### How It Works
- `AudioHardwareCreateProcessTap` creates a virtual tap into system audio
- Can capture global audio or filter by process
- Creates aggregate device that appears as audio input
- IOProc callbacks deliver audio data

#### Sample Code Pattern
```c
// 1. Create tap description
CATapDescription* tap = [[CATapDescription alloc]
    initStereoGlobalTapButExcludeProcesses:excluded_pids];
[tap setMuteBehavior:CATapUnmuted];
[tap setPrivate:YES];

// 2. Create process tap
AudioObjectID tap_id;
AudioHardwareCreateProcessTap(tap, &tap_id);

// 3. Create aggregate device with tap
NSDictionary* properties = @{
    kAudioAggregateDeviceTapListKey: @[@(tap_id)],
    kAudioAggregateDeviceIsPrivateKey: @YES
};
AudioDeviceID device_id;
AudioHardwareCreateAggregateDevice(properties, &device_id);

// 4. Set up IOProc callback
AudioDeviceCreateIOProcID(device_id, audio_callback, context, &proc_id);
AudioDeviceStart(device_id, proc_id);
```

#### Pros
- Official Apple API with ongoing support
- No virtual audio driver needed
- Can filter by process
- Low latency
- Audio-only permissions (less invasive than screen recording)

#### Cons
- **Requires macOS 14.2+** (December 2023)
- Poorly documented (AudioCap project exists as documentation)
- Permission handling is manual (no public API to request/check)
- Volume bug with >2 channel devices
- Complex setup (multiple CoreAudio APIs involved)

#### Rust Implementation Path
- Use `objc2-core-audio` for CATapDescription bindings
- Wrap unsafe CoreAudio calls in safe Rust API
- Reference: [AudioCap Swift implementation](https://github.com/insidegui/AudioCap)

---

### 2. CPAL (Cross-Platform Audio Library) ⭐ RECOMMENDED

**High-level Rust audio library with native macOS loopback support**

#### Version: 0.17.1 (January 4, 2026)

#### How It Works
- Abstracts CoreAudio, WASAPI, ALSA behind unified API
- macOS backend uses AudioUnit + CoreAudio Taps on 14.6+
- Provides device enumeration, stream configuration, callback-based capture

#### Sample Code Pattern
```rust
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

// Enumerate loopback devices
let host = cpal::default_host();
let device = host.default_input_device()
    .expect("No input device available");

// Configure stream
let config = device.default_input_config()?;

// Create capture stream
let stream = device.build_input_stream(
    &config.into(),
    |data: &[f32], _: &cpal::InputCallbackInfo| {
        // Process audio samples
    },
    |err| eprintln!("Stream error: {}", err),
    None
)?;

stream.play()?;
```

#### Pros
- **Cross-platform** - same API for Windows, macOS, Linux
- Active development (RustAudio organization)
- Native loopback support on macOS 14.6+
- Well-documented with examples
- Handles device hot-plugging
- Simplifies migration (similar patterns to WASAPI)

#### Cons
- Loopback only works on macOS 14.6+ (requires fallback for older OS)
- Less control than raw CoreAudio
- Still requires screen recording permissions (inherited from underlying ScreenCaptureKit)
- ScreenCaptureKit backend in PR, not yet stable

#### Migration Complexity: LOW-MEDIUM
- Replace `wasapi` crate with `cpal`
- Adapt device enumeration to CPAL API
- Keep existing audio processing pipeline
- Add macOS version detection for fallback strategy

---

### 3. ScreenCaptureKit (macOS 13.0+)

**Apple's screen recording framework with audio capture**

#### How It Works
- Designed for screen + audio recording
- `SCStreamConfiguration` with `.capturesAudio(true)`
- Delivers audio via `CMSampleBuffer` callbacks
- Can filter by application/window

#### Rust Crate: `screencapturekit-rs` v1.5.0

#### Sample Code Pattern
```rust
// Configure stream
let config = SCStreamConfiguration::new();
config.set_captures_audio(true);
config.set_sample_rate(48000);
config.set_channel_count(2);

// Create stream
let stream = SCStream::new(filter, config, error_handler);

// Add audio handler
stream.add_output_handler(
    audio_handler,
    SCStreamOutputType::Audio,
    queue
);

stream.start_capture()?;
```

#### Pros
- Works on macOS 13.0+ (earlier than CoreAudio Taps)
- Per-application audio capture
- Official Apple framework
- Good for screen + audio workflows

#### Cons
- **Requires screen recording permissions** (more invasive than audio-only)
- Designed for screen capture, not audio-only
- Less efficient for audio-only use cases
- Known issues on macOS 15 (SCStreamErrorDomain -3805)
- "Audio samples not received or empty" issues reported

#### Apple Recommendation
> "If you are not capturing the screen and only capturing audio, it would be best to use a Core Audio tap than ScreenCaptureKit."
> — Apple Developer Forums

#### Use Case: Fallback for macOS 13.x when user needs audio capture

---

### 4. Virtual Audio Devices (All macOS Versions)

**Use AudioUnit/AudioQueue to capture from virtual audio driver**

#### How It Works
- Install virtual audio driver (BlackHole, Loopback, SoundFlower)
- Driver creates virtual input device
- macOS routes system audio through virtual device
- Capture from virtual device like any microphone

#### Detection Code Pattern
```rust
// Check transport type
let transport_type = get_device_property(
    device_id,
    kAudioDevicePropertyTransportType
);

if transport_type == kAudioDeviceTransportTypeVirtual {
    // This is a virtual device
    // Check manufacturer for specific drivers:
    // - "Existential Audio Inc." = BlackHole
    // - "ma++ ingalls for Cycling '74" = SoundFlower
}
```

#### Recommended Virtual Device: **BlackHole**
- Actively maintained (2024 updates)
- Open source
- 2ch and 16ch versions
- macOS 10.10+ support
- Free

#### Pros
- Works on all macOS versions (10.10+)
- No special permissions beyond microphone access
- Well-tested and stable
- Simple implementation (standard audio input)

#### Cons
- Requires user to install third-party driver
- User must manually route audio
- Not zero-configuration
- Additional support burden

#### Use Case: Fallback for macOS < 14.2, or user preference

---

## Rust Crate Analysis

### Top Recommendations

| Crate | Version | Best For | macOS Support | Loopback |
|-------|---------|----------|---------------|----------|
| **cpal** | 0.17.1 | Cross-platform abstraction | 14.6+ native | ✅ Built-in |
| **objc2-core-audio** | 0.3.1 | Low-level CoreAudio control | All versions | ⚙️ DIY |
| **screencapturekit-rs** | 1.5.0 | Screen + audio capture | 13.0+ | ✅ Built-in |
| **coreaudio-rs** | 0.14.0 | AudioUnit wrappers | All versions | ⚙️ DIY |
| **audiotee** | 0.1.0 | Quick prototyping | 14.2+ | ✅ Wrapper |

### Detailed Crate Breakdown

#### CPAL (Cross-Platform Audio Library)
- **Downloads:** 6M+ all-time
- **Last Update:** January 4, 2026
- **Maintainer:** RustAudio organization
- **CoreAudio Backend:** Uses AudioUnit + CoreAudio Taps
- **Loopback:** Native support on macOS 14.6+ (PR #894)
- **Examples:** `cargo run --example enumerate`, `record_wav`, `feedback`
- **Recommendation:** **PRIMARY CHOICE** for cross-platform compatibility

#### objc2-core-audio
- **Version:** 0.3.1
- **Maintainer:** objc2 project
- **Features:** Modern Objective-C bindings for CoreAudio
- **Includes:** CATapDescription, AudioHardware, AudioServerPlugIn
- **Loopback:** Full access to AudioHardwareCreateProcessTap
- **Recommendation:** Use if you need low-level tap control or CPAL doesn't meet needs

#### coreaudio-rs
- **Downloads:** 6M+ all-time
- **Last Update:** January 13, 2026
- **Maintainer:** RustAudio organization
- **Focus:** AudioUnit wrappers
- **Loopback:** No direct support, must implement manually
- **Recommendation:** Alternative to objc2-core-audio for AudioUnit-based approach

#### screencapturekit-rs
- **Version:** 1.5.0
- **Last Update:** December 20, 2025
- **macOS Required:** 12.3+ (audio requires 13.0+)
- **Loopback:** Built-in system audio capture
- **Permissions:** Screen recording required
- **Recommendation:** Fallback for macOS 13.x or screen+audio workflows

#### audiotee
- **Version:** 0.1.0
- **Last Update:** November 5, 2025
- **Type:** Wrapper around `audiotee` CLI tool
- **Loopback:** Full CoreAudio Taps support
- **Limitation:** Requires external binary
- **Recommendation:** Prototyping only, not production

---

## Recommended Implementation Strategy

### Tiered Approach Based on macOS Version

```rust
match macos_version {
    14.6+ => {
        // Primary: CPAL with native loopback
        use_cpal_loopback()
    }
    14.2..14.6 => {
        // Alternative 1: CoreAudio Taps via objc2
        // Alternative 2: Virtual device detection
        try_coreaudio_taps().or_else(|| detect_virtual_device())
    }
    13.0..14.2 => {
        // ScreenCaptureKit or virtual device
        try_screencapturekit().or_else(|| detect_virtual_device())
    }
    _ => {
        // Virtual device only (BlackHole, Loopback)
        detect_virtual_device()
    }
}
```

### Phase 1: CPAL Migration (macOS 14.6+)

**Goal:** Replace WASAPI with CPAL for cross-platform consistency

**Benefits:**
- Minimal code changes (similar API patterns)
- Cross-platform testing becomes easier
- Future-proof for Linux support
- Active maintenance

**Implementation:**
1. Add `cpal = "0.17"` to Cargo.toml
2. Create `macos_loopback_enumerator.rs` using CPAL device enumeration
3. Create `macos_capture_engine.rs` using CPAL input streams
4. Reuse existing `audio_processor.rs` pipeline
5. Add macOS version detection
6. Implement permission checking/requesting

**Estimated Complexity:** Medium (API differences, permission handling)

---

### Phase 2: CoreAudio Taps Fallback (macOS 14.2-14.5)

**Goal:** Support newer macOS without CPAL's 14.6 requirement

**Implementation:**
1. Add `objc2-core-audio = "0.3"` to Cargo.toml
2. Implement tap creation wrapper
3. Implement aggregate device creation
4. Implement IOProc callback handler
5. Convert AudioStreamBasicDescription to Rust types
6. Handle tap lifecycle (creation, cleanup)

**Reference Implementation:** [AudioCap](https://github.com/insidegui/AudioCap) (Swift, translate to Rust)

**Estimated Complexity:** High (unsafe code, CoreAudio expertise needed)

---

### Phase 3: Legacy Support (macOS 13.x)

**Options:**

**Option A: ScreenCaptureKit**
- Add `screencapturekit = "1.5"` to Cargo.toml
- Implement SCStream configuration
- Implement audio sample handler
- Request screen recording permissions

**Option B: Virtual Device Detection**
- Enumerate audio devices
- Filter by `kAudioDeviceTransportTypeVirtual`
- Detect BlackHole/Loopback by manufacturer
- Use standard AudioUnit capture
- Provide user instructions for driver installation

**Recommendation:** Option B (virtual device) for simplicity and user control

---

## Permission Requirements

### macOS 14.2+ (CoreAudio Taps)

**Info.plist Entry:**
```xml
<key>NSAudioCaptureUsageDescription</key>
<string>Enteract needs to capture system audio for transcription</string>
```

**User Permission:**
- Settings → Privacy & Security → Screen & System Audio Recording
- Prompt appears on first capture attempt
- No public API to pre-request permission
- Full app restart often required after granting

**Entitlements (for notarized apps):**
```xml
<key>com.apple.security.device.audio-input</key>
<true/>
```

### macOS 13.x (ScreenCaptureKit)

**Info.plist Entry:**
```xml
<key>NSScreenCaptureUsageDescription</key>
<string>Enteract needs screen recording access to capture system audio</string>
```

**User Permission:**
- Settings → Privacy & Security → Screen Recording
- More invasive than audio-only permission

### Virtual Audio Device

**Info.plist Entry:**
```xml
<key>NSMicrophoneUsageDescription</key>
<string>Enteract needs microphone access to capture audio</string>
```

**User Actions:**
1. Install BlackHole driver
2. Configure Audio MIDI Setup (optional)
3. Grant microphone permission to Enteract

---

## Compatibility Matrix

| macOS Version | CoreAudio Taps | ScreenCaptureKit | CPAL Loopback | Virtual Device |
|---------------|----------------|------------------|---------------|----------------|
| 13.0-13.6 | ❌ | ✅ | ❌ | ✅ |
| 14.0-14.1 | ❌ | ✅ | ❌ | ✅ |
| 14.2-14.5 | ✅ | ✅ | ❌ | ✅ |
| 14.6+ | ✅ | ✅ | ✅ | ✅ |
| 15.0+ | ✅ | ⚠️ (known issues) | ✅ | ✅ |

**Legend:**
- ✅ Fully supported
- ⚠️ Supported with known issues
- ❌ Not available

---

## Known Gotchas and Risks

### 1. macOS Version Fragmentation
**Risk:** Different APIs required for different OS versions
**Mitigation:** Implement tiered approach with runtime version detection

### 2. Permission Handling Complexity
**Risk:** No API to programmatically request audio capture permission
**Mitigation:**
- Provide clear user instructions
- Detect permission state by attempting capture and catching errors
- Guide users to System Settings

### 3. CPAL Loopback Limitations
**Risk:** CPAL loopback only works on 14.6+, released July 2024
**Mitigation:** Implement fallback for 14.2-14.5 using direct CoreAudio Taps

### 4. Virtual Device Installation Burden
**Risk:** Users must install third-party drivers for full compatibility
**Mitigation:**
- Make virtual device optional
- Auto-detect if already installed
- Provide one-click installer link
- Clear documentation

### 5. CoreAudio Taps Documentation
**Risk:** Apple's documentation is sparse, community examples in Swift/Objective-C
**Mitigation:**
- Reference AudioCap and audiotee implementations
- Extensive testing on multiple macOS versions
- Build internal documentation during implementation

### 6. Audio Quality with Multi-Channel Devices
**Risk:** Volume halving bug with >2 channel devices
**Mitigation:** Implement channel compensation logic (reference AudioCap)

### 7. ScreenCaptureKit Issues on macOS 15
**Risk:** SCStreamErrorDomain -3805 errors reported
**Mitigation:** Prefer CoreAudio Taps on macOS 14.2+, use SCK only as fallback

### 8. Breaking Changes in Future macOS
**Risk:** Apple may change CoreAudio Taps behavior
**Mitigation:**
- Abstract audio capture behind trait
- Monitor Apple developer forums
- Maintain virtual device fallback

---

## Testing Strategy

### Test Matrix

| Scenario | macOS 13.6 | macOS 14.2 | macOS 14.6 | macOS 15.0 |
|----------|-----------|-----------|-----------|-----------|
| System audio capture | Virtual Device | CoreAudio Taps | CPAL | CPAL |
| Device enumeration | Standard | Standard | CPAL | CPAL |
| Sample rate conversion | ✅ | ✅ | ✅ | ✅ |
| Mono conversion | ✅ | ✅ | ✅ | ✅ |
| Whisper transcription | ✅ | ✅ | ✅ | ✅ |
| Permission handling | Mic | Audio Capture | Audio Capture | Audio Capture |

### Test Devices

**Required Hardware:**
- Mac with built-in speakers (test default device)
- Mac with external speakers (test device switching)
- Mac with >2 channel audio (test volume compensation)

**Virtual Devices:**
- BlackHole 2ch (most common)
- BlackHole 16ch (multi-channel test)
- Loopback (if available)

### Test Cases

1. **Device Enumeration**
   - List all available input devices
   - Identify virtual devices by transport type
   - Detect BlackHole/Loopback by manufacturer
   - Handle device hot-plug/unplug

2. **Audio Capture**
   - Capture system audio during music playback
   - Capture system audio during video playback
   - Verify sample rate conversion (48kHz → 16kHz)
   - Verify mono conversion from stereo
   - Verify DC offset removal

3. **Transcription Pipeline**
   - Capture 4-second buffer
   - Transcribe every 800ms
   - Filter music/silence markers
   - Emit to frontend

4. **Permission Handling**
   - Detect when permission not granted
   - Guide user to System Settings
   - Retry after permission granted
   - Handle permission revocation

5. **Error Handling**
   - No audio devices available
   - Audio device in use by another app
   - Permission denied
   - macOS version too old

---

## Performance Considerations

### Expected Performance (based on Windows implementation)

| Metric | Target | Notes |
|--------|--------|-------|
| Capture latency | <100ms | Event-driven callbacks |
| Buffer duration | 4 seconds | For Whisper context |
| Transcription interval | 800ms | Matches Windows |
| Sample rate | 16kHz output | Whisper requirement |
| CPU usage | <5% | Idle capture, <15% during transcription |
| Memory usage | ~50MB | Audio buffers + Whisper model |

### Optimization Opportunities

1. **Resampling**: Consider using `rubato` on macOS like Windows (already in dependencies)
2. **Buffer Management**: Reuse buffers instead of allocating
3. **Transcription Queue**: Offload to separate thread pool
4. **Device Switching**: Cache device list, update on notification

---

## Recommended Dependencies

### Primary Implementation (CPAL-based)

```toml
[target.'cfg(target_os = "macos")'.dependencies]
cpal = "0.17"                    # Primary audio abstraction
objc2-core-audio = "0.3"         # For CoreAudio Taps fallback
core-foundation = "0.10"         # macOS version detection
libc = "0.2"                     # System calls

# Keep existing cross-platform deps
rubato = "0.15"                  # Audio resampling
hound = "3.5"                    # WAV handling
```

### Alternative Implementation (CoreAudio-first)

```toml
[target.'cfg(target_os = "macos")'.dependencies]
objc2-core-audio = "0.3"         # CoreAudio Taps
coreaudio-rs = "0.14"            # AudioUnit wrappers
screencapturekit = "1.5"         # Fallback for macOS 13.x
core-foundation = "0.10"         # Version detection

rubato = "0.15"
hound = "3.5"
```

---

## Migration Checklist

### Pre-Implementation
- [ ] Verify macOS version distribution of users (Tauri analytics)
- [ ] Decide on CPAL-first vs CoreAudio-first approach
- [ ] Set up macOS test machines (13.x, 14.2, 14.6, 15.x)
- [ ] Install BlackHole for virtual device testing
- [ ] Review AudioCap source code for patterns

### Phase 1: CPAL Migration (14.6+)
- [ ] Add CPAL dependency
- [ ] Implement device enumeration
- [ ] Implement capture engine with callbacks
- [ ] Port audio processing pipeline
- [ ] Add macOS version detection
- [ ] Implement permission checking
- [ ] Test on macOS 14.6+
- [ ] Test sample rate conversion
- [ ] Test Whisper transcription
- [ ] Update UI for permission instructions

### Phase 2: CoreAudio Taps Fallback (14.2-14.5)
- [ ] Add objc2-core-audio dependency
- [ ] Implement tap creation wrapper
- [ ] Implement aggregate device creation
- [ ] Implement IOProc callback
- [ ] Handle ASBD conversion
- [ ] Test on macOS 14.2-14.5
- [ ] Document permission flow

### Phase 3: Legacy Support (13.x)
- [ ] Implement virtual device detection
- [ ] Add BlackHole manufacturer/UID detection
- [ ] Implement AudioUnit capture from virtual device
- [ ] OR: Implement ScreenCaptureKit fallback
- [ ] Test on macOS 13.x
- [ ] Document driver installation process

### Testing & Validation
- [ ] Test all macOS versions (13.x, 14.2, 14.6, 15.x)
- [ ] Test device hot-plugging
- [ ] Test permission denial/grant flow
- [ ] Test audio quality (compare to Windows)
- [ ] Load testing (continuous capture for hours)
- [ ] Memory leak testing
- [ ] Test with various audio sources (YouTube, Spotify, FaceTime)

### Documentation
- [ ] User guide for permissions
- [ ] Virtual device installation guide
- [ ] Troubleshooting guide
- [ ] API documentation
- [ ] Update README with macOS requirements

---

## Timeline Estimate

**Assumptions:**
- Single developer (crew member)
- 4-6 hours/day focused work
- Testing on multiple macOS versions available

| Phase | Description | Estimated Time |
|-------|-------------|----------------|
| Research | ✅ Complete | 1 day |
| CPAL Migration | Device enum + capture | 3-4 days |
| CoreAudio Taps | Fallback for 14.2-14.5 | 4-5 days |
| Legacy Support | Virtual device or SCK | 2-3 days |
| Testing | All versions + edge cases | 3-4 days |
| Documentation | User guides + API docs | 1-2 days |
| **Total** | | **14-19 days** |

**Critical Path:**
1. CPAL migration (foundational)
2. Testing on 14.6+
3. CoreAudio Taps fallback
4. Comprehensive testing

---

## Decision Matrix

### Should We Use CPAL or Direct CoreAudio?

| Factor | CPAL | Direct CoreAudio |
|--------|------|------------------|
| Cross-platform code reuse | ✅ High | ❌ Low |
| Learning curve | ✅ Low | ❌ High |
| Control over implementation | ⚠️ Medium | ✅ High |
| macOS version support | ⚠️ 14.6+ native | ✅ 14.2+ |
| Maintenance burden | ✅ Low | ❌ High |
| Performance | ✅ Good | ✅ Excellent |
| Audio quality control | ⚠️ Medium | ✅ High |
| Community support | ✅ RustAudio org | ⚠️ Limited |

**Recommendation:** **Start with CPAL**, add direct CoreAudio fallback as needed

### Rationale:
1. CPAL provides similar API patterns to WASAPI (easier migration)
2. Cross-platform testing becomes feasible
3. Active maintenance by RustAudio organization
4. Can always drop down to CoreAudio for 14.2-14.5 support
5. Reduces platform-specific code complexity

---

## Open Questions

1. **What is the actual macOS version distribution of Enteract users?**
   - Determines which fallbacks are necessary
   - May simplify implementation if most users on 14.6+

2. **Should we bundle BlackHole with the installer?**
   - Licensing: MIT (compatible)
   - Size: ~1MB
   - User experience: One-click install vs manual download

3. **How should we handle permission denial?**
   - Block feature entirely?
   - Show persistent banner?
   - Provide one-click path to System Settings?

4. **Should we support per-application audio filtering?**
   - CoreAudio Taps supports this
   - WASAPI doesn't (Windows limitation)
   - Would this be a macOS-exclusive feature?

5. **What's the migration timeline pressure?**
   - Is this blocking a release?
   - Can we ship Windows-only first, macOS later?

---

## References

### Official Apple Documentation
- [Capturing system audio with Core Audio taps](https://developer.apple.com/documentation/CoreAudio/capturing-system-audio-with-core-audio-taps)
- [AudioHardwareCreateProcessTap](https://developer.apple.com/documentation/coreaudio/audiohardwarecreateprocesstap)
- [ScreenCaptureKit](https://developer.apple.com/documentation/screencapturekit/)
- [Technical Note TN2091: HAL Output Audio Unit](https://developer.apple.com/library/archive/technotes/tn2091/)

### Rust Crates
- [CPAL](https://github.com/RustAudio/cpal) - Cross-platform audio
- [objc2-core-audio](https://docs.rs/objc2-core-audio) - CoreAudio bindings
- [screencapturekit-rs](https://github.com/svtlabs/screencapturekit-rs) - ScreenCaptureKit bindings
- [coreaudio-rs](https://github.com/RustAudio/coreaudio-rs) - AudioUnit wrappers

### Example Implementations
- [AudioCap](https://github.com/insidegui/AudioCap) - Swift CoreAudio Taps reference
- [audiotee](https://github.com/makeusabrew/audiotee) - Swift CLI tool for audio capture
- [BackgroundMusic](https://github.com/kyleneideck/BackgroundMusic) - Full loopback device

### Technical Articles
- [Audio APIs: Core Audio / macOS](https://bastibe.de/2017-06-17-audio-apis-coreaudio.html)
- [AudioTee: Capture System Audio on macOS](https://stronglytyped.uk/articles/audiotee-capture-system-audio-output-macos)
- [From Core Audio to LLMs](https://dev.to/yingzhong_xu_20d6f4c5d4ce/from-core-audio-to-llms-native-macos-audio-capture-for-ai-powered-tools-dkg)

### Community Resources
- [CPAL macOS loopback PR #894](https://github.com/RustAudio/cpal/pull/894)
- [Apple Developer Forums - Core Audio](https://developer.apple.com/forums/tags/core-audio)
- [BlackHole Audio Driver](https://github.com/ExistentialAudio/BlackHole)

---

## Next Steps

1. **Get stakeholder input on open questions**
2. **Validate macOS version distribution**
3. **Set up test environment with multiple macOS versions**
4. **Create implementation plan bead**
5. **Begin CPAL migration prototyping**

---

**Research completed:** January 25, 2026
**Researcher:** jack (enteract crew)
**Document status:** Ready for implementation planning
