import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { useConversationTempo } from './useConversationTempo'

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
  priority?: 'immediate' | 'soon' | 'normal' | 'low'
  confidence?: number
}

export function useLiveAI() {
  const isActive = ref(false)
  const sessionId = ref<string | null>(null)
  const response = ref('')
  const suggestions = ref<SuggestionItem[]>([])
  const isProcessing = ref(false)
  const error = ref<string | null>(null)
  const isAnalyzing = ref(false)
  const preemptiveAnalysisInProgress = ref(false)
  const customSystemPrompt = ref<string | null>(null)
  
  // Conversation tempo tracking
  const {
    currentTempo,
    tempoMetrics,
    suggestedResponseTypes,
    analyzeConversationTempo,
    getResponsePriority
  } = useConversationTempo()
  
  let streamListener: any = null
  let analysisTimeout: number | null = null
  let lastAnalysisTime = 0

  const startLiveAI = async (messages: any[]): Promise<void> => {
    try {
      error.value = null
      isProcessing.value = true
      
      // Store messages for reference in listener
      const currentMessages = messages
      
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
          // Clear old suggestions when starting fresh analysis
          suggestions.value = []
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
              contextLength: currentMessages?.length || 0,
              priority: getResponsePriority(),
              confidence: 0.85
            }
            
            // Replace all suggestions with the new one (keep it simple)
            suggestions.value = [suggestion]
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
          text: "AI Assistant is active. Contextual responses will appear automatically during conversation pauses.",
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

  const analyzeConversationContext = async (messages: any[], isPreemptive = false): Promise<void> => {
    if (!isActive.value || !sessionId.value) return
    
    // Don't run multiple analyses simultaneously
    if (isAnalyzing.value || (isPreemptive && preemptiveAnalysisInProgress.value)) {
      console.log('⏳ Analysis already in progress, skipping')
      return
    }
    
    try {
      if (isPreemptive) {
        preemptiveAnalysisInProgress.value = true
      } else {
        isProcessing.value = true
      }
      isAnalyzing.value = true
      
      // Analyze conversation tempo first
      const tempo = analyzeConversationTempo(messages)
      
      // Get last 5 messages for better context
      const contextSize = 5
      
      // Properly label conversation context
      const conversationContext = messages
        .filter(msg => !msg.isPreview)
        .slice(-contextSize)
        .map(msg => {
          // Properly identify speaker based on source
          const speaker = msg.source === 'loopback' ? 'System' : 'User'
          // Keep full message for context, but limit to 200 chars
          const content = msg.content.length > 200 
            ? msg.content.substring(0, 200) + '...' 
            : msg.content
          return `${speaker}: ${content}`
        })
        .join('\n')
      
      if (conversationContext.trim()) {
        console.log(`💬 ${isPreemptive ? 'Preemptive' : 'Regular'} analysis with tempo: ${tempo.pace}, urgency: ${tempo.urgencyLevel}`)
        
        // Generate AI responses with proper context
        await invoke('generate_conversational_ai', {
          conversationContext,
          sessionId: sessionId.value,
          customSystemPrompt: customSystemPrompt.value || 'You are a helpful conversation assistant. Provide natural, contextual responses based on the conversation flow.',
          tempoContext: {
            pace: tempo.pace,
            urgencyLevel: tempo.urgencyLevel,
            conversationType: tempo.conversationType,
            responseTypes: suggestedResponseTypes.value,
            priority: getResponsePriority()
          }
        })
      }
    } catch (err) {
      console.error('Failed to analyze conversation context:', err)
      error.value = err instanceof Error ? err.message : 'Failed to analyze conversation'
    } finally {
      isAnalyzing.value = false
      if (isPreemptive) {
        preemptiveAnalysisInProgress.value = false
      } else {
        isProcessing.value = false
      }
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

  // Simplified conversation change handler
  const onConversationChange = async (messages: any[]): Promise<void> => {
    if (!isActive.value) return
    
    // Filter out preview messages and get recent context
    const realMessages = messages.filter(msg => !msg.isPreview)
    if (realMessages.length === 0) return
    
    // Analyze conversation tempo to determine timing
    const tempo = analyzeConversationTempo(realMessages)
    
    // Clear any existing analysis timeouts
    if (analysisTimeout) {
      clearTimeout(analysisTimeout)
    }
    
    // Detect conversation pauses and analyze
    const lastMessage = realMessages[realMessages.length - 1]
    const isSystemSpeaking = lastMessage?.source === 'loopback'
    
    // Determine appropriate wait time based on context
    let waitTime = 1500 // Default 1.5 seconds
    
    if (isSystemSpeaking) {
      // Wait longer after system speaks to let user respond
      waitTime = 2000
    } else if (tempo.pace === 'rapid') {
      // Shorter wait for fast conversations
      waitTime = 1000
    } else if (tempo.pace === 'slow') {
      // Longer wait for slow conversations
      waitTime = 2500
    }
    
    // Set timeout for analysis
    analysisTimeout = window.setTimeout(async () => {
      if (!isActive.value || isProcessing.value) return
      
      const now = Date.now()
      const timeSinceLastAnalysis = now - lastAnalysisTime
      
      // Minimum interval between analyses (3 seconds)
      if (timeSinceLastAnalysis < 3000 && suggestions.value.length > 0) {
        console.log(`⏳ Skipping - analyzed recently`)
        return
      }
      
      console.log(`💭 Analyzing conversation (${tempo.pace} tempo)...`)
      lastAnalysisTime = Date.now()
      
      await analyzeConversationContext(realMessages, false)
    }, waitTime)
  }

  const updateSystemPrompt = (prompt: string) => {
    customSystemPrompt.value = prompt
    console.log('🔧 System prompt updated for LiveAI:', prompt.substring(0, 100) + '...')
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
    customSystemPrompt.value = null
    lastAnalysisTime = 0
  }

  return {
    isActive,
    sessionId,
    response,
    suggestions,
    isProcessing,
    error,
    currentTempo,
    tempoMetrics,
    startLiveAI,
    stopLiveAI,
    analyzeConversationContext,
    onSystemSpeaking,
    onConversationChange,
    updateSystemPrompt,
    reset
  }
}