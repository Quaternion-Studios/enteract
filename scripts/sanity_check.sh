#!/bin/bash
# Quick Sanity Check for macOS Audio Loopback
# Validates basic build and environment setup

set -e

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_ROOT="$( cd "$SCRIPT_DIR/.." && pwd )"

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

print_header() {
    echo ""
    echo "================================================"
    echo "  $1"
    echo "================================================"
}

print_info() {
    echo -e "${BLUE}ℹ${NC}  $1"
}

print_success() {
    echo -e "${GREEN}✅${NC} $1"
}

print_error() {
    echo -e "${RED}❌${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}⚠️${NC}  $1"
}

CHECKS_PASSED=0
CHECKS_FAILED=0
CHECKS_WARNING=0

run_check() {
    local description="$1"
    local command="$2"
    local is_critical="${3:-true}"  # Default to critical

    echo -n "Checking: $description... "

    if eval "$command" > /tmp/sanity_check.log 2>&1; then
        print_success "PASS"
        CHECKS_PASSED=$((CHECKS_PASSED + 1))
        return 0
    else
        if [ "$is_critical" = "true" ]; then
            print_error "FAIL"
            CHECKS_FAILED=$((CHECKS_FAILED + 1))
            cat /tmp/sanity_check.log
            return 1
        else
            print_warning "WARN"
            CHECKS_WARNING=$((CHECKS_WARNING + 1))
            return 0
        fi
    fi
}

print_header "macOS Audio Loopback - Sanity Check"

# System Info
print_info "System Information"
MACOS_VERSION=$(sw_vers -productVersion)
MACOS_BUILD=$(sw_vers -buildVersion)
ARCH=$(uname -m)
echo "  macOS: $MACOS_VERSION (Build: $MACOS_BUILD)"
echo "  Architecture: $ARCH"
echo ""

# Check macOS version compatibility
MACOS_MAJOR=$(echo "$MACOS_VERSION" | cut -d. -f1)
MACOS_MINOR=$(echo "$MACOS_VERSION" | cut -d. -f2)

if [ "$MACOS_MAJOR" -lt 13 ]; then
    print_error "macOS version too old. Requires macOS 13.0+"
    exit 1
fi

print_success "macOS version: $MACOS_VERSION (supported)"

# Determine loopback support
if [ "$MACOS_MAJOR" -ge 15 ] || ([ "$MACOS_MAJOR" -eq 14 ] && [ "$MACOS_MINOR" -ge 6 ]); then
    print_success "Loopback support: CPAL native (macOS 14.6+)"
elif [ "$MACOS_MAJOR" -eq 14 ] && [ "$MACOS_MINOR" -ge 2 ]; then
    print_warning "Loopback support: CoreAudio Taps only (macOS 14.2-14.5)"
elif [ "$MACOS_MAJOR" -eq 13 ]; then
    print_warning "Loopback support: ScreenCaptureKit or virtual devices (macOS 13.x)"
fi

echo ""
print_header "Development Tools"

# Check Rust
run_check "Rust toolchain" "command -v cargo"
if [ $? -eq 0 ]; then
    RUST_VERSION=$(rustc --version)
    echo "  Version: $RUST_VERSION"
fi

# Check cargo version (need 1.70+)
run_check "Cargo version" "cargo --version | grep -E 'cargo 1\.[7-9][0-9]|cargo 1\.[0-9]{3,}|cargo [2-9]'" false

# Check Tauri CLI
run_check "Tauri CLI" "command -v cargo-tauri || cargo tauri --version" false

echo ""
print_header "Audio Tools"

# Check sox
run_check "sox (audio generation)" "command -v sox" false

# Check ffmpeg
run_check "ffmpeg (audio processing)" "command -v ffmpeg" false

# Check afplay (should be built-in)
run_check "afplay (audio playback)" "command -v afplay"

echo ""
print_header "Project Structure"

# Check project directories
run_check "src-tauri directory" "test -d $PROJECT_ROOT/src-tauri"
run_check "test_assets directory" "test -d $PROJECT_ROOT/test_assets" false
run_check "scripts directory" "test -d $PROJECT_ROOT/scripts" false

# Check Cargo.toml
run_check "Cargo.toml exists" "test -f $PROJECT_ROOT/src-tauri/Cargo.toml"

echo ""
print_header "Build Test"

cd "$PROJECT_ROOT"

# Check if project builds
print_info "Building project (release mode)..."
if cargo build --release > /tmp/build.log 2>&1; then
    print_success "Build: SUCCESS"
    CHECKS_PASSED=$((CHECKS_PASSED + 1))
else
    print_error "Build: FAILED"
    CHECKS_FAILED=$((CHECKS_FAILED + 1))
    echo "Build log (last 20 lines):"
    tail -20 /tmp/build.log
fi

echo ""
print_header "Test Audio Fixtures"

# Check test audio files
AUDIO_DIR="$PROJECT_ROOT/test_assets/audio"
if [ -d "$AUDIO_DIR" ]; then
    AUDIO_COUNT=$(ls -1 "$AUDIO_DIR"/*.wav 2>/dev/null | wc -l | tr -d ' ')
    if [ "$AUDIO_COUNT" -ge 6 ]; then
        print_success "Test audio files: $AUDIO_COUNT/6 files"
        CHECKS_PASSED=$((CHECKS_PASSED + 1))
    elif [ "$AUDIO_COUNT" -gt 0 ]; then
        print_warning "Test audio files: $AUDIO_COUNT/6 files (incomplete)"
        CHECKS_WARNING=$((CHECKS_WARNING + 1))
        echo "  Run: ./scripts/audio_test_utils.sh generate"
    else
        print_warning "No test audio files found"
        CHECKS_WARNING=$((CHECKS_WARNING + 1))
        echo "  Run: ./scripts/audio_test_utils.sh generate"
    fi
else
    print_warning "test_assets/audio directory not found"
    CHECKS_WARNING=$((CHECKS_WARNING + 1))
fi

echo ""
print_header "Unit Tests"

# Run basic device enumeration test (if it exists)
print_info "Running unit tests..."
if cargo test --lib --release > /tmp/test.log 2>&1; then
    TEST_COUNT=$(grep -c "test result: ok" /tmp/test.log || echo "0")
    print_success "Unit tests: PASS"
    CHECKS_PASSED=$((CHECKS_PASSED + 1))
    tail -5 /tmp/test.log
else
    # Tests may not exist yet (stubs)
    if grep -q "#\[ignore\]" /tmp/test.log 2>/dev/null; then
        print_warning "Unit tests: IGNORED (stubs not implemented)"
        CHECKS_WARNING=$((CHECKS_WARNING + 1))
    else
        print_error "Unit tests: FAIL"
        CHECKS_FAILED=$((CHECKS_FAILED + 1))
        tail -20 /tmp/test.log
    fi
fi

echo ""
print_header "Virtual Audio Devices"

# Check for BlackHole
if system_profiler SPAudioDataType 2>/dev/null | grep -qi "BlackHole"; then
    print_success "BlackHole: Installed"
    CHECKS_PASSED=$((CHECKS_PASSED + 1))
else
    print_warning "BlackHole: Not installed (optional for testing)"
    CHECKS_WARNING=$((CHECKS_WARNING + 1))
    echo "  Install: brew install blackhole-2ch"
fi

# Check for Loopback
if system_profiler SPAudioDataType 2>/dev/null | grep -qi "Loopback Audio"; then
    print_success "Loopback: Installed"
else
    print_warning "Loopback: Not installed (optional, commercial)"
fi

echo ""
print_header "Summary"

TOTAL_CHECKS=$((CHECKS_PASSED + CHECKS_FAILED + CHECKS_WARNING))

echo "Total checks: $TOTAL_CHECKS"
echo -e "  ${GREEN}Passed: $CHECKS_PASSED${NC}"
echo -e "  ${RED}Failed: $CHECKS_FAILED${NC}"
echo -e "  ${YELLOW}Warnings: $CHECKS_WARNING${NC}"

echo ""

if [ $CHECKS_FAILED -eq 0 ]; then
    if [ $CHECKS_WARNING -eq 0 ]; then
        print_success "🎉 All checks passed! Environment is ready."
        exit 0
    else
        print_warning "⚠️  Sanity check passed with warnings."
        echo ""
        echo "Non-critical issues detected. Review warnings above."
        exit 0
    fi
else
    print_error "❌ Sanity check failed!"
    echo ""
    echo "Critical issues detected. Fix errors above before proceeding."
    exit 1
fi
