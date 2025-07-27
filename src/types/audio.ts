// Audio processing types and configurations

// GPU configuration for audio processing
export interface GpuConfig {
  use_gpu: boolean
  gpu_device: number | null
  gpu_type: 'Cuda' | 'Metal' | 'OpenCL' | 'Cpu'
  gpu_name: string | null
}

// Optimized audio configuration with streaming support
export interface OptimizedAudioConfig {
  // GPU settings
  useGpu: boolean
  gpuDevice?: number
  
  // VAD (Voice Activity Detection) settings
  vadEnabled: boolean
  vadThreshold: number // Energy threshold for voice detection (0.0 - 1.0)
  vadSilenceFrames: number // Number of silent frames before ending voice segment
  
  // Streaming settings
  chunkDurationMs: number // Target chunk duration in milliseconds
  maxChunkDurationMs: number // Maximum chunk duration before forced processing
  streamingEnabled: boolean
  
  // Performance settings
  maxLatencyMs: number // Maximum acceptable latency
  bufferSizeFrames: number // Audio buffer size in frames
  
  // Audio quality settings
  sampleRate: number
  channels: number
  bitDepth: number
}

// Audio processing statistics
export interface AudioProcessingStats {
  averageLatencyMs: number
  maxLatencyMs: number
  minLatencyMs: number
  totalChunksProcessed: number
  totalDurationProcessed: number
  gpuAccelerated: boolean
  droppedChunks: number
}

// VAD detection result
export interface VadResult {
  isSpeech: boolean
  energy: number
  timestamp: number
  confidence: number
}

// Streaming chunk metadata
export interface StreamingChunk {
  id: string
  timestamp: number
  duration: number
  sampleCount: number
  energy: number
  isVoiceActive: boolean
}

// Default optimized configurations
export const DEFAULT_OPTIMIZED_CONFIG: OptimizedAudioConfig = {
  // GPU
  useGpu: true,
  gpuDevice: 0,
  
  // VAD
  vadEnabled: true,
  vadThreshold: 0.02,
  vadSilenceFrames: 30, // ~1 second at 30fps
  
  // Streaming
  chunkDurationMs: 2000, // 2 second chunks
  maxChunkDurationMs: 5000, // 5 second max
  streamingEnabled: true,
  
  // Performance
  maxLatencyMs: 500, // 500ms max latency for real-time feel
  bufferSizeFrames: 4096,
  
  // Audio quality
  sampleRate: 16000, // Optimal for speech recognition
  channels: 1, // Mono
  bitDepth: 16
}

// Performance profiles
export const AUDIO_PROFILES = {
  realtime: {
    ...DEFAULT_OPTIMIZED_CONFIG,
    chunkDurationMs: 1000,
    maxLatencyMs: 300,
    bufferSizeFrames: 2048
  },
  balanced: {
    ...DEFAULT_OPTIMIZED_CONFIG,
    chunkDurationMs: 2000,
    maxLatencyMs: 500,
    bufferSizeFrames: 4096
  },
  quality: {
    ...DEFAULT_OPTIMIZED_CONFIG,
    chunkDurationMs: 3000,
    maxLatencyMs: 1000,
    bufferSizeFrames: 8192
  }
} as const

export type AudioProfile = keyof typeof AUDIO_PROFILES