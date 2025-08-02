// useChatManagement.ts - Main composable that orchestrates all chat functionality
import { ref, watch, onMounted, onUnmounted, type Ref } from 'vue'
import type { ChatMessage } from '../types/chat'
import { sharedChatState } from './sharedState'
import { StorageService } from './storageService'
import { SessionManager } from './sessionManager'
import { VisionService } from './visionService'
import { AgentService } from './agentService'
import { FileService } from './fileService'
import { ContextManager } from './contextManager'
import { MarkdownRenderer } from './markdownRenderer'

export const useChatManagement = (selectedModel: string | null, scrollChatToBottom: () => void, currentAgent?: Ref<string>) => {
  const chatMessage = ref('')
  const fileInput = ref<HTMLInputElement>()
  
  
  // Use the shared state instead of creating new instances
  const { chatSessions, currentChatId, isInitialized } = sharedChatState
  
  // Initialize all services with scroll callback
  SessionManager.init(scrollChatToBottom)
  VisionService.init(scrollChatToBottom)
  AgentService.init(scrollChatToBottom)
  FileService.init(scrollChatToBottom)

  // Computed properties from SessionManager
  const currentChatHistory = SessionManager.getCurrentChatHistory()
  const currentChatSession = SessionManager.getCurrentChatSession()

  // Send message function that uses AgentService
  const sendMessage = async (agentType: string = 'enteract', customMessage?: string) => {
    // Use custom message if provided, otherwise use chatMessage.value
    const messageToSend = customMessage || chatMessage.value.trim()
    if (!messageToSend) return
    
    // Only clear chatMessage if we're using it (not a custom message)
    if (!customMessage) {
      chatMessage.value = ''
    }
    
    await AgentService.sendMessage(messageToSend, selectedModel, agentType)
  }
  
  // Cancel an active AI response
  const cancelResponse = async (messageId: number) => {
    await AgentService.cancelResponse(messageId)
  }

  // Keyboard handler
  const handleChatKeydown = (event: KeyboardEvent) => {
    if (event.key === 'Enter' && !event.shiftKey) {
      event.preventDefault()
      // Use the current agent if provided, otherwise default to 'enteract'
      const agentToUse = currentAgent?.value || 'enteract'
      sendMessage(agentToUse)
    }
  }

  // File upload functions using FileService
  const triggerFileUpload = () => {
    FileService.triggerFileUpload(fileInput.value)
  }

  const handleFileUpload = async (event: Event, showChatWindow: any) => {
    await FileService.handleFileUpload(event, showChatWindow)
  }

  // Agent activation functions using AgentService
  const startDeepResearch = async (showChatWindow: any) => {
    await AgentService.startDeepResearch(showChatWindow)
  }

  const startConversationalAgent = async (showChatWindow: any) => {
    await AgentService.startConversationalAgent(showChatWindow)
  }

  const startCodingAgent = async (showChatWindow: any) => {
    await AgentService.startCodingAgent(showChatWindow)
  }

  const startComputerUseAgent = async (showChatWindow: any) => {
    await AgentService.startComputerUseAgent(showChatWindow)
  }


  // Vision service function
  const takeScreenshotAndAnalyze = async (showChatWindow: any) => {
    await VisionService.takeScreenshotAndAnalyze(showChatWindow)
  }

  // Session management functions from SessionManager
  const createNewChat = (initialMessage?: ChatMessage) => {
    SessionManager.createNewChat(selectedModel, initialMessage)
  }

  const switchChat = (chatId: string) => {
    SessionManager.switchChat(chatId)
  }

  const deleteChat = (chatId: string) => {
    SessionManager.deleteChat(chatId)
  }

  const renameChat = (chatId: string, newTitle: string) => {
    SessionManager.renameChat(chatId, newTitle)
  }

  const clearChat = () => {
    SessionManager.clearChat()
  }

  const loadAllChats = async () => {
    await SessionManager.loadAllChats(selectedModel)
  }

  const saveAllChats = async () => {
    await StorageService.saveAllChats(chatSessions.value)
  }

  // Utility functions
  const estimateTokens = (text: string): number => {
    return ContextManager.estimateTokens(text)
  }

  const renderMarkdown = (text: string): string => {
    return MarkdownRenderer.render(text)
  }

  // Initialize chat sessions on mount
  onMounted(async () => {
    // Only initialize shared state once
    if (!isInitialized.value) {
      await loadAllChats()
      
      // Set up watchers after initial load
      watch(chatSessions, () => {
        StorageService.debouncedSaveChats(chatSessions.value)
      }, { deep: true })
      
      
      isInitialized.value = true
      console.log('✅ Chat management initialized')
    }
  })


  return {
    // Legacy exports for compatibility
    chatMessage,
    chatHistory: currentChatHistory, // Export as computed for backward compatibility
    fileInput,
    
    // New multi-session exports
    chatSessions,
    currentChatId,
    currentChatHistory,
    currentChatSession,
    
    // Session management functions
    createNewChat,
    switchChat,
    deleteChat,
    renameChat,
    clearChat,
    loadAllChats,
    saveAllChats,
    
    // Utility functions
    estimateTokens,
    renderMarkdown,
    
    // Agent and messaging functions
    takeScreenshotAndAnalyze,
    startDeepResearch,
    startConversationalAgent,
    startCodingAgent,
    startComputerUseAgent,
    sendMessage,
    handleChatKeydown,
    
    // File upload functions
    triggerFileUpload,
    handleFileUpload,
    
    // Cancel function
    cancelResponse
  }
}