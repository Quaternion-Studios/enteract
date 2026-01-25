# Transcription Validation Guide

This guide explains how to validate transcription quality using the test samples.

## Quick Start

Once the transcription pipeline is integrated:

```bash
# Test clean samples
python3 scripts/validate_transcription.py

# Test augmented samples (with defects)
python3 scripts/validate_transcription.py --augmented

# Verbose output with details
python3 scripts/validate_transcription.py --verbose

# Save results to JSON
python3 scripts/validate_transcription.py --output results.json
```

## Metrics Explained

### Word Error Rate (WER)
- Measures transcription accuracy
- WER = (Substitutions + Deletions + Insertions) / Total Words
- Lower is better
- Target: < 5% for clean speech, < 15% for augmented

### Confidence Score
- Self-reported confidence from the model (0.0 - 1.0)
- Should correlate with actual accuracy
- Target: > 0.85 for clean, > 0.70 for augmented

### Fragmentation
- Number of transcript fragments produced
- Fewer fragments = better sentence boundary detection
- Target: 1-2 fragments per sample

### Hallucinations
- Words in output that aren't in the ground truth
- Common on silence or noise
- Target: 0 for clean samples, minimize for augmented

## Test Sample Categories

### Clean Samples (test_assets/audio/transcription/clean/)
- 8 samples covering different speech patterns
- Perfect audio quality
- Use these to establish baseline performance

### Augmented Samples (test_assets/audio/transcription/augmented/)
- 7 samples with added defects
- Tests robustness: noise, whispers, music, silence
- Use these to test edge case handling

## Expected Performance Targets

| Sample Type | WER Target | Confidence Target | Hallucinations |
|-------------|------------|-------------------|----------------|
| Clean | < 5% | > 0.85 | 0 |
| Noise | < 15% | > 0.70 | < 3 words |
| Whisper | < 25% | > 0.60 | < 2 words |
| Music | < 10% | > 0.75 | 0 (should ignore music) |
| Silence | < 5% | > 0.80 | 0 (should not hallucinate) |

## Integration with Transcription Pipeline

The `validate_transcription.py` script currently has a placeholder for the transcription function. To integrate:

1. **Option A: Direct Rust FFI**
   - Add Python bindings to the Rust transcription module
   - Call directly from Python script

2. **Option B: CLI Wrapper**
   - Create a Rust binary that takes audio file and outputs JSON
   - Call from Python subprocess

3. **Option C: IPC**
   - Start the Tauri app in headless mode
   - Send transcription requests via IPC
   - Collect responses

Recommended: Option B (CLI wrapper) for simplicity.

## Sample Integration (Option B)

Create `src-tauri/src/bin/transcribe_cli.rs`:

```rust
// Example CLI wrapper
use std::env;
use std::fs::File;
use std::io::Read;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let audio_path = &args[1];

    // Load audio file
    let mut file = File::open(audio_path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;

    // Transcribe using your pipeline
    // let result = audio_processor::transcribe(&buffer)?;

    // Output JSON
    println!("{{");
    println!("  \"text\": \"Example transcription\",");
    println!("  \"confidence\": 0.95,");
    println!("  \"fragments\": 1,");
    println!("  \"duration_ms\": 3500");
    println!("}}");

    Ok(())
}
```

Then update `transcribe_audio()` in the Python script to call this binary.

## Manual Testing (Before Integration)

You can manually test individual samples:

```bash
# Play a sample
afplay test_assets/audio/transcription/clean/sample1.wav

# View its transcript
cat test_assets/audio/transcription/clean/sample1.txt

# Check file info
soxi test_assets/audio/transcription/clean/sample1.wav
```

## Regression Testing

After each improvement to the transcription pipeline:

1. Run validation on clean samples
2. Run validation on augmented samples
3. Compare metrics to baseline
4. Document improvements/regressions

Keep a log of results:

```bash
# Baseline (before improvements)
python3 scripts/validate_transcription.py --output baseline.json

# After B1: Smart Buffering
python3 scripts/validate_transcription.py --output after_b1.json

# Compare
diff baseline.json after_b1.json
```

## Troubleshooting

**No transcription output**
- Check that the transcription pipeline is integrated
- Verify audio files are in correct format (16kHz, mono, WAV)
- Check for error messages in the script output

**High WER on clean samples**
- May indicate a fundamental issue with the pipeline
- Check audio preprocessing (sample rate conversion, normalization)
- Verify Whisper model is loaded correctly

**Hallucinations on silence/noise**
- Check VAD (Voice Activity Detection) is working
- Verify quality filtering is enabled
- May need to adjust silence thresholds

## Next Steps

1. Implement transcription integration (see above)
2. Run baseline validation
3. Share results with Nic (transcription improvements)
4. Iterate on improvements
5. Re-validate after each change

## Contact

- Jack: Validation and testing coordination
- Nic: Transcription quality improvements (Workstream B)
- Evan: Storage and diarization (Workstreams C & A)
