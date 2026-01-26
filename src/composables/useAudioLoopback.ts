// composables/useAudioLoopback.ts
import { ref, computed, onUnmounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen, UnlistenFn } from '@tauri-apps/api/event'
import { transcribeAudioBase64 } from '../services/whisperService'

// Types matching the Rust backend
export interface AudioLoopbackDevice {
  id: string
  name: string
  is_default: boolean
  sample_rate: number
  channels: number
  format: string
  device_type: 'Render' | 'Capture'
  loopback_method: 'RenderLoopback' | 'CaptureDevice' | 'StereoMix'
}

export interface AudioDeviceSettings {
  selectedLoopbackDevice: string | null
  loopbackEnabled: boolean
  bufferSize: number
  sampleRate: number
}

export interface AudioChunkEvent {
  deviceId: string
  audioData: string // base64 encoded PCM16
  sampleRate: number
  channels: number
  level: number
  timestamp: number
  duration: number
  totalSamples: number
}

export interface TranscriptionResult {
  text: string
  confidence: number
  timestamp: number
  deviceId: string
}

// Audio buffer for accumulating chunks before transcription
class AudioBuffer {
  private chunks: Uint8Array[] = []
  private totalLength = 0
  private readonly maxBufferDuration: number // in seconds
  private readonly sampleRate: number
  private readonly bytesPerSample = 2 // PCM16
  
  constructor(sampleRate: number, maxBufferDuration = 3) {
    this.sampleRate = sampleRate
    this.maxBufferDuration = maxBufferDuration
  }
  
  addChunk(base64Audio: string) {
    const binaryString = atob(base64Audio)
    const bytes = new Uint8Array(binaryString.length)
    for (let i = 0; i < binaryString.length; i++) {
      bytes[i] = binaryString.charCodeAt(i)
    }
    
    this.chunks.push(bytes)
    this.totalLength += bytes.length
  }
  
  shouldProcess(): boolean {
    const durationSeconds = this.totalLength / (this.sampleRate * this.bytesPerSample)
    return durationSeconds >= this.maxBufferDuration
  }
  
  getAudioData(): { data: Uint8Array; duration: number } | null {
    if (this.chunks.length === 0) return null
    
    // Combine all chunks
    const combined = new Uint8Array(this.totalLength)
    let offset = 0
    for (const chunk of this.chunks) {
      combined.set(chunk, offset)
      offset += chunk.length
    }
    
    const duration = this.totalLength / (this.sampleRate * this.bytesPerSample)
    
    // Clear buffer
    this.clear()
    
    return { data: combined, duration }
  }
  
  clear() {
    this.chunks = []
    this.totalLength = 0
  }
}

export function useAudioLoopback() {
  // State
  const devices = ref<AudioLoopbackDevice[]>([])
  const selectedDevice = ref<AudioLoopbackDevice | null>(null)
  const isCapturing = ref(false)
  const isLoadingDevices = ref(false)
  const audioLevel = ref(-60)
  const captureError = ref<string | null>(null)
  const transcriptions = ref<TranscriptionResult[]>([])
  const isProcessingAudio = ref(false)
  
  // Settings
  const settings = ref<AudioDeviceSettings>({
    selectedLoopbackDevice: null,
    loopbackEnabled: false,
    bufferSize: 4096,
    sampleRate: 16000
  })
  
  // Audio processing
  let audioBuffer: AudioBuffer | null = null
  let audioEventUnlisten: UnlistenFn | null = null
  let lastTranscriptionTime = 0
  const minTranscriptionInterval = 500 // Minimum time between transcriptions (ms)
  
  // Computed
  const hasDevices = computed(() => devices.value.length > 0)
  const canCapture = computed(() => selectedDevice.value !== null && !isCapturing.value)
  
  // Device enumeration
  const enumerateDevices = async () => {
    isLoadingDevices.value = true
    captureError.value = null
    
    try {
      const foundDevices = await invoke<AudioLoopbackDevice[]>('enumerate_loopback_devices')
      devices.value = foundDevices
      console.log('🔊 Found audio devices:', foundDevices)
      
      // Auto-select best device if none selected
      if (!selectedDevice.value && foundDevices.length > 0) {
        await autoSelectDevice()
      }
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error)
      captureError.value = message
      console.error('Failed to enumerate devices:', error)
    } finally {
      isLoadingDevices.value = false
    }
  }
  
  // Auto-select best device
  const autoSelectDevice = async () => {
    try {
      const bestDevice = await invoke<AudioLoopbackDevice | null>('auto_select_best_device')
      if (bestDevice) {
        selectedDevice.value = bestDevice
        settings.value.selectedLoopbackDevice = bestDevice.id
        console.log('🎯 Auto-selected device:', bestDevice.name)
      }
    } catch (error) {
      console.error('Failed to auto-select device:', error)
    }
  }
  
  // Test device
  const testDevice = async (deviceId: string): Promise<boolean> => {
    try {
      const result = await invoke<boolean>('test_audio_device', { deviceId })
      return result
    } catch (error) {
      console.error('Failed to test device:', error)
      return false
    }
  }
  
  // Select device
  const selectDevice = async (device: AudioLoopbackDevice) => {
    const testResult = await testDevice(device.id)
    if (testResult) {
      selectedDevice.value = device
      settings.value.selectedLoopbackDevice = device.id
      console.log('🔊 Selected audio device:', device.name)
    } else {
      captureError.value = 'Failed to initialize selected audio device'
    }
  }
  
  // Start audio capture
  const startCapture = async () => {
    if (!selectedDevice.value || isCapturing.value) return
    
    captureError.value = null
    
    try {
      // Initialize audio buffer
      audioBuffer = new AudioBuffer(settings.value.sampleRate)
      
      // Listen for audio chunks
      audioEventUnlisten = await listen<AudioChunkEvent>('audio-chunk', async (event) => {
        const chunk = event.payload
        
        // Update audio level
        audioLevel.value = chunk.level
        
        // Add to buffer
        if (audioBuffer && settings.value.loopbackEnabled) {
          audioBuffer.addChunk(chunk.audioData)
          
          // Process if buffer is full
          if (audioBuffer.shouldProcess() && !isProcessingAudio.value) {
            await processAudioBuffer()
          }
        }
      })
      
      // Start capture on backend
      await invoke('start_audio_loopback_capture', {
        deviceId: selectedDevice.value.id
      })
      
      isCapturing.value = true
      console.log('🎤 Started audio capture')
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error)
      captureError.value = message
      console.error('Failed to start capture:', error)
      
      // Cleanup on error
      if (audioEventUnlisten) {
        audioEventUnlisten()
        audioEventUnlisten = null
      }
    }
  }
  
  // Stop audio capture
  const stopCapture = async () => {
    if (!isCapturing.value) return
    
    try {
      // Stop backend capture
      await invoke('stop_audio_loopback_capture')
      
      // Unlisten to events
      if (audioEventUnlisten) {
        audioEventUnlisten()
        audioEventUnlisten = null
      }
      
      // Process any remaining audio
      if (audioBuffer) {
        await processAudioBuffer()
        audioBuffer = null
      }
      
      isCapturing.value = false
      audioLevel.value = -60
      console.log('⏹️ Stopped audio capture')
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error)
      captureError.value = message
      console.error('Failed to stop capture:', error)
    }
  }
  
  // Process audio buffer for transcription
  const processAudioBuffer = async () => {
    if (!audioBuffer || isProcessingAudio.value) return
    
    const audioData = audioBuffer.getAudioData()
    if (!audioData || audioData.data.length === 0) return
    
    // Check minimum interval
    const now = Date.now()
    if (now - lastTranscriptionTime < minTranscriptionInterval) return
    
    isProcessingAudio.value = true
    lastTranscriptionTime = now
    
    try {
      // Convert to base64 for Whisper
      const base64Audio = btoa(String.fromCharCode(...audioData.data))

      // Transcribe using Whisper
      const result = await transcribeAudioBase64(base64Audio, {
        modelSize: 'base',
        sampleRate: settings.value.sampleRate,
        channels: 1, // Always mono after processing
        language: 'en',
        translate: false
      })
      
      if (result.text && result.text.trim().length > 0) {
        const transcription: TranscriptionResult = {
          text: result.text,
          confidence: 0.5, // Default confidence since segments is not available
          timestamp: Date.now(),
          deviceId: selectedDevice.value?.id || ''
        }
        
        // Add to transcriptions
        transcriptions.value.push(transcription)
        
        // Keep only last 50 transcriptions
        if (transcriptions.value.length > 50) {
          transcriptions.value = transcriptions.value.slice(-50)
        }
        
        console.log('📝 Transcription:', transcription.text)
      }
    } catch (error) {
      console.error('Transcription error:', error)
    } finally {
      isProcessingAudio.value = false
    }
  }
  
  // Get recent transcriptions as text
  const getRecentTranscriptionsText = (count = 10): string => {
    return transcriptions.value
      .slice(-count)
      .map(t => t.text)
      .join(' ')
      .trim()
  }
  
  // Clear transcriptions
  const clearTranscriptions = () => {
    transcriptions.value = []
  }
  
  // Save settings
  const saveSettings = async () => {
    try {
      await invoke('save_audio_settings', { settings: settings.value })
      console.log('💾 Audio settings saved')
    } catch (error) {
      console.error('Failed to save settings:', error)
    }
  }
  
  // Load settings
  const loadSettings = async () => {
    try {
      const loaded = await invoke<AudioDeviceSettings | null>('load_audio_settings')
      if (loaded) {
        settings.value = loaded
        
        // Find and select the saved device
        if (loaded.selectedLoopbackDevice) {
          const device = devices.value.find(d => d.id === loaded.selectedLoopbackDevice)
          if (device) {
            selectedDevice.value = device
          }
        }
      }
    } catch (error) {
      console.error('Failed to load settings:', error)
    }
  }
  
  // Cleanup on unmount
  onUnmounted(() => {
    if (isCapturing.value) {
      stopCapture()
    }
  })
  
  return {
    // State
    devices,
    selectedDevice,
    isCapturing,
    isLoadingDevices,
    audioLevel,
    captureError,
    transcriptions,
    isProcessingAudio,
    settings,
    
    // Computed
    hasDevices,
    canCapture,
    
    // Methods
    enumerateDevices,
    autoSelectDevice,
    testDevice,
    selectDevice,
    startCapture,
    stopCapture,
    getRecentTranscriptionsText,
    clearTranscriptions,
    saveSettings,
    loadSettings
  }
}