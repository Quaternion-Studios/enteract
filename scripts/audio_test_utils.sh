#!/bin/bash
# Audio Test Utilities for macOS Audio Loopback Testing
# Part of Enteract macOS migration test fixtures

set -e

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_ROOT="$( cd "$SCRIPT_DIR/.." && pwd )"
AUDIO_DIR="$PROJECT_ROOT/test_assets/audio"

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Print with color
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

# Generate test audio files
generate_test_audio() {
    print_info "Generating test audio files..."

    if ! command -v sox &> /dev/null; then
        print_error "sox not found. Install with: brew install sox"
        exit 1
    fi

    mkdir -p "$AUDIO_DIR"
    cd "$AUDIO_DIR"

    # Synthetic speech (male, 48kHz)
    print_info "Generating speech_male_48k.wav..."
    sox -n -r 48000 -c 2 speech_male_48k.wav synth 30 sine 120 sine 250 sine 500 sine 1000 sine 2000 tremolo 5 0.3

    # Synthetic speech (female, 44.1kHz)
    print_info "Generating speech_female_44k.wav..."
    sox -n -r 44100 -c 2 speech_female_44k.wav synth 30 sine 200 sine 400 sine 800 sine 1600 sine 3200 tremolo 6 0.3

    # Classical music (chord progression)
    print_info "Generating music_classical.wav..."
    sox -n -r 48000 -c 2 music_classical.wav synth 30 sine 261.63 sine 329.63 sine 392.00 sine 523.25 tremolo 2 0.2

    # Silence
    print_info "Generating silence.wav..."
    sox -n -r 48000 -c 2 silence.wav synth 10 sine 0

    # White noise
    print_info "Generating noise_white.wav..."
    sox -n -r 48000 -c 2 noise_white.wav synth 10 whitenoise

    # 440 Hz tone
    print_info "Generating tone_440hz.wav..."
    sox -n -r 48000 -c 2 tone_440hz.wav synth 10 sine 440

    print_success "All test audio files generated in $AUDIO_DIR"
    ls -lh "$AUDIO_DIR"/*.wav
}

# Play test audio through default output
play_test_audio() {
    local file="$1"

    if [ -z "$file" ]; then
        echo "Usage: $0 play <filename>"
        echo "Available files:"
        ls -1 "$AUDIO_DIR"/*.wav 2>/dev/null || echo "No test files found. Run: $0 generate"
        exit 1
    fi

    local audio_file="$AUDIO_DIR/$file"

    if [ ! -f "$audio_file" ]; then
        audio_file="$file"  # Try absolute path
    fi

    if [ ! -f "$audio_file" ]; then
        print_error "File not found: $file"
        exit 1
    fi

    print_info "Playing: $(basename "$audio_file")"
    afplay "$audio_file" &
    local pid=$!
    print_success "Playing audio (PID: $pid)"
    echo "Press Ctrl+C to stop"
    wait $pid
}

# List all audio devices
list_audio_devices() {
    print_info "Audio devices on this system:"
    echo ""
    system_profiler SPAudioDataType
}

# Check BlackHole installation
check_blackhole() {
    print_info "Checking for BlackHole virtual audio device..."

    if system_profiler SPAudioDataType | grep -qi "BlackHole"; then
        print_success "BlackHole detected"
        system_profiler SPAudioDataType | grep -A 5 "BlackHole"
        return 0
    else
        print_warning "BlackHole not installed"
        echo ""
        echo "To install BlackHole:"
        echo "  brew install blackhole-2ch"
        echo "  OR download from: https://existential.audio/blackhole/"
        return 1
    fi
}

# Check Loopback installation (Rogue Amoeba)
check_loopback() {
    print_info "Checking for Loopback audio device..."

    if system_profiler SPAudioDataType | grep -qi "Loopback Audio"; then
        print_success "Loopback detected"
        system_profiler SPAudioDataType | grep -A 5 "Loopback"
        return 0
    else
        print_warning "Loopback not installed (commercial product)"
        echo ""
        echo "Loopback is available from: https://rogueamoeba.com/loopback/"
        return 1
    fi
}

# Reset audio permissions for Enteract
reset_audio_permissions() {
    local app_id="${1:-com.quaternionstudios.enteract}"

    print_warning "Resetting audio permissions for: $app_id"
    echo ""
    echo "This will reset:"
    echo "  - Microphone access"
    echo "  - Screen Recording access (for ScreenCaptureKit)"
    echo ""
    read -p "Continue? (y/n) " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        print_info "Cancelled"
        exit 0
    fi

    # Reset microphone (for virtual devices)
    print_info "Resetting Microphone permission..."
    tccutil reset Microphone "$app_id" 2>/dev/null || print_warning "Microphone permission reset may have failed"

    # Reset screen recording (for ScreenCaptureKit)
    print_info "Resetting Screen Recording permission..."
    tccutil reset ScreenCapture "$app_id" 2>/dev/null || print_warning "Screen Capture permission reset may have failed"

    print_success "Permissions reset"
    echo ""
    print_warning "Restart Enteract to trigger new permission prompts"

    # Note: Audio capture permission (macOS 14.2+) doesn't have a tccutil category yet
    # Full reset requires manual deletion from TCC database (requires SIP disable)
}

# Check macOS version
check_macos_version() {
    local version=$(sw_vers -productVersion)
    local build=$(sw_vers -buildVersion)

    print_info "macOS Version: $version (Build: $build)"

    local major=$(echo "$version" | cut -d. -f1)
    local minor=$(echo "$version" | cut -d. -f2)

    echo ""
    echo "Audio Loopback Support:"

    if [ "$major" -ge 15 ] || ([ "$major" -eq 14 ] && [ "$minor" -ge 6 ]); then
        print_success "CPAL native loopback supported (macOS 14.6+)"
    elif [ "$major" -eq 14 ] && [ "$minor" -ge 2 ]; then
        print_success "CoreAudio Taps supported (macOS 14.2+)"
        print_warning "CPAL native loopback NOT supported (requires 14.6+)"
    elif [ "$major" -eq 13 ]; then
        print_warning "ScreenCaptureKit or virtual devices required (macOS 13.x)"
        print_warning "CoreAudio Taps NOT supported (requires 14.2+)"
    else
        print_error "macOS version too old. Requires macOS 13.0+"
    fi

    echo ""
}

# Verify test environment
verify_environment() {
    print_info "Verifying test environment..."
    echo ""

    # Check macOS version
    check_macos_version

    # Check sox
    if command -v sox &> /dev/null; then
        print_success "sox installed: $(sox --version | head -1)"
    else
        print_error "sox not installed. Run: brew install sox"
    fi

    # Check ffmpeg
    if command -v ffmpeg &> /dev/null; then
        print_success "ffmpeg installed: $(ffmpeg -version | head -1 | cut -d' ' -f3)"
    else
        print_warning "ffmpeg not installed (optional). Run: brew install ffmpeg"
    fi

    # Check afplay (should be built-in)
    if command -v afplay &> /dev/null; then
        print_success "afplay available"
    else
        print_error "afplay not found (should be built-in macOS tool)"
    fi

    # Check Rust
    if command -v cargo &> /dev/null; then
        print_success "Rust installed: $(rustc --version)"
    else
        print_error "Rust not installed. Install from: https://rustup.rs/"
    fi

    # Check test audio files
    if [ -d "$AUDIO_DIR" ] && [ "$(ls -A $AUDIO_DIR/*.wav 2>/dev/null | wc -l)" -gt 0 ]; then
        local count=$(ls -1 "$AUDIO_DIR"/*.wav 2>/dev/null | wc -l | tr -d ' ')
        print_success "Test audio files: $count files in $AUDIO_DIR"
    else
        print_warning "No test audio files. Run: $0 generate"
    fi

    # Check virtual audio devices
    echo ""
    check_blackhole || true
    check_loopback || true

    echo ""
    print_success "Environment check complete"
}

# Show file info
show_file_info() {
    local file="$1"

    if [ -z "$file" ]; then
        echo "Usage: $0 info <filename>"
        exit 1
    fi

    local audio_file="$AUDIO_DIR/$file"

    if [ ! -f "$audio_file" ]; then
        audio_file="$file"  # Try absolute path
    fi

    if [ ! -f "$audio_file" ]; then
        print_error "File not found: $file"
        exit 1
    fi

    print_info "File information for: $(basename "$audio_file")"
    echo ""

    if command -v soxi &> /dev/null; then
        soxi "$audio_file"
    elif command -v sox &> /dev/null; then
        sox --info "$audio_file"
    else
        afinfo "$audio_file"
    fi
}

# Display help
show_help() {
    cat << EOF
Audio Test Utilities for macOS Audio Loopback Testing

Usage: $0 <command> [options]

Commands:
    generate             Generate all test audio files
    play <file>          Play audio file through default output
    list                 List all audio devices on system
    check                Check for virtual audio devices (BlackHole, Loopback)
    reset [app-id]       Reset audio permissions for app (default: Enteract)
    version              Check macOS version and loopback support
    verify               Verify test environment setup
    info <file>          Show detailed info about audio file
    help                 Show this help message

Examples:
    $0 generate
    $0 play speech_male_48k.wav
    $0 list
    $0 check
    $0 reset
    $0 verify
    $0 info tone_440hz.wav

Test Audio Files:
    speech_male_48k.wav       30s male speech at 48kHz stereo
    speech_female_44k.wav     30s female speech at 44.1kHz stereo
    music_classical.wav       30s classical music
    silence.wav               10s digital silence
    noise_white.wav           10s white noise
    tone_440hz.wav            10s 440 Hz sine wave

EOF
}

# Main command dispatcher
main() {
    case "${1:-help}" in
        generate|gen)
            generate_test_audio
            ;;
        play)
            play_test_audio "$2"
            ;;
        list|devices)
            list_audio_devices
            ;;
        check|blackhole)
            check_blackhole
            echo ""
            check_loopback
            ;;
        reset|permissions)
            reset_audio_permissions "$2"
            ;;
        version|macos)
            check_macos_version
            ;;
        verify|env|environment)
            verify_environment
            ;;
        info|show)
            show_file_info "$2"
            ;;
        help|--help|-h)
            show_help
            ;;
        *)
            print_error "Unknown command: $1"
            echo ""
            show_help
            exit 1
            ;;
    esac
}

# Run main function
main "$@"
