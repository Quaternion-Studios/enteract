import { ref, computed, onMounted, onUnmounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type {
  TranscriptionResult,
  WhisperConfig,
  TranscriptionSession,
  SpeechRecognitionConfig,
  AudioStreamConfig,
  TranscriptionStatus
} from '../types/speechTranscription'

// Extend the SpeechRecognition interface
declare global {
  interface Window {
    SpeechRecognition: typeof SpeechRecognition
    webkitSpeechRecognition: typeof SpeechRecognition
  }
}

export function useSpeechTranscription() {
  // State
  const isInitialized = ref(false)
  const isRecording = ref(false)
  const isProcessing = ref(false)
  const isListening = ref(false)
  const isTranscribing = ref(false)
  const hasWebSpeechSupport = ref(false)
  const hasWhisperModel = ref(false)
  const currentText = ref('')
  const interimText = ref('')
  const finalText = ref('')
  const currentTranscript = ref('')
  const transcriptionHistory = ref<TranscriptionResult[]>([])
  const currentSession = ref<TranscriptionSession | null>(null)
  const error = ref<string | null>(null)
  const autoSendToChat = ref(true) // Control whether to auto-send to main chat
  const continuousMode = ref(false) // Keep mic open during conversations

  // Audio recording
  let mediaRecorder: MediaRecorder | null = null
  let audioChunks: Blob[] = []
  let recognition: SpeechRecognition | null = null
  let audioStream: MediaStream | null = null

  // Silence detection and VAD
  let silenceTimer: number | null = null
  let audioContext: AudioContext | null = null
  let analyser: AnalyserNode | null = null
  let silenceThreshold = 0.01
  let silenceDuration = 2000 // 2 seconds for faster response
  let lastAudioTime = 0
  
  // Streaming configuration
  let isVoiceActive = false
  let voiceChunks: Float32Array[] = []
  let silenceFrameCount = 0
  const VAD_THRESHOLD = 0.02 // Energy threshold for voice activity
  const SILENCE_FRAMES_THRESHOLD = 30 // ~1 second at 30fps
  const MAX_CHUNK_FRAMES = 150 // ~5 seconds max chunk size
  let scriptProcessor: ScriptProcessorNode | null = null

  // Configuration
  const defaultWhisperConfig: WhisperConfig = {
    modelSize: 'small',
    language: 'en',
    enableVAD: true,
    silenceThreshold: 0.01,
    maxSegmentLength: 30
  }

  const defaultSpeechConfig: SpeechRecognitionConfig = {
    continuous: true,
    interimResults: true,
    language: 'en-US',
    maxAlternatives: 1
  }

  const defaultAudioConfig: AudioStreamConfig = {
    sampleRate: 16000,
    channels: 1,
    bufferSize: 4096,
    mimeType: 'audio/wav'
  }

  // Computed
  const status = computed<TranscriptionStatus>(() => ({
    isRecording: isRecording.value,
    isProcessing: isProcessing.value,
    hasWebSpeechSupport: hasWebSpeechSupport.value,
    hasWhisperModel: hasWhisperModel.value,
    currentSession: currentSession.value || undefined
  }))

  const combinedText = computed(() => {
    if (interimText.value) {
      return finalText.value + ' ' + interimText.value
    }
    return finalText.value
  })

  // Initialize the transcription system
  async function initialize(whisperConfig: Partial<WhisperConfig> = {}) {
    try {
      error.value = null

      // Check Web Speech API support
      const SpeechRecognition = window.SpeechRecognition || window.webkitSpeechRecognition
      hasWebSpeechSupport.value = !!SpeechRecognition

      if (!hasWebSpeechSupport.value) {
        console.warn('Web Speech API not supported in this browser. Falling back to Whisper only.')
      }

      if (hasWebSpeechSupport.value) {
        recognition = new SpeechRecognition()
        setupSpeechRecognition()
      }

      // Load settings to get the selected whisper model for microphone
      let selectedModel = defaultWhisperConfig.modelSize
      let enableGpu = true
      try {
        console.log('🔍 [SPEECH_INIT] Loading general settings...')
        const storedSettings = await invoke<any>('load_general_settings')
        console.log('🔍 [SPEECH_INIT] Stored settings:', storedSettings)
        
        if (storedSettings?.microphoneWhisperModel) {
          selectedModel = storedSettings.microphoneWhisperModel
          console.log(`🎤 [SPEECH_INIT] Using stored microphone Whisper model: ${selectedModel}`)
        } else {
          console.log(`🎤 [SPEECH_INIT] No stored microphone model, using default: ${selectedModel}`)
        }
        
        if (storedSettings?.enableGpuAcceleration !== undefined) {
          enableGpu = storedSettings.enableGpuAcceleration
          console.log(`🎮 [SPEECH_INIT] GPU acceleration: ${enableGpu ? 'enabled' : 'disabled'}`)
        } else {
          console.log(`🎮 [SPEECH_INIT] No GPU setting found, using default: ${enableGpu}`)
        }
      } catch (settingsError) {
        console.warn('❌ [SPEECH_INIT] Failed to load model settings, using default:', settingsError)
      }

      // Initialize Whisper model with selected model size
      const config = { 
        ...defaultWhisperConfig, 
        modelSize: selectedModel,
        ...whisperConfig 
      }
      
      // Check if model exists with proper error handling
      try {
        const modelExists = await invoke<boolean>('check_whisper_model_availability', {
          modelSize: config.modelSize
        })

        if (!modelExists) {
          console.log(`Downloading Whisper model: ${config.modelSize}`)
          await invoke<string>('download_whisper_model', {
            modelSize: config.modelSize
          })
        }

        // Initialize the model with GPU support
        console.log(`🚀 [SPEECH_INIT] Calling initialize_whisper_model_with_gpu with:`, {
          modelSize: config.modelSize,
          language: config.language,
          enableVad: config.enableVAD,
          silenceThreshold: config.silenceThreshold,
          maxSegmentLength: config.maxSegmentLength,
          useGpu: enableGpu
        })
        
        const initResult = await invoke<string>('initialize_whisper_model_with_gpu', {
          config: {
            modelSize: config.modelSize,
            language: config.language,
            enableVad: config.enableVAD,
            silenceThreshold: config.silenceThreshold,
            maxSegmentLength: config.maxSegmentLength
          },
          useGpu: enableGpu
        })
        
        console.log(`✅ [SPEECH_INIT] Whisper initialization result:`, initResult)

        hasWhisperModel.value = true
      } catch (whisperError) {
        console.warn('Whisper initialization failed:', whisperError)
        hasWhisperModel.value = false
        
        // Continue without Whisper if Web Speech API is available
        if (!hasWebSpeechSupport.value) {
          throw new Error(`Both Web Speech API and Whisper failed. Whisper error: ${whisperError}`)
        }
      }

      isInitialized.value = true
      
      console.log('Speech transcription system initialized successfully', {
        webSpeech: hasWebSpeechSupport.value,
        whisper: hasWhisperModel.value
      })
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err)
      error.value = `Failed to initialize: ${message}`
      console.error('Initialization error:', err)
      throw err
    }
  }

  // Setup Web Speech Recognition
  function setupSpeechRecognition() {
    if (!recognition) return

    recognition.continuous = defaultSpeechConfig.continuous
    recognition.interimResults = defaultSpeechConfig.interimResults
    recognition.lang = defaultSpeechConfig.language
    recognition.maxAlternatives = defaultSpeechConfig.maxAlternatives

    recognition.onstart = () => {
      console.log('Speech recognition started')
      isListening.value = true
      error.value = null
      emitTranscriptionEvent('transcription-started')
    }

    recognition.onresult = (event) => {
      let interim = ''
      let final = ''

      for (let i = event.resultIndex; i < event.results.length; i++) {
        const result = event.results[i]
        const transcript = result[0].transcript

        if (result.isFinal) {
          final += transcript
        } else {
          interim += transcript
        }
      }

      if (final) {
        finalText.value += (finalText.value ? ' ' : '') + final
        interimText.value = ''
        currentTranscript.value = finalText.value
        
        // Add to history
        const transcriptionResult: TranscriptionResult = {
          text: final,
          isFinal: true,
          confidence: event.results[event.resultIndex]?.[0]?.confidence || 0.9,
          timestamp: Date.now(),
          source: 'web-speech'
        }
        
        transcriptionHistory.value.push(transcriptionResult)
        
        // Emit final transcription event
        emitTranscriptionEvent('transcription-final', {
          text: final,
          confidence: transcriptionResult.confidence,
          timestamp: transcriptionResult.timestamp
        })
        
        // Reset silence timer on speech activity
        resetSilenceTimer()
        lastAudioTime = Date.now()
      } else {
        interimText.value = interim
        currentTranscript.value = finalText.value + (finalText.value ? ' ' : '') + interim
        
        // Emit interim transcription event
        emitTranscriptionEvent('transcription-interim', {
          text: currentTranscript.value,
          confidence: 0.5,
          timestamp: Date.now()
        })
        
        // Reset silence timer on interim results
        resetSilenceTimer()
        lastAudioTime = Date.now()
      }
    }

    recognition.onerror = (event) => {
      console.log('Speech recognition error:', event.error)
      
      // Handle different types of errors differently
      if (event.error === 'aborted') {
        console.log('Speech recognition was aborted (likely due to wake word detection conflict)')
        // Don't emit error for aborted - it's usually intentional
        return
      }
      
      if (event.error === 'no-speech') {
        console.log('No speech detected - continuing...')
        // Don't emit error for no-speech - it's normal
        return
      }
      
      // Only emit errors for actual problems
      if (event.error !== 'network' || isRecording.value) {
        emitTranscriptionEvent('transcription-error', {
          error: event.error,
          message: `Speech recognition error: ${event.error}`
        })
      }
    }

    recognition.onend = () => {
      console.log('Speech recognition ended')
      isListening.value = false
      
      if (isRecording.value && hasWebSpeechSupport.value) {
        // Implement retry logic with backoff
        setTimeout(() => {
          if (isRecording.value && recognition) {
            try {
              recognition.start()
            } catch (err) {
              console.warn('Failed to restart speech recognition:', err)
              error.value = 'Failed to restart speech recognition. Please try again.'
            }
          }
        }, 1000) // 1 second delay before retry
      } else {
        emitTranscriptionEvent('transcription-stopped')
      }
    }
  }

  // Event emitter for transcription updates
  function emitTranscriptionEvent(eventType: string, data?: any) {
    const event = new CustomEvent(eventType, { 
      detail: { 
        ...data,
        isRecording: isRecording.value,
        isTranscribing: isTranscribing.value,
        currentTranscript: currentTranscript.value,
        finalText: finalText.value,
        interimText: interimText.value
      } 
    })
    window.dispatchEvent(event)
  }

  // Silence detection setup with streaming VAD
  function setupSilenceDetection(stream: MediaStream) {
    try {
      audioContext = new (window.AudioContext || (window as any).webkitAudioContext)({ sampleRate: 16000 })
      const source = audioContext.createMediaStreamSource(stream)
      analyser = audioContext.createAnalyser()
      
      analyser.fftSize = 512
      analyser.minDecibels = -90
      analyser.maxDecibels = -10
      analyser.smoothingTimeConstant = 0.85
      
      source.connect(analyser)
      
      // Setup script processor for real-time audio processing
      scriptProcessor = audioContext.createScriptProcessor(4096, 1, 1)
      scriptProcessor.onaudioprocess = handleAudioProcess
      
      source.connect(scriptProcessor)
      scriptProcessor.connect(audioContext.destination)
      
      monitorAudioLevel()
    } catch (err) {
      console.warn('Failed to setup silence detection:', err)
    }
  }

  // Monitor audio levels for silence detection
  function monitorAudioLevel() {
    if (!analyser || !isRecording.value) return
    
    const bufferLength = analyser.frequencyBinCount
    const dataArray = new Uint8Array(bufferLength)
    analyser.getByteFrequencyData(dataArray)
    
    // Calculate RMS (Root Mean Square) for audio level
    let sum = 0
    for (let i = 0; i < bufferLength; i++) {
      sum += dataArray[i] * dataArray[i]
    }
    const rms = Math.sqrt(sum / bufferLength) / 255
    
    if (rms > silenceThreshold) {
      // Audio detected, reset silence timer
      resetSilenceTimer()
      lastAudioTime = Date.now()
    } else {
      // Silence detected, start/continue timer
      if (!silenceTimer && isRecording.value) {
        startSilenceTimer()
      }
    }
    
    // Continue monitoring
    requestAnimationFrame(monitorAudioLevel)
  }

  // Start silence timer
  function startSilenceTimer() {
    if (silenceTimer) return
    
    // Don't auto-stop in continuous mode (for conversations)
    if (continuousMode.value) return
    
    silenceTimer = window.setTimeout(() => {
      const timeSinceLastAudio = Date.now() - lastAudioTime
      if (timeSinceLastAudio >= silenceDuration && isRecording.value && !continuousMode.value) {
        console.log('Auto-stopping transcription due to silence')
        stopRecording()
        emitTranscriptionEvent('transcription-auto-stopped', { reason: 'silence' })
      }
    }, silenceDuration)
  }

  // Reset silence timer
  function resetSilenceTimer() {
    if (silenceTimer) {
      clearTimeout(silenceTimer)
      silenceTimer = null
    }
  }

  // Calculate audio energy for VAD
  function calculateAudioEnergy(buffer: Float32Array): number {
    let sum = 0
    for (let i = 0; i < buffer.length; i++) {
      sum += buffer[i] * buffer[i]
    }
    return Math.sqrt(sum / buffer.length)
  }

  // Handle real-time audio processing with VAD
  function handleAudioProcess(event: AudioProcessingEvent) {
    if (!isRecording.value) return

    const inputBuffer = event.inputBuffer.getChannelData(0)
    const energy = calculateAudioEnergy(inputBuffer)
    const isCurrentlyActive = energy > VAD_THRESHOLD

    if (isCurrentlyActive) {
      isVoiceActive = true
      silenceFrameCount = 0
      voiceChunks.push(new Float32Array(inputBuffer))
    } else {
      silenceFrameCount++

      // If we have voice data and sufficient silence, process chunk
      if (isVoiceActive && silenceFrameCount > SILENCE_FRAMES_THRESHOLD) {
        processStreamingChunk(voiceChunks.splice(0))
        isVoiceActive = false
      }
    }

    // Prevent chunks from getting too large
    if (voiceChunks.length > MAX_CHUNK_FRAMES) {
      processStreamingChunk(voiceChunks.splice(0, voiceChunks.length / 2))
    }
  }

  // Process streaming audio chunk
  async function processStreamingChunk(chunks: Float32Array[]) {
    if (chunks.length === 0 || !hasWhisperModel.value) return

    try {
      // Concatenate audio chunks
      const totalLength = chunks.reduce((sum, chunk) => sum + chunk.length, 0)
      const audioBuffer = new Float32Array(totalLength)
      let offset = 0
      for (const chunk of chunks) {
        audioBuffer.set(chunk, offset)
        offset += chunk.length
      }

      // Convert to WAV
      const wavBlob = await createWavBlob(audioBuffer, 16000)
      const audioBase64 = await blobToBase64(wavBlob)

      // Calculate appropriate timeout based on chunk duration
      const chunkDuration = audioBuffer.length / 16000
      const timeoutMs = Math.max(3000, chunkDuration * 1000 * 2) // 2x real-time

      console.log(`🎯 Processing ${chunkDuration.toFixed(1)}s chunk with VAD`)

      // Get GPU setting
      const enableGpu = await getGpuSettingFromStorage()

      // Process with Whisper
      const result = await invoke<{
        text: string
        confidence: number
        start_time: number
        end_time: number
        language?: string
      }>('transcribe_audio_base64', {
        audioData: audioBase64,
        config: {
          modelSize: defaultWhisperConfig.modelSize,
          language: defaultWhisperConfig.language,
          enableVad: defaultWhisperConfig.enableVAD,
          silenceThreshold: defaultWhisperConfig.silenceThreshold,
          maxSegmentLength: defaultWhisperConfig.maxSegmentLength
        }
      })

      if (result.text.trim()) {
        // Append to final text
        finalText.value = (finalText.value + ' ' + result.text).trim()
        currentTranscript.value = finalText.value
        
        // Add to history
        const transcriptionResult: TranscriptionResult = {
          text: result.text,
          finalText: result.text,
          isFinal: true,
          confidence: result.confidence,
          timestamp: Date.now(),
          source: 'whisper-stream',
          sessionId: currentSession.value?.id
        }
        
        transcriptionHistory.value.push(transcriptionResult)
        
        // Emit streaming update
        emitTranscriptionEvent('streaming-update', {
          text: result.text,
          totalText: finalText.value
        })
      }
    } catch (err) {
      console.warn('Chunk processing failed:', err)
    }
  }

  // Helper function to get GPU setting from storage
  async function getGpuSettingFromStorage(): Promise<boolean> {
    try {
      const settings = await invoke<any>('load_general_settings')
      return settings?.enableGpuAcceleration ?? true
    } catch {
      return true // Default to enabled
    }
  }

  // Create WAV blob from Float32Array
  async function createWavBlob(audioData: Float32Array, sampleRate: number): Promise<Blob> {
    const length = audioData.length
    const arrayBuffer = new ArrayBuffer(44 + length * 2)
    const view = new DataView(arrayBuffer)

    // WAV header
    const writeString = (offset: number, string: string) => {
      for (let i = 0; i < string.length; i++) {
        view.setUint8(offset + i, string.charCodeAt(i))
      }
    }

    writeString(0, 'RIFF')
    view.setUint32(4, 36 + length * 2, true)
    writeString(8, 'WAVE')
    writeString(12, 'fmt ')
    view.setUint32(16, 16, true) // fmt chunk size
    view.setUint16(20, 1, true) // PCM format
    view.setUint16(22, 1, true) // Mono
    view.setUint32(24, sampleRate, true)
    view.setUint32(28, sampleRate * 2, true) // byte rate
    view.setUint16(32, 2, true) // block align
    view.setUint16(34, 16, true) // bits per sample
    writeString(36, 'data')
    view.setUint32(40, length * 2, true)

    // Convert float32 to int16
    let offset = 44
    for (let i = 0; i < length; i++) {
      const sample = Math.max(-1, Math.min(1, audioData[i]))
      view.setInt16(offset, sample * 0x7FFF, true)
      offset += 2
    }

    return new Blob([arrayBuffer], { type: 'audio/wav' })
  }

  // Request microphone permission
  async function requestMicrophonePermission(): Promise<boolean> {
    try {
      const stream = await navigator.mediaDevices.getUserMedia({ audio: true })
      stream.getTracks().forEach(track => track.stop())
      return true
    } catch (error) {
      console.error('Microphone permission denied:', error)
      return false
    }
  }

  // Start recording with enhanced features
  async function startRecording() {
    if (!isInitialized.value) {
      throw new Error('Transcription system not initialized')
    }

    try {
      error.value = null
      
      // Check microphone permission first
      const hasPermission = await requestMicrophonePermission()
      if (!hasPermission) {
        throw new Error('Microphone permission required for speech transcription')
      }
      
      isRecording.value = true
      isTranscribing.value = true

      // Clear previous results
      interimText.value = ''
      finalText.value = ''
      currentTranscript.value = ''
      audioChunks = []
      lastAudioTime = Date.now()

      // Create session
      currentSession.value = {
        id: `session_${Date.now()}`,
        isActive: true,
        startTime: Date.now(),
        language: defaultWhisperConfig.language || 'en',
        config: defaultWhisperConfig
      }

      // Get audio stream
      audioStream = await navigator.mediaDevices.getUserMedia({
        audio: {
          sampleRate: defaultAudioConfig.sampleRate,
          channelCount: defaultAudioConfig.channels,
          echoCancellation: true,
          noiseSuppression: true
        }
      })

      // Setup silence detection
      setupSilenceDetection(audioStream)

      // Start Web Speech API for interim results
      if (recognition && hasWebSpeechSupport.value) {
        try {
          recognition.start()
          console.log('🗣️ Web Speech API started for interim results')
        } catch (speechError) {
          console.warn('Web Speech API failed to start:', speechError)
          // Continue with audio recording only
        }
      }

      // Start audio recording for Whisper processing (if available)
      if (hasWhisperModel.value && audioStream) {
        // Try to use the best available audio format
        let mimeType = 'audio/webm;codecs=opus'
        if (MediaRecorder.isTypeSupported('audio/wav')) {
          mimeType = 'audio/wav'
        } else if (MediaRecorder.isTypeSupported('audio/webm')) {
          mimeType = 'audio/webm'
        } else if (MediaRecorder.isTypeSupported('audio/mp4')) {
          mimeType = 'audio/mp4'
        }

        mediaRecorder = new MediaRecorder(audioStream, {
          mimeType: mimeType
        })

        mediaRecorder.ondataavailable = (event) => {
          if (event.data.size > 0) {
            audioChunks.push(event.data)
          }
        }

        mediaRecorder.onstop = async () => {
          if (audioChunks.length > 0) {
            await processAudioWithWhisper()
          }
        }

        // Record in chunks for real-time processing
        mediaRecorder.start(1000) // 1 second chunks
        
        console.log(`🎤 Started recording with MIME type: ${mimeType}`)
      }

      console.log('Recording started with silence detection', {
        webSpeech: hasWebSpeechSupport.value && !!recognition,
        whisper: hasWhisperModel.value,
        silenceThreshold: silenceThreshold,
        silenceDuration: silenceDuration
      })

    } catch (err) {
      isRecording.value = false
      isTranscribing.value = false
      const message = err instanceof Error ? err.message : String(err)
      error.value = `Failed to start recording: ${message}`
      throw err
    }
  }

  // Enhanced stop recording with cleanup and auto-send
  const stopRecording = async () => {
    if (!isRecording.value) return

    try {
      // Immediately reset recording state for responsive UX
      isRecording.value = false
      isTranscribing.value = false
      
      // Clean up silence detection
      resetSilenceTimer()
      if (audioContext && audioContext.state !== 'closed') {
        // Disconnect script processor
        if (scriptProcessor) {
          scriptProcessor.disconnect()
          scriptProcessor = null
        }
        await audioContext.close()
        audioContext = null
        analyser = null
      }
      
      // Clear voice chunks
      voiceChunks = []
      isVoiceActive = false
      silenceFrameCount = 0

      // Stop Web Speech API
      if (recognition) {
        recognition.stop()
        isListening.value = false
      }

      // Stop MediaRecorder
      if (mediaRecorder && mediaRecorder.state !== 'inactive') {
        mediaRecorder.stop()
      }

      // Stop audio stream
      if (audioStream) {
        audioStream.getTracks().forEach(track => track.stop())
        audioStream = null
      }

      // End session
      if (currentSession.value) {
        currentSession.value.isActive = false
        currentSession.value.endTime = Date.now()
      }

      console.log('Recording stopped - button state immediately reset')
      
      // Auto-send the transcribed message if there's content and auto-send is enabled
      if (finalText.value.trim() && autoSendToChat.value) {
        // Emit event to send the message to chat
        const sendMessageEvent = new CustomEvent('send-transcribed-message', {
          detail: {
            text: finalText.value.trim(),
            timestamp: Date.now()
          }
        })
        window.dispatchEvent(sendMessageEvent)
        console.log('📤 Auto-sending transcribed message:', finalText.value.trim())
      }
      
      // Emit final state
      emitTranscriptionEvent('transcription-complete', {
        finalText: finalText.value,
        totalDuration: currentSession.value ? 
          (Date.now() - currentSession.value.startTime) / 1000 : 0
      })

    } catch (err) {
      console.error('Error stopping recording:', err)
      error.value = `Error stopping recording: ${err}`
    } finally {
      // Ensure processing state is reset even if there are errors
      isProcessing.value = false
    }
  }

  // Auto-start transcription (called by wake word detection)
  async function startTranscription() {
    console.log('🎤 Starting transcription triggered by mic button')
    try {
      await startRecording()
      emitTranscriptionEvent('mic-button-triggered')
    } catch (err) {
      console.error('Failed to start transcription from mic button:', err)
      error.value = `Failed to start transcription: ${err}`
    }
  }

  // Process audio with Whisper (optimized for tiny model)
  async function processAudioWithWhisper() {
    if (audioChunks.length === 0) {
      console.log('⚠️ No audio chunks to process')
      return
    }

    try {
      // Don't set isProcessing to true here - let background processing happen
      // without blocking the UI state
      console.log(`🔄 Processing ${audioChunks.length} audio chunks with Whisper (small model)...`)

      // Combine audio chunks
      const audioBlob = new Blob(audioChunks, { type: mediaRecorder?.mimeType || 'audio/webm' })
      console.log(`📦 Combined audio blob: ${audioBlob.size} bytes, type: ${audioBlob.type}`)
      
      // Convert audio to WAV format for Whisper
      const wavBlob = await convertToWav(audioBlob)
      console.log(`🎵 Converted to WAV: ${wavBlob.size} bytes`)
      
      // Convert to base64
      const audioBase64 = await blobToBase64(wavBlob)
      console.log(`📝 Base64 audio data length: ${audioBase64.length}`)

      // Send to Whisper for transcription with timeout for tiny model
      const result = await Promise.race([
        invoke<{
          text: string
          confidence: number
          start_time: number
          end_time: number
          language?: string
        }>('transcribe_audio_base64', {
          audioData: audioBase64,
          config: {
            modelSize: defaultWhisperConfig.modelSize,
            language: defaultWhisperConfig.language,
            enableVad: defaultWhisperConfig.enableVAD,
            silenceThreshold: defaultWhisperConfig.silenceThreshold,
            maxSegmentLength: defaultWhisperConfig.maxSegmentLength
          }
        }),
        // Timeout after 15 seconds for small model (slower than tiny but more accurate)
        new Promise<never>((_, reject) => 
          setTimeout(() => reject(new Error('Whisper processing timeout')), 15000)
        )
      ])

      console.log('🎯 Whisper result:', result)

      if (result.text.trim()) {
        // Update final text with Whisper result
        const newText = result.text.trim()
        
        // If we have interim text from Web Speech API, replace it
        if (interimText.value || !finalText.value) {
          finalText.value = newText
          interimText.value = ''
        } else {
          // Append to existing text
          finalText.value += (finalText.value ? ' ' : '') + newText
        }
        
        currentTranscript.value = finalText.value

        // Add to history
        const transcriptionResult: TranscriptionResult = {
          text: newText,
          isFinal: true,
          confidence: result.confidence,
          timestamp: Date.now(),
          source: 'whisper'
        }

        transcriptionHistory.value.push(transcriptionResult)

        // Emit final transcription event
        emitTranscriptionEvent('transcription-final', {
          text: newText,
          confidence: result.confidence,
          timestamp: Date.now()
        })

        console.log('✅ Whisper transcription (small model):', newText)
      } else {
        console.log('ℹ️ Whisper returned empty text')
      }

      // Clear processed chunks
      audioChunks = []
    } catch (err) {
      console.error('❌ Whisper processing error:', err)
      error.value = `Whisper processing failed: ${err}`
      
      // Emit error event for UI feedback
      emitTranscriptionEvent('transcription-error', {
        error: err instanceof Error ? err.message : String(err),
        timestamp: Date.now()
      })
    }
    // Note: No finally block - we don't want to reset isProcessing here
    // as it should be managed by the recording state, not processing state
  }

  // Convert audio blob to WAV format using Web Audio API
  async function convertToWav(audioBlob: Blob): Promise<Blob> {
    try {
      const arrayBuffer = await audioBlob.arrayBuffer()
      const audioContext = new (window.AudioContext || (window as any).webkitAudioContext)({ sampleRate: 16000 })
      
      let audioBuffer: AudioBuffer
      try {
        // Try to decode the audio buffer
        audioBuffer = await audioContext.decodeAudioData(arrayBuffer)
      } catch (decodeError) {
        console.warn('Failed to decode audio data, using original blob:', decodeError)
        audioContext.close()
        return audioBlob
      }
      
      // Resample to 16kHz mono if needed
      const sampleRate = 16000
      const channels = 1
      const length = Math.floor(audioBuffer.duration * sampleRate)
      
      const offlineContext = new OfflineAudioContext(channels, length, sampleRate)
      const source = offlineContext.createBufferSource()
      source.buffer = audioBuffer
      source.connect(offlineContext.destination)
      source.start()
      
      const resampledBuffer = await offlineContext.startRendering()
      
      // Convert to WAV format
      const wavBuffer = createWavBuffer(resampledBuffer)
      const wavBlob = new Blob([wavBuffer], { type: 'audio/wav' })
      
      audioContext.close()
      return wavBlob
      
    } catch (error) {
      console.warn('Audio conversion failed, using original blob:', error)
      return audioBlob
    }
  }

  // Create WAV buffer from AudioBuffer
  function createWavBuffer(audioBuffer: AudioBuffer): ArrayBuffer {
    const length = audioBuffer.length
    const sampleRate = audioBuffer.sampleRate
    const arrayBuffer = new ArrayBuffer(44 + length * 2)
    const view = new DataView(arrayBuffer)
    const channels = audioBuffer.numberOfChannels
    
    // WAV header
    const writeString = (offset: number, string: string) => {
      for (let i = 0; i < string.length; i++) {
        view.setUint8(offset + i, string.charCodeAt(i))
      }
    }
    
    // RIFF chunk descriptor
    writeString(0, 'RIFF')
    view.setUint32(4, 36 + length * 2, true)
    writeString(8, 'WAVE')
    
    // fmt sub-chunk
    writeString(12, 'fmt ')
    view.setUint32(16, 16, true) // sub-chunk size
    view.setUint16(20, 1, true) // audio format (1 = PCM)
    view.setUint16(22, 1, true) // number of channels (mono)
    view.setUint32(24, sampleRate, true) // sample rate
    view.setUint32(28, sampleRate * 2, true) // byte rate
    view.setUint16(32, 2, true) // block align
    view.setUint16(34, 16, true) // bits per sample
    
    // data sub-chunk
    writeString(36, 'data')
    view.setUint32(40, length * 2, true)
    
    // Convert float samples to 16-bit PCM
    const channelData = audioBuffer.getChannelData(0) // Use first channel for mono
    let offset = 44
    for (let i = 0; i < length; i++) {
      const sample = Math.max(-1, Math.min(1, channelData[i]))
      view.setInt16(offset, sample < 0 ? sample * 0x8000 : sample * 0x7FFF, true)
      offset += 2
    }
    
    return arrayBuffer
  }

  // Clear transcription
  function clearTranscription() {
    finalText.value = ''
    interimText.value = ''
    transcriptionHistory.value = []
    error.value = null
  }

  // Control auto-send to chat
  function setAutoSendToChat(enabled: boolean) {
    autoSendToChat.value = enabled
    console.log(`📤 Auto-send to chat ${enabled ? 'enabled' : 'disabled'}`)
  }

  // Control continuous mode (keeps mic open during conversations)
  function setContinuousMode(enabled: boolean) {
    continuousMode.value = enabled
    console.log(`🎤 Continuous mode ${enabled ? 'enabled' : 'disabled'}`)
  }

  // Reinitialize with new model settings
  async function reinitializeWithSettings() {
    try {
      console.log('🔄 Reinitializing speech transcription with new settings...')
      
      // Store current recording state
      const wasRecording = isRecording.value
      
      // Stop recording if active
      if (wasRecording) {
        await stopRecording()
      }
      
      // Reset initialization state
      isInitialized.value = false
      hasWhisperModel.value = false
      
      // Reinitialize with current settings
      await initialize()
      
      console.log('✅ Speech transcription reinitialized successfully')
    } catch (error) {
      console.error('❌ Failed to reinitialize speech transcription:', error)
      throw error
    }
  }

  // Get available models
  async function getAvailableModels() {
    try {
      return await invoke<string[]>('list_available_models')
    } catch (err) {
      console.error('Failed to get available models:', err)
      return []
    }
  }

  // Helper function to convert blob to base64
  function blobToBase64(blob: Blob): Promise<string> {
    return new Promise((resolve, reject) => {
      const reader = new FileReader()
      reader.onload = () => {
        const result = reader.result as string
        // Remove data URL prefix to get just the base64 data
        const base64 = result.split(',')[1]
        resolve(base64)
      }
      reader.onerror = reject
      reader.readAsDataURL(blob)
    })
  }

  // Listen for whisper model changes from settings
  onMounted(() => {
    const handleWhisperModelChange = async (event: any) => {
      console.log('🔄 [SPEECH_TRANSCRIPTION] Whisper models changed event received:', event.detail)
      
      // Update the default config with new model
      if (event.detail.microphoneModel) {
        console.log('🔄 [SPEECH_TRANSCRIPTION] Updating microphone model from', defaultWhisperConfig.modelSize, 'to', event.detail.microphoneModel)
        defaultWhisperConfig.modelSize = event.detail.microphoneModel
      }
      
      // Reinitialize with new settings
      try {
        console.log('🔄 [SPEECH_TRANSCRIPTION] Reinitializing with new model:', defaultWhisperConfig.modelSize)
        await initialize()
        console.log('✅ [SPEECH_TRANSCRIPTION] Reinitialization complete')
      } catch (error) {
        console.error('❌ [SPEECH_TRANSCRIPTION] Failed to reinitialize whisper:', error)
      }
    }
    
    console.log('🎧 [SPEECH_TRANSCRIPTION] Setting up whisper-models-changed event listener')
    window.addEventListener('whisper-models-changed', handleWhisperModelChange)
    
    // Cleanup listener on unmount
    onUnmounted(() => {
      console.log('🎧 [SPEECH_TRANSCRIPTION] Removing whisper-models-changed event listener')
      window.removeEventListener('whisper-models-changed', handleWhisperModelChange)
    })
  })

  // Cleanup
  onUnmounted(() => {
    if (isRecording.value) {
      stopRecording().catch(console.error)
    }
  })

  return {
    // State
    isInitialized,
    isRecording,
    isProcessing,
    isListening,
    isTranscribing,
    hasWebSpeechSupport,
    hasWhisperModel,
    currentText: combinedText,
    interimText,
    finalText,
    currentTranscript,
    transcriptionHistory,
    currentSession,
    error,
    status,
    autoSendToChat,
    continuousMode,

    // Methods
    initialize,
    startRecording,
    stopRecording,
    clearTranscription,
    getAvailableModels,
    startTranscription,
    setAutoSendToChat,
    setContinuousMode,
    reinitializeWithSettings
  }
} 