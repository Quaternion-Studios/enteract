import { defineStore } from 'pinia'
import { ref, computed, watch } from 'vue'
import type { ChatMessage, WindowPosition } from '../types'
import { useSpeechTranscription } from '../composables/useSpeechTranscription'
import { getNextMessageId } from '../composables/messageIdGenerator'

export const useAppStore = defineStore('app', () => {
  // State
  const micEnabled = ref(false)
  const windowCollapsed = ref(false)
  const viewCollapsed = ref(true) // Start collapsed due to compact initial window size
  const isRecording = ref(false)
  const recordingTime = ref(0)
  

  
  // Speech transcription state
  const speechTranscription = useSpeechTranscription()
  const currentTranscriptionId = ref<number | null>(null)
  const isTranscriptionEnabled = ref(false)
  const transcriptionCleanup = ref<(() => void) | null>(null)
  
  const chatMessages = ref<ChatMessage[]>([
    { id: getNextMessageId(), text: "Welcome to your agentic assistant", sender: "assistant", timestamp: new Date() },
    { id: getNextMessageId(), text: "How can I help you today?", sender: "assistant", timestamp: new Date() }
  ])
  
  const windowPosition = ref<WindowPosition>({ x: 0, y: 0 })

  // Computed
  const speechStatus = computed(() => ({
    isInitialized: speechTranscription.isInitialized.value,
    isRecording: speechTranscription.isRecording.value,
    isProcessing: speechTranscription.isProcessing.value,
    hasWebSpeechSupport: speechTranscription.hasWebSpeechSupport.value,
    hasWhisperModel: speechTranscription.hasWhisperModel.value,
    error: speechTranscription.error.value
  }))

  // Actions
  const toggleMic = () => {
    micEnabled.value = !micEnabled.value
  }

  // toggleChat removed - chat is now part of home screen

  const toggleWindowCollapse = () => {
    windowCollapsed.value = !windowCollapsed.value
  }

  const toggleViewCollapse = () => {
    viewCollapsed.value = !viewCollapsed.value
  }

  const toggleRecording = () => {
    isRecording.value = !isRecording.value
    if (isRecording.value) {
      startRecordingTimer()
    } else {
      recordingTime.value = 0
    }
  }



  const startRecordingTimer = () => {
    recordingTime.value = 0
    const interval = setInterval(() => {
      if (!isRecording.value) {
        clearInterval(interval)
        return
      }
      recordingTime.value++
    }, 1000)
  }

  const addMessage = (text: string, sender: 'user' | 'assistant' | 'transcription', options?: {
    isInterim?: boolean
    confidence?: number
    source?: 'web-speech' | 'whisper' | 'typed'
  }) => {
    const message: ChatMessage = {
      id: getNextMessageId(),
      text,
      sender,
      timestamp: new Date(),
      ...options
    }
    
    // For transcription messages, always update the current active transcription
    if (sender === 'transcription') {
      if (currentTranscriptionId.value) {
        // Update existing transcription message
        const existingIndex = chatMessages.value.findIndex(m => m.id === currentTranscriptionId.value)
        if (existingIndex !== -1) {
          // Keep the original ID and timestamp for the thought stream
          chatMessages.value[existingIndex] = {
            ...message,
            id: currentTranscriptionId.value,
            timestamp: chatMessages.value[existingIndex].timestamp
          }
          return
        }
      }
      
      // Create new transcription message
      chatMessages.value.push(message)
      currentTranscriptionId.value = message.id
      return
    }
    
    // For non-transcription messages, just add normally
    chatMessages.value.push(message)
  }

  const finalizeTranscription = () => {
    // Mark the current transcription as final and clear the ID
    if (currentTranscriptionId.value) {
      const existingIndex = chatMessages.value.findIndex(m => m.id === currentTranscriptionId.value)
      if (existingIndex !== -1) {
        chatMessages.value[existingIndex].isInterim = false
      }
      currentTranscriptionId.value = null
    }
  }

  const updateMessage = (messageId: number, newText: string) => {
    const messageIndex = chatMessages.value.findIndex(m => m.id === messageId)
    if (messageIndex !== -1) {
      chatMessages.value[messageIndex].text = newText
      chatMessages.value[messageIndex].source = 'typed' // Mark as manually edited
    }
  }

  // Speech transcription actions
  const initializeSpeechTranscription = async (modelSize: 'tiny' | 'base' | 'small' | 'medium' | 'large' = 'tiny') => {
    try {
      await speechTranscription.initialize({ modelSize })
      isTranscriptionEnabled.value = true
      addMessage("🎤 Speech transcription initialized (tiny model for faster processing)", "assistant")
    } catch (error) {
      console.error('Failed to initialize speech transcription:', error)
      addMessage(`❌ Failed to initialize speech transcription: ${error}`, "assistant")
    }
  }

  const startSpeechTranscription = async () => {
    if (!speechTranscription.isInitialized.value) {
      await initializeSpeechTranscription()
    }
    
    try {
      await speechTranscription.startRecording()
      addMessage("🎙️ Started listening...", "assistant")
      currentTranscriptionId.value = null
      
      // Set up event listeners for transcription updates
      transcriptionCleanup.value = setupTranscriptionEventListeners()
    } catch (error) {
      console.error('Failed to start speech transcription:', error)
      addMessage(`❌ Failed to start recording: ${error}`, "assistant")
    }
  }

  const stopSpeechTranscription = async () => {
    try {
      await speechTranscription.stopRecording()
      addMessage("⏹️ Stopped listening", "assistant")
      currentTranscriptionId.value = null
      
      // Clean up event listeners
      if (transcriptionCleanup.value) {
        transcriptionCleanup.value()
        transcriptionCleanup.value = null
      }
    } catch (error) {
      console.error('Failed to stop speech transcription:', error)
      addMessage(`❌ Failed to stop recording: ${error}`, "assistant")
    }
  }

  const setupTranscriptionEventListeners = () => {
    // Handle interim transcription events
    const handleInterim = (event: Event) => {
      const customEvent = event as CustomEvent
      const text = customEvent.detail.text || ''
      if (text && text.trim()) {
        addMessage(text, 'transcription', {
          isInterim: true,
          source: 'web-speech',
          confidence: customEvent.detail.confidence || 0.5
        })
      }
    }

    // Handle final transcription events
    const handleFinal = (event: Event) => {
      const customEvent = event as CustomEvent
      const text = customEvent.detail.text || ''
      if (text && text.trim()) {
        addMessage(text, 'transcription', {
          isInterim: false,
          source: 'web-speech',
          confidence: customEvent.detail.confidence || 0.9
        })
        // Finalize this transcription
        finalizeTranscription()
      }
    }

    // Handle transcription errors
    const handleError = (event: Event) => {
      const customEvent = event as CustomEvent
      addMessage(`❌ Transcription error: ${customEvent.detail.error}`, 'assistant')
    }

    // Handle transcription completion
    const handleComplete = (event: Event) => {
      const customEvent = event as CustomEvent
      if (customEvent.detail.finalText && customEvent.detail.finalText.trim()) {
        addMessage(customEvent.detail.finalText, 'transcription', {
          isInterim: false,
          source: 'whisper',
          confidence: 0.9
        })
        finalizeTranscription()
      }
    }

    // Handle auto-send transcribed message
    const handleSendTranscribedMessage = (event: Event) => {
      const customEvent = event as CustomEvent
      const text = customEvent.detail.text || ''
      if (text && text.trim()) {
        addMessage(text, 'user', {
          source: 'whisper'
        })
        console.log('📤 Auto-sent transcribed message to chat:', text)
      }
    }

    // Add event listeners
    window.addEventListener('transcription-interim', handleInterim)
    window.addEventListener('transcription-final', handleFinal)
    window.addEventListener('transcription-error', handleError)
    window.addEventListener('transcription-complete', handleComplete)
    window.addEventListener('send-transcribed-message', handleSendTranscribedMessage)

    // Store cleanup function
    const cleanup = () => {
      window.removeEventListener('transcription-interim', handleInterim)
      window.removeEventListener('transcription-final', handleFinal)
      window.removeEventListener('transcription-error', handleError)
      window.removeEventListener('transcription-complete', handleComplete)
      window.removeEventListener('send-transcribed-message', handleSendTranscribedMessage)
    }

    // Return cleanup function for later use
    return cleanup
  }

  const clearTranscription = () => {
    speechTranscription.clearTranscription()
    currentTranscriptionId.value = null
    addMessage("🧹 Transcription history cleared", "assistant")
  }

  const clearChat = () => {
    chatMessages.value = []
    currentTranscriptionId.value = null
    // Add a welcome message back
    addMessage("Welcome to your agentic assistant", "assistant")
    addMessage("How can I help you today?", "assistant")
  }

  const updateWindowPosition = (x: number, y: number) => {
    windowPosition.value = { x, y }
  }

  const formatRecordingTime = () => {
    const minutes = Math.floor(recordingTime.value / 60)
    const seconds = recordingTime.value % 60
    return `${minutes.toString().padStart(2, '0')}:${seconds.toString().padStart(2, '0')}`
  }

  // Event listener for whisper model changes
  const handleWhisperModelChange = async (event: CustomEvent) => {
    try {
      console.log('🔄 App store received whisper model change event:', event.detail)
      
      // Reinitialize speech transcription with new settings
      if (speechTranscription.isInitialized.value) {
        await speechTranscription.reinitializeWithSettings()
        addMessage(`🔄 Updated Whisper models: Mic=${event.detail.microphoneModel}, Audio=${event.detail.loopbackModel}`, "assistant")
      }
    } catch (error) {
      console.error('❌ Failed to reinitialize speech transcription with new models:', error)
      addMessage(`❌ Failed to update Whisper models: ${error}`, "assistant")
    }
  }

  // Set up event listener for whisper model changes
  window.addEventListener('whisper-models-changed', handleWhisperModelChange as unknown as EventListener)

  return {
    // State
    micEnabled,
    windowCollapsed,
    viewCollapsed,
    isRecording,
    recordingTime,
    chatMessages,
    windowPosition,
    isTranscriptionEnabled,
    speechStatus,
    // Actions
    toggleMic,
    toggleWindowCollapse,
    toggleViewCollapse,
    toggleRecording,
    addMessage,
    updateMessage,
    finalizeTranscription,
    updateWindowPosition,
    formatRecordingTime,
    // Speech transcription actions
    initializeSpeechTranscription,
    startSpeechTranscription,
    stopSpeechTranscription,
    clearTranscription,
    clearChat
  }
}) 