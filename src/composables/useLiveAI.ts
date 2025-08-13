import { ref, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { useConversationStore } from '../stores/conversation'

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

export interface InsightItem {
  id: string
  text: string
  timestamp: number
  contextLength: number
  type: 'insight' | 'welcome'
}

export function useLiveAI() {
  const conversationStore = useConversationStore()
  
  const isActive = ref(false)
  const sessionId = ref<string | null>(null)
  const response = ref('')
  const isProcessing = ref(false)
  const error = ref<string | null>(null)
  const isAnalyzing = ref(false)
  
  // Get insights from the current conversation session
  const insights = computed(() => conversationStore.currentSession?.insights || [])
  
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
      streamListener = await listen(`ollama-stream-${newSessionId}`, async (event: any) => {
        const data = event.payload
        
        if (data.type === 'start') {
          console.log('🚀 Live AI streaming started')
          isProcessing.value = true
          response.value = ''
          // Clear old insights when starting fresh analysis
          // Insights are now managed by the conversation store
        } else if (data.type === 'chunk') {
          response.value += data.text
        } else if (data.type === 'complete') {
          console.log('✅ Live AI streaming completed')
          isProcessing.value = false
          
          // Add the completed response to conversation store
          if (response.value.trim()) {
            const insight: InsightItem = {
              id: `insight-${Date.now()}`,
              text: response.value.trim(),
              timestamp: Date.now(),
              contextLength: currentMessages?.length || 0,
              type: 'insight'
            }
            
            // Save to conversation store
            await conversationStore.addInsight(insight)
          }
        } else if (data.type === 'error') {
          console.error('❌ Live AI streaming error:', data.error)
          error.value = data.error
          isProcessing.value = false
        }
      })
      
      isActive.value = true
      console.log('🚀 Live AI session started:', newSessionId)
      
      // Load existing insights for this conversation
      await conversationStore.loadCurrentSessionInsights()
      
      // Add welcome message if this is a new conversation with no insights
      if (insights.value.length === 0) {
        const welcomeInsight: InsightItem = {
          id: 'welcome',
          text: "AI Conversation Coach is active. I'll provide real-time insights and suggestions to help you navigate important conversations more effectively.",
          timestamp: Date.now(),
          contextLength: 0,
          type: 'welcome'
        }
        await conversationStore.addInsight(welcomeInsight)
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
      // Insights remain in the conversation store
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
    
    // Don't run multiple analyses simultaneously
    if (isAnalyzing.value) {
      console.log('⏳ Analysis already in progress, skipping')
      return
    }
    
    try {
      isProcessing.value = true
      isAnalyzing.value = true
      
      // Get last 5 messages for context
      const contextSize = 5
      
      // Simple conversation context
      const conversationContext = messages
        .filter(msg => !msg.isPreview)
        .slice(-contextSize)
        .map(msg => {
          const speaker = msg.source === 'loopback' ? 'System' : 'User'
          const content = msg.content.length > 200 
            ? msg.content.substring(0, 200) + '...' 
            : msg.content
          return `${speaker}: ${content}`
        })
        .join('\n')
      
      if (conversationContext.trim()) {
        console.log('💬 Analyzing conversation for insights...')
        
        // Generate AI insights
        await invoke('generate_conversational_ai', {
          conversationContext,
          sessionId: sessionId.value
        })
      }
    } catch (err) {
      console.error('Failed to analyze conversation context:', err)
      error.value = err instanceof Error ? err.message : 'Failed to analyze conversation'
    } finally {
      isAnalyzing.value = false
      isProcessing.value = false
    }
  }

  // Function to trigger response assistance when system is speaking
  const onSystemSpeaking = async (messages: any[]): Promise<void> => {
    if (!isActive.value) return
    
    // Only trigger if the last message is from system/loopback
    const lastMessage = messages[messages.length - 1]
    if (lastMessage && lastMessage.source === 'loopback' && !lastMessage.isPreview) {
      console.log('🎤 System is speaking, generating conversation insights...')
      await analyzeConversationContext(messages)
    }
  }

  // Simplified conversation change handler
  const onConversationChange = async (messages: any[]): Promise<void> => {
    if (!isActive.value) return
    
    // Filter out preview messages
    const realMessages = messages.filter(msg => !msg.isPreview)
    if (realMessages.length === 0) return
    
    // Clear any existing analysis timeouts
    if (analysisTimeout) {
      clearTimeout(analysisTimeout)
    }
    
    // Simple wait time - 2 seconds after last message
    const waitTime = 2000
    
    // Set timeout for analysis
    analysisTimeout = window.setTimeout(async () => {
      if (!isActive.value || isProcessing.value) return
      
      const now = Date.now()
      const timeSinceLastAnalysis = now - lastAnalysisTime
      
      // Minimum interval between analyses (7.5 seconds)
      if (timeSinceLastAnalysis < 7500 && insights.value.length > 1) {
        console.log(`⏳ Skipping - analyzed recently`)
        return
      }
      
      console.log('💭 Analyzing conversation...')
      lastAnalysisTime = Date.now()
      
      await analyzeConversationContext(realMessages)
    }, waitTime)
  }

  const updateSystemPrompt = (_prompt: string) => {
    // Removed - no longer using custom prompts
    console.log('🔧 System prompts are now fixed for simplified operation')
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
    // Insights remain in the conversation store
    isProcessing.value = false
    error.value = null
    lastAnalysisTime = 0
  }

  return {
    isActive,
    sessionId,
    response,
    insights,
    isProcessing,
    error,
    startLiveAI,
    stopLiveAI,
    analyzeConversationContext,
    onSystemSpeaking,
    onConversationChange,
    updateSystemPrompt,
    reset
  }
}