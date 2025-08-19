// mcpService.ts - Handles MCP (Model Context Protocol) operations and tool calling
import { invoke } from '@tauri-apps/api/core'
import { SessionManager } from './sessionManager'
import { getNextMessageId } from './messageIdGenerator'

export interface MCPSessionInfo {
  id: string
  created_at: string
  config: any
  tools_available: any[]
  status: string
  approvals_pending: number
}

export interface ToolExecutionResult {
  success: boolean
  result: any
  error?: string
  execution_time_ms: number
  tool_name: string
}

export class MCPService {
  private static scrollChatToBottom: () => void
  private static activeMCPSessions: Map<string, MCPSessionInfo> = new Map()
  private static currentSessionId: string | null = null

  static init(scrollCallback: () => void) {
    MCPService.scrollChatToBottom = scrollCallback
  }

  // Initialize MCP session if not already active
  static async ensureMCPSession(): Promise<string> {
    console.log('🔧 [MCP] ensureMCPSession called, currentSessionId:', MCPService.currentSessionId)
    
    if (MCPService.currentSessionId) {
      // Check if session is still active
      try {
        console.log('🔧 [MCP] Checking existing session:', MCPService.currentSessionId)
        const sessionInfo = await invoke<MCPSessionInfo>('get_mcp_session_info', {
          sessionId: MCPService.currentSessionId
        })
        console.log('🔧 [MCP] Existing session status:', sessionInfo.status)
        if (sessionInfo.status === 'Active') {
          return MCPService.currentSessionId
        }
      } catch (error) {
        console.log('🔧 [MCP] Current MCP session no longer active, creating new one. Error:', error)
      }
    }

    // Create new MCP session
    try {
      const sessionInfo = await invoke<MCPSessionInfo>('start_mcp_session', {
        config: {
          require_approval: false, // Auto-approve for @enteract usage
          session_timeout_seconds: 600,
          enable_logging: true,
          server_name: 'enteract-mcp-server',
          server_version: '1.0.0'
        }
      })
      
      MCPService.currentSessionId = sessionInfo.id
      MCPService.activeMCPSessions.set(sessionInfo.id, sessionInfo)
      
      console.log(`✅ MCP Session created: ${sessionInfo.id} with ${sessionInfo.tools_available.length} tools`)
      return sessionInfo.id
    } catch (error) {
      console.error('Failed to create MCP session:', error)
      throw new Error(`Failed to initialize MCP session: ${error}`)
    }
  }

  // List available MCP tools
  static async getAvailableTools(sessionId: string): Promise<any[]> {
    try {
      const tools = await invoke<any[]>('list_mcp_tools', { sessionId })
      return tools
    } catch (error) {
      console.error('Failed to get MCP tools:', error)
      return []
    }
  }

  // Execute MCP tool
  static async executeTool(sessionId: string, toolName: string, parameters: any): Promise<ToolExecutionResult> {
    try {
      const result = await invoke<ToolExecutionResult>('execute_mcp_tool', {
        sessionId,
        toolName,
        parameters
      })
      return result
    } catch (error) {
      console.error(`Failed to execute MCP tool ${toolName}:`, error)
      throw error
    }
  }

  // Process @enteract message and route to appropriate MCP tools
  static async processEnteractMessage(message: string, _selectedModel: string | null) {
    console.log('🔧 [MCP] Processing @enteract message:', message)
    try {
      // Remove @enteract prefix and trim
      const cleanMessage = message.replace(/^@enteract\s*/i, '').trim()
      console.log('🔧 [MCP] Clean message:', cleanMessage)
      
      if (!cleanMessage) {
        SessionManager.addMessageToCurrentChat({
          id: getNextMessageId(),
          sender: 'assistant',
          text: '🔧 **MCP Mode** - Please provide a command after @enteract\n\nExample: `@enteract take a screenshot`',
          timestamp: new Date(),
          messageType: 'text'
        })
        return
      }

      // Add user message to chat
      SessionManager.addMessageToCurrentChat({
        id: getNextMessageId(),
        sender: 'user',
        text: message,
        timestamp: new Date(),
        messageType: 'text'
      })

      // Ensure MCP session is active
      console.log('🔧 [MCP] Ensuring MCP session is active...')
      const sessionId = await MCPService.ensureMCPSession()
      console.log('🔧 [MCP] Session ID:', sessionId)
      
      // Add thinking message
      const thinkingMessageId = getNextMessageId()
      SessionManager.addMessageToCurrentChat({
        id: thinkingMessageId,
        sender: 'assistant',
        text: '🔧 **MCP Agent** - Analyzing request and selecting appropriate tools▋',
        timestamp: new Date(),
        messageType: 'text',
        isStreaming: true
      })

      setTimeout(() => {
        MCPService.scrollChatToBottom()
      }, 50)

      // Get available tools
      console.log('🔧 [MCP] Getting available tools...')
      const availableTools = await MCPService.getAvailableTools(sessionId)
      console.log('🔧 [MCP] Available tools:', availableTools.map(t => t.name))
      
      // Simple tool selection logic based on message content
      const toolActions = MCPService.selectToolsForMessage(cleanMessage, availableTools)
      console.log('🔧 [MCP] Selected tool actions:', toolActions)
      
      if (toolActions.length === 0) {
        // Update thinking message to show no tools found
        const currentHistory = SessionManager.getCurrentChatHistory().value
        const messageIndex = currentHistory.findIndex(m => m.id === thinkingMessageId)
        if (messageIndex !== -1) {
          currentHistory[messageIndex].text = '🔧 **MCP Agent** - No specific tools found for this request. Available tools:\n\n' + 
            availableTools.map(tool => `• **${tool.name}**: ${tool.description}`).join('\n')
          currentHistory[messageIndex].isStreaming = false
        }
        return
      }

      // Check if any tools require approval
      const compoundTools = ['click_on_text', 'click_at']
      const requiresApproval = toolActions.some(action => compoundTools.includes(action.toolName))
      
      if (requiresApproval) {
        // Show approval request
        const currentHistory = SessionManager.getCurrentChatHistory().value
        const messageIndex = currentHistory.findIndex(m => m.id === thinkingMessageId)
        if (messageIndex !== -1) {
          const toolDescriptions = toolActions.map(action => 
            `• **${action.toolName}**: ${JSON.stringify(action.parameters)}`
          ).join('\n')
          
          currentHistory[messageIndex].text = `🔧 **MCP Agent - Approval Required**\n\nI want to execute these tools:\n\n${toolDescriptions}\n\n⚠️ **These actions will interact with your computer. Proceed?**\n\n[This would show approval buttons in a real implementation]`
          currentHistory[messageIndex].isStreaming = false
        }
        
        // For demo purposes, we'll auto-approve after showing the message
        // In a real implementation, this would wait for user interaction
        console.log('🔒 [MCP] Approval required for compound tools. Auto-approving for demo...')
        await new Promise(resolve => setTimeout(resolve, 2000))
      }
      // Execute tools sequentially
      let results: string[] = []
      for (const action of toolActions) {
        try {
          // Update status
          const currentHistory = SessionManager.getCurrentChatHistory().value
          const messageIndex = currentHistory.findIndex(m => m.id === thinkingMessageId)
          if (messageIndex !== -1) {
            currentHistory[messageIndex].text = `🔧 **MCP Agent** - Executing ${action.toolName}...▋`
          }
          
          setTimeout(() => {
            MCPService.scrollChatToBottom()
          }, 10)

          const result = await MCPService.executeTool(sessionId, action.toolName, action.parameters)
          
          if (result.success) {
            results.push(`✅ **${action.toolName}**: ${MCPService.formatToolResult(result)}`)
          } else {
            results.push(`❌ **${action.toolName}**: ${result.error || 'Unknown error'}`)
          }
        } catch (error) {
          results.push(`❌ **${action.toolName}**: ${error}`)
        }
      }

      // Update final message with results
      const currentHistory = SessionManager.getCurrentChatHistory().value
      const messageIndex = currentHistory.findIndex(m => m.id === thinkingMessageId)
      if (messageIndex !== -1) {
        const finalText = `🔧 **MCP Agent Results**\n\n${results.join('\n\n')}`
        currentHistory[messageIndex].text = finalText
        currentHistory[messageIndex].isStreaming = false
      }

    } catch (error) {
      console.error('Error processing @enteract message:', error)
      
      SessionManager.addMessageToCurrentChat({
        id: getNextMessageId(),
        sender: 'assistant',
        text: `❌ **MCP Error**: ${error instanceof Error ? error.message : 'Unknown error occurred'}`,
        timestamp: new Date(),
        messageType: 'text'
      })
    }
    
    setTimeout(() => {
      MCPService.scrollChatToBottom()
    }, 50)
  }

  // Enhanced tool selection with compound tools
  private static selectToolsForMessage(message: string, availableTools: any[]): { toolName: string, parameters: any }[] {
    const actions: { toolName: string, parameters: any }[] = []
    const lowerMessage = message.toLowerCase()

    // Compound tool: Click and type (highest priority - for textbox interactions)
    if ((lowerMessage.includes('type') && (lowerMessage.includes('into') || lowerMessage.includes('in'))) ||
        (lowerMessage.includes('search') && lowerMessage.includes('for')) ||
        (lowerMessage.includes('enter') && lowerMessage.includes('text'))) {
      const clickAndTypeTool = availableTools.find(tool => tool.name === 'click_and_type')
      if (clickAndTypeTool) {
        // Try to extract what to click and what to type
        const typeMatch = lowerMessage.match(/type\s+["']([^"']+)["']/) || 
                         lowerMessage.match(/search\s+for\s+["']([^"']+)["']/) ||
                         lowerMessage.match(/enter\s+["']([^"']+)["']/) ||
                         lowerMessage.match(/type\s+(\w+)/) ||
                         lowerMessage.match(/search\s+for\s+(\w+)/) ||
                         lowerMessage.match(/enter\s+(\w+)/)
        
        const clickMatch = lowerMessage.match(/into\s+["']([^"']+)["']/) ||
                          lowerMessage.match(/in\s+the\s+["']([^"']+)["']/) ||
                          lowerMessage.match(/\b(search|text|input|field|box|google)\b/)
        
        // Extract text to type with better fallbacks
        let textToType = 'test search' // Better default
        if (typeMatch) {
          textToType = typeMatch[1] || typeMatch[0]
        } else {
          // Try to extract any meaningful words from the message
          const words = lowerMessage.replace(/\b(type|search|for|into|in|the|and|or|a|an)\b/g, '').trim().split(/\s+/)
          const meaningfulWords = words.filter(word => word.length > 2 && !/^(can|you|please|help|me|my|i|we|our|your)$/.test(word))
          if (meaningfulWords.length > 0) {
            textToType = meaningfulWords.slice(0, 3).join(' ') // Take first 3 meaningful words
          }
        }
        
        const clickTarget = clickMatch ? (clickMatch[1] || clickMatch[0]) : 'Search'
        
        actions.push({
          toolName: 'click_and_type',
          parameters: { 
            click_target: clickTarget,
            text_to_type: textToType,
            press_enter: lowerMessage.includes('enter') || lowerMessage.includes('search')
          }
        })
        return actions // Return early - this is a compound action
      }
    }

    // Compound tool: Click on text (second priority)
    if ((lowerMessage.includes('click') && lowerMessage.includes('text')) || 
        (lowerMessage.includes('click') && lowerMessage.includes('on'))) {
      const clickOnTextTool = availableTools.find(tool => tool.name === 'click_on_text')
      if (clickOnTextTool) {
        // Extract quoted text or common button words
        const textMatch = lowerMessage.match(/["']([^"']+)["']/) || 
                         lowerMessage.match(/\b(submit|login|sign in|register|continue|next|back|cancel|ok|yes|no)\b/)
        const textToFind = textMatch ? textMatch[1] || textMatch[0] : 'Submit'
        
        actions.push({
          toolName: 'click_on_text',
          parameters: { text: textToFind }
        })
        return actions // Return early - this is a compound action
      }
    }

    // Atomic tool: Find text only
    if (lowerMessage.includes('find') && lowerMessage.includes('text')) {
      const findTextTool = availableTools.find(tool => tool.name === 'find_text')
      if (findTextTool) {
        const textMatch = lowerMessage.match(/["']([^"']+)["']/) || 
                         lowerMessage.match(/\b(submit|login|sign in|register|continue|next)\b/)
        const textToFind = textMatch ? textMatch[1] || textMatch[0] : 'Submit'
        
        actions.push({
          toolName: 'find_text',
          parameters: { text: textToFind }
        })
      }
    }
    // Screenshot tools
    if (lowerMessage.includes('screenshot') || lowerMessage.includes('capture')) {
      const screenshotTool = availableTools.find(tool => 
        tool.name.toLowerCase().includes('screenshot') || 
        tool.name.toLowerCase().includes('capture')
      )
      if (screenshotTool) {
        actions.push({
          toolName: screenshotTool.name,
          parameters: {}
        })
      }
    }

    // Atomic click at coordinates
    if (lowerMessage.includes('click') && !actions.length) {
      // Try to extract coordinates if mentioned
      const coordMatch = lowerMessage.match(/(\d+)[,\s]+(\d+)/)
      if (coordMatch) {
        const clickAtTool = availableTools.find(tool => tool.name === 'click_at')
        if (clickAtTool) {
          actions.push({
            toolName: 'click_at',
            parameters: { 
              x: parseInt(coordMatch[1]), 
              y: parseInt(coordMatch[2]) 
            }
          })
        }
      } else {
        // Fallback to old click tool
        const clickTool = availableTools.find(tool => tool.name === 'click')
        if (clickTool) {
          actions.push({
            toolName: clickTool.name,
            parameters: {}
          })
        }
      }
    }

    // Type tools
    if (lowerMessage.includes('type') || lowerMessage.includes('text')) {
      const typeTool = availableTools.find(tool => 
        tool.name.toLowerCase().includes('type') || 
        tool.name.toLowerCase().includes('text')
      )
      if (typeTool) {
        // Extract text to type (simple extraction)
        const textMatch = lowerMessage.match(/type\s+["']([^"']+)["']/) || 
                         lowerMessage.match(/text\s+["']([^"']+)["']/)
        const text = textMatch ? textMatch[1] : 'Hello World'
        
        actions.push({
          toolName: typeTool.name,
          parameters: { text }
        })
      }
    }

    // Cursor position
    if (lowerMessage.includes('cursor') || lowerMessage.includes('mouse position')) {
      const cursorTool = availableTools.find(tool => 
        tool.name.toLowerCase().includes('cursor') || 
        tool.name.toLowerCase().includes('position')
      )
      if (cursorTool) {
        actions.push({
          toolName: cursorTool.name,
          parameters: {}
        })
      }
    }

    // Screen info
    if (lowerMessage.includes('screen info') || lowerMessage.includes('display')) {
      const screenTool = availableTools.find(tool => 
        tool.name.toLowerCase().includes('screen') && 
        tool.name.toLowerCase().includes('info')
      )
      if (screenTool) {
        actions.push({
          toolName: screenTool.name,
          parameters: {}
        })
      }
    }

    // Debug OCR
    if (lowerMessage.includes('debug') && lowerMessage.includes('ocr')) {
      const debugOcrTool = availableTools.find(tool => tool.name === 'debug_ocr')
      if (debugOcrTool) {
        actions.push({
          toolName: 'debug_ocr',
          parameters: {}
        })
      }
    }

    return actions
  }

  // Format tool execution results for display
  private static formatToolResult(result: ToolExecutionResult): string {
    if (typeof result.result === 'string') {
      return result.result
    }
    
    if (typeof result.result === 'object') {
      // Handle specific result types
      if (result.result.image_base64) {
        return `Screenshot captured (${result.result.width}x${result.result.height})`
      }
      
      if (result.result.x !== undefined && result.result.y !== undefined) {
        return `Position: (${result.result.x}, ${result.result.y})`
      }
      
      // Generic object formatting
      return JSON.stringify(result.result, null, 2)
    }
    
    return String(result.result)
  }

  // Clean up MCP sessions
  static async cleanup() {
    if (MCPService.currentSessionId) {
      try {
        await invoke('end_mcp_session', { sessionId: MCPService.currentSessionId })
        console.log(`🔄 MCP Session ended: ${MCPService.currentSessionId}`)
      } catch (error) {
        console.error('Error ending MCP session:', error)
      }
      MCPService.currentSessionId = null
      MCPService.activeMCPSessions.clear()
    }
  }
}