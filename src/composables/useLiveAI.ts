import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { useConversationTempo } from './useConversationTempo'
import { useResponseGenerator } from './useResponseGenerator'

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
  responseType?: string
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
    dynamicDebounceTime,
    dynamicAnalysisInterval,
    shouldTriggerPreemptiveAnalysis,
    suggestedResponseTypes,
    analyzeConversationTempo,
    getResponsePriority,
    shouldWaitForUserToFinish
  } = useConversationTempo()
  
  // Response generation
  const {
    generateMultipleResponseTypes,
    generateQuickResponse,
    adaptResponseToTempo
  } = useResponseGenerator()
  
  let streamListener: any = null
  let analysisTimeout: number | null = null
  let preemptiveAnalysisTimeout: number | null = null
  let lastAnalysisTime = 0
  let lastPreemptiveAnalysisTime = 0

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
          // Ensure at least two immediate, readable suggestions are visible while streaming begins
          if (suggestions.value.length < 2) {
            const standby: SuggestionItem[] = [
              {
                id: `standby-ack-${Date.now()}`,
                text: 'I understand',
                timestamp: Date.now(),
                contextLength: messages.length,
                priority: 'immediate',
                responseType: 'quick-acknowledgment',
                confidence: 0.6
              },
              {
                id: `standby-q-${Date.now()}`,
                text: 'Could you clarify that a bit?',
                timestamp: Date.now(),
                contextLength: messages.length,
                priority: 'immediate',
                responseType: 'question',
                confidence: 0.6
              }
            ]
            // Prepend without duplicating if similar ones already present
            suggestions.value = [...standby, ...suggestions.value]
          }
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
              contextLength: 0, // Will be set by caller
              priority: getResponsePriority(),
              responseType: suggestedResponseTypes.value[0] || 'contextual'
            }
            suggestions.value.unshift(suggestion) // Add to beginning of list
            
            // Keep only last 5 suggestions to prevent UI overflow
            if (suggestions.value.length > 5) {
              suggestions.value = suggestions.value.slice(0, 5)
            }
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
        // Also provide two immediate default options
        const standbyA: SuggestionItem = {
          id: `standby-ack-${Date.now()}`,
          text: 'That makes sense',
          timestamp: Date.now(),
          contextLength: 0,
          priority: 'immediate',
          responseType: 'quick-acknowledgment',
          confidence: 0.6
        }
        const standbyQ: SuggestionItem = {
          id: `standby-q-${Date.now()}`,
          text: 'What would you like me to focus on?',
          timestamp: Date.now(),
          contextLength: 0,
          priority: 'immediate',
          responseType: 'question',
          confidence: 0.6
        }
        suggestions.value = [standbyA, standbyQ, welcomeSuggestion]
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
      
      // Reduce context for speed - only last 3 messages
      const contextSize = 3
      
      // Simplified context - just the essentials
      const conversationContext = messages
        .filter(msg => !msg.isPreview)
        .slice(-contextSize)
        .map(msg => {
          // Truncate long messages for speed
          const content = msg.content.length > 50 
            ? msg.content.substring(0, 50) + '...' 
            : msg.content
          return `${msg.type === 'user' ? 'User' : 'System'}: ${content}`
        })
        .join('\n')
      
      if (conversationContext.trim()) {
        console.log(`💬 ${isPreemptive ? 'Preemptive' : 'Regular'} analysis with tempo: ${tempo.pace}, urgency: ${tempo.urgencyLevel}`)
        
        // Generate multiple response types based on tempo (use markdown templates when available)
        if (tempo.urgencyLevel === 'high' || tempo.pace === 'rapid') {
          // For urgent situations, generate quick responses locally
          const quickResponse = await generateQuickResponse(conversationContext)
          const adaptedResponse = adaptResponseToTempo(quickResponse, tempo)
          
          // Add as immediate suggestion
          const quickSuggestion: SuggestionItem = {
            id: `quick-${Date.now()}`,
            text: adaptedResponse,
            timestamp: Date.now(),
            contextLength: messages.length,
            priority: 'immediate',
            responseType: 'quick-acknowledgment',
            confidence: 0.8
          }
          suggestions.value.unshift(quickSuggestion)
          
          // Keep only last 5 suggestions
          if (suggestions.value.length > 5) {
            suggestions.value = suggestions.value.slice(0, 5)
          }
        }
        
        // Also generate AI responses or curated markdown for more thoughtful suggestions
        await invoke('generate_conversational_ai', {
          conversationContext,
          sessionId: sessionId.value,
          customSystemPrompt: customSystemPrompt.value,
          tempoContext: {
            pace: tempo.pace,
            urgencyLevel: tempo.urgencyLevel,
            conversationType: tempo.conversationType,
            responseTypes: suggestedResponseTypes.value,
            priority: getResponsePriority()
          }
        })
        
        // Generate typed responses based on conversation type
        if (!isPreemptive) {
          const typedResponses = await generateMultipleResponseTypes(
            conversationContext,
            suggestedResponseTypes.value.slice(0, 3),
            tempo
          )
          
          // Add typed responses as suggestions
          for (const typedResponse of typedResponses) {
            const suggestion: SuggestionItem = {
              id: `typed-${Date.now()}-${Math.random()}`,
              text: typedResponse.text,
              timestamp: Date.now(),
              contextLength: messages.length,
              priority: getResponsePriority(),
              responseType: typedResponse.type,
              confidence: typedResponse.confidence
            }
            suggestions.value.push(suggestion)
          }
          
          // Keep only last 5 suggestions
          if (suggestions.value.length > 5) {
            suggestions.value = suggestions.value.slice(0, 5)
          }
        }
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

  // Enhanced conversation change handler with tempo awareness
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
    if (preemptiveAnalysisTimeout) {
      clearTimeout(preemptiveAnalysisTimeout)
    }
    
    // Check if we should wait for user to finish speaking
    if (shouldWaitForUserToFinish()) {
      console.log('⏸️ User still speaking, waiting...')
      // Set a shorter timeout to recheck
      analysisTimeout = window.setTimeout(() => {
        onConversationChange(messages)
      }, 500)
      return
    }
    
    // Preemptive analysis for fast-paced conversations
    if (shouldTriggerPreemptiveAnalysis.value && !preemptiveAnalysisInProgress.value) {
      const timeSinceLastPreemptive = Date.now() - lastPreemptiveAnalysisTime
      if (timeSinceLastPreemptive > dynamicAnalysisInterval.value / 2) {
        console.log('🚀 Triggering preemptive analysis for fast-paced conversation')
        lastPreemptiveAnalysisTime = Date.now()
        // Run preemptive analysis immediately without blocking
        analyzeConversationContext(realMessages, true)
      }
    }
    
    // Regular debounced analysis
    const now = Date.now()
    const timeSinceLastAnalysis = now - lastAnalysisTime
    
    // Use dynamic intervals based on tempo
    const minInterval = dynamicAnalysisInterval.value
    const debounceTime = dynamicDebounceTime.value
    
    if (timeSinceLastAnalysis < minInterval && suggestions.value.length > 0 && tempo.urgencyLevel !== 'high') {
      console.log(`⏳ Skipping analysis - too soon (${timeSinceLastAnalysis}ms < ${minInterval}ms)`)
      return
    }
    
    // Set a dynamically debounced timeout based on conversation tempo
    analysisTimeout = window.setTimeout(async () => {
      if (!isActive.value || isProcessing.value) return
      
      console.log(`💭 Analyzing with ${tempo.pace} tempo (${debounceTime}ms debounce)...`)
      lastAnalysisTime = Date.now()
      
      // Update context length for the upcoming suggestion
      const contextLength = realMessages.length
      await analyzeConversationContext(realMessages, false)
      
      // Update the context length and priority of the most recent suggestion
      if (suggestions.value.length > 0) {
        suggestions.value[0].contextLength = contextLength
        suggestions.value[0].priority = getResponsePriority()
      }
    }, debounceTime)
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