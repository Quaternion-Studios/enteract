// sessionManager.ts - Handles chat session operations
import { v4 as uuidv4 } from 'uuid'
import { computed } from 'vue'
import type { ChatSession, ChatMessage } from '../types/chat'
import { sharedChatState } from './sharedState'
import { StorageService } from './storageService'

export class SessionManager {
  private static scrollChatToBottom: () => void

  static init(scrollCallback: () => void) {
    SessionManager.scrollChatToBottom = scrollCallback
  }

  // Get current chat session
  static getCurrentChatSession() {
    return computed(() => {
      if (!sharedChatState.currentChatId.value) return null
      return sharedChatState.chatSessions.value.find(session => session.id === sharedChatState.currentChatId.value) || null
    })
  }

  // Get current chat history
  static getCurrentChatHistory() {
    return computed(() => {
      if (!sharedChatState.currentChatId.value) return []
      const currentSession = sharedChatState.chatSessions.value.find(session => session.id === sharedChatState.currentChatId.value)
      const history = currentSession?.history || []
      return history
    })
  }

  static createNewChat(selectedModel: string | null, initialMessage?: ChatMessage) {
    const newChatId = uuidv4()
    const now = new Date().toISOString()
    
    const newSession: ChatSession = {
      id: newChatId,
      title: 'New Chat',
      history: initialMessage ? [initialMessage] : [],
      createdAt: now,
      updatedAt: now,
      modelId: selectedModel || undefined
    }
    
    sharedChatState.chatSessions.value.unshift(newSession)
    sharedChatState.currentChatId.value = newChatId
    console.log(`📝 [SHARED STATE] Created new chat session: ${newChatId}`)
    console.log(`📝 [SHARED STATE] Current chat ID updated to: ${newChatId}`)
    console.log(`📝 [SHARED STATE] Total sessions: ${sharedChatState.chatSessions.value.length}`)
  }

  static switchChat(chatId: string) {
    const chatExists = sharedChatState.chatSessions.value.some(session => session.id === chatId)
    if (chatExists) {
      const oldChatId = sharedChatState.currentChatId.value
      sharedChatState.currentChatId.value = chatId
      console.log(`🔄 [SHARED STATE] Switched from chat: ${oldChatId} to chat: ${chatId}`)
      
      // Find the session and log its state
      const session = sharedChatState.chatSessions.value.find(s => s.id === chatId)
      if (session) {
        console.log(`🔄 [SHARED STATE] New chat has ${session.history.length} messages`)
      }
      
      setTimeout(() => {
        SessionManager.scrollChatToBottom()
      }, 100)
    } else {
      console.error('❌ Chat not found:', chatId)
    }
  }

  static deleteChat(chatId: string) {
    const chatIndex = sharedChatState.chatSessions.value.findIndex(session => session.id === chatId)
    if (chatIndex !== -1) {
      const wasCurrentChat = chatId === sharedChatState.currentChatId.value
      sharedChatState.chatSessions.value.splice(chatIndex, 1)
      
      // If deleted chat was current, switch to another or create new
      if (wasCurrentChat) {
        if (sharedChatState.chatSessions.value.length > 0) {
          sharedChatState.currentChatId.value = sharedChatState.chatSessions.value[0].id
        } else {
          SessionManager.createNewChat(null)
        }
      }
      
      // Immediately save after deletion (not debounced)
      StorageService.saveAllChats(sharedChatState.chatSessions.value)
      console.log(`🗑️ [SHARED STATE] Deleted chat: ${chatId}`)
      if (wasCurrentChat) {
        console.log(`🔄 [SHARED STATE] Switched to ${sharedChatState.currentChatId.value} after deletion`)
      }
    }
  }

  static renameChat(chatId: string, newTitle: string) {
    const session = sharedChatState.chatSessions.value.find(s => s.id === chatId)
    if (session) {
      session.title = newTitle
      session.updatedAt = new Date().toISOString()
      console.log(`✏️ Renamed chat ${chatId} to: ${newTitle}`)
    }
  }

  static clearChat() {
    const currentSession = SessionManager.getCurrentChatSession().value
    if (currentSession) {
      currentSession.history = []
      currentSession.updatedAt = new Date().toISOString()
      console.log(`🧹 Cleared chat: ${sharedChatState.currentChatId.value}`)
    }
  }

  static addMessageToCurrentChat(message: ChatMessage) {
    // Ensure we have an active chat session
    if (!sharedChatState.currentChatId.value || !SessionManager.getCurrentChatSession().value) {
      SessionManager.createNewChat(null)
    }
    
    const currentSession = SessionManager.getCurrentChatSession().value
    if (currentSession) {
      currentSession.history.push(message)
      currentSession.updatedAt = new Date().toISOString()
      
      // Auto-title: If this is the first user message in a new chat, use it as title
      if (currentSession.title === 'New Chat' && 
          message.sender === 'user' && 
          currentSession.history.length === 1) {
        const title = message.text.length > 50 
          ? message.text.substring(0, 47) + '...'
          : message.text
        currentSession.title = title
      }
    }
  }

  static async loadAllChats(selectedModel: string | null) {
    try {
      const chats = await StorageService.loadAllChats()
      if (chats.length > 0) {
        sharedChatState.chatSessions.value = chats
        // Set current chat to most recently updated, or first one
        const sortedByUpdated = [...chats].sort((a, b) => 
          new Date(b.updatedAt).getTime() - new Date(a.updatedAt).getTime()
        )
        sharedChatState.currentChatId.value = sortedByUpdated[0].id
        console.log(`✅ Loaded ${chats.length} chat sessions`)
      } else {
        // No chats exist, create a new one
        SessionManager.createNewChat(selectedModel)
        console.log('📝 No existing chats found, created new chat session')
      }
    } catch (error) {
      console.error('❌ Failed to load chat sessions:', error)
      // Fallback: create new chat
      SessionManager.createNewChat(selectedModel)
    }
  }
}