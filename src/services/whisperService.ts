import { invoke } from '@tauri-apps/api/core'

export interface TranscriptionOptions {
  modelSize?: 'tiny' | 'base' | 'small' | 'medium' | 'large'
  language?: string
  translate?: boolean
  sampleRate?: number
  channels?: number
  enableVad?: boolean
  silenceThreshold?: number
  maxSegmentLength?: number
}

export interface TranscriptionResult {
  text: string
  confidence?: number
  duration?: number
  language?: string
}

/**
 * Transcribe base64-encoded audio using Whisper
 */
export async function transcribeAudioBase64(
  audioBase64: string, 
  options: TranscriptionOptions = {}
): Promise<TranscriptionResult> {
  const config = {
    modelSize: options.modelSize || 'large',
    language: options.language || 'en', 
    enableVad: options.enableVad ?? true,
    silenceThreshold: options.silenceThreshold || 0.01,
    maxSegmentLength: options.maxSegmentLength || 30,
    translate: options.translate ?? false
  }

  try {
    const result = await invoke<TranscriptionResult>('transcribe_audio_base64', {
      audioData: audioBase64,
      config
    })

    return result
  } catch (error) {
    console.error('Whisper transcription error:', error)
    throw new Error(`Transcription failed: ${error}`)
  }
}

/**
 * Check if Whisper model is available
 */
export async function checkWhisperModelAvailability(modelSize: string = 'large'): Promise<boolean> {
  try {
    return await invoke<boolean>('check_whisper_model_availability', { modelSize })
  } catch (error) {
    console.error('Failed to check Whisper model availability:', error)
    return false
  }
}

/**
 * Download Whisper model if not available
 */
export async function downloadWhisperModel(modelSize: string = 'large'): Promise<void> {
  try {
    await invoke('download_whisper_model', { modelSize })
  } catch (error) {
    console.error('Failed to download Whisper model:', error)
    throw new Error(`Failed to download model: ${error}`)
  }
}

/**
 * Initialize Whisper model
 */
export async function initializeWhisperModel(modelSize: string = 'large'): Promise<void> {
  try {
    await invoke('initialize_whisper_model', { modelSize })
  } catch (error) {
    console.error('Failed to initialize Whisper model:', error)
    throw new Error(`Failed to initialize model: ${error}`)
  }
}

/**
 * List available Whisper models
 */
export async function listAvailableModels(): Promise<string[]> {
  try {
    return await invoke<string[]>('list_available_models')
  } catch (error) {
    console.error('Failed to list available models:', error)
    return []
  }
}