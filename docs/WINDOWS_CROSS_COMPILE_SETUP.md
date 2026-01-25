# Windows Cross-Compile Verification Setup

**Author:** evan (enteract crew)
**Date:** 2026-01-25
**Purpose:** UTM-based Windows ARM64 setup for verifying x86_64 Windows builds on macOS

---

## Overview

This guide sets up a Windows 11 ARM64 VM on macOS (via UTM) to verify cross-compilation of Enteract for x86_64 Windows targets. This enables build verification without requiring physical Windows hardware.

**Scope:** Cross-compilation verification only (syntax, linking, dependencies). Not for x64 runtime testing.

---

## Prerequisites

- macOS with Apple Silicon (M1+)
- At least 8 GB RAM available for VM
- 64+ GB disk space
- UTM installed (`brew install --cask utm`)

---

## Quick Start

### 1. Download Windows 11 ARM64 ISO

```bash
# Visit Microsoft's Windows Insider Program
open https://www.microsoft.com/en-us/software-download/windowsinsiderpreviewARM64
```

- Requires Microsoft account (can create free account)
- Download the latest ARM64 ISO

### 2. Create UTM VM

1. Open UTM and click **"Create a New Virtual Machine"**
2. Select **"Virtualize"** (NOT Emulate - this uses Apple's Hypervisor for fast ARM64)
3. Configure:
   - **OS:** Windows
   - **RAM:** 8 GB (or half your available memory)
   - **CPU Cores:** 4 cores (auto-managed)
   - **Storage:** 64 GB minimum
   - ✅ **Enable:** "Install Drivers and SPICE guest tools"
4. **Browse** to your Windows 11 ARM64 ISO
5. Click **"Save"** and start the VM

### 3. Install Windows 11

1. Boot VM and follow setup wizard
2. **Skip Microsoft account** (use offline account):
   - Click "Sign-in options" → "Domain join instead"
   - Create local account
3. Complete setup and install all Windows Updates
4. Reboot when prompted

### 4. Install Development Tools

Open PowerShell as Administrator:

```powershell
# Install Rust
winget install Rustlang.Rustup

# Install Visual Studio Build Tools (required for MSVC toolchain)
winget install Microsoft.VisualStudio.2022.BuildTools

# Install Git
winget install Git.Git

# Verify installations
rustc --version
git --version
```

**Note:** After installing Rust, close and reopen PowerShell to refresh PATH.

### 5. Add x86_64 Target

```powershell
# Add x86_64 Windows MSVC target for cross-compilation
rustup target add x86_64-pc-windows-msvc

# Verify target is installed
rustup target list | Select-String "x86_64-pc-windows-msvc"
```

Expected output:
```
x86_64-pc-windows-msvc (installed)
```

---

## Cross-Compile Verification Workflow

### Clone Repository in VM

```powershell
# Clone Enteract repo
git clone https://github.com/Quaternion-Studios/enteract.git
cd enteract\src-tauri
```

### Build for x86_64 Windows

```powershell
# Cross-compile from ARM64 Windows to x86_64 Windows
cargo build --target x86_64-pc-windows-msvc

# Release build (optimized)
cargo build --release --target x86_64-pc-windows-msvc
```

**Expected output:**
```
   Compiling enteract v0.1.0
   ...
   Finished `dev` profile [unoptimized + debuginfo] target(s) in 45.2s
```

**Build artifacts location:**
```
target\x86_64-pc-windows-msvc\debug\enteract.exe     # Debug build
target\x86_64-pc-windows-msvc\release\enteract.exe   # Release build
```

---

## Verification Script

Create `scripts/verify-windows-build.ps1` in the repo root:

```powershell
# scripts/verify-windows-build.ps1
# Windows cross-compile verification script
# Run from src-tauri directory

param(
    [switch]$Release
)

$ErrorActionPreference = "Stop"

Write-Host "=== Windows x86_64 Cross-Compile Verification ===" -ForegroundColor Cyan
Write-Host ""

# Check we're in src-tauri directory
if (!(Test-Path "Cargo.toml")) {
    Write-Host "ERROR: Must run from src-tauri directory" -ForegroundColor Red
    exit 1
}

# Verify target is installed
$targetInstalled = rustup target list | Select-String "x86_64-pc-windows-msvc \(installed\)"
if (!$targetInstalled) {
    Write-Host "ERROR: x86_64-pc-windows-msvc target not installed" -ForegroundColor Red
    Write-Host "Run: rustup target add x86_64-pc-windows-msvc" -ForegroundColor Yellow
    exit 1
}

# Build target
$buildType = if ($Release) { "release" } else { "debug" }
$buildFlag = if ($Release) { "--release" } else { "" }

Write-Host "Building for x86_64-pc-windows-msvc ($buildType)..." -ForegroundColor Yellow

try {
    if ($Release) {
        cargo build --target x86_64-pc-windows-msvc --release
    } else {
        cargo build --target x86_64-pc-windows-msvc
    }

    $exitCode = $LASTEXITCODE
    if ($exitCode -ne 0) {
        Write-Host "❌ Build FAILED (exit code: $exitCode)" -ForegroundColor Red
        exit $exitCode
    }

    # Check binary exists
    $binaryPath = "target\x86_64-pc-windows-msvc\$buildType\enteract.exe"
    if (!(Test-Path $binaryPath)) {
        Write-Host "❌ Build reported success but binary not found: $binaryPath" -ForegroundColor Red
        exit 1
    }

    $binarySize = (Get-Item $binaryPath).Length / 1MB

    Write-Host ""
    Write-Host "✅ Build SUCCESSFUL" -ForegroundColor Green
    Write-Host "   Binary: $binaryPath" -ForegroundColor Gray
    Write-Host "   Size: $([math]::Round($binarySize, 2)) MB" -ForegroundColor Gray
    Write-Host ""

    exit 0

} catch {
    Write-Host "❌ Build FAILED: $_" -ForegroundColor Red
    exit 1
}
```

### Usage

```powershell
# From src-tauri directory

# Debug build
.\scripts\verify-windows-build.ps1

# Release build
.\scripts\verify-windows-build.ps1 -Release
```

---

## Integration with Phase 1 Workflow

After making changes to the Rust audio loopback code:

1. **On macOS (host):**
   ```bash
   # Make changes to src-tauri/src/audio_loopback/...
   git add .
   git commit -m "Phase 1: Refactor audio loopback"
   git push
   ```

2. **In Windows VM:**
   ```powershell
   # Pull latest changes
   git pull

   # Verify Windows build works
   cd src-tauri
   .\scripts\verify-windows-build.ps1 -Release
   ```

3. **If build succeeds:** Phase 1 verification complete ✅
4. **If build fails:** Fix errors on macOS, repeat

---

## Troubleshooting

### Error: "link.exe not found"

**Cause:** Visual Studio Build Tools not installed or not in PATH

**Fix:**
```powershell
# Install/reinstall VS Build Tools
winget install Microsoft.VisualStudio.2022.BuildTools

# Restart PowerShell to refresh PATH
```

### Error: "could not find native static library wasapi"

**Cause:** Windows-specific dependencies trying to compile for wrong target

**Fix:** Verify `Cargo.toml` has platform-specific dependencies:
```toml
[target.'cfg(windows)'.dependencies]
wasapi = "0.13"
```

### Error: "failed to run custom build command"

**Cause:** Missing build dependencies or toolchain components

**Fix:**
```powershell
# Update Rust toolchain
rustup update

# Reinstall target
rustup target remove x86_64-pc-windows-msvc
rustup target add x86_64-pc-windows-msvc
```

### Slow VM Performance

**Cause:** Using "Emulate" mode instead of "Virtualize"

**Fix:**
- Recreate VM using **"Virtualize"** mode (not Emulate)
- Ensure "Drivers and SPICE tools" are installed
- Allocate at least 8 GB RAM to VM

---

## Limitations

### What This Setup VERIFIES:
- ✅ Code compiles for x86_64 Windows
- ✅ Dependencies link correctly
- ✅ No syntax errors or type mismatches
- ✅ Platform-specific `#[cfg(windows)]` code works

### What This Setup DOES NOT VERIFY:
- ❌ Runtime behavior on x86_64 hardware
- ❌ Audio device enumeration on physical hardware
- ❌ WASAPI capture quality on x86_64
- ❌ Performance characteristics

**For full x86_64 runtime testing:** Use GitHub Actions CI with x86_64 Windows runners (recommended) or physical x86_64 hardware.

---

## GitHub Actions Alternative

For faster feedback without VM setup, create a PR and let CI verify:

```yaml
# .github/workflows/build.yml
name: Build
on: [push, pull_request]
jobs:
  build-windows-x64:
    runs-on: windows-latest  # x86_64 runner
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo build --release
```

**Pros:** No VM setup, tests real x86_64 runtime
**Cons:** Slower feedback (3-5 min), requires internet connection

---

## Verification Checklist

Before marking Phase 1 complete:

- [ ] UTM VM created with Windows 11 ARM64
- [ ] Rust toolchain installed in VM
- [ ] x86_64-pc-windows-msvc target added
- [ ] Verification script created and tested
- [ ] Phase 1 refactored code builds successfully for x86_64
- [ ] No linker errors or missing dependencies
- [ ] Binary artifact generated in `target/x86_64-pc-windows-msvc/`

---

## References

- [UTM Documentation](https://docs.getutm.app/)
- [Rust Cross-Compilation Guide](https://rust-lang.github.io/rustup/cross-compilation.html)
- [Full Testing Strategy](../resources/MACOS_TESTING_STRATEGY.md)
- Phase 1 Bead: en-pfn
