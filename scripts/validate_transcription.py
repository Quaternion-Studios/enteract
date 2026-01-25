#!/usr/bin/env python3
"""
Simple transcription validation script.

Feeds audio samples through the transcription pipeline and compares
output to ground truth transcripts. Measures accuracy metrics.

Usage:
    python3 scripts/validate_transcription.py [--augmented] [--verbose]
"""

import argparse
import json
import os
import sys
from pathlib import Path
from typing import Dict, List, Tuple
import difflib


def calculate_wer(reference: str, hypothesis: str) -> float:
    """
    Calculate Word Error Rate (WER) between reference and hypothesis.

    WER = (S + D + I) / N
    where S = substitutions, D = deletions, I = insertions, N = total words
    """
    ref_words = reference.lower().split()
    hyp_words = hypothesis.lower().split()

    if len(ref_words) == 0:
        return 0.0 if len(hyp_words) == 0 else 1.0

    # Use difflib to compute operations
    matcher = difflib.SequenceMatcher(None, ref_words, hyp_words)
    operations = matcher.get_opcodes()

    substitutions = 0
    deletions = 0
    insertions = 0

    for op, i1, i2, j1, j2 in operations:
        if op == 'replace':
            substitutions += max(i2 - i1, j2 - j1)
        elif op == 'delete':
            deletions += i2 - i1
        elif op == 'insert':
            insertions += j2 - j1

    wer = (substitutions + deletions + insertions) / len(ref_words)
    return wer


def transcribe_audio(audio_path: Path) -> Dict:
    """
    Transcribe audio file using the app's pipeline.

    TODO: Implement actual transcription call once the pipeline is ready.
    For now, returns a placeholder.

    Returns:
        dict with keys: 'text', 'confidence', 'fragments', 'duration_ms'
    """
    # PLACEHOLDER - Replace with actual transcription call
    # This might involve:
    # 1. Loading the audio file
    # 2. Calling Whisper via Rust FFI or subprocess
    # 3. Running through quality_filter
    # 4. Returning structured results

    print(f"  [TODO] Transcribe: {audio_path.name}")
    print(f"  [TODO] Pipeline not yet integrated")

    return {
        'text': "",
        'confidence': 0.0,
        'fragments': 0,
        'duration_ms': 0,
        'error': 'Transcription pipeline not yet implemented'
    }


def validate_sample(audio_path: Path, transcript_path: Path, verbose: bool = False) -> Dict:
    """
    Validate a single audio sample against its ground truth transcript.

    Returns metrics dictionary.
    """
    # Load ground truth
    with open(transcript_path, 'r') as f:
        ground_truth = f.read().strip()

    if verbose:
        print(f"\nValidating: {audio_path.name}")
        print(f"  Ground truth: {ground_truth}")

    # Transcribe
    result = transcribe_audio(audio_path)

    if 'error' in result:
        return {
            'file': audio_path.name,
            'status': 'error',
            'error': result['error']
        }

    # Calculate metrics
    hypothesis = result['text']
    wer = calculate_wer(ground_truth, hypothesis)

    # Check for hallucinations (words not in reference)
    ref_words = set(ground_truth.lower().split())
    hyp_words = set(hypothesis.lower().split())
    hallucinations = hyp_words - ref_words

    metrics = {
        'file': audio_path.name,
        'status': 'success',
        'wer': wer,
        'confidence': result['confidence'],
        'fragments': result['fragments'],
        'duration_ms': result['duration_ms'],
        'hallucinations': len(hallucinations),
        'hallucination_words': list(hallucinations) if len(hallucinations) > 0 else [],
        'ground_truth': ground_truth,
        'hypothesis': hypothesis
    }

    if verbose:
        print(f"  Hypothesis: {hypothesis}")
        print(f"  WER: {wer:.2%}")
        print(f"  Confidence: {result['confidence']:.2f}")
        if hallucinations:
            print(f"  Hallucinations: {hallucinations}")

    return metrics


def run_validation_suite(test_dir: Path, verbose: bool = False) -> List[Dict]:
    """
    Run validation on all samples in a directory.
    """
    results = []

    # Find all .wav files
    wav_files = sorted(test_dir.glob("*.wav"))

    for wav_file in wav_files:
        txt_file = wav_file.with_suffix('.txt')

        if not txt_file.exists():
            print(f"Warning: No transcript found for {wav_file.name}")
            continue

        result = validate_sample(wav_file, txt_file, verbose)
        results.append(result)

    return results


def print_summary(results: List[Dict]):
    """
    Print summary statistics across all test samples.
    """
    print("\n" + "="*60)
    print("VALIDATION SUMMARY")
    print("="*60)

    successful = [r for r in results if r['status'] == 'success']
    errors = [r for r in results if r['status'] == 'error']

    print(f"\nTotal samples: {len(results)}")
    print(f"Successful: {len(successful)}")
    print(f"Errors: {len(errors)}")

    if len(errors) > 0:
        print("\nErrors:")
        for r in errors:
            print(f"  - {r['file']}: {r['error']}")

    if len(successful) > 0:
        avg_wer = sum(r['wer'] for r in successful) / len(successful)
        avg_conf = sum(r['confidence'] for r in successful) / len(successful)
        total_hallucinations = sum(r['hallucinations'] for r in successful)

        print(f"\nAverage WER: {avg_wer:.2%}")
        print(f"Average Confidence: {avg_conf:.2f}")
        print(f"Total Hallucinations: {total_hallucinations}")

        print("\nPer-sample results:")
        print(f"{'File':<30} {'WER':<10} {'Conf':<10} {'Frags':<10}")
        print("-" * 60)
        for r in successful:
            print(f"{r['file']:<30} {r['wer']:>8.2%} {r['confidence']:>9.2f} {r['fragments']:>9}")


def main():
    parser = argparse.ArgumentParser(description="Validate transcription quality")
    parser.add_argument('--augmented', action='store_true',
                       help="Test augmented samples instead of clean")
    parser.add_argument('--verbose', '-v', action='store_true',
                       help="Verbose output")
    parser.add_argument('--output', '-o', type=str,
                       help="Output results to JSON file")

    args = parser.parse_args()

    # Determine test directory
    project_root = Path(__file__).parent.parent
    if args.augmented:
        test_dir = project_root / "test_assets" / "audio" / "transcription" / "augmented"
        print("Testing AUGMENTED samples (with defects)")
    else:
        test_dir = project_root / "test_assets" / "audio" / "transcription" / "clean"
        print("Testing CLEAN samples")

    if not test_dir.exists():
        print(f"Error: Test directory not found: {test_dir}")
        sys.exit(1)

    print(f"Test directory: {test_dir}")
    print()

    # Run validation
    results = run_validation_suite(test_dir, args.verbose)

    # Print summary
    print_summary(results)

    # Save results to file if requested
    if args.output:
        output_path = Path(args.output)
        with open(output_path, 'w') as f:
            json.dump(results, f, indent=2)
        print(f"\nResults saved to: {output_path}")

    # Exit code based on errors
    errors = sum(1 for r in results if r['status'] == 'error')
    sys.exit(1 if errors > 0 else 0)


if __name__ == '__main__':
    main()
