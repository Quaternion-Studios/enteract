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

   static async handleEnteractMessage(message: string) {
    try {
      console.log('🎯 [MCP] @enteract triggered - entering intelligent MCP workflow')
      
      // Clean the message
      const cleanMessage = message.replace(/^@enteract\s*/i, '').trim()
      if (!cleanMessage) {
        SessionManager.addMessageToCurrentChat({
          id: messageIdCounter++,
          sender: 'assistant',
          text: '🎯 **MCP Agent** - Please provide a request after @enteract\n\nExample: `@enteract take a screenshot and find the submit button`',
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
      const sessionId = await MCPService.ensureMCPSession()
      console.log('🎯 [MCP] Session ready:', sessionId)
      
      // Start planning phase
      const planningMessageId = messageIdCounter++
      SessionManager.addMessageToCurrentChat({
        id: planningMessageId,
        sender: 'assistant',
        text: '🎯 **MCP Planning** - Round 1: Analyzing request and generating intelligent execution plan...▋',
        timestamp: new Date(),
        messageType: 'text',
        isStreaming: true
      })

      setTimeout(() => MCPService.scrollChatToBottom(), 50)

      // Listen for planning progress
      const planningListener = await listen('mcp_planning_progress', (event: any) => {
        const progress = event.payload
        MCPService.updatePlanningProgress(planningMessageId, progress)
      })

      try {
        // Generate intelligent execution plan
        const executionPlan = await invoke<any>('create_execution_plan_iterative', {
          sessionId,
          userRequest: cleanMessage
        })

        console.log('🎯 [MCP] Generated execution plan:', executionPlan)

        // Show plan to user for approval
        await MCPService.displayPlanForApproval(planningMessageId, executionPlan)

        // Wait for user approval (for now, simulate approval)
        console.log('🔒 [MCP] Awaiting user approval...')
        await new Promise(resolve => setTimeout(resolve, 2000))

        // Execute the approved plan
        await MCPService.executeApprovedPlanInteractive(executionPlan, planningMessageId)

      } finally {
        // Clean up listener
        planningListener()
      }

    } catch (error) {
      console.error('❌ [MCP] Error in @enteract workflow:', error)
      SessionManager.addMessageToCurrentChat({
        id: messageIdCounter++,
        sender: 'assistant',
        text: `❌ **MCP Error**: ${error instanceof Error ? error.message : 'Unknown error occurred'}`,
        timestamp: new Date(),
        messageType: 'text'
      })
    }
  }

  // Update planning progress in real-time
  private static updatePlanningProgress(messageId: number, progress: any) {
    const currentHistory = SessionManager.getCurrentChatHistory().value
    const messageIndex = currentHistory.findIndex(m => m.id === messageId)
    if (messageIndex !== -1) {
      const statusEmoji: Record<string, string> = { // Add explicit typing
        'Analyzing': '🔍',
        'Questioning': '❓',
        'Planning': '📋',
        'Validating': '✅',
        'Complete': '🎯',
        'Failed': '❌'
      }

      const emoji = statusEmoji[progress.status] || '🔄' // Now properly typed

      currentHistory[messageIndex].text = `${emoji} **MCP Planning** - Round ${progress.iteration}/${progress.max_iterations}: ${progress.message}${progress.status !== 'Complete' && progress.status !== 'Failed' ? '▋' : ''}`
      
      if (progress.status === 'Complete' || progress.status === 'Failed') {
        currentHistory[messageIndex].isStreaming = false
      }
    }
  }

  // Display execution plan for user approval
  private static async displayPlanForApproval(messageId: number, executionPlan: any) {
    const currentHistory = SessionManager.getCurrentChatHistory().value
    const messageIndex = currentHistory.findIndex(m => m.id === messageId)
    if (messageIndex !== -1) {
      const stepDescriptions = executionPlan.steps.map((step: any, i: number) => 
        `**${i + 1}.** ${step.tool_name}: ${step.description}\n   ${step.requires_permission ? '🔒 ' : ''}Parameters: \`${JSON.stringify(step.parameters)}\``
      ).join('\n\n')
      
      const riskLevels: Record<string, string> = { // Add explicit typing
        'Low': '🟢 Low Risk',
        'Medium': '🟡 Medium Risk', 
        'High': '🟠 High Risk',
        'Critical': '🔴 Critical Risk'
      }
      
      const riskLevel = riskLevels[executionPlan.overall_risk] || '🟡 Medium Risk' // Now properly typed

      currentHistory[messageIndex].text = `🎯 **Intelligent Execution Plan**

  **Request**: ${executionPlan.user_request}
  **Steps**: ${executionPlan.steps.length}
  **Risk Level**: ${riskLevel}

  ${stepDescriptions}

  ⚠️ **Ready to execute ${executionPlan.steps.length} steps. Approve execution?**

  ${executionPlan.steps.some((s: any) => s.requires_permission) ? '🔒 Some steps require individual approval during execution.' : ''}`
      
      currentHistory[messageIndex].isStreaming = false
    }
  }

  // Execute approved plan with real-time updates
  private static async executeApprovedPlanInteractive(executionPlan: any, messageId: number) {
    const results: string[] = []
    // Remove: let currentStepIndex = 0  // This was unused

    // Listen for execution progress
    const progressListener = await listen('mcp_execution_progress', (event: any) => {
      const progress = event.payload
      MCPService.updateExecutionProgress(messageId, progress, results)
    })

    try {
      // Execute the plan
      const executionResults = await invoke<any[]>('execute_plan_interactive', {
        plan: executionPlan
      })

      // Process results
      for (let i = 0; i < executionResults.length; i++) {
        const result = executionResults[i]
        const step = executionPlan.steps[i]
        
        if (result.success) {
          results.push(`✅ **Step ${i + 1}**: ${step.description} - ${MCPService.formatToolResult(result)}`)
        } else {
          results.push(`❌ **Step ${i + 1}**: ${step.description} - ${result.error || 'Failed'}`)
        }
      }

      // Show final results
      MCPService.displayFinalResults(messageId, results, executionPlan.steps.length)

    } catch (error) {
      results.push(`❌ **Execution Error**: ${error}`)
      MCPService.displayFinalResults(messageId, results, executionPlan.steps.length)
    } finally {
      progressListener()
    }
  }

  // Update execution progress
  private static updateExecutionProgress(messageId: number, progress: any, results: string[]) {
    const currentHistory = SessionManager.getCurrentChatHistory().value
    const messageIndex = currentHistory.findIndex(m => m.id === messageId)
    if (messageIndex !== -1) {
      const statusEmojis: Record<string, string> = { // Add explicit typing
        'Pending': '⏳',
        'Executing': '🔄',
        'WaitingApproval': '🔒',
        'Complete': '✅',
        'Failed': '❌'
      }
      
      const statusEmoji = statusEmojis[progress.status] || '🔄' // Now properly typed

      currentHistory[messageIndex].text = `🎯 **MCP Execution Progress**

  ${results.join('\n\n')}

  ${statusEmoji} **Currently executing step ${progress.step_number}/${progress.total_steps}**: ${progress.step_description}...`
    }
    
    setTimeout(() => MCPService.scrollChatToBottom(), 10)
  }


  // Display final execution results
  private static displayFinalResults(messageId: number, results: string[], totalSteps: number) {
    const currentHistory = SessionManager.getCurrentChatHistory().value
    const messageIndex = currentHistory.findIndex(m => m.id === messageId)
    if (messageIndex !== -1) {
      const successCount = results.filter(r => r.startsWith('✅')).length
      const failureCount = results.filter(r => r.startsWith('❌')).length
      
      currentHistory[messageIndex].text = `🎯 **MCP Execution Complete**

${results.join('\n\n')}

📊 **Summary**: ${successCount}/${totalSteps} steps completed successfully${failureCount > 0 ? `, ${failureCount} failed` : ''}
✨ **Execution finished!**`
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
      // Fallback to regex-based selection
      return MCPService.selectToolsForMessage(message, availableTools)
    }
  }

  // Fallback regex-based tool selection
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