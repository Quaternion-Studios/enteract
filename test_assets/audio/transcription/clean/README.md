# Clean Speech Test Samples

Real speech samples with known ground truth transcripts for testing transcription quality.

## Files

| File | Duration | Content | Purpose |
|------|----------|---------|---------|
| `sample1.wav` | ~3s | Simple greeting | Basic transcription test |
| `sample2.wav` | ~6s | Numbers and IPs | Technical content, numeric handling |
| `sample3.wav` | ~5s | Questions | Punctuation detection, sentence boundaries |
| `sample4.wav` | ~10s | Multi-sentence passage | Natural pauses, longer content |
| `sample5.wav` | ~6s | Proper nouns | Name recognition (Dr. Smith, Stanford, etc.) |
| `sample6.wav` | ~3s | Classic pangram | Clear simple statement |
| `sample7.wav` | ~7s | Technical jargon | API, JSON, HTTP, authentication |
| `sample8.wav` | ~6s | Dates and times | Time format handling |

## Format Specifications

- **Sample Rate**: 16,000 Hz (Whisper format)
- **Channels**: Mono
- **Encoding**: 16-bit PCM WAV
- **Voices**: Alex (male), Samantha (female)

## Ground Truth Transcripts

Each `.wav` file has a corresponding `.txt` file containing the exact text that was synthesized.

## Generation Method

All samples were generated using macOS `say` command:
```bash
say -o sample.aiff -f transcript.txt
sox sample.aiff -r 16000 -c 1 sample.wav
```

## Usage

To test transcription:
1. Feed `sampleN.wav` through the transcription pipeline
2. Compare output to `sampleN.txt`
3. Measure accuracy, confidence, fragmentation

## Next Steps

These clean samples will be augmented with:
- Background noise
- Silence gaps
- Volume variations
- Mixed audio scenarios
