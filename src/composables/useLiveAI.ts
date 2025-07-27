import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'

export interface LiveAISession {
  id: string
  startTime: number
  endTime?: number
  isActive: boolean
}

export interface LiveAIResponse {
  text: string
  timestamp: number
  confidence: number
  sessionId: string
}

export interface SuggestionItem {
  id: string
  text: string
  timestamp: number
  contextLength: number
}

export function useLiveAI() {
  const isActive = ref(false)
  const sessionId = ref<string | null>(null)
  const response = ref('')
  const suggestions = ref<SuggestionItem[]>([])
  const isProcessing = ref(false)
  const error = ref<string | null>(null)
  const isModelPreloaded = ref(false)
  let streamListener: any = null
  let analysisTimeout: number | null = null
  let lastAnalysisTime = 0
  const ANALYSIS_DEBOUNCE_MS = 3000 // Wait 3 seconds after last message before analyzing
  const MIN_ANALYSIS_INTERVAL_MS = 5000 // Minimum 5 seconds between analyses
  const SUGGESTION_DISPLAY_TIME = 5000 // 5 seconds minimum display time per suggestion

  const startLiveAI = async (messages: any[]): Promise<void> => {
    try {
      error.value = null
      isProcessing.value = true
      
      // Create a new session
      const newSessionId = `live-ai-${Date.now()}`
      sessionId.value = newSessionId
      
      // Set up streaming listener for live AI responses
      streamListener = await listen(`ollama-stream-${newSessionId}`, (event: any) => {
        const data = event.payload
        
        if (data.type === 'start') {
          console.log('🚀 Live AI streaming started')
          isProcessing.value = true
          response.value = ''
        } else if (data.type === 'chunk') {
          response.value += data.text
        } else if (data.type === 'complete') {
          console.log('✅ Live AI streaming completed')
          isProcessing.value = false
          
          // Add the completed response to suggestions list
          if (response.value.trim()) {
            const suggestion: SuggestionItem = {
              id: `suggestion-${Date.now()}`,
              text: response.value.trim(),
              timestamp: Date.now(),
              contextLength: 0 // Will be set by caller
            }
            suggestions.value.unshift(suggestion) // Add to beginning of list
            
            // Keep only last 5 suggestions to prevent UI overflow
            if (suggestions.value.length > 5) {
              suggestions.value = suggestions.value.slice(0, 5)
            }
            
            // Schedule removal of old suggestions after display time
            setTimeout(() => {
              removeSuggestion(suggestion.id)
            }, SUGGESTION_DISPLAY_TIME)
          }
        } else if (data.type === 'error') {
          console.error('❌ Live AI streaming error:', data.error)
          error.value = data.error
          isProcessing.value = false
        }
      })
      
      isActive.value = true
      console.log('🚀 Live AI session started:', newSessionId)
      
      // Initial analysis of current conversation context
      if (messages.length > 0) {
        await analyzeConversationContext(messages)
      } else {
        // Add welcome message to suggestions
        const welcomeSuggestion: SuggestionItem = {
          id: 'welcome',
          text: "Live AI Response Assistant is now active. The AI will provide response suggestions when there are pauses in the conversation.",
          timestamp: Date.now(),
          contextLength: 0
        }
        suggestions.value = [welcomeSuggestion]
      }
    } catch (err) {
      error.value = err instanceof Error ? err.message : 'Failed to start Live AI'
      console.error('Failed to start Live AI:', err)
    } finally {
      isProcessing.value = false
    }
  }

  const stopLiveAI = async (): Promise<void> => {
    if (!sessionId.value) return
    
    try {
      console.log('⏹️ Live AI session stopped:', sessionId.value)
      
      // Clean up stream listener
      if (streamListener) {
        streamListener()
        streamListener = null
      }
      
      isActive.value = false
      response.value = ''
      suggestions.value = []
      sessionId.value = null
      
      // Clear any pending analysis
      if (analysisTimeout) {
        clearTimeout(analysisTimeout)
        analysisTimeout = null
      }
    } catch (err) {
      error.value = err instanceof Error ? err.message : 'Failed to stop Live AI'
      console.error('Failed to stop Live AI:', err)
    }
  }

  const analyzeConversationContext = async (messages: any[]): Promise<void> => {
    if (!isActive.value || !sessionId.value) return
    
    try {
      isProcessing.value = true
      
      // Format conversation context for the AI
      const conversationContext = messages
        .filter(msg => !msg.isPreview) // Exclude preview messages
        .slice(-10) // Only take the last 10 messages for context
        .map(msg => `${msg.type === 'user' ? 'User' : 'System'}: ${msg.content}`)
        .join('\n')
      
      if (conversationContext.trim()) {
        console.log('💬 Analyzing conversation context for response suggestions')
        
        // Call the conversational AI backend function
        await invoke('generate_conversational_ai', {
          conversationContext,
          sessionId: sessionId.value
        })
      }
    } catch (err) {
      console.error('Failed to analyze conversation context:', err)
      error.value = err instanceof Error ? err.message : 'Failed to analyze conversation'
    }
  }

  // Function to trigger response assistance when system is speaking
  const onSystemSpeaking = async (messages: any[]): Promise<void> => {
    if (!isActive.value) return
    
    // Only trigger if the last message is from system/loopback
    const lastMessage = messages[messages.length - 1]
    if (lastMessage && lastMessage.source === 'loopback' && !lastMessage.isPreview) {
      console.log('🎤 System is speaking, generating response suggestions...')
      await analyzeConversationContext(messages)
    }
  }

  // Function to continuously analyze conversation changes with debouncing
  const onConversationChange = async (messages: any[]): Promise<void> => {
    if (!isActive.value) return
    
    // Filter out preview messages and get recent context
    const realMessages = messages.filter(msg => !msg.isPreview)
    if (realMessages.length === 0) return
    
    // Clear any existing analysis timeout
    if (analysisTimeout) {
      clearTimeout(analysisTimeout)
    }
    
    // Check if enough time has passed since last analysis
    const now = Date.now()
    const timeSinceLastAnalysis = now - lastAnalysisTime
    
    if (timeSinceLastAnalysis < MIN_ANALYSIS_INTERVAL_MS && suggestions.value.length > 0) {
      console.log('⏳ Skipping analysis - too soon since last update')
      return
    }
    
    // Set a debounced timeout to analyze after conversation pause
    analysisTimeout = window.setTimeout(async () => {
      if (!isActive.value || isProcessing.value) return
      
      console.log('💭 Conversation paused, analyzing for response suggestions...')
      lastAnalysisTime = Date.now()
      
      // Update context length for the upcoming suggestion
      const contextLength = realMessages.length
      await analyzeConversationContext(realMessages)
      
      // Update the context length of the most recent suggestion
      if (suggestions.value.length > 0) {
        suggestions.value[0].contextLength = contextLength
      }
    }, ANALYSIS_DEBOUNCE_MS)
  }

  const preloadModel = async (): Promise<void> => {
    if (isModelPreloaded.value) return
    
    try {
      console.log('🔄 Pre-loading gemma3 model for Live AI...')
      await invoke('preload_ollama_model', {
        model: 'gemma3:1b-it-qat'
      })
      isModelPreloaded.value = true
      console.log('✅ gemma3 model pre-loaded successfully')
    } catch (err) {
      console.error('❌ Failed to pre-load gemma3 model:', err)
      error.value = err instanceof Error ? err.message : 'Failed to pre-load model'
    }
  }
  
  const removeSuggestion = (suggestionId: string) => {
    const index = suggestions.value.findIndex(s => s.id === suggestionId)
    if (index > -1) {
      suggestions.value.splice(index, 1)
      console.log(`🗑️ Removed suggestion ${suggestionId} after display time`)
    }
  }
  
  const reset = () => {
    if (streamListener) {
      streamListener()
      streamListener = null
    }
    if (analysisTimeout) {
      clearTimeout(analysisTimeout)
      analysisTimeout = null
    }
    isActive.value = false
    sessionId.value = null
    response.value = ''
    suggestions.value = []
    isProcessing.value = false
    error.value = null
    lastAnalysisTime = 0
  }

  return {
    isActive,
    sessionId,
    response,
    suggestions,
    isProcessing,
    error,
    isModelPreloaded,
    startLiveAI,
    stopLiveAI,
    preloadModel,
    analyzeConversationContext,
    onSystemSpeaking,
    onConversationChange,
    reset
  }
}