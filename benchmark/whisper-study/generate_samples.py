#!/usr/bin/env python3
"""
Generate test audio samples for whisper benchmark.

Creates synthetic test audio using text-to-speech, or provides
instructions for recording real samples.
"""

import os
import sys
import subprocess
from pathlib import Path

SCRIPT_DIR = Path(__file__).parent
SAMPLES_DIR = SCRIPT_DIR / "audio-samples"

# Test phrases for different categories
TEST_PHRASES = {
    "clear_speech": {
        "text": "The quick brown fox jumps over the lazy dog. This is a test of clear speech recognition with normal pacing and pronunciation.",
        "description": "Clear, well-enunciated speech at normal pace"
    },
    "technical_jargon": {
        "text": "The API endpoint uses OAuth 2.0 authentication with JWT tokens. Configure the Kubernetes deployment using kubectl apply with the YAML manifest.",
        "description": "Technical terminology, acronyms, programming terms"
    },
    "fast_speech": {
        "text": "I need to quickly explain that we're running out of time and the deadline is approaching rapidly so we must hurry.",
        "description": "Rapid speech, compressed timing"
    },
    "noisy_background": {
        "text": "Please listen carefully. I'm speaking with background noise. Can you understand what I'm saying?",
        "description": "Speech with simulated background noise"
    },
    "numbers_dates": {
        "text": "The meeting is scheduled for January 15th, 2025 at 3:30 PM. The total cost is $1,234.56 for 42 items.",
        "description": "Numbers, dates, currency, quantities"
    },
}


def check_tts_available():
    """Check if macOS 'say' command or other TTS is available"""
    if sys.platform == "darwin":
        # macOS has built-in 'say' command
        return "say"
    elif sys.platform == "linux":
        # Check for espeak
        try:
            subprocess.run(["espeak", "--version"], capture_output=True)
            return "espeak"
        except FileNotFoundError:
            pass
        # Check for festival
        try:
            subprocess.run(["festival", "--version"], capture_output=True)
            return "festival"
        except FileNotFoundError:
            pass
    return None


def generate_with_macos_say(text: str, output_path: Path, rate: int = 175):
    """Generate audio using macOS 'say' command"""
    # say outputs to AIFF, then convert to WAV
    aiff_path = output_path.with_suffix(".aiff")

    subprocess.run([
        "say",
        "-o", str(aiff_path),
        "-r", str(rate),
        text
    ], check=True)

    # Convert AIFF to WAV using ffmpeg or afconvert
    try:
        subprocess.run([
            "ffmpeg", "-y", "-i", str(aiff_path),
            "-ar", "16000", "-ac", "1",
            str(output_path)
        ], check=True, capture_output=True)
    except FileNotFoundError:
        # Fall back to afconvert
        subprocess.run([
            "afconvert", "-f", "WAVE", "-d", "LEI16@16000",
            str(aiff_path), str(output_path)
        ], check=True)

    # Clean up AIFF
    aiff_path.unlink(missing_ok=True)


def generate_with_espeak(text: str, output_path: Path, rate: int = 175):
    """Generate audio using espeak"""
    subprocess.run([
        "espeak",
        "-w", str(output_path),
        "-s", str(rate),
        text
    ], check=True)


def add_noise(input_path: Path, output_path: Path, noise_level: float = 0.02):
    """Add white noise to audio file"""
    try:
        import soundfile as sf
        import numpy as np

        audio, sr = sf.read(input_path)
        noise = np.random.normal(0, noise_level, len(audio))
        noisy_audio = audio + noise
        sf.write(output_path, noisy_audio.astype(np.float32), sr)
        return True
    except ImportError:
        print("soundfile not available for noise addition")
        return False


def generate_samples():
    """Generate all test audio samples"""
    SAMPLES_DIR.mkdir(exist_ok=True)

    tts = check_tts_available()
    if not tts:
        print("No TTS engine found. Please install espeak or use macOS.")
        print("\nAlternatively, record samples manually:")
        for name, info in TEST_PHRASES.items():
            print(f"\n{name}.wav:")
            print(f"  Text: \"{info['text']}\"")
            print(f"  Description: {info['description']}")
        return False

    print(f"Using TTS engine: {tts}")
    print(f"Generating samples in {SAMPLES_DIR}\n")

    for name, info in TEST_PHRASES.items():
        output_path = SAMPLES_DIR / f"{name}.wav"
        print(f"Generating {name}...")

        rate = 175  # Normal rate
        if name == "fast_speech":
            rate = 250  # Faster

        if tts == "say":
            generate_with_macos_say(info["text"], output_path, rate)
        elif tts == "espeak":
            generate_with_espeak(info["text"], output_path, rate)

        print(f"  Created: {output_path}")

        # For noisy_background, add noise to the clean version
        if name == "noisy_background":
            noisy_path = SAMPLES_DIR / "noisy_background_with_noise.wav"
            if add_noise(output_path, noisy_path):
                print(f"  Created noisy version: {noisy_path}")

    # Create ground truth file
    ground_truth_path = SAMPLES_DIR / "ground_truth.json"
    import json
    with open(ground_truth_path, 'w') as f:
        json.dump({
            name: info["text"] for name, info in TEST_PHRASES.items()
        }, f, indent=2)
    print(f"\nGround truth saved to: {ground_truth_path}")

    print("\nDone! Samples ready for benchmarking.")
    return True


def main():
    print("=== Whisper Benchmark Sample Generator ===\n")

    if "--record" in sys.argv:
        print("Recording mode instructions:")
        print("Use: python benchmark.py --record <seconds> --name <sample_name>")
        print("\nSuggested recordings:")
        for name, info in TEST_PHRASES.items():
            print(f"\n  {name}:")
            print(f"    python benchmark.py --record 10 --name {name}")
            print(f"    Read: \"{info['text'][:60]}...\"")
        return

    generate_samples()


if __name__ == "__main__":
    main()
