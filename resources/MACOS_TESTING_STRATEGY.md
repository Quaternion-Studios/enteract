# macOS Migration Testing Strategy

**Author:** evan (enteract crew)
**Date:** 2026-01-25
**Purpose:** Comprehensive testing strategy for Enteract's cross-platform audio loopback migration

---

## Executive Summary

This document outlines the testing approach for migrating Enteract from Windows-only (WASAPI) to cross-platform audio support (macOS CoreAudio via CPAL). The strategy covers four key areas:

1. **VM Testing** - Running Windows builds on macOS for validation
2. **Frontend Testing** - Automated testing of Tauri IPC and Vue components
3. **Rust Unit Testing** - Mocking audio hardware and testing async capture engine
4. **Cross-Platform CI** - Automated builds and tests on macOS + Windows

**Key Decision:** Use trait-based abstraction with dependency injection to enable mocking of audio hardware APIs.

---

## 1. VM Testing Setup: UTM on Apple Silicon

### Why UTM?

- **Free and open source** - No license costs vs Parallels ($99/year)
- **Native Apple Silicon support** - Uses Apple's Hypervisor framework for near-native ARM64 performance
- **Active development** - Latest version 4.7.5 (January 2026)
- **Windows 11 ARM support** - Full compatibility with ARM64 Windows

### Setup Instructions

#### Prerequisites
- macOS with Apple Silicon (M1-M5)
- At least 8 GB RAM available
- 64+ GB disk space for VM

#### Installation Steps

1. **Download UTM**
   ```bash
   # Download from official site
   open https://mac.getutm.app/

   # Or via Homebrew
   brew install --cask utm
   ```

2. **Download Windows 11 ARM64 ISO**
   - Get from Microsoft's [Windows Insider Program](https://www.microsoft.com/en-us/software-download/windowsinsiderpreviewARM64)
   - Requires Microsoft account (can bypass during setup)

3. **Create VM in UTM**
   - Click "Create a New Virtual Machine"
   - Select **"Virtualize"** (not Emulate) for Apple Silicon efficiency
   - **Operating System:** Windows
   - **RAM:** Half of available memory (8GB recommended for 16GB Mac)
   - **CPU Cores:** Leave at default (auto-managed)
   - **Storage:** 64GB minimum, 128GB recommended
   - **CRITICAL:** Enable "Install Drivers and SPICE guest tools" for stability

4. **Install Windows 11**
   - Boot VM and follow Windows setup
   - Skip Microsoft account requirement (use offline account)
   - Install all Windows updates

5. **Install Development Tools in VM**
   ```powershell
   # Install Rust
   winget install Rustlang.Rustup

   # Install Node.js
   winget install OpenJS.NodeJS.LTS

   # Install Visual Studio Build Tools (required for Rust)
   winget install Microsoft.VisualStudio.2022.BuildTools
   ```

### Testing Enteract in VM

```powershell
# Clone repo in Windows VM
git clone <enteract-repo-url>
cd enteract

# Build Tauri app
cd src-tauri
cargo build --release

# Run tests
cargo test

# Build frontend
cd ..
npm install
npm run tauri:build
```

### Performance Considerations

- **ARM64 native:** Near-native performance (~90% of bare metal)
- **x86 emulation:** Significantly slower if needed (avoid if possible)
- **GPU acceleration:** Limited in VMs, may affect UI performance
- **Audio:** WASAPI works in VM, can test Windows audio capture

### Automation Potential

UTM provides a CLI (`utmctl`) for VM automation:

```bash
# Start VM headlessly
utmctl start "Windows 11"

# SSH into VM for testing
ssh user@<vm-ip>

# Run tests via CI
utmctl start "Windows 11" && \
  ssh user@vm "cd enteract && cargo test" && \
  utmctl stop "Windows 11"
```

**Limitation:** Requires VM to be configured with SSH server and static IP.

### References
- [UTM Official Documentation](https://docs.getutm.app/guides/windows/)
- [Xanzhu Setup Guide](https://xanzhu.com/blog/install-windows-utm-apple-silicon) - Updated January 2026
- [OSXDaily Installation Guide](https://osxdaily.com/2024/11/07/how-to-install-windows-11-on-mac-with-utm/)

---

## 2. Frontend Testing: Playwright + Tauri WebDriver

### Challenge: Testing Tauri IPC Commands

Tauri apps communicate via IPC (Inter-Process Communication) between the frontend (TypeScript/Vue) and backend (Rust). Testing requires either:

1. **Mock IPC** - For unit testing frontend without Rust backend
2. **WebDriver** - For end-to-end testing with real backend

### Approach 1: Mocking IPC with Playwright

**Use for:** Fast unit tests of Vue components that call Tauri commands

#### Setup

```bash
npm install --save-dev @playwright/test @tauri-apps/api
```

#### Example: Mocking Audio Device Enumeration

```typescript
// tests/audio-devices.spec.ts
import { test, expect } from '@playwright/test';
import { mockIPC } from '@tauri-apps/api/mocks';

test('should display audio devices', async ({ page }) => {
  // Mock the enumerate_loopback_devices Tauri command
  await mockIPC((cmd, args) => {
    if (cmd === 'enumerate_loopback_devices') {
      return [
        {
          id: 'device-1',
          name: 'Speakers (Mock)',
          is_default: true,
          sample_rate: 48000,
          channels: 2,
          format: 'PCM 16bit',
          device_type: 'Render',
          loopback_method: 'RenderLoopback'
        }
      ];
    }
  });

  await page.goto('http://localhost:5173');
  await expect(page.locator('.device-list')).toContainText('Speakers (Mock)');
});
```

#### Limitations

- **Large surface area** - Mocking all Tauri plugins is tedious
- **Not end-to-end** - Doesn't test actual Rust backend
- **Maintenance burden** - Mocks must stay in sync with Rust API

### Approach 2: WebDriver for E2E Testing (RECOMMENDED)

**Use for:** Full integration tests with real Tauri backend

#### Platform Support
- ✅ **Windows** - Full support
- ✅ **Linux** - Full support
- ❌ **macOS** - NOT SUPPORTED (WKWebView has no driver)
- ✅ **iOS/Android** - Via Appium 2 (not streamlined yet)

**Implication:** E2E tests must run on Windows/Linux, not macOS. Use UTM VM or Linux CI runners.

#### Setup with WebdriverIO

```bash
npm install --save-dev @wdio/cli @crabnebula/tauri-driver
npx wdio config
```

#### Example Configuration

```javascript
// wdio.conf.js
export const config = {
  specs: ['./tests/e2e/**/*.spec.ts'],
  capabilities: [{
    'tauri:options': {
      application: './src-tauri/target/release/enteract.exe'
    }
  }],
  services: [
    ['tauri', {
      tauriDriverPath: require.resolve('@crabnebula/tauri-driver')
    }]
  ],
  framework: 'mocha',
  reporters: ['spec']
};
```

#### Example E2E Test

```typescript
// tests/e2e/audio-capture.spec.ts
describe('Audio Capture', () => {
  it('should start and stop audio capture', async () => {
    // Open app
    await browser.url('/');

    // Click "Start Capture" button
    const startBtn = await $('button[data-test="start-capture"]');
    await startBtn.click();

    // Verify capture started
    const status = await $('[data-test="capture-status"]');
    await expect(status).toHaveText('Capturing...');

    // Invoke Tauri command directly (alternative to UI clicks)
    const result = await browser.execute(() => {
      return window.__TAURI__.invoke('stop_audio_loopback_capture');
    });

    await expect(status).toHaveText('Stopped');
  });
});
```

#### Running Tests

```bash
# Start WebDriver
npx tauri-driver

# Run tests
npx wdio run wdio.conf.js
```

### Component Testing with Vitest

**Use for:** Isolated Vue component testing

```bash
npm install --save-dev vitest @vue/test-utils
```

```typescript
// tests/components/AudioDeviceSelector.spec.ts
import { mount } from '@vue/test-utils';
import { describe, it, expect, vi } from 'vitest';
import AudioDeviceSelector from '@/components/AudioDeviceSelector.vue';

describe('AudioDeviceSelector', () => {
  it('renders device list', async () => {
    const mockDevices = [
      { id: '1', name: 'Speakers', is_default: true }
    ];

    const wrapper = mount(AudioDeviceSelector, {
      props: { devices: mockDevices }
    });

    expect(wrapper.text()).toContain('Speakers');
    expect(wrapper.find('.default-badge')).toBeTruthy();
  });
});
```

### References
- [Tauri WebDriver Documentation](https://v2.tauri.app/develop/tests/webdriver/)
- [WebdriverIO Example](https://v2.tauri.app/develop/tests/webdriver/example/webdriverio/)
- [Tauri WebDriver GitHub Example](https://github.com/tauri-apps/webdriver-example)
- [TestDriver.ai Tauri Support](https://docs.testdriver.ai/v6/apps/tauri-apps)

---

## 3. Rust Unit Testing Strategy

### Core Pattern: Trait-Based Dependency Injection

**Problem:** Can't test audio capture code without real audio hardware.

**Solution:** Abstract hardware APIs behind traits, inject mocks in tests.

### Architecture

```
┌─────────────────────────────────────┐
│  Tauri Commands (Public API)       │
├─────────────────────────────────────┤
│  Platform Dispatcher (#[cfg(...)])  │
├──────────────┬──────────────────────┤
│   Windows    │      macOS           │
│ (WASAPI)     │   (CoreAudio/CPAL)   │
└──────────────┴──────────────────────┘
         ▲              ▲
         └──────┬───────┘
                │
         Shared Traits
```

### Example Trait Definitions

```rust
// src-tauri/src/audio_loopback/traits.rs

use anyhow::Result;
use async_trait::async_trait;

/// Trait for enumerating audio devices
#[async_trait]
pub trait AudioDeviceEnumerator: Send + Sync {
    /// List all available loopback devices
    async fn enumerate(&self) -> Result<Vec<AudioDevice>>;

    /// Find device by ID
    async fn find_by_id(&self, id: &str) -> Result<Option<AudioDevice>>;

    /// Get default device
    async fn get_default(&self) -> Result<Option<AudioDevice>>;
}

/// Trait for audio capture engine
#[async_trait]
pub trait AudioCaptureEngine: Send + Sync {
    /// Start capturing from a device
    async fn start(&mut self, device_id: &str) -> Result<()>;

    /// Stop capturing
    async fn stop(&mut self) -> Result<()>;

    /// Check if currently capturing
    fn is_capturing(&self) -> bool;

    /// Get captured audio samples (non-blocking)
    fn get_samples(&mut self) -> Vec<f32>;
}

/// Trait for audio processing (shared between platforms)
pub trait AudioProcessor {
    /// Process raw audio chunk (resample, convert, normalize)
    fn process(&self, data: &[u8], bits: u16, channels: u16, sample_rate: u32) -> Vec<f32>;

    /// Calculate audio level in dB
    fn calculate_level(&self, samples: &[f32]) -> f32;
}

#[derive(Clone, Debug)]
pub struct AudioDevice {
    pub id: String,
    pub name: String,
    pub is_default: bool,
    pub sample_rate: u32,
    pub channels: u16,
}
```

### Mocking with Mockall

```bash
cargo add --dev mockall
```

```rust
// src-tauri/src/audio_loopback/tests/mocks.rs

use mockall::mock;
use crate::audio_loopback::traits::*;

mock! {
    pub DeviceEnumerator {}

    #[async_trait]
    impl AudioDeviceEnumerator for DeviceEnumerator {
        async fn enumerate(&self) -> Result<Vec<AudioDevice>>;
        async fn find_by_id(&self, id: &str) -> Result<Option<AudioDevice>>;
        async fn get_default(&self) -> Result<Option<AudioDevice>>;
    }
}

mock! {
    pub CaptureEngine {}

    #[async_trait]
    impl AudioCaptureEngine for CaptureEngine {
        async fn start(&mut self, device_id: &str) -> Result<()>;
        async fn stop(&mut self) -> Result<()>;
        fn is_capturing(&self) -> bool;
        fn get_samples(&mut self) -> Vec<f32>;
    }
}
```

### Example Unit Test

```rust
// src-tauri/src/audio_loopback/tests/capture_tests.rs

#[cfg(test)]
mod tests {
    use super::*;
    use mockall::predicate::*;

    #[tokio::test]
    async fn test_start_capture_with_valid_device() {
        // Create mock
        let mut mock_engine = MockCaptureEngine::new();

        // Set expectations
        mock_engine
            .expect_start()
            .with(eq("device-123"))
            .times(1)
            .returning(|_| Ok(()));

        mock_engine
            .expect_is_capturing()
            .times(1)
            .returning(|| true);

        // Test
        let result = mock_engine.start("device-123").await;
        assert!(result.is_ok());
        assert!(mock_engine.is_capturing());
    }

    #[tokio::test]
    async fn test_enumerate_returns_devices() {
        let mut mock_enum = MockDeviceEnumerator::new();

        mock_enum
            .expect_enumerate()
            .times(1)
            .returning(|| Ok(vec![
                AudioDevice {
                    id: "dev-1".to_string(),
                    name: "Speakers".to_string(),
                    is_default: true,
                    sample_rate: 48000,
                    channels: 2,
                }
            ]));

        let devices = mock_enum.enumerate().await.unwrap();
        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].name, "Speakers");
    }
}
```

### Testing Async Code with Tokio

```rust
// Cargo.toml
[dev-dependencies]
tokio = { version = "1", features = ["test-util", "macros"] }

// Test with time control
#[tokio::test(start_paused = true)]
async fn test_periodic_transcription() {
    let mut engine = MockCaptureEngine::new();

    // Advance time by 800ms (transcription interval)
    tokio::time::sleep(Duration::from_millis(800)).await;

    // Verify transcription was triggered
    // ...
}
```

### Testing Audio Processing (No Mocks Needed)

```rust
#[test]
fn test_audio_resampling_16bit_to_float() {
    let processor = AudioProcessorImpl::new();

    // Create test PCM16 data: 1 second @ 48kHz, stereo
    let sample_rate = 48000;
    let duration = 1.0; // seconds
    let samples = (sample_rate as f32 * duration) as usize;

    // Generate 1kHz sine wave
    let mut pcm16_data = Vec::new();
    for i in 0..samples {
        let t = i as f32 / sample_rate as f32;
        let sample = (2.0 * std::f32::consts::PI * 1000.0 * t).sin();
        let pcm_sample = (sample * 32767.0) as i16;

        // Stereo: duplicate to both channels
        pcm16_data.extend_from_slice(&pcm_sample.to_le_bytes());
        pcm16_data.extend_from_slice(&pcm_sample.to_le_bytes());
    }

    // Process: 48kHz stereo → 16kHz mono
    let result = processor.process(&pcm16_data, 16, 2, 48000);

    // Verify resampling
    assert_eq!(result.len(), 16000); // 1 second @ 16kHz

    // Verify amplitude is preserved
    let max_amp = result.iter().map(|&s| s.abs()).fold(0.0, f32::max);
    assert!((max_amp - 1.0).abs() < 0.01, "Amplitude should be ~1.0");
}

#[test]
fn test_audio_level_calculation() {
    let processor = AudioProcessorImpl::new();

    // Silent audio
    let silent = vec![0.0f32; 1000];
    assert!(processor.calculate_level(&silent) < -90.0);

    // Full scale
    let loud = vec![1.0f32; 1000];
    assert!(processor.calculate_level(&loud) > -1.0);
}
```

### Integration Tests with Real Platform Code

```rust
// tests/integration_test.rs (separate file, not in src/)

#[cfg(target_os = "windows")]
#[tokio::test]
async fn test_wasapi_device_enumeration() {
    use enteract::audio_loopback::windows::WASAPIEnumerator;

    let enumerator = WASAPIEnumerator::new().unwrap();
    let devices = enumerator.enumerate().await.unwrap();

    // Just verify it doesn't crash - can't assert device count (varies by system)
    assert!(devices.len() >= 0);
}

#[cfg(target_os = "macos")]
#[tokio::test]
async fn test_cpal_device_enumeration() {
    use enteract::audio_loopback::macos::CPALEnumerator;

    let enumerator = CPALEnumerator::new().unwrap();
    let devices = enumerator.enumerate().await.unwrap();

    assert!(devices.len() >= 0);
}
```

### References
- [Tokio Testing Guide](https://tokio.rs/tokio/topics/testing)
- [Mockall Documentation](https://docs.rs/mockall)
- [Rust DI with Traits](https://jmmv.dev/2022/04/rust-traits-and-dependency-injection.html)
- [Mocking Dependencies in Rust](https://medium.com/@mpnunez28/mocking-dependencies-with-traits-for-unit-testing-in-rust-ef987fafd27e)
- [World Without Eng: Dependency Inversion](https://worldwithouteng.com/articles/make-your-rust-code-unit-testable-with-dependency-inversion/)

---

## 4. Cross-Platform CI: GitHub Actions

### Strategy

```
┌──────────────┬──────────────┬──────────────┐
│   macOS      │   Windows    │    Linux     │
├──────────────┼──────────────┼──────────────┤
│ Build        │ Build        │ Build        │
│ Unit tests   │ Unit tests   │ Unit tests   │
│ (no E2E)     │ E2E tests    │ E2E tests    │
└──────────────┴──────────────┴──────────────┘
```

**Why no macOS E2E?** WebDriver doesn't support WKWebView (macOS limitation).

### Cost Considerations

GitHub Actions pricing (per minute):
- **Linux:** 1x ($0.008)
- **Windows:** 2x ($0.016)
- **macOS:** 10x ($0.08)

**Optimization:** Run most tests on Linux, use macOS only for builds and unit tests.

### Example Workflow

```yaml
# .github/workflows/ci.yml

name: CI

on:
  push:
    branches: [ main ]
  pull_request:
    branches: [ main ]

jobs:
  # Fast checks on Linux (cheapest)
  lint:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo fmt -- --check
      - run: cargo clippy -- -D warnings

  # Unit tests on all platforms
  test:
    strategy:
      matrix:
        os: [ubuntu-latest, windows-latest, macos-latest]
        include:
          - os: ubuntu-latest
            target: x86_64-unknown-linux-gnu
          - os: windows-latest
            target: x86_64-pc-windows-msvc
          - os: macos-latest
            target: aarch64-apple-darwin

    runs-on: ${{ matrix.os }}

    steps:
      - uses: actions/checkout@v4

      - name: Setup Rust
        uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}

      - name: Cache cargo registry
        uses: actions/cache@v4
        with:
          path: ~/.cargo/registry
          key: ${{ runner.os }}-cargo-registry-${{ hashFiles('**/Cargo.lock') }}

      - name: Cache cargo build
        uses: actions/cache@v4
        with:
          path: target
          key: ${{ runner.os }}-cargo-build-${{ hashFiles('**/Cargo.lock') }}

      - name: Run unit tests
        run: cargo test --target ${{ matrix.target }}

      - name: Build
        run: cargo build --release --target ${{ matrix.target }}

  # E2E tests (Windows/Linux only - WebDriver limitation)
  e2e:
    needs: test
    strategy:
      matrix:
        os: [ubuntu-latest, windows-latest]

    runs-on: ${{ matrix.os }}

    steps:
      - uses: actions/checkout@v4

      - name: Setup Node
        uses: actions/setup-node@v4
        with:
          node-version: '20'

      - name: Install dependencies
        run: |
          npm ci
          npm install -g @crabnebula/tauri-driver

      - name: Build Tauri app
        run: npm run tauri:build

      - name: Run E2E tests
        run: npm run test:e2e

  # Code coverage (Linux only for speed)
  coverage:
    runs-on: ubuntu-latest

    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable

      - name: Install tarpaulin
        run: cargo install cargo-tarpaulin

      - name: Generate coverage
        run: cargo tarpaulin --out Xml --output-dir coverage

      - name: Upload to Codecov
        uses: codecov/codecov-action@v4
        with:
          files: ./coverage/cobertura.xml
          token: ${{ secrets.CODECOV_TOKEN }}
```

### Advanced: Cross-Compilation

```yaml
# Build macOS binary on Linux (faster, cheaper)
cross-compile:
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4

    - name: Setup cross-compilation toolchain
      uses: taiki-e/setup-cross-toolchain-action@v1
      with:
        target: aarch64-apple-darwin

    - name: Build for macOS
      run: cargo build --release --target aarch64-apple-darwin
```

**Note:** Limited usefulness for Tauri (needs macOS SDK for WebView).

### Coverage Tools Comparison

| Tool | Platform | Accuracy | Speed | Notes |
|------|----------|----------|-------|-------|
| **Tarpaulin** | Linux only | Good | Fast | Recommended for CI |
| **cargo-llvm-cov** | All | Best | Medium | Source-based, most accurate |
| **grcov** | All | Good | Slow | Uses llvm-cov internally |

**Recommendation:** Use `cargo-llvm-cov` for best accuracy:

```bash
cargo install cargo-llvm-cov

# Generate coverage
cargo llvm-cov --html

# Upload to Codecov
cargo llvm-cov --codecov --output-path codecov.json
```

### References
- [Tauri CI Documentation](https://v2.tauri.app/develop/tests/webdriver/ci/)
- [Cross-Compiling Rust in GitHub Actions](https://blog.urth.org/2023/03/05/cross-compiling-rust-projects-in-github-actions/)
- [setup-cross-toolchain-action](https://github.com/taiki-e/setup-cross-toolchain-action)
- [Rust Coverage Tools](https://rustprojectprimer.com/measure/coverage.html)
- [Codecov Rust Guide](https://about.codecov.io/language/rust/)

---

## 5. Visual Regression Testing (Frontend)

### Tools Comparison

| Tool | Best For | Storybook Required | Pricing | AI Features |
|------|----------|-------------------|---------|-------------|
| **Percy** | CI/staging flows | No | Free tier + paid | OCR, smart baselines |
| **Chromatic** | Component dev | Yes | Free OSS + paid | Component diffing |
| **BackstopJS** | Self-hosted | No | Free | Basic diffing |

### Recommendation: Percy for CI Integration

```bash
npm install --save-dev @percy/cli @percy/playwright
```

```typescript
// tests/visual/homepage.spec.ts
import { test } from '@playwright/test';
import percySnapshot from '@percy/playwright';

test('homepage visual regression', async ({ page }) => {
  await page.goto('http://localhost:5173');

  // Wait for dynamic content
  await page.waitForSelector('[data-test="device-list"]');

  // Take visual snapshot
  await percySnapshot(page, 'Homepage - Default State');

  // Test different states
  await page.click('[data-test="settings-button"]');
  await percySnapshot(page, 'Homepage - Settings Open');
});
```

```yaml
# Add to CI workflow
- name: Run visual tests
  run: npx percy exec -- npm run test:visual
  env:
    PERCY_TOKEN: ${{ secrets.PERCY_TOKEN }}
```

### References
- [Percy vs Chromatic Comparison](https://medium.com/@crissyjoshua/percy-vs-chromatic-which-visual-regression-testing-tool-to-use-6cdce77238dc)
- [Percy Documentation](https://blog.theodo.com/2022/10/visual-regression-testing-percy/)
- [Visual Testing Tools 2025](https://apidog.com/blog/best-10-visual-testing-tools/)

---

## 6. Example Test Structure

```
enteract/
├── src-tauri/
│   ├── src/
│   │   ├── audio_loopback/
│   │   │   ├── traits.rs          # Platform-agnostic interfaces
│   │   │   ├── mod.rs              # Platform dispatcher
│   │   │   ├── shared/
│   │   │   │   └── processor.rs    # Shared audio processing
│   │   │   ├── windows/
│   │   │   │   ├── mod.rs
│   │   │   │   ├── enumerator.rs   # WASAPI implementation
│   │   │   │   └── capture.rs      # WASAPI capture engine
│   │   │   ├── macos/
│   │   │   │   ├── mod.rs
│   │   │   │   ├── enumerator.rs   # CPAL implementation
│   │   │   │   └── capture.rs      # CPAL capture engine
│   │   │   └── tests/
│   │   │       ├── mocks.rs        # Mock implementations
│   │   │       ├── processor_tests.rs
│   │   │       └── integration_tests.rs
│   ├── tests/                      # Integration tests
│   │   └── audio_capture_test.rs
│   └── Cargo.toml
├── src/                            # Frontend (Vue)
│   ├── components/
│   │   ├── AudioDeviceSelector.vue
│   │   └── __tests__/
│   │       └── AudioDeviceSelector.spec.ts
│   └── composables/
│       ├── useAudioCapture.ts
│       └── __tests__/
│           └── useAudioCapture.spec.ts
├── tests/
│   ├── unit/                       # Frontend unit tests
│   │   └── ...
│   ├── e2e/                        # WebDriver tests
│   │   ├── audio-capture.spec.ts
│   │   └── settings.spec.ts
│   └── visual/                     # Visual regression
│       └── snapshots.spec.ts
├── wdio.conf.js                    # WebDriver config
├── vitest.config.ts                # Unit test config
└── .github/
    └── workflows/
        └── ci.yml
```

---

## 7. CI/CD Pipeline Design

### Pipeline Stages

```
┌─────────────────────────────────────────────────────────────┐
│ STAGE 1: Fast Checks (2 min)                               │
│ - Lint (Rust fmt, clippy)                                   │
│ - TypeScript type check                                     │
│ - Frontend lint (ESLint)                                    │
│ Run on: ubuntu-latest                                       │
└─────────────────────────────────────────────────────────────┘
                          ↓
┌─────────────────────────────────────────────────────────────┐
│ STAGE 2: Unit Tests - Parallel (5 min)                     │
│ ┌─────────────┬──────────────┬──────────────┐              │
│ │ macOS       │ Windows      │ Linux        │              │
│ │ Rust tests  │ Rust tests   │ Rust tests   │              │
│ │ Build       │ Build        │ Build        │              │
│ └─────────────┴──────────────┴──────────────┘              │
└─────────────────────────────────────────────────────────────┘
                          ↓
┌─────────────────────────────────────────────────────────────┐
│ STAGE 3: Frontend Tests - Parallel (3 min)                 │
│ ┌─────────────────────┬──────────────────────┐             │
│ │ Component tests     │ E2E tests            │             │
│ │ (Vitest)            │ (WebDriver)          │             │
│ │ Run on: Linux       │ Run on: Win + Linux  │             │
│ └─────────────────────┴──────────────────────┘             │
└─────────────────────────────────────────────────────────────┘
                          ↓
┌─────────────────────────────────────────────────────────────┐
│ STAGE 4: Coverage & Quality (3 min)                        │
│ - Code coverage (cargo-llvm-cov)                            │
│ - Upload to Codecov                                         │
│ - Visual regression (Percy)                                 │
│ Run on: ubuntu-latest                                       │
└─────────────────────────────────────────────────────────────┘
                          ↓
┌─────────────────────────────────────────────────────────────┐
│ STAGE 5: Build Artifacts (Windows VM test - optional)      │
│ - Build release binaries (all platforms)                    │
│ - Create installers (DMG, MSI)                              │
│ - Upload artifacts                                          │
│ Only on: main branch, tags                                  │
└─────────────────────────────────────────────────────────────┘
```

**Total time:** ~15 minutes (with parallel jobs)
**Monthly cost estimate:** ~$50-100 for active development (assuming 100 PR builds/month)

### Optimization Tips

1. **Cache aggressively**
   - Cargo registry and build artifacts
   - Node modules
   - Rust toolchains

2. **Skip redundant jobs**
   - Only run E2E on PRs labeled "needs-e2e"
   - Skip visual tests for backend-only changes

3. **Use Linux for coverage**
   - Tarpaulin is faster than macOS alternatives
   - Linux runners are 10x cheaper than macOS

4. **Matrix strategy**
   - Test minimum and maximum supported versions
   - Don't test every patch version

---

## 8. Testing Checklist

### Phase 1: Foundation (Before Migration)

- [ ] Set up UTM with Windows 11 VM
- [ ] Verify WASAPI tests pass in VM
- [ ] Create trait abstractions for audio APIs
- [ ] Add Mockall to dev dependencies
- [ ] Write unit tests for audio processor
- [ ] Set up basic CI pipeline (lint + test)

### Phase 2: Platform Abstraction

- [ ] Create mock implementations of all traits
- [ ] Test platform dispatcher with mocks
- [ ] Add integration tests for Windows code
- [ ] Set up WebDriver for E2E tests
- [ ] Configure test coverage reporting

### Phase 3: macOS Implementation

- [ ] Write unit tests for CPAL enumerator
- [ ] Write unit tests for CPAL capture engine
- [ ] Add macOS to CI matrix
- [ ] Test in UTM Windows VM (regression)
- [ ] Run full E2E suite on Linux

### Phase 4: Release

- [ ] Achieve >80% code coverage
- [ ] All E2E tests passing on Windows/Linux
- [ ] Visual regression baseline approved
- [ ] Manual testing on real macOS + Windows hardware
- [ ] Performance benchmarks (capture latency, CPU usage)

---

## 9. Open Questions & Risks

### Questions for Team

1. **VM Performance:** Is UTM fast enough for daily dev, or should we invest in Parallels?
2. **macOS E2E:** How critical is macOS WebDriver support? (Currently unavailable)
3. **Test Infrastructure:** Self-host runners vs GitHub-hosted?
4. **Coverage Target:** What's the minimum acceptable coverage percentage?

### Known Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| macOS has no WebDriver | High | Use manual testing + unit tests on macOS |
| CPAL behavior differs from WASAPI | High | Extensive integration testing, user beta testing |
| UTM performance degradation | Medium | Benchmark early, fallback to Parallels if needed |
| CI costs exceed budget | Medium | Optimize job matrix, cache aggressively |
| Mock tests don't catch real hardware issues | High | Supplement with integration tests on real devices |

---

## 10. Recommended Next Steps

1. **Week 1: Setup**
   - Install UTM, create Windows 11 VM
   - Set up basic GitHub Actions workflow
   - Install test frameworks (Mockall, Vitest, WebdriverIO)

2. **Week 2: Trait Abstraction**
   - Design trait interfaces for audio APIs
   - Create mock implementations
   - Write first unit tests

3. **Week 3: CI Pipeline**
   - Configure matrix builds (macOS, Windows, Linux)
   - Add code coverage reporting
   - Set up WebDriver for E2E tests

4. **Week 4: Validation**
   - Run full test suite in VM
   - Benchmark test execution time
   - Get team feedback on test coverage

---

## 11. Conclusion

This testing strategy provides comprehensive coverage for the macOS migration:

- **VM testing** enables Windows validation on macOS dev machines
- **Trait-based mocking** allows testing without real hardware
- **Multi-tier testing** (unit, E2E, visual) catches bugs at all levels
- **Cross-platform CI** ensures both platforms stay working

**Key Success Metrics:**
- ✅ All tests pass on macOS and Windows
- ✅ Code coverage >80%
- ✅ CI pipeline completes in <15 minutes
- ✅ Zero regressions in Windows functionality

The foundation is trait-based abstraction with dependency injection. This unlocks comprehensive unit testing and makes the codebase resilient to platform differences.

---

**Document Status:** Draft
**Review Needed:** Chase (overseer), Jack (audio research lead)
**Next Update:** After team review and UTM validation
