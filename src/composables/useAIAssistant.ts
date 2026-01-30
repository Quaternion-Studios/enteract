import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'

interface Message {
  id: string
  type: 'user' | 'system'
  source: 'microphone' | 'loopback'
  content: string
  confidence?: number
  timestamp: number
  isPreview?: boolean
}

export function useAIAssistant() {
  const aiResponse = ref('')
  const aiIsProcessing = ref(false)
  const aiError = ref<string | null>(null)
  
  const queryAI = async (query: string, messages: Message[]): Promise<string | null> => {
    if (!query.trim()) return null
    
    aiIsProcessing.value = true
    aiError.value = null
    aiResponse.value = ''
    
    try {
      // Build context from messages
      const contextMessages = messages
        .filter(msg => !msg.isPreview)
        .slice(-10) // Use last 10 messages for context
        .map(msg => ({
          role: msg.type === 'user' ? 'user' : 'assistant',
          content: msg.content
        }))
      
      // Use the simple generate response command (non-streaming for AI Assistant)
      const response = await invoke<string>('generate_ollama_response', {
        model: 'gemma3:1b-it-qat',
        prompt: `You are an AI conversation assistant helping to answer specific questions about an ongoing conversation.

Recent Conversation Context:
${contextMessages.map(msg => `${msg.role === 'user' ? 'User' : 'System'}: ${msg.content}`).join('\n')}

User Question: ${query}

Please provide a helpful, direct answer based on the conversation context. Be concise but comprehensive. Focus on addressing the specific question asked.`
      })
      
      aiResponse.value = response
      return response
    } catch (error) {
      aiError.value = error instanceof Error ? error.message : 'Failed to get AI response'
      console.error('AI Assistant error:', error)
      return null
    } finally {
      aiIsProcessing.value = false
    }
  }
  
  const reset = () => {
    aiResponse.value = ''
    aiIsProcessing.value = false
    aiError.value = null
  }
  
  return {
    aiResponse,
    aiIsProcessing,
    aiError,
    queryAI,
    reset
  }
}