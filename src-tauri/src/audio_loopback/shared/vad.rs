// src-tauri/src/audio_loopback/shared/vad.rs
// Voice Activity Detection (VAD) module
//
// Distinguishes speech from background noise using:
// - Energy-based detection (RMS per frame)
// - Zero-crossing rate (speech vs noise)
// - Hangover logic to avoid cutting words

/// Voice Activity Detector
///
/// Detects speech segments in audio using energy and zero-crossing analysis.
pub struct VoiceActivityDetector {
    energy_threshold: f32,           // RMS threshold for speech detection
    zcr_threshold: f32,               // Zero-crossing rate threshold
    frame_size_ms: u32,               // Frame size in milliseconds (20-30ms)
    hangover_frames: usize,           // Frames to continue after speech ends
    sample_rate: u32,                 // Audio sample rate
}

impl VoiceActivityDetector {
    /// Create a new VAD with default settings
    pub fn new(sample_rate: u32) -> Self {
        Self {
            energy_threshold: 0.01,      // ~-40dB
            zcr_threshold: 0.3,          // 30% of max ZCR
            frame_size_ms: 30,           // 30ms frames
            hangover_frames: 10,         // ~300ms hangover
            sample_rate,
        }
    }

    /// Create a VAD with custom thresholds
    pub fn with_thresholds(
        sample_rate: u32,
        energy_threshold: f32,
        zcr_threshold: f32,
    ) -> Self {
        Self {
            energy_threshold,
            zcr_threshold,
            frame_size_ms: 30,
            hangover_frames: 10,
            sample_rate,
        }
    }

    /// Check if a single frame contains speech
    ///
    /// Uses both energy (RMS) and zero-crossing rate for detection
    pub fn is_speech(&self, frame: &[f32]) -> bool {
        if frame.is_empty() {
            return false;
        }

        // Energy check (RMS)
        let rms = calculate_rms(frame);
        if rms < self.energy_threshold {
            return false;
        }

        // Zero-crossing rate check
        let zcr = calculate_zero_crossing_rate(frame);

        // Speech typically has moderate ZCR
        // Too low = silence/pure tone, too high = noise
        // We accept anything above the threshold and below 0.7 (70%)
        zcr >= self.zcr_threshold && zcr <= 0.7
    }

    /// Get speech segments from audio buffer
    ///
    /// Returns (start_sample, end_sample) pairs for each speech segment
    pub fn get_speech_segments(&self, audio: &[f32]) -> Vec<(usize, usize)> {
        if audio.is_empty() {
            return Vec::new();
        }

        let frame_size = (self.sample_rate as f32 * (self.frame_size_ms as f32 / 1000.0)) as usize;
        if frame_size == 0 {
            return Vec::new();
        }

        let mut segments = Vec::new();
        let mut in_speech = false;
        let mut speech_start = 0;
        let mut hangover_counter = 0;

        // Process audio in frames
        for (frame_idx, frame) in audio.chunks(frame_size).enumerate() {
            let is_speech_frame = self.is_speech(frame);
            let current_sample = frame_idx * frame_size;

            if is_speech_frame {
                if !in_speech {
                    // Speech started
                    speech_start = current_sample;
                    in_speech = true;
                }
                // Reset hangover counter on speech detection
                hangover_counter = 0;
            } else if in_speech {
                // Silence during speech - check hangover
                hangover_counter += 1;

                if hangover_counter > self.hangover_frames {
                    // Hangover expired - speech ended
                    let speech_end = current_sample;
                    segments.push((speech_start, speech_end));
                    in_speech = false;
                    hangover_counter = 0;
                }
            }
        }

        // Close final segment if still in speech
        if in_speech {
            segments.push((speech_start, audio.len()));
        }

        segments
    }

    /// Extract only speech portions from audio
    ///
    /// Returns concatenated speech samples, removing silence
    pub fn extract_speech(&self, audio: &[f32]) -> Vec<f32> {
        let segments = self.get_speech_segments(audio);

        let mut speech_audio = Vec::new();
        for (start, end) in segments {
            if end <= audio.len() {
                speech_audio.extend_from_slice(&audio[start..end]);
            }
        }

        speech_audio
    }
}

/// Calculate RMS (Root Mean Square) of audio samples
fn calculate_rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    (samples.iter().map(|&x| x * x).sum::<f32>() / samples.len() as f32).sqrt()
}

/// Calculate zero-crossing rate
///
/// Returns ratio of sign changes to total samples (0.0 to 1.0)
fn calculate_zero_crossing_rate(samples: &[f32]) -> f32 {
    if samples.len() < 2 {
        return 0.0;
    }

    let mut crossings = 0;
    for i in 0..samples.len() - 1 {
        // Count sign changes
        if (samples[i] >= 0.0 && samples[i + 1] < 0.0) ||
           (samples[i] < 0.0 && samples[i + 1] >= 0.0) {
            crossings += 1;
        }
    }

    crossings as f32 / (samples.len() - 1) as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rms_calculation() {
        let silent = vec![0.0; 100];
        assert_eq!(calculate_rms(&silent), 0.0);

        let loud = vec![1.0; 100];
        assert_eq!(calculate_rms(&loud), 1.0);

        let mixed = vec![0.5, -0.5, 0.5, -0.5];
        assert_eq!(calculate_rms(&mixed), 0.5);
    }

    #[test]
    fn test_zero_crossing_rate() {
        // Pure tone (high ZCR)
        let tone: Vec<f32> = (0..100).map(|i| (i as f32 * 0.1).sin()).collect();
        let zcr = calculate_zero_crossing_rate(&tone);
        assert!(zcr > 0.2); // High crossing rate

        // DC signal (no crossings)
        let dc = vec![1.0; 100];
        let zcr_dc = calculate_zero_crossing_rate(&dc);
        assert_eq!(zcr_dc, 0.0);
    }

    #[test]
    fn test_silence_detection() {
        let vad = VoiceActivityDetector::new(16000);

        // Silent frame
        let silent = vec![0.0; 480]; // 30ms at 16kHz
        assert!(!vad.is_speech(&silent));

        // Speech-like frame (moderate energy + ZCR)
        let mut speech = vec![0.0; 480];
        for (i, sample) in speech.iter_mut().enumerate() {
            *sample = 0.1 * (i as f32 * 0.05).sin(); // Moderate amplitude
        }
        assert!(vad.is_speech(&speech));
    }

    #[test]
    fn test_speech_segments() {
        let vad = VoiceActivityDetector::new(16000);
        let frame_size = (16000.0 * 0.03) as usize; // 30ms

        // Create audio: silence, speech, silence, speech, silence
        let mut audio = Vec::new();

        // 100ms silence
        audio.extend(vec![0.0; frame_size * 3]);

        // 200ms speech
        for _ in 0..(frame_size * 6) {
            audio.push(0.1 * (audio.len() as f32 * 0.05).sin());
        }

        // 100ms silence
        audio.extend(vec![0.0; frame_size * 3]);

        // 100ms speech
        for _ in 0..(frame_size * 3) {
            audio.push(0.1 * (audio.len() as f32 * 0.05).sin());
        }

        // 100ms silence
        audio.extend(vec![0.0; frame_size * 3]);

        let segments = vad.get_speech_segments(&audio);

        // Should detect 2 speech segments (with hangover, they might merge)
        assert!(segments.len() >= 1);
        assert!(segments.len() <= 2);
    }
}
