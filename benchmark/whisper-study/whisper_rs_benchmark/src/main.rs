//! Whisper-rs Benchmark Binary
//!
//! Benchmarks whisper-rs (whisper.cpp Rust bindings) for comparison
//! with faster-whisper Python implementation.
//!
//! Usage: whisper_rs_benchmark <audio_file.wav> <model_size>
//! Output: JSON with timing, transcription, and resource metrics

use anyhow::{Context, Result};
use hound::WavReader;
use indicatif::{ProgressBar, ProgressStyle};
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Instant;
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

const SAMPLE_RATE: u32 = 16000;
const MODEL_BASE_URL: &str = "https://huggingface.co/ggerganov/whisper.cpp/resolve/main";

#[derive(Debug, Serialize, Deserialize)]
struct BenchmarkOutput {
    transcription: String,
    load_time_sec: f64,
    first_token_time_sec: f64,
    total_time_sec: f64,
    peak_memory_mb: f64,
    avg_cpu_percent: f64,
    gpu_used: bool,
    model_size: String,
    segments: Vec<SegmentInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
struct SegmentInfo {
    start_ms: i64,
    end_ms: i64,
    text: String,
}

fn get_model_path(model_size: &str) -> PathBuf {
    let cache_dir = env::temp_dir().join("enteract").join("whisper_models");
    fs::create_dir_all(&cache_dir).ok();

    let filename = format!("ggml-{}.bin", model_size);
    cache_dir.join(&filename)
}

fn download_model(model_size: &str) -> Result<PathBuf> {
    let model_path = get_model_path(model_size);

    if model_path.exists() {
        eprintln!("Model {} already cached at {:?}", model_size, model_path);
        return Ok(model_path);
    }

    let filename = format!("ggml-{}.bin", model_size);
    let url = format!("{}/{}", MODEL_BASE_URL, filename);

    eprintln!("Downloading model {} from {}...", model_size, url);

    let client = reqwest::blocking::Client::new();
    let response = client
        .get(&url)
        .send()
        .context("Failed to download model")?;

    let total_size = response.content_length().unwrap_or(0);

    let pb = ProgressBar::new(total_size);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")
            .unwrap()
            .progress_chars("#>-"),
    );

    let bytes = response.bytes().context("Failed to read model bytes")?;
    pb.finish_with_message("Download complete");

    let mut file = fs::File::create(&model_path).context("Failed to create model file")?;
    file.write_all(&bytes)
        .context("Failed to write model file")?;

    eprintln!("Model saved to {:?}", model_path);
    Ok(model_path)
}

fn load_audio(path: &Path) -> Result<Vec<f32>> {
    let reader = WavReader::open(path).context("Failed to open WAV file")?;
    let spec = reader.spec();

    // Read samples
    let samples: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Int => {
            let max_val = (1 << (spec.bits_per_sample - 1)) as f32;
            reader
                .into_samples::<i32>()
                .filter_map(|s| s.ok())
                .map(|s| s as f32 / max_val)
                .collect()
        }
        hound::SampleFormat::Float => reader
            .into_samples::<f32>()
            .filter_map(|s| s.ok())
            .collect(),
    };

    // Convert to mono if stereo
    let mono_samples = if spec.channels > 1 {
        samples
            .chunks(spec.channels as usize)
            .map(|chunk| chunk.iter().sum::<f32>() / chunk.len() as f32)
            .collect()
    } else {
        samples
    };

    // Resample to 16kHz if needed
    if spec.sample_rate != SAMPLE_RATE {
        eprintln!(
            "Note: Resampling from {} to {} Hz",
            spec.sample_rate, SAMPLE_RATE
        );
        // Simple linear interpolation resampling
        let ratio = spec.sample_rate as f64 / SAMPLE_RATE as f64;
        let new_len = (mono_samples.len() as f64 / ratio) as usize;
        let resampled: Vec<f32> = (0..new_len)
            .map(|i| {
                let src_idx = i as f64 * ratio;
                let idx = src_idx as usize;
                let frac = src_idx - idx as f64;
                if idx + 1 < mono_samples.len() {
                    mono_samples[idx] * (1.0 - frac as f32) + mono_samples[idx + 1] * frac as f32
                } else {
                    mono_samples[idx.min(mono_samples.len() - 1)]
                }
            })
            .collect();
        Ok(resampled)
    } else {
        Ok(mono_samples)
    }
}

fn get_memory_usage_mb() -> f64 {
    // Try to get memory usage from /proc on Linux
    #[cfg(target_os = "linux")]
    {
        if let Ok(status) = fs::read_to_string("/proc/self/status") {
            for line in status.lines() {
                if line.starts_with("VmRSS:") {
                    if let Some(kb_str) = line.split_whitespace().nth(1) {
                        if let Ok(kb) = kb_str.parse::<f64>() {
                            return kb / 1024.0;
                        }
                    }
                }
            }
        }
    }

    // macOS: use sys-info
    #[cfg(target_os = "macos")]
    {
        if let Ok(mem) = sys_info::mem_info() {
            // This gives total memory, not process memory
            // For accurate process memory on macOS, we'd need mach APIs
            return (mem.total - mem.avail) as f64 / 1024.0;
        }
    }

    0.0
}

fn run_benchmark(audio_path: &Path, model_size: &str) -> Result<BenchmarkOutput> {
    // Download model if needed
    let model_path = download_model(model_size)?;

    // Load model (measure time)
    eprintln!("Loading model {}...", model_size);
    let load_start = Instant::now();

    let ctx = WhisperContext::new_with_params(
        model_path.to_str().unwrap(),
        WhisperContextParameters::default(),
    )
    .context("Failed to load whisper model")?;

    let load_time = load_start.elapsed().as_secs_f64();
    eprintln!("Model loaded in {:.2}s", load_time);

    // Load audio
    eprintln!("Loading audio from {:?}...", audio_path);
    let audio_samples = load_audio(audio_path)?;
    let audio_duration = audio_samples.len() as f64 / SAMPLE_RATE as f64;
    eprintln!(
        "Audio loaded: {:.2}s ({} samples)",
        audio_duration,
        audio_samples.len()
    );

    // Setup whisper params
    let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
    params.set_language(Some("en"));
    params.set_print_progress(false);
    params.set_print_special(false);
    params.set_print_realtime(false);
    params.set_print_timestamps(false);

    // Measure memory before
    let mem_before = get_memory_usage_mb();

    // Run transcription
    eprintln!("Transcribing...");
    let transcribe_start = Instant::now();

    let mut state = ctx.create_state().context("Failed to create state")?;
    state
        .full(params, &audio_samples)
        .context("Failed to run whisper")?;

    let total_time = transcribe_start.elapsed().as_secs_f64();
    let mem_after = get_memory_usage_mb();

    // Collect segments
    let num_segments = state.full_n_segments().context("Failed to get segments")?;
    let mut segments = Vec::new();
    let mut full_text = String::new();

    for i in 0..num_segments {
        let text = state
            .full_get_segment_text(i)
            .context("Failed to get segment text")?;
        let start = state
            .full_get_segment_t0(i)
            .context("Failed to get segment start")?;
        let end = state
            .full_get_segment_t1(i)
            .context("Failed to get segment end")?;

        segments.push(SegmentInfo {
            start_ms: start,
            end_ms: end,
            text: text.clone(),
        });

        full_text.push_str(&text);
    }

    // First token time approximation (whisper-rs doesn't expose this directly)
    // Use the end time of first segment as proxy
    let first_token_time = if !segments.is_empty() {
        // Approximate: assume linear processing, scale by first segment duration
        let first_seg_duration = segments[0].end_ms as f64 / 1000.0;
        (first_seg_duration / audio_duration) * total_time
    } else {
        total_time
    };

    eprintln!(
        "Transcription complete in {:.2}s (RTF: {:.2}x)",
        total_time,
        total_time / audio_duration
    );

    Ok(BenchmarkOutput {
        transcription: full_text.trim().to_string(),
        load_time_sec: load_time,
        first_token_time_sec: first_token_time,
        total_time_sec: total_time,
        peak_memory_mb: mem_after.max(mem_before),
        avg_cpu_percent: 0.0, // Would need threading to measure
        gpu_used: false,      // whisper-rs uses CPU by default
        model_size: model_size.to_string(),
        segments,
    })
}

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 3 {
        eprintln!("Usage: {} <audio_file.wav> <model_size>", args[0]);
        eprintln!("Model sizes: tiny, base, small, medium, large, large-v3");
        std::process::exit(1);
    }

    let audio_path = Path::new(&args[1]);
    let model_size = &args[2];

    if !audio_path.exists() {
        eprintln!("Audio file not found: {:?}", audio_path);
        std::process::exit(1);
    }

    let result = run_benchmark(audio_path, model_size)?;

    // Output JSON to stdout (for Python to parse)
    println!("{}", serde_json::to_string_pretty(&result)?);

    Ok(())
}
