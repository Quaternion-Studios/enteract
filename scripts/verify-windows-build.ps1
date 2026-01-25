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
