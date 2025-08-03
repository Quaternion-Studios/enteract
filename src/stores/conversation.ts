import { defineStore } from 'pinia'
import { ref, computed, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'

export interface ConversationMessage {
  id: string
  type: 'user' | 'system'
  source: 'microphone' | 'loopback'
  content: string
  timestamp: number
  confidence?: number
  isPreview?: boolean
  isTyping?: boolean
}

export interface ConversationSession {
  id: string
  name: string
  startTime: number
  endTime?: number
  messages: ConversationMessage[]
  isActive: boolean
}

export const useConversationStore = defineStore('conversation', () => {
  // State
  const currentSession = ref<ConversationSession | null>(null)
  const sessions = ref<ConversationSession[]>([])
  const isRecording = ref(false)
  const isAudioLoopbackActive = ref(false)

  // Persistence key
  const STORAGE_KEY = 'conversation-sessions'

  // Load sessions from Rust backend
  const loadSessions = async () => {
    try {
      console.log('📁 Store: Attempting to load conversations from backend...')
      const response = await invoke<{conversations: ConversationSession[]}>('load_conversations')
      sessions.value = response.conversations
      console.log(`📁 Store: Successfully loaded ${sessions.value.length} conversation sessions from backend:`, sessions.value.map(s => ({ id: s.id, name: s.name, messageCount: s.messages.length })))
    } catch (error) {
      console.error('📁 Store: Failed to load conversation sessions from backend:', error)
      // Fallback to localStorage for migration
      try {
        const stored = localStorage.getItem(STORAGE_KEY)
        if (stored) {
          const parsed = JSON.parse(stored)
          sessions.value = parsed
          console.log(`📁 Store: Migrated ${parsed.length} conversation sessions from localStorage`)
          // Save to backend and clear localStorage
          await saveSessions()
          localStorage.removeItem(STORAGE_KEY)
        }
      } catch (migrationError) {
        console.error('📁 Store: Failed to migrate from localStorage:', migrationError)
      }
    }
  }

  // Save sessions to Rust backend with proper state management
  const saveSessions = async (forceImmediate = false) => {
    if (isSaving.value && !forceImmediate) {
      console.log('💾 Store: Save already in progress, will queue this save')
      pendingSave.value = true
      return
    }

    try {
      isSaving.value = true
      pendingSave.value = false
      
      console.log(`💾 Store: Attempting to save ${sessions.value.length} conversation sessions to backend...`)
      console.log(`💾 Store: Sessions to save:`, sessions.value.map(s => ({ id: s.id, name: s.name, messageCount: s.messages.length, isActive: s.isActive, endTime: s.endTime })))
      
      await invoke('save_conversations', {
        payload: { conversations: sessions.value }
      })
      console.log(`💾 Store: Successfully saved ${sessions.value.length} conversation sessions to backend`)
      
      // Verify save by immediately loading back
      const response = await invoke<{conversations: ConversationSession[]}>('load_conversations')
      const savedCount = response.conversations.length
      const expectedCount = sessions.value.length
      
      if (savedCount !== expectedCount) {
        console.error(`💾 Store: Save verification failed! Expected ${expectedCount}, got ${savedCount}`)
        throw new Error(`Save verification failed: expected ${expectedCount}, got ${savedCount}`)
      }
      
      console.log(`💾 Store: Save verified successfully - ${savedCount} sessions persisted`)
      
    } catch (error) {
      console.error('💾 Store: Failed to save conversation sessions to backend:', error)
      throw error // Re-throw to let caller handle
    } finally {
      isSaving.value = false
      
      // If there was a pending save, execute it now
      if (pendingSave.value) {
        console.log('💾 Store: Executing queued save')
        setTimeout(() => saveSessions().catch(console.error), 100)
      }
    }
  }

  // Save state management
  const isSaving = ref(false)
  const pendingSave = ref(false)
  
  // Watch for changes and auto-save (debounced, but can be disabled)
  let saveTimeout: number | null = null
  let autoSaveEnabled = ref(true)
  
  watch(sessions, () => {
    if (!autoSaveEnabled.value || isSaving.value) {
      pendingSave.value = true
      return
    }
    
    if (saveTimeout) clearTimeout(saveTimeout)
    saveTimeout = window.setTimeout(() => {
      if (!isSaving.value) {
        saveSessions().catch(console.error)
      }
    }, 1000) // Debounce saves by 1 second
  }, { deep: true })

  // Initialize on store creation
  loadSessions().catch(console.error)

  // Computed
  const currentMessages = computed(() => {
    return currentSession.value?.messages || []
  })

  const activeSessions = computed(() => {
    return sessions.value.filter(session => session.isActive)
  })

  const recentSessions = computed(() => {
    return [...sessions.value]
      .sort((a, b) => b.startTime - a.startTime)
      .slice(0, 10)
  })

  // Actions
  const createSession = async (name?: string): Promise<ConversationSession> => {
    console.log('🆕 Store: Creating new session')
    
    // Disable auto-save during session creation to prevent race conditions
    autoSaveEnabled.value = false
    
    try {
      const session: ConversationSession = {
        id: `session_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`,
        name: name || `Conversation ${new Date().toLocaleTimeString()}`,
        startTime: Date.now(),
        messages: [],
        isActive: true
      }

      // Deactivate any existing current session
      if (currentSession.value) {
        currentSession.value.isActive = false
        console.log('🆕 Store: Deactivated previous session')
      }

      sessions.value.push(session)
      currentSession.value = session
      console.log('🆕 Store: Session created successfully:', session.id)
      
      // Force immediate save of new session and wait for completion
      await saveSessions(true)
      
      return session
    } finally {
      // Re-enable auto-save immediately after successful save
      autoSaveEnabled.value = true
      
      // Process any pending saves that accumulated during the disable period
      if (pendingSave.value) {
        console.log('💾 Store: Processing pending save after session creation')
        setTimeout(() => saveSessions().catch(console.error), 100)
      }
    }
  }

  const endSession = async (sessionId?: string) => {
    const targetSession = sessionId 
      ? sessions.value.find(s => s.id === sessionId)
      : currentSession.value

    if (targetSession) {
      // Disable auto-save during critical operation
      autoSaveEnabled.value = false
      
      try {
        targetSession.isActive = false
        targetSession.endTime = Date.now()
        console.log(`🏁 Store: Session ended with ${targetSession.messages.length} messages:`, targetSession.id)
        
        if (currentSession.value?.id === targetSession.id) {
          currentSession.value = null
          console.log('🏁 Store: Cleared current session reference')
        }
        
        // Force immediate save when ending session to ensure persistence
        console.log('💾 Store: Force saving session on end with verification')
        await saveSessions(true) // Force immediate save
        
        console.log('🏁 Store: Session end operation completed successfully')
        
      } catch (error) {
        console.error('🏁 Store: Failed to end session properly:', error)
        throw error
      } finally {
        // Re-enable auto-save
        autoSaveEnabled.value = true
      }
    }
  }

  // Complete a session without clearing currentSession - keeps it accessible for review and continuation
  const completeSession = async (sessionId?: string) => {
    const targetSession = sessionId 
      ? sessions.value.find(s => s.id === sessionId)
      : currentSession.value

    if (targetSession) {
      // Disable auto-save during critical operation
      autoSaveEnabled.value = false
      
      try {
        targetSession.isActive = false
        targetSession.endTime = Date.now()
        
        // DON'T clear currentSession - this keeps the window open
        // and allows for continued interaction with the completed session
        console.log(`🏁 Store: Session completed but remains accessible: ${targetSession.id}`)
        
        // Force immediate save when completing session to ensure persistence
        console.log('💾 Store: Force saving completed session with verification')
        await saveSessions(true) // Force immediate save
        
        console.log('🏁 Store: Session completion operation finished successfully')
        
      } catch (error) {
        console.error('🏁 Store: Failed to complete session properly:', error)
        throw error
      } finally {
        // Re-enable auto-save
        autoSaveEnabled.value = true
      }
    }
  }

  const switchToSession = (sessionId: string) => {
    const session = sessions.value.find(s => s.id === sessionId)
    if (session) {
      console.log('🔄 Store: Switching to session:', sessionId)
      
      // Simply deactivate current session without ending it (no endTime)
      if (currentSession.value) {
        console.log('🔄 Store: Deactivating current session:', currentSession.value.id)
        currentSession.value.isActive = false
      }
      
      // Activate the target session
      session.isActive = true
      currentSession.value = session
      console.log('🔄 Store: Session switched successfully')
    } else {
      console.error('🔄 Store: Session not found:', sessionId)
    }
  }

  // Resume/continue an existing conversation - reactivates it for new messages
  const resumeSession = async (sessionId: string) => {
    const session = sessions.value.find(s => s.id === sessionId)
    if (session) {
      console.log('▶️ Store: Resuming session for continuation:', sessionId)
      
      // Disable auto-save during critical operation
      autoSaveEnabled.value = false
      
      try {
        // Complete current session if there is one
        if (currentSession.value && currentSession.value.id !== sessionId) {
          console.log('🏁 Store: Completing current session before resume')
          await completeSession()
        }
        
        // Reactivate the target session
        session.isActive = true
        // Clear endTime to indicate it's active again
        session.endTime = undefined
        // Update the session name to show it's been resumed
        if (!session.name.includes('(Resumed)')) {
          session.name += ' (Resumed)'
        }
        
        currentSession.value = session
        console.log('▶️ Store: Session resumed successfully and ready for new messages')
        
        // Force immediate save to persist the resume state
        await saveSessions(true)
        
      } catch (error) {
        console.error('▶️ Store: Failed to resume session properly:', error)
        throw error
      } finally {
        // Re-enable auto-save
        autoSaveEnabled.value = true
      }
    } else {
      console.error('▶️ Store: Session not found for resume:', sessionId)
      throw new Error(`Session ${sessionId} not found`)
    }
  }

  const addMessage = (messageData: Omit<ConversationMessage, 'id'>) => {
    // Don't automatically create sessions - require explicit session management
    if (!currentSession.value) {
      console.error('❌ Attempting to add message without active session:', messageData)
      return null
    }

    // Add deduplication check to prevent duplicate messages
    const existingMessages = currentSession.value.messages || []
    const isDuplicate = existingMessages.some(msg => 
      msg.content === messageData.content && 
      msg.source === messageData.source &&
      Math.abs(msg.timestamp - (messageData.timestamp || Date.now())) < 1000 // Within 1 second
    )

    if (isDuplicate) {
      console.log('🚫 Skipping duplicate message:', messageData.content)
      return null
    }

    const message: ConversationMessage = {
      id: `msg_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`,
      ...messageData
    }

    currentSession.value.messages.push(message)
    
    // If this is a resumed session (has endTime), update it to show continued activity
    if (currentSession.value.endTime) {
      console.log('📝 Updating resumed session timestamp due to new message')
      currentSession.value.endTime = Date.now()
      
      // Add edit indicator to session name if not already present
      if (!currentSession.value.name.includes('(Edited)')) {
        currentSession.value.name = currentSession.value.name.replace(' (Resumed)', '') + ' (Edited)'
      }
    }
    
    console.log(`📝 Added message to session ${currentSession.value.id}:`, message.content.substring(0, 50))
    console.log(`📝 Session now has ${currentSession.value.messages.length} total messages`)
    
    // Force a save every 5 messages to ensure messages don't get lost
    if (currentSession.value.messages.length % 5 === 0) {
      console.log('💾 Store: Auto-saving due to message count milestone')
      saveSessions().catch(console.error)
    }
    
    return message
  }

  const updateMessage = (messageId: string, updates: Partial<ConversationMessage>) => {
    if (!currentSession.value) return null

    const messageIndex = currentSession.value.messages.findIndex(m => m.id === messageId)
    if (messageIndex !== -1) {
      currentSession.value.messages[messageIndex] = {
        ...currentSession.value.messages[messageIndex],
        ...updates
      }
      return currentSession.value.messages[messageIndex]
    }
    return null
  }

  const deleteMessage = (messageId: string) => {
    if (!currentSession.value) return false

    const messageIndex = currentSession.value.messages.findIndex(m => m.id === messageId)
    if (messageIndex !== -1) {
      currentSession.value.messages.splice(messageIndex, 1)
      return true
    }
    return false
  }

  const clearCurrentSession = () => {
    if (currentSession.value) {
      currentSession.value.messages = []
    }
  }

  const renameSession = async (sessionId: string, newName: string) => {
    if (!newName || !newName.trim()) {
      throw new Error('Session name cannot be empty')
    }
    
    try {
      console.log(`✏️ Store: Renaming session ${sessionId} to "${newName}"`)
      
      const session = sessions.value.find(s => s.id === sessionId)
      if (!session) {
        throw new Error(`Session ${sessionId} not found`)
      }
      
      // Disable auto-save during rename operation
      autoSaveEnabled.value = false
      
      try {
        const trimmedName = newName.trim()
        const oldName = session.name
        session.name = trimmedName
        console.log(`✏️ Store: Session renamed from "${oldName}" to "${trimmedName}": ${sessionId}`)
        
        // Force immediate save to persist the rename
        await saveSessions(true)
        console.log(`✏️ Store: Rename saved successfully`)
        
      } finally {
        // Re-enable auto-save
        autoSaveEnabled.value = true
      }
      
      return session
    } catch (error) {
      console.error('✏️ Store: Failed to rename conversation session:', error)
      throw error
    }
  }

  const deleteSession = async (sessionId: string) => {
    try {
      console.log(`🗑️ Store: Deleting session ${sessionId}`)
      await invoke('delete_conversation', { conversationId: sessionId })
      console.log(`🗑️ Store: Backend delete successful for ${sessionId}`)
      
      const sessionIndex = sessions.value.findIndex(s => s.id === sessionId)
      if (sessionIndex !== -1) {
        const deletedSession = sessions.value.splice(sessionIndex, 1)[0]
        console.log(`🗑️ Store: Removed session from array: ${sessionId}`)
        
        // If we deleted the current session, clear it
        if (currentSession.value?.id === sessionId) {
          currentSession.value = null
          console.log(`🗑️ Store: Cleared current session reference`)
        }
        
        console.log(`🗑️ Store: Deleted conversation session successfully: ${sessionId}`)
        return deletedSession
      } else {
        console.warn(`🗑️ Store: Session not found in array: ${sessionId}`)
      }
    } catch (error) {
      console.error('🗑️ Store: Failed to delete conversation session:', error)
      throw error // Re-throw to let caller handle it
    }
    return null
  }

  const setRecordingState = (recording: boolean) => {
    isRecording.value = recording
  }

  const setAudioLoopbackState = (active: boolean) => {
    isAudioLoopbackActive.value = active
  }

  // Export messages to main chat (will be used for sending selected messages to main chat)
  const exportMessagesToMainChat = (messageIds: string[]) => {
    if (!currentSession.value) return []

    const messagesToExport = currentSession.value.messages.filter(m => 
      messageIds.includes(m.id)
    )

    // Emit custom event that can be caught by main chat system
    const exportEvent = new CustomEvent('conversation-export-to-chat', {
      detail: {
        messages: messagesToExport,
        sessionId: currentSession.value.id,
        sessionName: currentSession.value.name
      }
    })
    window.dispatchEvent(exportEvent)

    return messagesToExport
  }

  // Get session statistics
  const getSessionStats = (sessionId?: string) => {
    const session = sessionId 
      ? sessions.value.find(s => s.id === sessionId)
      : currentSession.value

    if (!session) return null

    const microphoneMessages = session.messages.filter(m => m.source === 'microphone')
    const loopbackMessages = session.messages.filter(m => m.source === 'loopback')
    const userMessages = session.messages.filter(m => m.type === 'user')
    const systemMessages = session.messages.filter(m => m.type === 'system')
    const duration = session.endTime 
      ? session.endTime - session.startTime 
      : Date.now() - session.startTime

    const confidenceValues = session.messages
      .filter(m => m.confidence !== undefined)
      .map(m => m.confidence || 0)

    return {
      totalMessages: session.messages.length,
      userMessages: userMessages.length,
      systemMessages: systemMessages.length,
      microphoneMessages: microphoneMessages.length,
      loopbackMessages: loopbackMessages.length,
      duration: Math.round(duration / 1000), // in seconds
      averageConfidence: confidenceValues.length > 0 
        ? confidenceValues.reduce((sum, conf) => sum + conf, 0) / confidenceValues.length
        : 0,
      isActive: session.isActive,
      isCompleted: !!session.endTime,
      isResumed: session.name.includes('(Resumed)') || session.name.includes('(Edited)')
    }
  }
  
  // Get all conversation statistics for dashboard/debugging
  const getAllConversationStats = () => {
    return {
      totalConversations: sessions.value.length,
      activeConversations: sessions.value.filter(s => s.isActive).length,
      completedConversations: sessions.value.filter(s => s.endTime).length,
      totalMessages: sessions.value.reduce((sum, s) => sum + s.messages.length, 0),
      averageMessagesPerConversation: sessions.value.length > 0 
        ? sessions.value.reduce((sum, s) => sum + s.messages.length, 0) / sessions.value.length 
        : 0,
      longestConversation: sessions.value.length > 0 
        ? sessions.value.reduce((longest, current) => 
            current.messages.length > longest.messages.length ? current : longest)
        : null,
      oldestConversation: sessions.value.length > 0
        ? sessions.value.reduce((oldest, current) => 
            current.startTime < oldest.startTime ? current : oldest)
        : null,
      newestConversation: sessions.value.length > 0
        ? sessions.value.reduce((newest, current) => 
            current.startTime > newest.startTime ? current : newest)
        : null
    }
  }


  // Export session data for backup/sharing
  const exportSessionData = (sessionId?: string) => {
    const sessionToExport = sessionId 
      ? sessions.value.find(s => s.id === sessionId)
      : currentSession.value

    if (!sessionToExport) return null

    return {
      session: sessionToExport,
      exportedAt: Date.now(),
      version: '1.0'
    }
  }

  // Import session data from backup/sharing
  const importSessionData = (sessionData: any) => {
    try {
      if (!sessionData.session) throw new Error('Invalid session data')
      
      const session: ConversationSession = {
        ...sessionData.session,
        id: `imported_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`, // New ID to avoid conflicts
        isActive: false
      }

      sessions.value.push(session)
      return session
    } catch (error) {
      console.error('Failed to import session data:', error)
      return null
    }
  }

  // Clear all sessions (with confirmation)
  const clearAllSessions = async () => {
    try {
      await invoke('clear_all_conversations')
      sessions.value = []
      currentSession.value = null
      console.log('🗑️ Cleared all conversation sessions')
    } catch (error) {
      console.error('Failed to clear all conversation sessions:', error)
    }
  }

  // Get storage usage info
  const getStorageInfo = () => {
    const data = localStorage.getItem(STORAGE_KEY)
    const sizeBytes = data ? new Blob([data]).size : 0
    const sizeKB = Math.round(sizeBytes / 1024 * 100) / 100

    return {
      sessionCount: sessions.value.length,
      totalMessages: sessions.value.reduce((sum, s) => sum + s.messages.length, 0),
      storageSize: `${sizeKB} KB`,
      lastSaved: data ? 'Auto-saved' : 'Never'
    }
  }

  // Auto-save control methods
  const disableAutoSave = () => {
    autoSaveEnabled.value = false
    console.log('🔒 Store: Auto-save disabled')
  }
  
  const enableAutoSave = () => {
    autoSaveEnabled.value = true
    console.log('🔓 Store: Auto-save enabled')
  }
  
  const waitForSaveCompletion = async (timeoutMs = 5000) => {
    const startTime = Date.now()
    while (isSaving.value && Date.now() - startTime < timeoutMs) {
      await new Promise(resolve => setTimeout(resolve, 50))
    }
    if (isSaving.value) {
      throw new Error('Save operation did not complete within timeout')
    }
  }

  return {
    // State
    currentSession,
    sessions,
    isRecording,
    isAudioLoopbackActive,

    // Save state
    isSaving,
    pendingSave,

    // Computed
    currentMessages,
    activeSessions,
    recentSessions,

    // Actions
    createSession,
    endSession,
    completeSession,
    switchToSession,
    resumeSession,
    addMessage,
    updateMessage,
    deleteMessage,
    clearCurrentSession,
    renameSession,
    deleteSession,
    setRecordingState,
    setAudioLoopbackState,
    exportMessagesToMainChat,
    getSessionStats,
    getAllConversationStats,
    
    // Persistence actions
    loadSessions,
    saveSessions,
    exportSessionData,
    importSessionData,
    clearAllSessions,
    getStorageInfo,
    
    // Auto-save control
    disableAutoSave,
    enableAutoSave,
    waitForSaveCompletion
  }
})