// mcpService.ts - Handles MCP (Model Context Protocol) operations and tool calling
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { ref } from 'vue'
import { SessionManager } from './sessionManager'

let messageIdCounter = 1000 // Use higher counter to avoid conflicts

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
  static async processEnteractMessage(message: string, selectedModel: string | null) {
    console.log('🔧 [MCP] Processing @enteract message:', message)
    try {
      // Remove @enteract prefix and trim
      const cleanMessage = message.replace(/^@enteract\s*/i, '').trim()
      console.log('🔧 [MCP] Clean message:', cleanMessage)
      
      if (!cleanMessage) {
        SessionManager.addMessageToCurrentChat({
          id: messageIdCounter++,
          sender: 'assistant',
          text: '🔧 **MCP Mode** - Please provide a command after @enteract\n\nExample: `@enteract take a screenshot`',
          timestamp: new Date(),
          messageType: 'text'
        })
        return
      }

      // Add user message to chat
      SessionManager.addMessageToCurrentChat({
        id: messageIdCounter++,
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
      const thinkingMessageId = messageIdCounter++
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
        id: messageIdCounter++,
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

  // Simple tool selection based on message keywords
  private static selectToolsForMessage(message: string, availableTools: any[]): { toolName: string, parameters: any }[] {
    const actions: { toolName: string, parameters: any }[] = []
    const lowerMessage = message.toLowerCase()

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

    // Click tools
    if (lowerMessage.includes('click')) {
      const clickTool = availableTools.find(tool => 
        tool.name.toLowerCase().includes('click')
      )
      if (clickTool) {
        // Try to extract coordinates if mentioned
        const coordMatch = lowerMessage.match(/(\d+)[,\s]+(\d+)/)
        const params = coordMatch ? { x: parseInt(coordMatch[1]), y: parseInt(coordMatch[2]) } : {}
        
        actions.push({
          toolName: clickTool.name,
          parameters: params
        })
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