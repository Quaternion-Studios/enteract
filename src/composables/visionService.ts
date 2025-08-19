// visionService.ts - Handles screenshot analysis and vision capabilities
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import type { ScreenshotResponse } from '../types/chat'
import { SessionManager } from './sessionManager'
import { getNextMessageId } from './messageIdGenerator'

export class VisionService {
  private static scrollChatToBottom: () => void

  static init(scrollCallback: () => void) {
    VisionService.scrollChatToBottom = scrollCallback
  }

  static async takeScreenshotAndAnalyze(showChatWindow: any) {
    try {
      console.log('🔍 Analyzing screen for vision analysis...')
      
      // Take screenshot
      const screenshot = await invoke<ScreenshotResponse>('capture_screenshot')
      
      // Auto-open chat window if not open
      if (!showChatWindow.value) {
        showChatWindow.value = true
        setTimeout(() => {
          VisionService.scrollChatToBottom()
        }, 150)
      }
      
      // Add screen analysis message to current chat
      SessionManager.addMessageToCurrentChat({
        id: getNextMessageId(),
        sender: 'user',
        text: `🔍 Screen captured for analysis (${screenshot.width}×${screenshot.height})`,
        timestamp: new Date(),
        messageType: 'text'
      })
      
      // Generate unique session ID
      const sessionId = `vision-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`
      
      // Add streaming message placeholder with more detailed status
      const currentHistory = SessionManager.getCurrentChatHistory().value
      const streamingMessageIndex = currentHistory.length
      SessionManager.addMessageToCurrentChat({
        id: getNextMessageId(),
        sender: 'assistant',
        text: '🔍 Initializing Qwen vision model for analysis▋',
        timestamp: new Date(),
        messageType: 'text'
      })
      
      setTimeout(() => {
        VisionService.scrollChatToBottom()
      }, 50)
      
      let fullResponse = ''
      let isTyping = true
      let hasStarted = false
      
      // Set up event listener for vision analysis
      const unlisten = await listen(`ollama-stream-${sessionId}`, (event: any) => {
        const data = event.payload
        const currentHistory = SessionManager.getCurrentChatHistory().value
        
        switch (data.type) {
          case 'start':
            hasStarted = true
            console.log(`👁️ Started vision analysis with ${data.model}`)
            if (currentHistory[streamingMessageIndex]) {
              currentHistory[streamingMessageIndex].text = `👁️ Qwen ${data.model} is analyzing your screenshot▋`
            }
            setTimeout(() => {
              VisionService.scrollChatToBottom()
            }, 10)
            break
            
          case 'chunk':
            if (data.text) {
              // First chunk - update status
              if (fullResponse === '') {
                if (currentHistory[streamingMessageIndex]) {
                  currentHistory[streamingMessageIndex].text = '👁️ Vision Analysis:\n\n'
                }
              }
              
              fullResponse += data.text
              if (currentHistory[streamingMessageIndex]) {
                currentHistory[streamingMessageIndex].text = '👁️ Vision Analysis:\n\n' + fullResponse + (isTyping ? '▋' : '')
              }
              
              setTimeout(() => {
                VisionService.scrollChatToBottom()
              }, 10)
            }
            
            if (data.done) {
              isTyping = false
              if (currentHistory[streamingMessageIndex]) {
                currentHistory[streamingMessageIndex].text = '👁️ Vision Analysis:\n\n' + fullResponse
              }
              console.log('✅ Vision analysis streaming completed')
            }
            break
            
          case 'error':
            isTyping = false
            console.error('Vision analysis error:', data.error)
            if (currentHistory[streamingMessageIndex]) {
              if (data.error.includes('qwen2.5vl:3b')) {
                currentHistory[streamingMessageIndex].text = `❌ Vision model not found. Please install qwen2.5vl:3b first:\n\n\`\`\`bash\nollama pull qwen2.5vl:3b\n\`\`\``
              } else {
                currentHistory[streamingMessageIndex].text = `❌ Vision analysis error: ${data.error}`
              }
            }
            break
            
          case 'complete':
            isTyping = false
            console.log('🎉 Vision analysis session completed')
            unlisten()
            break
        }
      })
      
      // Add a timeout to show model loading if it takes too long
      const loadingTimeout = setTimeout(() => {
        const currentHistory = SessionManager.getCurrentChatHistory().value
        if (!hasStarted && currentHistory[streamingMessageIndex]) {
          currentHistory[streamingMessageIndex].text = '🔄 Loading Qwen vision model (this may take a moment for the first run)▋'
          setTimeout(() => {
            VisionService.scrollChatToBottom()
          }, 10)
        }
      }, 2000)
      
      // Start vision analysis
      await invoke('generate_vision_analysis', {
        prompt: 'Please analyze this screenshot in detail.',
        imageBase64: screenshot.image_base64,
        sessionId: sessionId
      })
      
      // Clear the loading timeout
      clearTimeout(loadingTimeout)
      
    } catch (error) {
      console.error('Failed to analyze screen:', error)
      
      // More detailed error messages
      const errorString = error instanceof Error ? error.message : String(error)
      let errorMessage = `❌ Failed to analyze screen: ${errorString}`
      if (errorString.includes('connection refused') || errorString.includes('ECONNREFUSED')) {
        errorMessage = `❌ Cannot connect to Ollama. Please make sure Ollama is running:\n\n\`\`\`bash\nollama serve\n\`\`\``
      } else if (errorString.includes('model') && errorString.includes('not found')) {
        errorMessage = `❌ Vision model not available. Install with:\n\n\`\`\`bash\nollama pull qwen2.5vl:3b\n\`\`\``
      }
      
      SessionManager.addMessageToCurrentChat({
        id: getNextMessageId(),
        sender: 'assistant',
        text: errorMessage,
        timestamp: new Date(),
        messageType: 'text'
      })
    }
  }
}