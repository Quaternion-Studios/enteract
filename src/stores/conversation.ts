import { defineStore } from 'pinia'
import { ref, computed, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useMessagePersistence } from '../composables/useMessagePersistence'

export interface ConversationMessage {
  id: string
  type: 'user' | 'system'
  source: 'microphone' | 'loopback'
  content: string
  timestamp: number
  confidence?: number
  isPreview?: boolean
  isTyping?: boolean
  persistenceState?: 'pending' | 'saving' | 'saved' | 'failed'
  retryCount?: number
  lastSaveAttempt?: number
  saveError?: string
}

export interface ConversationInsight {
  id: string
  text: string
  timestamp: number
  contextLength: number
  type: 'insight' | 'welcome' | 'question' | 'answer'
}

export interface ConversationSession {
  id: string
  name: string
  startTime: number
  endTime?: number
  messages: ConversationMessage[]
  isActive: boolean
  insights: ConversationInsight[]
}

export const useConversationStore = defineStore('conversation', () => {
  // Initialize message persistence
  const messagePersistence = useMessagePersistence()
  
  // State
  const currentSession = ref<ConversationSession | null>(null)
  const sessions = ref<ConversationSession[]>([])
  const isRecording = ref(false)
  const isAudioLoopbackActive = ref(false)

  // Persistence key
  const STORAGE_KEY = 'conversation-sessions'

  // Load sessions from SQLite backend
  const loadSessions = async () => {
    try {
      console.log('📁 Store: Loading conversations from SQLite backend...')
      const response = await invoke<{conversations: ConversationSession[]}>('load_conversations')
      
      // Ensure all sessions have insights field
      sessions.value = response.conversations.map(session => ({
        ...session,
        insights: session.insights || [] // Add empty insights array if missing
      }))
      
      console.log(`📁 Store: Successfully loaded ${sessions.value.length} conversation sessions:`, 
        sessions.value.map(s => ({ 
          id: s.id, 
          name: s.name, 
          messageCount: s.messages.length, 
          insightCount: s.insights?.length || 0 
        })))
    } catch (error) {
      console.error('📁 Store: Failed to load conversation sessions:', error)
      // Initialize empty state - no localStorage fallback
      sessions.value = []
    }
  }

  // Save sessions to SQLite backend with better error handling
  const saveSessions = async (forceImmediate = false) => {
    if (isSaving.value && !forceImmediate) {
      console.log('💾 Store: Save already in progress, will queue this save')
      pendingSave.value = true
      return
    }

    const saveId = `save_${Date.now()}`
    try {
      isSaving.value = true
      pendingSave.value = false
      
      console.log(`💾 Store: [${saveId}] Saving ${sessions.value.length} conversation sessions to SQLite...`)
      
      await invoke('save_conversations', {
        payload: { conversations: sessions.value }
      })
      console.log(`💾 Store: [${saveId}] Successfully saved ${sessions.value.length} conversation sessions`)
      
    } catch (error) {
      console.error(`💾 Store: [${saveId}] Failed to save conversation sessions:`, error)
      // Don't throw error unless it's a critical failure
      // Most save errors should be recoverable
    } finally {
      isSaving.value = false
      
      // If there was a pending save, execute it now with a delay
      if (pendingSave.value) {
        console.log(`💾 Store: [${saveId}] Executing queued save`)
        setTimeout(() => saveSessions().catch(console.error), 500) // Increased delay
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
        isActive: true,
        insights: []
      }

      // Deactivate any existing current session
      if (currentSession.value) {
        currentSession.value.isActive = false
        console.log('🆕 Store: Deactivated previous session')
      }

      sessions.value.push(session)
      currentSession.value = session
      console.log('🆕 Store: Session created successfully:', session.id)
      
      // For new sessions, use the existing save method which handles creation
      await saveSessions(true) // Force immediate save for new session creation
      console.log('✅ Store: New session created and saved')
      
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
        
        // Use incremental update instead of full save to avoid race conditions
        await invoke('update_session_metadata', {
          sessionId: targetSession.id,
          name: null, // Don't update name
          endTime: targetSession.endTime,
          isActive: targetSession.isActive
        })
        console.log('✅ Store: Session completion state updated incrementally')
        
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

  const switchToSession = async (sessionId: string) => {
    const session = sessions.value.find(s => s.id === sessionId)
    if (session) {
      console.log('🔄 Store: Switching to session for viewing:', sessionId)
      
      try {
        // Deactivate current session without ending it (no endTime)
        if (currentSession.value) {
          console.log('🔄 Store: Deactivating current session:', currentSession.value.id)
          currentSession.value.isActive = false
          
          // Update database incrementally
          await invoke('update_session_active_state', {
            sessionId: currentSession.value.id,
            isActive: false
          })
        }
        
        // Set as current session but don't activate it yet (viewing mode)
        // The session will be properly activated when recording starts
        currentSession.value = session
        console.log('🔄 Store: Session switched for viewing (not yet active for recording)')
      } catch (error) {
        console.error('🔄 Store: Failed to update session active state:', error)
        // Continue anyway since the UI state change is the priority
        currentSession.value = session
      }
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
        // Complete current session if there is one and it's different
        if (currentSession.value && currentSession.value.id !== sessionId) {
          console.log('🏁 Store: Completing current session before resume')
          await completeSession()
        }
        
        // Reactivate the target session
        session.isActive = true
        // Clear endTime to indicate it's active again (handles completed sessions)
        if (session.endTime) {
          session.endTime = undefined
          // Update the session name to show it's been resumed only if it was completed
          if (!session.name.includes('(Resumed)')) {
            session.name += ' (Resumed)'
          }
        }
        
        currentSession.value = session
        console.log('▶️ Store: Session activated and ready for recording new messages')
        
        // Use incremental session update instead of full save to avoid race conditions
        await invoke('update_session_metadata', {
          sessionId: session.id,
          name: session.name,
          endTime: session.endTime,
          isActive: session.isActive
        })
        console.log('✅ Store: Session metadata updated incrementally')
        
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

  const addMessage = async (messageData: Omit<ConversationMessage, 'id'>) => {
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
      persistenceState: 'pending',
      ...messageData
    }

    // Add message to local state immediately
    currentSession.value.messages.push(message)
    
    // Get reference to the message that's now in the reactive store
    const storeMessage = currentSession.value.messages.find(m => m.id === message.id)
    if (!storeMessage) {
      console.error(`❌ Store: Could not find message ${message.id} in store after adding`)
      return null
    }
    
    // Save message directly to backend (no dual system)
    try {
      // Don't set 'saving' state here - let saveMessageImmediately handle it
      console.log(`🔄 Store: Starting save process for message: ${message.id}`)
      console.log(`📋 Store: Initial message state: ${storeMessage.persistenceState}`)
      console.log(`🔍 Store: Working with store message reference:`, storeMessage === message)
      
      const success = await messagePersistence.saveMessageImmediately(storeMessage, currentSession.value.id)
      
      console.log(`📋 Store: After save attempt - message state: ${storeMessage.persistenceState}`)
      console.log(`📋 Store: Save success result: ${success}`)
      
      if (success) {
        console.log(`✅ Store: Message saved immediately: ${storeMessage.id}`)
        // Double-check the state is properly set (should already be done by persistence system)
        if (storeMessage.persistenceState !== 'saved') {
          console.log(`🔧 Store: Force-updating store message state to 'saved' for ${storeMessage.id}`)
          storeMessage.persistenceState = 'saved'
          storeMessage.retryCount = 0
          storeMessage.saveError = undefined
        }
      } else {
        console.log(`❌ Store: Message save failed: ${storeMessage.id}`)
        // Double-check the state is properly set (should already be done by persistence system)
        if (storeMessage.persistenceState !== 'failed') {
          console.log(`🔧 Store: Force-updating store message state to 'failed' for ${storeMessage.id}`)
          storeMessage.persistenceState = 'failed'
        }
      }
    } catch (error) {
      console.error(`❌ Store: Unexpected error during save: ${storeMessage.id}`, error)
      storeMessage.persistenceState = 'failed'
      storeMessage.saveError = error instanceof Error ? error.message : 'Unknown error'
    }
    
    // If this is a resumed session (has endTime), update it to show continued activity
    if (currentSession.value.endTime) {
      console.log('📝 Updating resumed session timestamp due to new message')
      currentSession.value.endTime = Date.now()
      
      // Add edit indicator to session name if not already present
      if (!currentSession.value.name.includes('(Edited)')) {
        currentSession.value.name = currentSession.value.name.replace(' (Resumed)', '') + ' (Edited)'
      }
    }
    
    console.log(`📝 Added message to session ${currentSession.value.id}:`, storeMessage.content.substring(0, 50))
    console.log(`📝 Session now has ${currentSession.value.messages.length} total messages`)
    console.log(`📋 Final message state before return: ${storeMessage.persistenceState}`)
    
    return storeMessage
  }

  const updateMessage = async (messageId: string, updates: Partial<ConversationMessage>) => {
    if (!currentSession.value) return null

    const messageIndex = currentSession.value.messages.findIndex(m => m.id === messageId)
    if (messageIndex !== -1) {
      currentSession.value.messages[messageIndex] = {
        ...currentSession.value.messages[messageIndex],
        ...updates
      }
      
      // Update in backend if message was already saved
      const message = currentSession.value.messages[messageIndex]
      if (message.persistenceState === 'saved') {
        await messagePersistence.updateMessage(messageId, currentSession.value.id, updates)
      }
      
      return message
    }
    return null
  }

  const deleteMessage = async (messageId: string) => {
    if (!currentSession.value) return false

    const messageIndex = currentSession.value.messages.findIndex(m => m.id === messageId)
    if (messageIndex !== -1) {
      const message = currentSession.value.messages[messageIndex]
      
      // Delete from backend if message was saved
      if (message.persistenceState === 'saved') {
        await messagePersistence.deleteMessage(messageId, currentSession.value.id)
      }
      
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
        
        // Use incremental update to persist the rename
        await invoke('update_session_metadata', {
          sessionId: sessionId,
          name: trimmedName,
          endTime: null, // Don't update end time
          isActive: null // Don't update active state
        })
        console.log(`✏️ Store: Rename saved incrementally`)
        
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
    return {
      sessionCount: sessions.value.length,
      totalMessages: sessions.value.reduce((sum, s) => sum + s.messages.length, 0),
      storageSize: 'SQLite Database',
      lastSaved: 'Auto-saved to SQLite'
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

  // Insight management functions
  const addInsight = async (insight: ConversationInsight) => {
    if (!currentSession.value) {
      console.warn('No active session to add insight to')
      return
    }

    try {
      // Add to current session
      currentSession.value.insights.push(insight)
      
      // Save to backend
      await invoke('save_conversation_insight', {
        sessionId: currentSession.value.id,
        insight: {
          id: insight.id,
          text: insight.text,
          timestamp: insight.timestamp,
          contextLength: insight.contextLength,
          insightType: insight.type
        }
      })
      
      console.log('💡 Added insight to session:', insight.id)
    } catch (error) {
      console.error('Failed to save insight:', error)
    }
  }

  const getInsightsForSession = async (sessionId: string): Promise<ConversationInsight[]> => {
    try {
      const insights = await invoke<ConversationInsight[]>('get_conversation_insights', {
        sessionId
      })
      return insights
    } catch (error) {
      console.error('Failed to load insights for session:', sessionId, error)
      return []
    }
  }

  const loadCurrentSessionInsights = async () => {
    if (!currentSession.value) return

    try {
      const insights = await getInsightsForSession(currentSession.value.id)
      currentSession.value.insights = insights
      console.log(`💡 Loaded ${insights.length} insights for current session`)
    } catch (error) {
      console.error('Failed to load insights for current session:', error)
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
    waitForSaveCompletion,
    
    // Insight management
    addInsight,
    getInsightsForSession,
    loadCurrentSessionInsights,
    
    // Message persistence
    getMessagePersistenceStatus: () => messagePersistence.getQueueStatus(),
    messagePersistence
  }
})