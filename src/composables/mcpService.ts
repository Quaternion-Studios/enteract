// mcpService.ts - Handles MCP (Model Context Protocol) operations and tool calling
import { invoke } from '@tauri-apps/api/core'
import { SessionManager } from './sessionManager'
import { listen } from '@tauri-apps/api/event'

let messageIdCounter = 1000 // Use higher counter to avoid conflicts

// Generate unique message ID to prevent database conflicts (i32 compatible)
function generateUniqueMessageId(): number {
  return Math.floor(Date.now() / 1000) + Math.floor(Math.random() * 1000) // Use seconds, not milliseconds
}

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

  // LLM-powered MCP workflow with tool access
  static async processEnteractMessageSimpleLLM(message: string, selectedModel: string | null) {
    const requestId = Date.now() + '-' + Math.random().toString(36).substr(2, 5)
    console.log(`🤖 [MCP] Request ${requestId}: LLM-powered @enteract workflow`)
    try {
      // Clean the message
      const cleanMessage = message.replace(/^@enteract\s*/i, '').trim()
      
      // Add user message to chat
      SessionManager.addMessageToCurrentChat({
        id: messageIdCounter++,
        sender: 'user',
        text: message,
        timestamp: new Date(),
        messageType: 'text'
      })
      
      if (!cleanMessage) {
        SessionManager.addMessageToCurrentChat({
          id: generateUniqueMessageId(),
          sender: 'assistant',
          text: '🤖 **MCP Agent** - Please provide a request after @enteract\n\nExample: `@enteract what tools are available?`',
          timestamp: new Date(),
          messageType: 'text'
        })
        return
      }

      // Ensure MCP session is active
      const sessionId = await MCPService.ensureMCPSession()
      const availableTools = await MCPService.getAvailableTools(sessionId)
      
      // Handle simple "list tools" request directly
      if (cleanMessage.toLowerCase().includes('tools') && (cleanMessage.toLowerCase().includes('available') || cleanMessage.toLowerCase().includes('list'))) {
        const toolList = availableTools.map(tool => 
          `• **${tool.name}**: ${tool.description}`
        ).join('\n')
        
        SessionManager.addMessageToCurrentChat({
          id: generateUniqueMessageId(),
          sender: 'assistant',
          text: `🤖 **Available MCP Tools**\n\n${toolList}\n\n✨ Use @enteract with any request to let me choose and execute the right tool!`,
          timestamp: new Date(),
          messageType: 'text'
        })
        return
      }

      // Add thinking message
      const thinkingMessageId = generateUniqueMessageId()
      SessionManager.addMessageToCurrentChat({
        id: thinkingMessageId,
        sender: 'assistant',
        text: '🤖 **MCP Agent** - Analyzing request...▋',
        timestamp: new Date(),
        messageType: 'text',
        isStreaming: true
      })

      setTimeout(() => MCPService.scrollChatToBottom(), 50)

      // Use LLM to intelligently select tools
      console.log(`🤖 [MCP] Request ${requestId}: Using LLM to select tools for: "${cleanMessage}"`)
      const toolActions = await MCPService.selectToolWithLLM(cleanMessage, availableTools, selectedModel)
      console.log(`🤖 [MCP] Request ${requestId}: LLM returned actions:`, toolActions)
      
      if (!toolActions || toolActions.length === 0) {
        const currentHistory = SessionManager.getCurrentChatHistory().value
        const messageIndex = currentHistory.findIndex(m => m.id === thinkingMessageId)
        if (messageIndex !== -1) {
          currentHistory[messageIndex].text = `🤖 **MCP Agent** - I don't recognize that request. Here's what I can do:\n\n${availableTools.map(tool => `• **${tool.name}**: ${tool.description}`).join('\n')}`
          currentHistory[messageIndex].isStreaming = false
        }
        return
      }

      // Execute the first matching tool
      const action = toolActions[0]
      const currentHistory = SessionManager.getCurrentChatHistory().value
      const messageIndex = currentHistory.findIndex(m => m.id === thinkingMessageId)
      
      if (messageIndex !== -1) {
        currentHistory[messageIndex].text = `🤖 **MCP Agent** - Executing ${action.toolName}...▋`
      }

      try {
        console.log(`🤖 [MCP] Request ${requestId}: Executing tool ${action.toolName} with parameters:`, action.parameters)
        const result = await MCPService.executeTool(sessionId, action.toolName, action.parameters)
        console.log(`🤖 [MCP] Request ${requestId}: Tool execution result:`, result)
        
        if (messageIndex !== -1) {
          if (result.success) {
            currentHistory[messageIndex].text = `🤖 **MCP Agent** - ✅ Done!\n\n**Tool Used**: ${action.toolName}\n**Result**: ${MCPService.formatToolResult(result)}`
          } else {
            currentHistory[messageIndex].text = `🤖 **MCP Agent** - ❌ Error\n\n**Tool**: ${action.toolName}\n**Error**: ${result.error || 'Unknown error'}`
          }
          currentHistory[messageIndex].isStreaming = false
        }
      } catch (error) {
        console.error(`❌ [MCP] Request ${requestId}: Tool execution failed:`, error)
        if (messageIndex !== -1) {
          currentHistory[messageIndex].text = `🤖 **MCP Agent** - ❌ Error: ${error}`
          currentHistory[messageIndex].isStreaming = false
        }
      }

    } catch (error) {
      console.error('❌ [MCP] Error in LLM workflow:', error)
      SessionManager.addMessageToCurrentChat({
        id: generateUniqueMessageId(),
        sender: 'assistant',
        text: `❌ **MCP Error**: ${error instanceof Error ? error.message : 'Unknown error occurred'}`,
        timestamp: new Date(),
        messageType: 'text'
      })
    } finally {
      // Ensure scroll happens
      setTimeout(() => MCPService.scrollChatToBottom(), 100)
    }
  }

  // LLM-based tool selection  
  private static async selectToolWithLLM(message: string, availableTools: any[], selectedModel: string | null): Promise<{ toolName: string, parameters: any }[]> {
    try {
      const requestId = Date.now() + '-' + Math.random().toString(36).substr(2, 9)
      const toolsDescription = availableTools.map(tool => 
        `- ${tool.name}: ${tool.description}`
      ).join('\n')

      const systemPrompt = `ANALYZE: "${message}"

DECISION:
If greeting/conversation (hi, hello, how are you) → DECLINE
If computer task → SELECT TOOL

Tools available:
${toolsDescription}

Examples:
"hi there" → {"decline": true, "reason": "greeting"}
"take screenshot" → {"tool_name": "take_screenshot", "parameters": {}}
"cursor position" → {"tool_name": "get_cursor_position", "parameters": {}}
"OCR results" → {"tool_name": "debug_ocr", "parameters": {}}

For "${message}":
JSON only:`

      const llmResponse = await invoke<string>('generate_ollama_response', {
        model: selectedModel || 'gemma3:1b-it-qat',
        prompt: systemPrompt
      })

      console.log('🤖 [MCP] LLM tool selection response:', llmResponse)
      console.log('🤖 [MCP] LLM response length:', llmResponse.length)
      console.log('🤖 [MCP] LLM response type:', typeof llmResponse)

      // Try to parse JSON response
      try {
        const jsonMatch = llmResponse.match(/\{[\s\S]*\}/)
        console.log('🤖 [MCP] JSON match found:', jsonMatch ? jsonMatch[0] : 'none')
        if (jsonMatch) {
          const toolSelection = JSON.parse(jsonMatch[0])
          console.log('🤖 [MCP] Parsed tool selection:', toolSelection)
          
          // Handle decline case
          if (toolSelection.decline) {
            console.log('🤖 [MCP] LLM declined request:', toolSelection.reason)
            return [] // Return empty to trigger "no tools found" response
          }
          
          if (toolSelection.tool_name) {
            console.log('🤖 [MCP] LLM selected tool:', toolSelection.tool_name, 'Parameters:', toolSelection.parameters, 'Reasoning:', toolSelection.reasoning)
            return [{
              toolName: toolSelection.tool_name,
              parameters: toolSelection.parameters || {}
            }]
          }
        }
      } catch (parseError) {
        console.error('❌ [MCP] Failed to parse LLM tool selection:', parseError)
        console.error('❌ [MCP] Raw LLM response:', llmResponse)
      }

      return []
    } catch (error) {
      console.error('❌ [MCP] LLM tool selection failed:', error)
      return []
    }
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