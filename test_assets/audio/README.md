# Test Audio Fixtures

This directory contains audio test fixtures for validating the macOS audio loopback implementation.

## Files

| File | Duration | Sample Rate | Channels | Purpose |
|------|----------|-------------|----------|---------|
| `speech_male_48k.wav` | 30s | 48,000 Hz | Stereo | Male speech simulation (120-2000 Hz) for transcription testing |
| `speech_female_44k.wav` | 30s | 44,100 Hz | Stereo | Female speech simulation (200-3200 Hz) for 44.1kHz conversion testing |
| `music_classical.wav` | 30s | 48,000 Hz | Stereo | Classical music simulation (musical chord) for music filtering tests |
| `silence.wav` | 10s | 48,000 Hz | Stereo | Digital silence for silence detection testing |
| `noise_white.wav` | 10s | 48,000 Hz | Stereo | White noise for noise detection testing |
| `tone_440hz.wav` | 10s | 48,000 Hz | Stereo | 440 Hz sine wave for signal integrity testing |

## Generation

All files were generated using `sox`:

```bash
# Synthetic speech (male, 48kHz)
sox -n -r 48000 -c 2 speech_male_48k.wav synth 30 sine 120 sine 250 sine 500 sine 1000 sine 2000 tremolo 5 0.3

# Synthetic speech (female, 44.1kHz)
sox -n -r 44100 -c 2 speech_female_44k.wav synth 30 sine 200 sine 400 sine 800 sine 1600 sine 3200 tremolo 6 0.3

# Classical music (chord progression)
sox -n -r 48000 -c 2 music_classical.wav synth 30 sine 261.63 sine 329.63 sine 392.00 sine 523.25 tremolo 2 0.2

# Silence
sox -n -r 48000 -c 2 silence.wav synth 10 sine 0

# White noise
sox -n -r 48000 -c 2 noise_white.wav synth 10 whitenoise

# 440 Hz tone
sox -n -r 48000 -c 2 tone_440hz.wav synth 10 sine 440
```

## Usage

### Play Test Audio

```bash
afplay test_assets/audio/speech_male_48k.wav
```

### Verify File Properties

```bash
soxi test_assets/audio/speech_male_48k.wav
```

Expected output:
```
Input File     : 'test_assets/audio/speech_male_48k.wav'
Channels       : 2
Sample Rate    : 48000
Precision      : 16-bit
Duration       : 00:00:30.00 = 1440000 samples
File Size      : 11.5M
Bit Rate       : 3.07M
Sample Encoding: 16-bit Signed Integer PCM
```

### Convert to Different Format (if needed)

```bash
# Convert to mono 16kHz (Whisper format)
sox test_assets/audio/speech_male_48k.wav -c 1 -r 16000 speech_male_16k_mono.wav

# Extract specific duration
sox test_assets/audio/music_classical.wav music_5s.wav trim 0 5
```

## Test Scenarios

### Sample Rate Conversion Testing
- Use `speech_male_48k.wav` → verify 48kHz → 16kHz conversion
- Use `speech_female_44k.wav` → verify 44.1kHz → 16kHz conversion

### Stereo to Mono Conversion
- All files are stereo → verify mono conversion logic
- Check intelligent channel selection (pick louder channel if difference > threshold)

### Audio Quality Validation
- Use `tone_440hz.wav` → verify frequency preservation
- Check spectrograms before/after processing

### Transcription Pipeline
- Use `speech_male_48k.wav` → verify transcription works
- Use `speech_female_44k.wav` → verify 44.1kHz path works
- Use `music_classical.wav` → verify music is filtered out (not transcribed)
- Use `silence.wav` → verify silence is not transcribed

### Edge Cases
- Use `noise_white.wav` → verify noise rejection
- Mix files → test transitions between different audio types

## Notes

- These are synthetic test files, not real speech/music
- Real speech files can be added later for accuracy testing
- File sizes: ~4MB for 10s files, ~10-11MB for 30s files
- All files use 16-bit PCM encoding (standard for testing)
- Frequencies chosen to simulate typical human speech ranges

## Regeneration

To regenerate all test files:

```bash
cd test_assets/audio
./../../scripts/generate_test_audio.sh
```

(Script to be created as part of test automation)
