// agentService.ts - Handles different AI agent modes and messaging
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { SessionManager } from './sessionManager'
import { ContextManager } from './contextManager'
import { ragClient } from '../services/rag/ragClient'
import { RagContextManager } from '../services/rag/contextManager'
import { MCPService } from './mcpService'

let messageIdCounter = 1

export class AgentService {
  private static scrollChatToBottom: () => void
  private static activeSessionIds: Map<number, string> = new Map() // Map message ID to session ID

  static init(scrollCallback: () => void) {
    AgentService.scrollChatToBottom = scrollCallback
    MCPService.init(scrollCallback)
  }

  // Agent activation functions
  static async startDeepResearch(showChatWindow: any) {
    // Auto-open chat window if not open
    if (!showChatWindow.value) {
      showChatWindow.value = true
      console.log('💬 Chat window auto-opened for deep research')
      setTimeout(() => {
        AgentService.scrollChatToBottom()
      }, 150)
    }
    
    // Add deep research message to current chat
    SessionManager.addMessageToCurrentChat({
      id: messageIdCounter++,
      sender: 'user',
      text: '🧠 Deep Research Mode activated - I will thoroughly research your next question.',
      timestamp: new Date(),
      messageType: 'text'
    })
    
    setTimeout(() => {
      AgentService.scrollChatToBottom()
    }, 50)
  }

  static async startConversationalAgent(showChatWindow: any) {
    // Auto-open chat window if not open
    if (!showChatWindow.value) {
      showChatWindow.value = true
      console.log('💬 Chat window auto-opened for conversational agent')
      setTimeout(() => {
        AgentService.scrollChatToBottom()
      }, 150)
    }
    
    // Add conversational agent message to current chat
    SessionManager.addMessageToCurrentChat({
      id: messageIdCounter++,
      sender: 'user',
      text: '🤖 Conversational AI Agent activated - Ready for natural conversation.',
      timestamp: new Date(),
      messageType: 'text'
    })
    
    setTimeout(() => {
      AgentService.scrollChatToBottom()
    }, 50)
  }

  static async startCodingAgent(showChatWindow: any) {
    // Auto-open chat window if not open
    if (!showChatWindow.value) {
      showChatWindow.value = true
      console.log('💬 Chat window auto-opened for coding agent')
      setTimeout(() => {
        AgentService.scrollChatToBottom()
      }, 150)
    }
    
    // Add coding agent message to current chat
    SessionManager.addMessageToCurrentChat({
      id: messageIdCounter++,
      sender: 'user',
      text: '💻 Coding Agent activated - Ready to help with programming tasks.',
      timestamp: new Date(),
      messageType: 'text'
    })
    
    setTimeout(() => {
      AgentService.scrollChatToBottom()
    }, 50)
  }

  static async startComputerUseAgent(showChatWindow: any) {
    // Auto-open chat window if not open
    if (!showChatWindow.value) {
      showChatWindow.value = true
      console.log('💬 Chat window auto-opened for computer use agent')
      setTimeout(() => {
        AgentService.scrollChatToBottom()
      }, 150)
    }
    
    // Add computer use agent message to current chat
    SessionManager.addMessageToCurrentChat({
      id: messageIdCounter++,
      sender: 'user',
      text: '🖥️ Computer Use Agent activated - Ready to assist with computer tasks.',
      timestamp: new Date(),
      messageType: 'text'
    })
    
    setTimeout(() => {
      AgentService.scrollChatToBottom()
    }, 50)
  }


  // Send message function
  static async sendMessage(userMessage: string, selectedModel: string | null, agentType: string = 'enteract', selectedDocumentIds: string[] = []) {
    // Check if this is an explicit @enteract MCP command
    const isExplicitMCP = userMessage.trim().toLowerCase().startsWith('@enteract')
    
    if (isExplicitMCP) {
      console.log('🔧 Detected explicit @enteract MCP command, routing to MCP service')
      
      // Ensure we have an active chat session
      const currentChatSession = SessionManager.getCurrentChatSession().value
      if (!currentChatSession) {
        SessionManager.createNewChat(selectedModel)
      }
      
      await MCPService.processEnteractMessage(userMessage, selectedModel)
      return
    }

    // Ensure we have an active chat session
    const currentChatSession = SessionManager.getCurrentChatSession().value
    if (!currentChatSession) {
      SessionManager.createNewChat(selectedModel)
    }
    
    console.log(`🤖 Sending message with ${agentType} agent:`, userMessage)
    
    // Add user message to current chat
    SessionManager.addMessageToCurrentChat({
      id: messageIdCounter++,
      sender: 'user',
      text: userMessage,
      timestamp: new Date(),
      messageType: 'text'
    })
    
    try {
      // Generate unique session ID for streaming
      const sessionId = `chat-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`
      
      // Store the session ID mapped to the streaming message
      const streamingMessageId = messageIdCounter
      
      // Get appropriate model and agent name based on agent type
      let modelToUse = selectedModel || 'gemma3:1b-it-qat'
      let agentName = 'Enteract AI'
      
      switch (agentType) {
        case 'coding':
          modelToUse = 'qwen2.5-coder:1.5b'
          agentName = 'Coding Assistant'
          break
        case 'research':
          modelToUse = 'deepseek-r1:1.5b'
          agentName = 'Deep Research AI'
          break
        case 'vision':
          modelToUse = 'qwen2.5vl:3b'
          agentName = 'Vision Analysis AI'
          break
        case 'enteract':
        default:
          modelToUse = selectedModel || 'gemma3:1b-it-qat'
          agentName = 'Enteract AI'
          break
      }
      
      // Add streaming response placeholder
      const currentHistory = SessionManager.getCurrentChatHistory().value
      const streamingMessageIndex = currentHistory.length
      SessionManager.addMessageToCurrentChat({
        id: streamingMessageId,
        sender: 'assistant',
        text: `🤖 ${agentName} is thinking▋`,
        timestamp: new Date(),
        messageType: 'text',
        isStreaming: true,
        sessionId: sessionId
      })
      messageIdCounter++
      
      // Store the mapping
      AgentService.activeSessionIds.set(streamingMessageId, sessionId)
      
      setTimeout(() => {
        AgentService.scrollChatToBottom()
      }, 50)
      
      // Track streaming state
      let fullResponse = ''
      let actualResponse = ''
      let thinkingContent = ''
      let isTyping = true
      let hasStarted = false
      let isInThinking = false
      
      // Set up streaming listener
      const unlisten = await listen(`ollama-stream-${sessionId}`, (event: any) => {
        const data = event.payload
        const currentHistory = SessionManager.getCurrentChatHistory().value
        
        switch (data.type) {
          case 'start':
            hasStarted = true
            console.log(`🤖 Started ${agentType} response with ${data.model}`)
            if (currentHistory[streamingMessageIndex]) {
              currentHistory[streamingMessageIndex].text = `🤖 ${agentName} (${data.model})▋`
            }
            setTimeout(() => {
              AgentService.scrollChatToBottom()
            }, 10)
            break
            
          case 'chunk':
            if (data.text) {
              fullResponse += data.text
              
              // For deep research, handle thinking vs response separately
              if (agentType === 'research') {
                // Improved thinking tag detection
                const thinkingStartMatch = data.text.match(/<thinking>/i)
                const thinkingEndMatch = data.text.match(/<\/thinking>/i)
                
                if (thinkingStartMatch) {
                  isInThinking = true
                }
                if (thinkingEndMatch) {
                  isInThinking = false
                }
                
                // Process text based on current state
                if (isInThinking || (fullResponse.includes('<thinking>') && !fullResponse.includes('</thinking>'))) {
                  // Extract thinking content more precisely
                  const cleanText = data.text.replace(/<\/?thinking>/gi, '')
                  if (cleanText.trim()) {
                    thinkingContent += cleanText
                  }
                } else {
                  // Extract response content, removing any remaining thinking tags
                  const cleanText = data.text.replace(/<\/?thinking>/gi, '')
                  if (cleanText.trim()) {
                    actualResponse += cleanText
                  }
                }
              } else {
                // For other agents, just accumulate normally
                actualResponse += data.text
              }
              
              // Update the message in real-time with improved formatting
              if (currentHistory[streamingMessageIndex]) {
                let displayText = ''
                
                if (agentType === 'research') {
                  displayText = `🧠 **Deep Research Analysis**\n\n${actualResponse.trim()}${isTyping ? '▋' : ''}`
                } else if (agentType === 'coding') {
                  displayText = `💻 **Coding Assistant**\n\n${actualResponse.trim()}${isTyping ? '▋' : ''}`
                } else if (agentType === 'vision') {
                  displayText = `👁️ **Vision Analysis**\n\n${actualResponse.trim()}${isTyping ? '▋' : ''}`
                } else {
                  displayText = `🤖 **${agentName}**\n\n${actualResponse.trim()}${isTyping ? '▋' : ''}`
                }
                
                currentHistory[streamingMessageIndex].text = displayText
              }
              
              setTimeout(() => {
                AgentService.scrollChatToBottom()
              }, 10)
            }
            
            if (data.done) {
              isTyping = false
              if (currentHistory[streamingMessageIndex]) {
                let finalText = ''
                
                if (agentType === 'research') {
                  if (thinkingContent.trim()) {
                    // Create collapsible thinking section with better styling
                    const thinkingDisplay = `<details style="margin: 15px 0; border: 1px solid rgba(147, 51, 234, 0.3); border-radius: 12px; padding: 15px; background: rgba(147, 51, 234, 0.05);">
<summary style="cursor: pointer; font-weight: 600; color: #a855f7; font-size: 14px;">🧠 View Reasoning Process</summary>
<div style="margin-top: 12px; padding: 12px; background: rgba(147, 51, 234, 0.1); border-radius: 8px; border-left: 3px solid #a855f7;">
<div style="font-family: 'SF Mono', 'Monaco', 'Cascadia Code', monospace; white-space: pre-wrap; font-size: 13px; line-height: 1.5; color: rgba(255,255,255,0.9);">${thinkingContent.trim()}</div>
</div>
</details>`
                    finalText = `🧠 **Deep Research Analysis**\n\n${thinkingDisplay}\n\n${actualResponse.trim()}`
                  } else {
                    finalText = `🧠 **Deep Research Analysis**\n\n${actualResponse.trim()}`
                  }
                } else if (agentType === 'coding') {
                  finalText = `💻 **Coding Assistant**\n\n${actualResponse.trim()}`
                } else if (agentType === 'vision') {
                  finalText = `👁️ **Vision Analysis**\n\n${actualResponse.trim()}`
                } else {
                  // Clean formatting for other agents
                  const agentDisplayName = agentName === 'Enteract AI' ? 'AI Assistant' : agentName
                  finalText = `🤖 **${agentDisplayName}**\n\n${actualResponse.trim()}`
                }
                
                currentHistory[streamingMessageIndex].text = finalText
              }
              console.log(`✅ ${agentType} streaming completed. Response length: ${actualResponse.length} chars`)
            }
            break
            
          case 'error':
            isTyping = false
            console.error(`${agentType} streaming error:`, data.error)
            // Update message to show error
            if (currentHistory[streamingMessageIndex]) {
              let errorMessage = `❌ Error: ${data.error}`
              if (data.error.includes('deepseek-r1:1.5b') && agentType === 'research') {
                errorMessage = `❌ DeepSeek R1 model not found. Please install it first:\n\n\`\`\`bash\nollama pull deepseek-r1:1.5b\n\`\`\``
              } else if (data.error.includes('qwen2.5-coder:1.5b') && agentType === 'coding') {
                errorMessage = `❌ Qwen2.5-Coder model not found. Please install it first:\n\n\`\`\`bash\nollama pull qwen2.5-coder:1.5b\n\`\`\``
              } else if (data.error.includes('qwen2.5vl:3b') && agentType === 'vision') {
                errorMessage = `❌ Qwen2.5-VL model not found. Please install it first:\n\n\`\`\`bash\nollama pull qwen2.5vl:3b\n\`\`\``
              } else if (data.error.includes('connection refused') || data.error.includes('ECONNREFUSED')) {
                errorMessage = `❌ Cannot connect to Ollama. Please make sure Ollama is running:\n\n\`\`\`bash\nollama serve\n\`\`\``
              }
              currentHistory[streamingMessageIndex].text = errorMessage
            }
            break
            
          case 'cancelled':
            isTyping = false
            console.log(`🛑 ${agentType} streaming cancelled by user`)
            if (currentHistory[streamingMessageIndex]) {
              currentHistory[streamingMessageIndex].text = `❌ Response cancelled by user`
              currentHistory[streamingMessageIndex].isStreaming = false
            }
            // Clean up
            AgentService.activeSessionIds.delete(streamingMessageId)
            unlisten()
            break
            
          case 'complete':
            isTyping = false
            console.log(`🎉 ${agentType} streaming session completed`)
            if (currentHistory[streamingMessageIndex]) {
              currentHistory[streamingMessageIndex].isStreaming = false
            }
            // Clean up
            AgentService.activeSessionIds.delete(streamingMessageId)
            unlisten()
            break
        }
      })
      
      // Add a timeout to show model loading if it takes too long
      const loadingTimeout = setTimeout(() => {
        const currentHistory = SessionManager.getCurrentChatHistory().value
        if (!hasStarted && currentHistory[streamingMessageIndex]) {
          currentHistory[streamingMessageIndex].text = `🔄 Loading ${agentName} model (this may take a moment for the first run)▋`
          setTimeout(() => {
            AgentService.scrollChatToBottom()
          }, 10)
        }
      }, 2000)
      
      // Smart RAG Search and Context Management
      let ragContextResult = { context: '', tokensUsed: 0, chunksIncluded: 0, chunksDropped: 0 }
      let enhancedPrompt = userMessage
      
      if (selectedDocumentIds.length > 0) {
        try {
          console.log(`🔍 Performing RAG search with ${selectedDocumentIds.length} selected documents`)
          
          // Validate document readiness
          const validationResult = await ragClient.validateDocuments(selectedDocumentIds)
          console.log(`📊 Document validation: ${validationResult.ready_documents.length} ready, ${validationResult.pending_documents.length} pending`)
          
          if (validationResult.ready_documents.length === 0 && validationResult.pending_documents.length > 0) {
            console.log('⏳ All selected documents are still processing embeddings, proceeding without RAG context')
          } else {
            // Perform search with ready documents
            const searchDocuments = validationResult.ready_documents.length > 0 
              ? validationResult.ready_documents 
              : selectedDocumentIds
              
            const ragChunks = await ragClient.searchDocuments(userMessage, searchDocuments)
            
            if (ragChunks.length > 0) {
              // Calculate optimal token allocation
              const tokenAllocation = RagContextManager.calculateOptimalTokenAllocation(ragChunks, 4000)
              
              // Format context with smart token management
              ragContextResult = RagContextManager.formatContextForAI(ragChunks, tokenAllocation.ragTokens)
              
              console.log(`📚 Smart RAG context: ${ragContextResult.chunksIncluded}/${ragChunks.length} chunks, ${ragContextResult.tokensUsed} tokens`)
              
              if (ragContextResult.chunksDropped > 0) {
                console.log(`⚠️ Dropped ${ragContextResult.chunksDropped} chunks due to token limits`)
              }
              
              // Prepare enhanced prompt
              if (ragContextResult.context) {
                enhancedPrompt = `Context from documents:\n${ragContextResult.context}\n\nUser question: ${userMessage}\n\nPlease answer the question using the provided document context when relevant.`
              }
            } else {
              console.log('📚 No relevant content found in selected documents')
            }
          }
        } catch (error) {
          console.error('❌ RAG search failed:', error)
          // Continue without RAG context if search fails
        }
      }
      
      // Generate conversation context with remaining tokens
      const remainingTokens = 4000 - ragContextResult.tokensUsed
      const truncatedContext = ContextManager.getLimitedContext(SessionManager.getCurrentChatHistory().value, remainingTokens)
      
      console.log(`📊 Token allocation: ${ragContextResult.tokensUsed} RAG + ${ContextManager.estimateTokens(truncatedContext.map(m => m.content).join(''))} conversation = ${ragContextResult.tokensUsed + ContextManager.estimateTokens(truncatedContext.map(m => m.content).join(''))} total`)
      
      if (ragContextResult.context) {
        console.log(`📚 Enhanced prompt with RAG context: ${enhancedPrompt.length} characters`)
      }
      
      // Route to appropriate agent based on type
      switch (agentType) {
        case 'coding':
          console.log('💻 FRONTEND: Calling generate_coding_agent_response (should use qwen2.5-coder:1.5b)')
          await invoke('generate_coding_agent_response', {
            prompt: enhancedPrompt,
            context: truncatedContext,
            sessionId: sessionId
          })
          break
          
        case 'research':
          console.log('🧠 FRONTEND: Calling generate_deep_research (should use deepseek-r1:1.5b)')
          await invoke('generate_deep_research', {
            prompt: enhancedPrompt,
            context: truncatedContext,
            sessionId: sessionId
          })
          break
          
        case 'vision':
          console.log('👁️ FRONTEND: Calling generate_vision_analysis (should use qwen2.5vl:3b)')
          // Note: Vision analysis typically needs an image, but we'll call it anyway
          await invoke('generate_vision_analysis', {
            prompt: enhancedPrompt,
            imageBase64: '', // Empty for text-only requests
            sessionId: sessionId
          })
          break
          
        case 'enteract':
        default:
          console.log('🛡️ FRONTEND: Calling generate_enteract_agent_response (should use gemma3:1b-it-qat)')
          await invoke('generate_enteract_agent_response', {
            prompt: enhancedPrompt,
            context: truncatedContext,
            sessionId: sessionId
          })
          break
      }
      
      // Clear the loading timeout
      clearTimeout(loadingTimeout)
      
      console.log(`🤖 Started streaming AI response from ${modelToUse}`)
      
    } catch (error) {
      const errorString = error instanceof Error ? error.message : String(error)
      console.error('Failed to start AI response streaming:', error)
      
      // Enhanced error messages
      let errorMessage = `❌ Failed to get AI response: ${errorString}. Make sure Ollama is running and the model "${selectedModel || 'gemma3:1b-it-qat'}" is available.`
      if (errorString.includes('connection refused') || errorString.includes('ECONNREFUSED')) {
        errorMessage = `❌ Cannot connect to Ollama. Please make sure Ollama is running:\n\n\`\`\`bash\nollama serve\n\`\`\``
      } else if (errorString.includes('model') && errorString.includes('not found')) {
        errorMessage = `❌ Model not available. Install with:\n\n\`\`\`bash\nollama pull ${selectedModel || 'gemma3:1b-it-qat'}\n\`\`\``
      }
      
      // Add error message to current chat
      SessionManager.addMessageToCurrentChat({
        id: messageIdCounter++,
        sender: 'assistant',
        text: errorMessage,
        timestamp: new Date(),
        messageType: 'text'
      })
    }
    
    // Auto-scroll to bottom
    setTimeout(() => {
      AgentService.scrollChatToBottom()
    }, 50)
  }
  
  // Cancel an active AI response
  static async cancelResponse(messageId: number) {
    const sessionId = AgentService.activeSessionIds.get(messageId)
    if (sessionId) {
      try {
        await invoke('cancel_ai_response', { sessionId })
        console.log(`🛑 Cancellation requested for message ${messageId}, session ${sessionId}`)
      } catch (error) {
        console.error('Failed to cancel AI response:', error)
      }
    } else {
      console.warn(`No active session found for message ${messageId}`)
    }
  }
}