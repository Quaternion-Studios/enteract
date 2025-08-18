import { ref, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { ConversationMessage, ConversationSession } from '../stores/conversation'

interface MessageSaveRequest {
  message: ConversationMessage
  sessionId: string
  retryCount: number
}

interface MessageSaveResult {
  success: boolean
  messageId: string
  error?: string
}

interface MessageBatch {
  sessionId: string
  messages: ConversationMessage[]
}

export function useMessagePersistence() {
  // State
  const pendingQueue = ref<MessageSaveRequest[]>([])
  const failedQueue = ref<MessageSaveRequest[]>([])
  const isSaving = ref(false)
  const concurrentSaves = ref(0)
  const saveStats = ref({
    totalSaved: 0,
    totalFailed: 0,
    lastSaveTime: 0,
    averageSaveTime: 0
  })

  // Configuration  
  const MAX_RETRY_COUNT = 3
  const RETRY_DELAY_BASE = 1000 // Base delay in ms
  const BATCH_SIZE = 5 // Reduced batch size for better reliability
  const SAVE_DEBOUNCE_MS = 200 // Faster debounce for better responsiveness
  const OFFLINE_CHECK_INTERVAL = 5000
  const MAX_CONCURRENT_SAVES = 2 // Limit concurrent operations

  // Timers
  let saveTimer: number | null = null
  let retryTimer: number | null = null
  let offlineCheckTimer: number | null = null

  // Online/offline detection
  const isOnline = ref(true)
  
  const checkOnlineStatus = async () => {
    try {
      // Try to invoke a simple Rust command to check connectivity
      await invoke('ping_backend')
      isOnline.value = true
    } catch {
      isOnline.value = false
    }
  }

  // Start periodic online status check
  const startOnlineMonitoring = () => {
    offlineCheckTimer = window.setInterval(() => {
      checkOnlineStatus()
    }, OFFLINE_CHECK_INTERVAL)
    
    // Also listen to browser online/offline events
    window.addEventListener('online', () => {
      isOnline.value = true
      processFailedQueue() // Retry failed messages when back online
    })
    
    window.addEventListener('offline', () => {
      isOnline.value = false
    })
  }

  // Save individual message to backend
  const saveMessageToBackend = async (
    message: ConversationMessage, 
    sessionId: string
  ): Promise<MessageSaveResult> => {
    try {
      const startTime = Date.now()
      
      // Debug logging
      console.log('📤 Sending message to backend:', {
        sessionId: sessionId,
        messageId: message.id,
        messageType: message.type
      })
      
      await invoke('save_conversation_message', {
        sessionId: sessionId, // Tauri expects camelCase and converts to snake_case
        message: {
          id: message.id,
          type: message.type, // Serde expects 'type' due to rename attribute
          source: message.source,
          content: message.content,
          timestamp: message.timestamp,
          confidence: message.confidence,
          // Optional fields with correct naming for serde
          isPreview: message.isPreview || false,
          isTyping: message.isTyping || false,
          persistenceState: message.persistenceState,
          retryCount: message.retryCount || 0,
          lastSaveAttempt: message.lastSaveAttempt,
          saveError: message.saveError
        }
      })
      
      const saveTime = Date.now() - startTime
      updateSaveStats(true, saveTime)
      
      return {
        success: true,
        messageId: message.id
      }
    } catch (error) {
      console.error('❌ saveMessageToBackend error:', error)
      updateSaveStats(false)
      return {
        success: false,
        messageId: message.id,
        error: error instanceof Error ? error.message : 'Unknown error'
      }
    }
  }

  // Batch save messages for efficiency
  const batchSaveMessages = async (batch: MessageBatch): Promise<void> => {
    try {
      const startTime = Date.now()
      
      await invoke('batch_save_conversation_messages', {
        sessionId: batch.sessionId, // Tauri expects camelCase and converts to snake_case
        messages: batch.messages.map(msg => ({
          id: msg.id,
          type: msg.type, // Serde expects 'type' due to rename attribute
          source: msg.source,
          content: msg.content,
          timestamp: msg.timestamp,
          confidence: msg.confidence,
          // Optional fields with correct naming for serde
          isPreview: msg.isPreview || false,
          isTyping: msg.isTyping || false,
          persistenceState: msg.persistenceState,
          retryCount: msg.retryCount || 0,
          lastSaveAttempt: msg.lastSaveAttempt,
          saveError: msg.saveError
        }))
      })
      
      const saveTime = Date.now() - startTime
      batch.messages.forEach(() => updateSaveStats(true, saveTime / batch.messages.length))
      
      // Mark all messages in batch as saved
      batch.messages.forEach(msg => {
        msg.persistenceState = 'saved'
        msg.retryCount = 0
        msg.saveError = undefined
      })
    } catch (error) {
      batch.messages.forEach(() => updateSaveStats(false))
      
      // Mark all messages in batch as failed
      batch.messages.forEach(msg => {
        msg.persistenceState = 'failed'
        msg.saveError = error instanceof Error ? error.message : 'Unknown error'
      })
      
      throw error
    }
  }

  // Update save statistics
  const updateSaveStats = (success: boolean, saveTime?: number) => {
    if (success) {
      saveStats.value.totalSaved++
      saveStats.value.lastSaveTime = Date.now()
      if (saveTime) {
        const currentAvg = saveStats.value.averageSaveTime
        const totalSaves = saveStats.value.totalSaved
        saveStats.value.averageSaveTime = 
          (currentAvg * (totalSaves - 1) + saveTime) / totalSaves
      }
    } else {
      saveStats.value.totalFailed++
    }
  }

  // Queue message for saving
  const queueMessage = (message: ConversationMessage, sessionId: string) => {
    // Don't queue preview or typing messages
    if (message.isPreview || message.isTyping) {
      return
    }

    // Check if message is already queued
    const existingIndex = pendingQueue.value.findIndex(
      req => req.message.id === message.id
    )
    
    if (existingIndex === -1) {
      // Mark message as pending
      message.persistenceState = 'pending'
      message.retryCount = 0
      
      pendingQueue.value.push({
        message,
        sessionId,
        retryCount: 0
      })
      
      // Debounce save operation
      if (saveTimer) {
        clearTimeout(saveTimer)
      }
      
      saveTimer = window.setTimeout(() => {
        processPendingQueue()
      }, SAVE_DEBOUNCE_MS)
    }
  }

  // Process pending message queue with concurrency control
  const processPendingQueue = async () => {
    if (concurrentSaves.value >= MAX_CONCURRENT_SAVES || pendingQueue.value.length === 0) {
      return
    }
    
    if (!isOnline.value) {
      console.log('📴 Offline - deferring message saves')
      return
    }
    
    concurrentSaves.value++
    isSaving.value = true
    
    try {
      // Group messages by session for batch processing
      const messagesBySession = new Map<string, MessageSaveRequest[]>()
      
      // Take up to BATCH_SIZE messages from queue
      const batch = pendingQueue.value.splice(0, BATCH_SIZE)
      
      batch.forEach(req => {
        const sessionMessages = messagesBySession.get(req.sessionId) || []
        sessionMessages.push(req)
        messagesBySession.set(req.sessionId, sessionMessages)
      })
      
      // Process each session's messages
      for (const [sessionId, requests] of messagesBySession) {
        const messages = requests.map(req => {
          req.message.persistenceState = 'saving'
          return req.message
        })
        
        try {
          if (messages.length === 1) {
            // Single message - save individually
            const result = await saveMessageToBackend(messages[0], sessionId)
            
            if (result.success) {
              messages[0].persistenceState = 'saved'
              messages[0].retryCount = 0
              messages[0].saveError = undefined
              console.log(`✅ Saved message: ${messages[0].id}`)
            } else {
              throw new Error(result.error)
            }
          } else {
            // Multiple messages - batch save
            await batchSaveMessages({ sessionId, messages })
            console.log(`✅ Batch saved ${messages.length} messages`)
          }
        } catch (error) {
          const errorMessage = error instanceof Error ? error.message : 'Unknown error'
          console.error('❌ Failed to save messages:', errorMessage)
          
          // Move failed messages to retry queue
          requests.forEach(req => {
            req.retryCount++
            req.message.persistenceState = 'failed'
            req.message.retryCount = req.retryCount
            req.message.lastSaveAttempt = Date.now()
            req.message.saveError = errorMessage
            
            // Emit error event for UI feedback
            window.dispatchEvent(new CustomEvent('message-save-error', {
              detail: {
                messageId: req.message.id,
                error: errorMessage,
                retryCount: req.message.retryCount
              }
            }))
            
            if (req.retryCount < MAX_RETRY_COUNT) {
              failedQueue.value.push(req)
            } else {
              console.error(`🚫 Message ${req.message.id} exceeded max retry count`)
              req.message.saveError = 'Max retries exceeded'
              
              // Emit final failure event
              window.dispatchEvent(new CustomEvent('message-save-final-failure', {
                detail: {
                  messageId: req.message.id,
                  error: errorMessage
                }
              }))
            }
          })
        }
      }
      
      // Process remaining messages in queue
      if (pendingQueue.value.length > 0) {
        setTimeout(() => processPendingQueue(), 100)
      }
    } finally {
      concurrentSaves.value--
      isSaving.value = concurrentSaves.value > 0
    }
  }

  // Process failed message queue with exponential backoff
  const processFailedQueue = async () => {
    if (failedQueue.value.length === 0 || !isOnline.value) {
      return
    }
    
    const now = Date.now()
    const readyToRetry: MessageSaveRequest[] = []
    const stillWaiting: MessageSaveRequest[] = []
    
    failedQueue.value.forEach(req => {
      const timeSinceLastAttempt = now - (req.message.lastSaveAttempt || 0)
      const retryDelay = RETRY_DELAY_BASE * Math.pow(2, req.retryCount - 1)
      
      if (timeSinceLastAttempt >= retryDelay) {
        readyToRetry.push(req)
      } else {
        stillWaiting.push(req)
      }
    })
    
    failedQueue.value = stillWaiting
    
    // Re-queue messages that are ready for retry
    readyToRetry.forEach(req => {
      console.log(`🔄 Retrying message ${req.message.id} (attempt ${req.retryCount + 1})`)
      pendingQueue.value.push(req)
    })
    
    if (readyToRetry.length > 0) {
      processPendingQueue()
    }
    
    // Schedule next retry check
    if (failedQueue.value.length > 0) {
      if (retryTimer) {
        clearTimeout(retryTimer)
      }
      retryTimer = window.setTimeout(() => {
        processFailedQueue()
      }, RETRY_DELAY_BASE)
    }
  }

  // Save message immediately (high priority)
  const saveMessageImmediately = async (
    message: ConversationMessage, 
    sessionId: string
  ): Promise<boolean> => {
    if (!isOnline.value) {
      queueMessage(message, sessionId)
      return false
    }
    
    message.persistenceState = 'saving'
    const result = await saveMessageToBackend(message, sessionId)
    
    if (result.success) {
      message.persistenceState = 'saved'
      message.retryCount = 0
      message.saveError = undefined
      return true
    } else {
      message.persistenceState = 'failed'
      message.saveError = result.error
      message.lastSaveAttempt = Date.now()
      
      // Queue for retry
      failedQueue.value.push({
        message,
        sessionId,
        retryCount: 1
      })
      
      processFailedQueue()
      return false
    }
  }

  // Update existing message
  const updateMessage = async (
    messageId: string,
    sessionId: string,
    updates: Partial<ConversationMessage>
  ): Promise<boolean> => {
    try {
      await invoke('update_conversation_message', {
        sessionId: sessionId, // Tauri expects camelCase and converts to snake_case
        messageId: messageId, // Tauri expects camelCase and converts to snake_case
        updates: {
          content: updates.content,
          confidence: updates.confidence,
          timestamp: updates.timestamp
        }
      })
      return true
    } catch (error) {
      console.error('Failed to update message:', error)
      return false
    }
  }

  // Delete message
  const deleteMessage = async (
    messageId: string,
    sessionId: string
  ): Promise<boolean> => {
    try {
      await invoke('delete_conversation_message', {
        sessionId: sessionId, // Tauri expects camelCase and converts to snake_case
        messageId: messageId  // Tauri expects camelCase and converts to snake_case
      })
      return true
    } catch (error) {
      console.error('Failed to delete message:', error)
      return false
    }
  }

  // Get queue status
  const getQueueStatus = () => ({
    pendingCount: pendingQueue.value.length,
    failedCount: failedQueue.value.length,
    isSaving: isSaving.value,
    isOnline: isOnline.value,
    stats: saveStats.value
  })

  // Clear all queues
  const clearQueues = () => {
    pendingQueue.value = []
    failedQueue.value = []
    if (saveTimer) {
      clearTimeout(saveTimer)
      saveTimer = null
    }
    if (retryTimer) {
      clearTimeout(retryTimer)
      retryTimer = null
    }
  }

  // Cleanup
  const cleanup = () => {
    clearQueues()
    if (offlineCheckTimer) {
      clearInterval(offlineCheckTimer)
      offlineCheckTimer = null
    }
    window.removeEventListener('online', () => {})
    window.removeEventListener('offline', () => {})
  }

  // Start monitoring
  startOnlineMonitoring()

  return {
    queueMessage,
    saveMessageImmediately,
    updateMessage,
    deleteMessage,
    processPendingQueue,
    processFailedQueue,
    getQueueStatus,
    clearQueues,
    cleanup,
    isOnline,
    isSaving,
    saveStats
  }
}