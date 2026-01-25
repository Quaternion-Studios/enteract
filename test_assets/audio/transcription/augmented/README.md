# Augmented Speech Test Samples

Speech samples with added defects to test transcription robustness under challenging conditions.

## Files

| File | Base Sample | Augmentation | Purpose |
|------|-------------|--------------|---------|
| `sample1_noise.wav` | sample1 | White noise (+10% volume) | Background noise handling |
| `sample1_whisper.wav` | sample1 | 70% volume reduction | Low volume / whisper detection |
| `sample1_music.wav` | sample1 | Classical music (+5% volume) | Music interference rejection |
| `sample1_silence001.wav` | sample1 | 1s silence gap in middle | Sentence boundary across silence |
| `sample3_noise.wav` | sample3 | White noise (+15% volume) | Noise with questions/punctuation |
| `sample4_whisper.wav` | sample4 | 75% volume reduction | Long passage at low volume |
| `sample5_music.wav` | sample5 | Classical music (+8% volume) | Proper nouns with music |

## Augmentation Types

### Background Noise
- Added white noise at 10-15% of speech volume
- Tests: VAD, noise rejection, confidence degradation
- Expected: Should still transcribe accurately with lower confidence

### Whisper (Low Volume)
- Reduced to 25-30% of original volume
- Tests: Low-energy speech detection
- Expected: May struggle but should detect speech activity

### Music Interference
- Added classical music at 5-8% of speech volume
- Tests: Music vs speech separation
- Expected: Should transcribe speech, ignore music

### Silence Gaps
- Inserted 1 second silence in middle of utterance
- Tests: Sentence boundary detection, fragment merging
- Expected: Should handle as natural pause or merge fragments

## Format Specifications

All augmented files maintain the same format as clean samples:
- **Sample Rate**: 16,000 Hz
- **Channels**: Mono
- **Encoding**: 16-bit PCM WAV

## Ground Truth Transcripts

Each augmented `.wav` file has a corresponding `.txt` file with the same transcript as its base sample. The audio is degraded but the expected text output is unchanged.

## Testing Strategy

1. Run each augmented sample through transcription
2. Compare to ground truth transcript
3. Measure:
   - Transcription accuracy (WER - Word Error Rate)
   - Confidence score (should be lower than clean)
   - Hallucinations (false positives from noise/music)
   - Fragmentation (splits vs complete transcription)

## Expected Behaviors

| Augmentation | Expected Accuracy | Expected Confidence | Notes |
|--------------|------------------|-------------------|-------|
| Clean | >95% | >0.85 | Baseline |
| Noise | >85% | >0.70 | Slight degradation |
| Whisper | >75% | >0.60 | May miss some words |
| Music | >90% | >0.75 | Should filter music |
| Silence | >95% | >0.80 | Should handle pause |

## Future Augmentations

- Cross-talk (multiple speakers)
- Accent variations
- Speech rate changes (fast/slow)
- Reverb / echo
- Phone/radio quality audio
