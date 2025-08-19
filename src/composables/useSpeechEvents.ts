// Removed unused imports
import { getNextMessageId } from './messageIdGenerator'

export const useSpeechEvents = (
  chatHistory: any,
  showChatWindow: any,
  scrollChatToBottom: () => void,
  chatMessage: any,
  sendMessage: (agentType?: string) => Promise<void>
) => {
  const setupSpeechTranscriptionListeners = () => {
    window.addEventListener('transcription-started', handleTranscriptionStarted)
    window.addEventListener('transcription-interim', handleTranscriptionInterim)
    window.addEventListener('transcription-final', handleTranscriptionFinal)
    window.addEventListener('transcription-error', handleTranscriptionError)
    window.addEventListener('transcription-stopped', handleTranscriptionStopped)
    window.addEventListener('transcription-complete', handleTranscriptionComplete)
    window.addEventListener('transcription-auto-stopped', handleTranscriptionAutoStopped)
  }

  const removeSpeechTranscriptionListeners = () => {
    window.removeEventListener('transcription-started', handleTranscriptionStarted)
    window.removeEventListener('transcription-interim', handleTranscriptionInterim)
    window.removeEventListener('transcription-final', handleTranscriptionFinal)
    window.removeEventListener('transcription-error', handleTranscriptionError)
    window.removeEventListener('transcription-stopped', handleTranscriptionStopped)
    window.removeEventListener('transcription-complete', handleTranscriptionComplete)
    window.removeEventListener('transcription-auto-stopped', handleTranscriptionAutoStopped)
  }

  const handleTranscriptionStarted = (event: Event) => {
    const customEvent = event as CustomEvent
    console.log('🎤 Transcription started', customEvent.detail)
    
    // Auto-open chat window when transcription starts by emitting an event
    if (!showChatWindow.value) {
      const openChatEvent = new CustomEvent('speech-open-chat-window', {
        detail: { source: 'transcription-started' }
      })
      window.dispatchEvent(openChatEvent)
      
      setTimeout(() => {
        scrollChatToBottom()
      }, 150)
    }
  }

  const handleTranscriptionInterim = (event: Event) => {
    const customEvent = event as CustomEvent
    console.log('🎤 Interim transcription', customEvent.detail)
    
    const interimText = customEvent.detail.text || ''
    if (interimText.trim()) {
      // Add or update interim message
      const lastMessage = chatHistory.value[chatHistory.value.length - 1]
      if (lastMessage && lastMessage.sender === 'transcription' && lastMessage.isInterim) {
        // Update existing interim message
        lastMessage.text = interimText
      } else {
        // Add new interim message
        chatHistory.value.push({
          id: getNextMessageId(),
          sender: 'transcription',
          text: interimText,
          timestamp: new Date(),
          messageType: 'text',
          isInterim: true,
          confidence: customEvent.detail.confidence || 0.5
        })
      }
      
      setTimeout(() => {
        scrollChatToBottom()
      }, 50)
    }
  }

  const handleTranscriptionFinal = async (event: Event) => {
    const customEvent = event as CustomEvent
    console.log('🎤 Final transcription', customEvent.detail)
    
    const finalText = customEvent.detail.text || ''
    if (finalText.trim()) {
      // Remove any interim transcription messages first
      const lastMessage = chatHistory.value[chatHistory.value.length - 1]
      if (lastMessage && lastMessage.sender === 'transcription' && lastMessage.isInterim) {
        chatHistory.value.pop() // Remove interim message
      }
      
      // Auto-send transcribed text to AI if confidence is high enough
      if ((customEvent.detail.confidence || 0.9) > 0.7) {
        console.log('🎤 Converting speech to user message and sending to Gemma:', finalText)
        
        // Set the transcribed text as the chat message and send it as a normal user message
        chatMessage.value = finalText
        await sendMessage('enteract') // This will add it as a user message and get AI response
      } else {
        // If confidence is low, still add as user message but with a note
        chatHistory.value.push({
          id: getNextMessageId(),
          sender: 'user',
          text: `${finalText} (transcribed)`,
          timestamp: new Date(),
          messageType: 'text'
        })
        
        setTimeout(() => {
          scrollChatToBottom()
        }, 50)
      }
    }
  }

  const handleTranscriptionError = (event: Event) => {
    const customEvent = event as CustomEvent
    console.log('❌ Transcription error', customEvent.detail)
    
    // Add error message to chat
    chatHistory.value.push({
      id: messageIdCounter++,
      sender: 'assistant',
      text: `❌ Transcription error: ${customEvent.detail.error}`,
      timestamp: new Date(),
      messageType: 'text'
    })
    
    setTimeout(() => {
      scrollChatToBottom()
    }, 50)
  }

  const handleTranscriptionStopped = (event: Event) => {
    const customEvent = event as CustomEvent
    console.log('⏹️ Transcription stopped', customEvent.detail)
  }

  const handleTranscriptionComplete = (event: Event) => {
    const customEvent = event as CustomEvent
    console.log('✅ Transcription complete', customEvent.detail)
  }

  const handleTranscriptionAutoStopped = (event: Event) => {
    const customEvent = event as CustomEvent
    console.log('🔄 Transcription auto-stopped', customEvent.detail)
    
    // Add system message about auto-stop
    chatHistory.value.push({
      id: messageIdCounter++,
      sender: 'assistant',
      text: `🔄 Transcription stopped automatically (${customEvent.detail.reason})`,
      timestamp: new Date(),
      messageType: 'text'
    })
    
    setTimeout(() => {
      scrollChatToBottom()
    }, 50)
  }

  return {
    setupSpeechTranscriptionListeners,
    removeSpeechTranscriptionListeners
  }
} 