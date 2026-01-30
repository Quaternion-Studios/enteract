// contextManager.ts - Handles context truncation and token estimation
import type { ChatMessage } from '../types'

export class ContextManager {
  // Token estimation utility (~4 characters per token heuristic)
  static estimateTokens(text: string): number {
    if (!text) return 0
    return Math.ceil(text.length / 4)
  }

  // Context truncation logic to fit within token limits
  static getLimitedContext(history: ChatMessage[], maxTokens: number): { role: string; content: string }[] {
    if (!history || history.length === 0) return []
    
    const context: { role: string; content: string }[] = []
    let currentTokens = 0
    
    // Step 1: Extract and preserve all system messages at the beginning
    const systemMessages = history.filter(msg => msg.sender === 'system')
    const nonSystemMessages = history.filter(msg => msg.sender !== 'system')
    
    // Add system messages first (they should always be preserved)
    for (const msg of systemMessages) {
      const tokens = ContextManager.estimateTokens(msg.text)
      currentTokens += tokens
      context.push({
        role: 'system',
        content: msg.text
      })
    }
    
    // Step 2: Add a truncation indicator if we'll need to truncate
    const totalTokensNeeded = history.reduce((sum, msg) => sum + ContextManager.estimateTokens(msg.text), 0)
    const needsTruncation = totalTokensNeeded > maxTokens
    
    if (needsTruncation) {
      const truncationMessage = '... (earlier conversation history truncated to fit context limit) ...'
      const truncationTokens = ContextManager.estimateTokens(truncationMessage)
      currentTokens += truncationTokens
      context.push({
        role: 'system',
        content: truncationMessage
      })
    }
    
    // Step 3: Iterate backwards through non-system messages to keep most recent
    const messagesToInclude: ChatMessage[] = []
    
    for (let i = nonSystemMessages.length - 1; i >= 0; i--) {
      const message = nonSystemMessages[i]
      const messageTokens = ContextManager.estimateTokens(message.text)
      
      // Check if adding this message would exceed the limit
      if (currentTokens + messageTokens > maxTokens) {
        break // Stop here to avoid exceeding the limit
      }
      
      // Add to the beginning of our array (since we're going backwards)
      messagesToInclude.unshift(message)
      currentTokens += messageTokens
    }
    
    // Step 4: Convert messages to Ollama API format and add to context
    for (const message of messagesToInclude) {
      let role: string
      switch (message.sender) {
        case 'user':
          role = 'user'
          break
        case 'assistant':
          role = 'assistant'
          break
        case 'transcription':
          role = 'user' // Treat transcriptions as user input
          break
        default:
          role = 'user' // Default fallback
      }
      
      context.push({
        role,
        content: message.text
      })
    }
    
    console.log(`📊 Context truncation: ${history.length} messages → ${messagesToInclude.length + systemMessages.length} messages, ~${currentTokens} tokens (limit: ${maxTokens})`)
    
    return context
  }
}