# macOS Development Environment Setup

**Bead:** en-p5b
**Status:** Complete
**Date:** 2026-01-25
**Owner:** evan (technical lead)

---

## Overview

This document describes the macOS development environment setup for Enteract. The environment is now configured to build successfully on Apple Silicon Macs, with audio loopback functionality stubbed out pending Phase 1 implementation.

---

## Prerequisites

### Required System Tools

1. **Xcode Command Line Tools**
   ```bash
   xcode-select --version
   # Should show: xcode-select version 2410 (or later)
   ```

   If not installed:
   ```bash
   xcode-select --install
   ```

2. **Homebrew** (optional but recommended)
   ```bash
   /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
   ```

### Required Development Tools

1. **Rust Toolchain**
   ```bash
   # Check installation
   rustc --version
   # Should show: rustc 1.88.0 or later

   # Install rustup if needed
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

   # Add macOS target
   rustup target add aarch64-apple-darwin
   rustup target add x86_64-apple-darwin  # For Intel Macs
   ```

2. **Node.js 20+**
   ```bash
   # Check version
   node --version  # Should be v20+
   npm --version   # Should be 9+

   # Install via Homebrew if needed
   brew install node
   ```

3. **Tauri Dependencies**
   - CoreGraphics (system framework - no install needed)
   - CoreFoundation (system framework - no install needed)
   - AudioToolbox (system framework - no install needed)

   All macOS system frameworks are automatically available.

---

## Build Verification

### Step 1: Clone and Navigate

```bash
cd /path/to/enteract
cd src-tauri
```

### Step 2: Build for macOS

```bash
cargo build --target aarch64-apple-darwin
```

**Expected output:**
```
   Compiling enteract v0.1.0
warning: ... (various warnings are OK)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in XX.XXs
```

### Step 3: Verify Binary

```bash
ls -lh target/aarch64-apple-darwin/debug/enteract
```

Should show a binary file ~100-200MB.

### Step 4: Build Frontend (Optional)

```bash
cd ..
npm install
npm run tauri:dev
```

---

## Current Implementation Status

### ✅ Working on macOS

- **Window Management** - CoreGraphics integration
- **Transparency** - Native macOS window transparency
- **Eye Tracking** - ML-based gaze tracking
- **Speech Transcription** - Whisper integration
- **Screenshot Capture** - xcap library
- **RAG System** - Document processing and search
- **MCP Integration** - Multi-command processing
- **SQLite Storage** - Cross-platform data persistence

### ❌ Not Yet Implemented (Coming in Phase 1)

- **Audio Loopback Capture**
  - `enumerate_loopback_devices` → Returns error
  - `start_audio_loopback_capture` → Returns error
  - `stop_audio_loopback_capture` → Returns error
  - All audio commands return: *"Audio loopback not yet implemented for macOS. Coming in Phase 1."*

**Why stubs?** Maintains API surface for frontend. Users get clear error messages instead of crashes.

---

## Platform-Specific Dependencies

### Cargo.toml Configuration

```toml
# Cross-platform dependencies
[dependencies]
rubato = "0.15"     # Audio resampling (used in Phase 1)
hound = "3.5"       # WAV file handling

# Windows-only
[target.'cfg(windows)'.dependencies]
wasapi = "0.13"     # Windows Audio Session API
windows = { version = "0.52", features = [...] }
winapi = { version = "0.3", features = [...] }

# macOS-only
[target.'cfg(target_os = "macos")'.dependencies]
objc = "0.2"
core-graphics = "0.23"
```

### Code Organization

```
src-tauri/src/
├── audio_loopback.rs          # Platform dispatcher
├── audio_loopback/
│   ├── types.rs               # #[cfg(windows)] Windows types
│   ├── device_enumerator.rs   # #[cfg(windows)] WASAPI enum
│   ├── capture_engine.rs      # #[cfg(windows)] WASAPI capture
│   ├── audio_processor.rs     # #[cfg(windows)] Shared processing
│   ├── quality_filter.rs      # #[cfg(windows)] Quality filtering
│   └── settings.rs            # #[cfg(windows)] Persistence
└── lib.rs                     # Imports audio_loopback (both platforms)
```

**macOS stubs are in:** `src/audio_loopback.rs` under `#[cfg(target_os = "macos")]`

---

## Bug Fixes Applied

### 1. CoreGraphics API Update

**Issue:** `core-graphics` 0.23 changed API from previous versions.

**Fix:**
```rust
// OLD (broken)
use core_graphics::display::CGMainDisplay;
let display = CGMainDisplay();

// NEW (working)
use core_graphics::display::CGDisplay;
let display = CGDisplay::main();
```

**Files changed:** `src/window_manager.rs:40`

### 2. Unsafe Function Call

**Issue:** `CGDisplayBounds()` is an unsafe extern "C" function.

**Fix:**
```rust
// OLD (broken)
let bounds = CGDisplayBounds(display);

// NEW (working)
let bounds = unsafe { CGDisplayBounds(display) };
```

**Files changed:** `src/window_manager.rs:151`

### 3. Windows-Only Dependencies

**Issue:** `wasapi` crate was in main `[dependencies]`, causing macOS linker errors.

**Fix:** Moved to `[target.'cfg(windows)'.dependencies]`

**Files changed:** `src-tauri/Cargo.toml`

---

## Testing the Build

### Unit Tests

```bash
cargo test --target aarch64-apple-darwin
```

**Note:** Tests that depend on Windows-specific code are automatically skipped on macOS via `#[cfg(windows)]`.

### Integration Tests

Currently no macOS-specific integration tests. Will be added in Phase 1.

### Frontend Development

```bash
npm run tauri:dev
```

Opens development UI. Audio loopback features will show error messages when clicked.

---

## Known Warnings (Safe to Ignore)

The build produces ~68 warnings, primarily:
- Unused variables in cross-platform code
- Non-snake-case identifiers (legacy code)
- Dead code warnings for Windows-specific functions

**Action:** These will be cleaned up incrementally. No impact on functionality.

---

## Troubleshooting

### Error: "xcrun: error: unable to find utility \"clang\""

**Solution:** Install Xcode Command Line Tools
```bash
xcode-select --install
```

### Error: "linker `cc` not found"

**Solution:** Same as above - install Xcode Command Line Tools

### Error: "failed to run custom build command for `core-graphics`"

**Solution:** Ensure macOS SDK is available
```bash
xcode-select --print-path
# Should show: /Library/Developer/CommandLineTools
```

### Build is Slow

**Normal:** First build downloads and compiles 200+ dependencies. Subsequent builds use cache and are much faster.

**Tip:** Use `cargo build --release` for optimized builds (slower compile, faster runtime).

---

## Next Steps

### Phase 1: Platform Abstraction (en-pfn)

Create trait-based abstraction layer:

1. **Define Traits** (`traits.rs`)
   - `AudioDeviceEnumerator`
   - `AudioCaptureEngine`
   - `AudioProcessor`

2. **Refactor Windows Code**
   - Move to `windows/` subfolder
   - Implement traits

3. **Implement macOS** (`macos/`)
   - CPAL-based enumerator
   - CPAL-based capture engine
   - Shared audio processor

4. **Platform Dispatch** (`mod.rs`)
   ```rust
   #[cfg(target_os = "windows")]
   pub use windows::*;

   #[cfg(target_os = "macos")]
   pub use macos::*;
   ```

**Reference:** `resources/MACOS_AUDIO_LOOPBACK_RESEARCH.md`

---

## Environment Verification Checklist

Before claiming Phase 1 (en-pfn), verify:

- [ ] `cargo build --target aarch64-apple-darwin` succeeds
- [ ] Binary created in `target/aarch64-apple-darwin/debug/`
- [ ] `npm run tauri:dev` launches app
- [ ] No linker errors related to Windows symbols
- [ ] All system frameworks accessible (no missing framework errors)
- [ ] Git status clean (changes committed)

**Status:** ✅ All checks passed (2026-01-25)

---

## References

- **Research:** [MACOS_AUDIO_LOOPBACK_RESEARCH.md](../resources/MACOS_AUDIO_LOOPBACK_RESEARCH.md)
- **Testing Strategy:** [MACOS_TESTING_STRATEGY.md](../resources/MACOS_TESTING_STRATEGY.md)
- **Bead:** en-p5b (macOS Development Environment Setup)
- **Next Bead:** en-pfn (Phase 1: Foundation - Platform abstraction)

---

**Document Status:** Complete
**Last Updated:** 2026-01-25
**Author:** evan (technical lead)
