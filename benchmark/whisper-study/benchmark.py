#!/usr/bin/env python3
"""
Whisper vs Faster-Whisper Benchmark Study

Compares:
- whisper-rs (whisper.cpp via Rust) - current implementation
- faster-whisper (Python CTranslate2) - potential replacement

Metrics:
- Transcription latency (time to first token, total time)
- Word Error Rate (WER) vs Google Speech-to-Text baseline
- Memory usage
- CPU/GPU utilization
"""

import os
import sys
import json
import time
import argparse
import subprocess
import tempfile
from pathlib import Path
from dataclasses import dataclass, asdict
from typing import Optional, List, Dict, Any
from datetime import datetime

import numpy as np
import soundfile as sf
import sounddevice as sd
import psutil
from jiwer import wer, cer
from tabulate import tabulate
from tqdm import tqdm

# Optional imports
try:
    from faster_whisper import WhisperModel
    FASTER_WHISPER_AVAILABLE = True
except ImportError:
    FASTER_WHISPER_AVAILABLE = False
    print("Warning: faster-whisper not installed. Run: pip install faster-whisper")

try:
    from google.cloud import speech
    GOOGLE_STT_AVAILABLE = True
except ImportError:
    GOOGLE_STT_AVAILABLE = False
    print("Warning: google-cloud-speech not installed. Ground truth will be unavailable.")


SCRIPT_DIR = Path(__file__).parent
AUDIO_SAMPLES_DIR = SCRIPT_DIR / "audio-samples"
RESULTS_DIR = SCRIPT_DIR / "results"
WHISPER_RS_BENCHMARK = SCRIPT_DIR / "whisper_rs_bin"

MODELS = ["tiny", "base", "small", "medium", "large-v3"]
SAMPLE_RATE = 16000


@dataclass
class BenchmarkResult:
    """Single benchmark measurement"""
    engine: str
    model: str
    audio_file: str
    audio_duration_sec: float

    # Timing
    load_time_sec: float
    first_token_time_sec: float
    total_time_sec: float
    realtime_factor: float  # total_time / audio_duration

    # Accuracy
    transcription: str
    ground_truth: Optional[str]
    wer: Optional[float]
    cer: Optional[float]

    # Resources
    peak_memory_mb: float
    avg_cpu_percent: float
    gpu_used: bool


@dataclass
class AudioSample:
    """Audio sample metadata"""
    name: str
    path: Path
    category: str  # clear, noisy, fast, technical
    description: str
    duration_sec: float
    ground_truth: Optional[str] = None


def record_audio(duration_sec: float, sample_rate: int = SAMPLE_RATE) -> np.ndarray:
    """Record audio from default microphone"""
    print(f"Recording {duration_sec}s from microphone...")
    audio = sd.rec(int(duration_sec * sample_rate), samplerate=sample_rate,
                   channels=1, dtype='float32')
    sd.wait()
    return audio.flatten()


def save_audio(audio: np.ndarray, path: Path, sample_rate: int = SAMPLE_RATE):
    """Save audio to WAV file"""
    sf.write(path, audio, sample_rate)


def load_audio(path: Path) -> tuple[np.ndarray, int]:
    """Load audio file, resample to 16kHz mono"""
    audio, sr = sf.read(path)

    # Convert to mono if stereo
    if len(audio.shape) > 1:
        audio = audio.mean(axis=1)

    # Resample if needed
    if sr != SAMPLE_RATE:
        import librosa
        audio = librosa.resample(audio, orig_sr=sr, target_sr=SAMPLE_RATE)

    return audio.astype(np.float32), SAMPLE_RATE


def get_google_transcription(audio_path: Path) -> Optional[str]:
    """Get ground truth transcription from Google Speech-to-Text"""
    if not GOOGLE_STT_AVAILABLE:
        return None

    try:
        client = speech.SpeechClient()

        with open(audio_path, 'rb') as f:
            content = f.read()

        audio = speech.RecognitionAudio(content=content)
        config = speech.RecognitionConfig(
            encoding=speech.RecognitionConfig.AudioEncoding.LINEAR16,
            sample_rate_hertz=SAMPLE_RATE,
            language_code="en-US",
            enable_automatic_punctuation=True,
        )

        response = client.recognize(config=config, audio=audio)

        transcripts = []
        for result in response.results:
            transcripts.append(result.alternatives[0].transcript)

        return " ".join(transcripts)
    except Exception as e:
        print(f"Google STT error: {e}")
        return None


def benchmark_faster_whisper(
    audio_path: Path,
    model_size: str,
    device: str = "cpu",
    compute_type: str = "int8"
) -> Dict[str, Any]:
    """Benchmark faster-whisper transcription"""
    if not FASTER_WHISPER_AVAILABLE:
        return {"error": "faster-whisper not available"}

    audio, sr = load_audio(audio_path)
    audio_duration = len(audio) / sr

    # Monitor resources
    process = psutil.Process()
    cpu_samples = []

    # Load model (measure separately)
    start_load = time.perf_counter()
    model = WhisperModel(model_size, device=device, compute_type=compute_type)
    load_time = time.perf_counter() - start_load

    # Save audio to temp file for faster-whisper
    with tempfile.NamedTemporaryFile(suffix=".wav", delete=False) as f:
        temp_path = f.name
        sf.write(temp_path, audio, sr)

    try:
        # Transcribe
        mem_before = process.memory_info().rss / 1024 / 1024
        start_time = time.perf_counter()
        first_token_time = None

        segments, info = model.transcribe(temp_path, beam_size=5)

        transcription_parts = []
        for segment in segments:
            if first_token_time is None:
                first_token_time = time.perf_counter() - start_time
            transcription_parts.append(segment.text)
            cpu_samples.append(psutil.cpu_percent(interval=0.01))

        total_time = time.perf_counter() - start_time
        mem_after = process.memory_info().rss / 1024 / 1024

        transcription = " ".join(transcription_parts).strip()

        return {
            "transcription": transcription,
            "load_time_sec": load_time,
            "first_token_time_sec": first_token_time or total_time,
            "total_time_sec": total_time,
            "audio_duration_sec": audio_duration,
            "peak_memory_mb": max(mem_after, mem_before),
            "avg_cpu_percent": np.mean(cpu_samples) if cpu_samples else 0,
            "gpu_used": device != "cpu",
            "language": info.language if hasattr(info, 'language') else "unknown",
        }
    finally:
        os.unlink(temp_path)


def benchmark_whisper_rs(
    audio_path: Path,
    model_size: str,
) -> Dict[str, Any]:
    """Benchmark whisper-rs (via compiled Rust binary)"""

    # Check if the Rust benchmark binary exists
    binary_path = WHISPER_RS_BENCHMARK
    if not binary_path.exists():
        return {"error": f"whisper-rs benchmark binary not found at {binary_path}"}

    audio, sr = load_audio(audio_path)
    audio_duration = len(audio) / sr

    # Save audio to temp file
    with tempfile.NamedTemporaryFile(suffix=".wav", delete=False) as f:
        temp_path = f.name
        sf.write(temp_path, audio, sr)

    try:
        # Run the Rust benchmark binary
        start_time = time.perf_counter()
        result = subprocess.run(
            [str(binary_path), temp_path, model_size],
            capture_output=True,
            text=True,
            timeout=300
        )
        total_wall_time = time.perf_counter() - start_time

        if result.returncode != 0:
            return {"error": f"whisper-rs failed: {result.stderr}"}

        # Parse JSON output from Rust binary
        output = json.loads(result.stdout)
        output["audio_duration_sec"] = audio_duration
        output["total_wall_time_sec"] = total_wall_time

        return output
    except subprocess.TimeoutExpired:
        return {"error": "whisper-rs timeout"}
    except json.JSONDecodeError:
        return {"error": f"Invalid JSON from whisper-rs: {result.stdout}"}
    finally:
        os.unlink(temp_path)


def calculate_wer_cer(transcription: str, ground_truth: str) -> tuple[float, float]:
    """Calculate Word Error Rate and Character Error Rate"""
    if not transcription or not ground_truth:
        return None, None

    # Normalize text
    trans_normalized = transcription.lower().strip()
    truth_normalized = ground_truth.lower().strip()

    word_error_rate = wer(truth_normalized, trans_normalized)
    char_error_rate = cer(truth_normalized, trans_normalized)

    return word_error_rate, char_error_rate


def run_benchmark(
    samples: List[AudioSample],
    models: List[str],
    engines: List[str],
    device: str = "cpu"
) -> List[BenchmarkResult]:
    """Run full benchmark suite"""
    results = []

    total_runs = len(samples) * len(models) * len(engines)

    with tqdm(total=total_runs, desc="Benchmarking") as pbar:
        for sample in samples:
            # Get ground truth if not already set
            if sample.ground_truth is None and GOOGLE_STT_AVAILABLE:
                print(f"\nGetting Google STT ground truth for {sample.name}...")
                sample.ground_truth = get_google_transcription(sample.path)
                if sample.ground_truth:
                    print(f"  Ground truth: {sample.ground_truth[:80]}...")

            for model in models:
                for engine in engines:
                    pbar.set_description(f"{engine}/{model}/{sample.name}")

                    if engine == "faster-whisper":
                        result_data = benchmark_faster_whisper(
                            sample.path, model, device=device
                        )
                    elif engine == "whisper-rs":
                        result_data = benchmark_whisper_rs(sample.path, model)
                    else:
                        result_data = {"error": f"Unknown engine: {engine}"}

                    if "error" in result_data:
                        print(f"\n  Error: {result_data['error']}")
                        pbar.update(1)
                        continue

                    # Calculate accuracy metrics
                    word_err, char_err = calculate_wer_cer(
                        result_data.get("transcription", ""),
                        sample.ground_truth
                    )

                    # Calculate realtime factor
                    audio_dur = result_data.get("audio_duration_sec", 1)
                    total_time = result_data.get("total_time_sec", 0)
                    rtf = total_time / audio_dur if audio_dur > 0 else 0

                    result = BenchmarkResult(
                        engine=engine,
                        model=model,
                        audio_file=sample.name,
                        audio_duration_sec=audio_dur,
                        load_time_sec=result_data.get("load_time_sec", 0),
                        first_token_time_sec=result_data.get("first_token_time_sec", 0),
                        total_time_sec=total_time,
                        realtime_factor=rtf,
                        transcription=result_data.get("transcription", ""),
                        ground_truth=sample.ground_truth,
                        wer=word_err,
                        cer=char_err,
                        peak_memory_mb=result_data.get("peak_memory_mb", 0),
                        avg_cpu_percent=result_data.get("avg_cpu_percent", 0),
                        gpu_used=result_data.get("gpu_used", False),
                    )
                    results.append(result)
                    pbar.update(1)

    return results


def generate_report(results: List[BenchmarkResult], output_path: Path):
    """Generate markdown benchmark report"""

    # Group results by engine and model
    by_engine = {}
    for r in results:
        if r.engine not in by_engine:
            by_engine[r.engine] = {}
        if r.model not in by_engine[r.engine]:
            by_engine[r.engine][r.model] = []
        by_engine[r.engine][r.model].append(r)

    with open(output_path, 'w') as f:
        f.write("# Whisper vs Faster-Whisper Benchmark Report\n\n")
        f.write(f"**Generated**: {datetime.now().isoformat()}\n\n")

        # System info
        f.write("## System Information\n\n")
        f.write(f"- **Platform**: {sys.platform}\n")
        f.write(f"- **Python**: {sys.version.split()[0]}\n")
        f.write(f"- **CPU Cores**: {psutil.cpu_count()}\n")
        f.write(f"- **RAM**: {psutil.virtual_memory().total / 1024**3:.1f} GB\n")
        f.write(f"- **faster-whisper available**: {FASTER_WHISPER_AVAILABLE}\n")
        f.write(f"- **Google STT available**: {GOOGLE_STT_AVAILABLE}\n\n")

        # Summary table
        f.write("## Summary: Average Metrics by Engine and Model\n\n")

        summary_rows = []
        for engine, models_data in by_engine.items():
            for model, model_results in models_data.items():
                avg_rtf = np.mean([r.realtime_factor for r in model_results])
                avg_wer = np.mean([r.wer for r in model_results if r.wer is not None])
                avg_mem = np.mean([r.peak_memory_mb for r in model_results])
                avg_first = np.mean([r.first_token_time_sec for r in model_results])

                summary_rows.append([
                    engine, model,
                    f"{avg_first:.3f}s",
                    f"{avg_rtf:.2f}x",
                    f"{avg_wer*100:.1f}%" if not np.isnan(avg_wer) else "N/A",
                    f"{avg_mem:.0f} MB"
                ])

        f.write(tabulate(
            summary_rows,
            headers=["Engine", "Model", "First Token", "RTF", "WER", "Memory"],
            tablefmt="pipe"
        ))
        f.write("\n\n")

        # Detailed results by audio sample
        f.write("## Detailed Results by Audio Sample\n\n")

        # Group by audio file
        by_audio = {}
        for r in results:
            if r.audio_file not in by_audio:
                by_audio[r.audio_file] = []
            by_audio[r.audio_file].append(r)

        for audio_name, audio_results in by_audio.items():
            f.write(f"### {audio_name}\n\n")

            if audio_results[0].ground_truth:
                f.write(f"**Ground Truth**: {audio_results[0].ground_truth}\n\n")

            rows = []
            for r in audio_results:
                rows.append([
                    r.engine, r.model,
                    f"{r.total_time_sec:.2f}s",
                    f"{r.realtime_factor:.2f}x",
                    f"{r.wer*100:.1f}%" if r.wer is not None else "N/A",
                    f"{r.peak_memory_mb:.0f} MB",
                    r.transcription[:50] + "..." if len(r.transcription) > 50 else r.transcription
                ])

            f.write(tabulate(
                rows,
                headers=["Engine", "Model", "Time", "RTF", "WER", "Memory", "Transcription"],
                tablefmt="pipe"
            ))
            f.write("\n\n")

        # Recommendations
        f.write("## Analysis & Recommendations\n\n")

        # Find best performers
        if results:
            # Best by speed (lowest RTF)
            fastest = min(results, key=lambda r: r.realtime_factor)
            f.write(f"### Speed\n")
            f.write(f"- **Fastest**: {fastest.engine}/{fastest.model} ")
            f.write(f"(RTF: {fastest.realtime_factor:.2f}x)\n\n")

            # Best by accuracy (lowest WER)
            with_wer = [r for r in results if r.wer is not None]
            if with_wer:
                most_accurate = min(with_wer, key=lambda r: r.wer)
                f.write(f"### Accuracy\n")
                f.write(f"- **Most Accurate**: {most_accurate.engine}/{most_accurate.model} ")
                f.write(f"(WER: {most_accurate.wer*100:.1f}%)\n\n")

            # Best by memory (lowest peak)
            lowest_mem = min(results, key=lambda r: r.peak_memory_mb)
            f.write(f"### Memory Efficiency\n")
            f.write(f"- **Lowest Memory**: {lowest_mem.engine}/{lowest_mem.model} ")
            f.write(f"({lowest_mem.peak_memory_mb:.0f} MB)\n\n")

        f.write("---\n\n")
        f.write("*Report generated by whisper-study benchmark harness*\n")

    print(f"\nReport saved to: {output_path}")


def create_sample_audio_files():
    """Create or download sample audio files for testing"""
    samples = []

    # Check for existing samples
    if AUDIO_SAMPLES_DIR.exists():
        for wav_file in AUDIO_SAMPLES_DIR.glob("*.wav"):
            audio, sr = sf.read(wav_file)
            duration = len(audio) / sr

            # Infer category from filename
            name = wav_file.stem
            if "clear" in name.lower():
                category = "clear"
            elif "noisy" in name.lower():
                category = "noisy"
            elif "fast" in name.lower():
                category = "fast"
            elif "tech" in name.lower():
                category = "technical"
            else:
                category = "general"

            samples.append(AudioSample(
                name=wav_file.name,
                path=wav_file,
                category=category,
                description=f"Audio sample: {name}",
                duration_sec=duration
            ))

    if not samples:
        print("\nNo audio samples found in", AUDIO_SAMPLES_DIR)
        print("Please add .wav files or use --record to record samples")

    return samples


def main():
    parser = argparse.ArgumentParser(
        description="Benchmark Whisper engines",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  # Run full benchmark with existing audio samples
  python benchmark.py

  # Record new audio sample from microphone
  python benchmark.py --record 10 --name "clear_speech"

  # Benchmark specific models only
  python benchmark.py --models tiny base small

  # Use GPU for faster-whisper
  python benchmark.py --device cuda

  # Benchmark single audio file
  python benchmark.py --audio /path/to/audio.wav
        """
    )

    parser.add_argument("--record", type=float, metavar="SECONDS",
                        help="Record audio from microphone")
    parser.add_argument("--name", type=str, default="recording",
                        help="Name for recorded audio (with --record)")
    parser.add_argument("--audio", type=Path,
                        help="Single audio file to benchmark")
    parser.add_argument("--models", nargs="+", default=MODELS,
                        help=f"Models to test (default: {MODELS})")
    parser.add_argument("--engines", nargs="+",
                        default=["faster-whisper", "whisper-rs"],
                        help="Engines to test")
    parser.add_argument("--device", default="cpu",
                        help="Device for faster-whisper (cpu/cuda)")
    parser.add_argument("--output", type=Path,
                        default=RESULTS_DIR / "benchmark_report.md",
                        help="Output report path")

    args = parser.parse_args()

    # Create directories
    AUDIO_SAMPLES_DIR.mkdir(exist_ok=True)
    RESULTS_DIR.mkdir(exist_ok=True)

    # Handle recording mode
    if args.record:
        audio = record_audio(args.record)
        output_path = AUDIO_SAMPLES_DIR / f"{args.name}.wav"
        save_audio(audio, output_path)
        print(f"Saved recording to: {output_path}")
        return

    # Load audio samples
    if args.audio:
        audio, sr = load_audio(args.audio)
        samples = [AudioSample(
            name=args.audio.name,
            path=args.audio,
            category="custom",
            description="User-provided audio",
            duration_sec=len(audio) / sr
        )]
    else:
        samples = create_sample_audio_files()

    if not samples:
        print("No audio samples available. Use --record or add .wav files to audio-samples/")
        return 1

    print(f"\nBenchmarking {len(samples)} audio samples")
    print(f"Models: {args.models}")
    print(f"Engines: {args.engines}")
    print(f"Device: {args.device}")
    print()

    # Run benchmarks
    results = run_benchmark(
        samples=samples,
        models=args.models,
        engines=args.engines,
        device=args.device
    )

    # Save raw results
    results_json = RESULTS_DIR / "benchmark_results.json"
    with open(results_json, 'w') as f:
        json.dump([asdict(r) for r in results], f, indent=2)
    print(f"\nRaw results saved to: {results_json}")

    # Generate report
    generate_report(results, args.output)

    return 0


if __name__ == "__main__":
    sys.exit(main() or 0)
