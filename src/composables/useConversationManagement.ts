import { ref, watch } from 'vue'
import { useConversationStore } from '../stores/conversation'

interface Conversation {
  id: string
  name: string
  startTime: number
  endTime?: number
  messages: any[]
  isActive?: boolean
}

export function useConversationManagement() {
  const conversationStore = useConversationStore()
  
  const allConversations = ref<Conversation[]>([])
  const isLoadingConversations = ref(false)
  
  // Watch for changes in the store's sessions and update allConversations
  watch(() => conversationStore.sessions, (newSessions) => {
    console.log(`📁 ConversationManagement: Store sessions changed:`, newSessions.length, newSessions.map(s => ({ id: s.id, name: s.name, messageCount: s.messages.length, isActive: s.isActive })))
    
    allConversations.value = newSessions.map(session => ({
      id: session.id,
      name: session.name,
      startTime: session.startTime,
      endTime: session.endTime,
      messages: session.messages,
      isActive: session.isActive
    }))
    
    console.log(`📁 ConversationManagement: Updated conversations array:`, allConversations.value.length, allConversations.value.map(c => ({ id: c.id, name: c.name, messageCount: c.messages.length })))
  }, { deep: true, immediate: true })
  
  const loadConversations = async () => {
    isLoadingConversations.value = true
    try {
      // Load sessions from the conversation store
      await conversationStore.loadSessions()
      console.log(`📁 ConversationManagement: Loaded ${conversationStore.sessions.length} conversations from store`)
      // The watcher will automatically update allConversations.value
    } catch (error) {
      console.error('Failed to load conversations:', error)
    } finally {
      isLoadingConversations.value = false
    }
  }
  
  const createNewConversation = async () => {
    try {
      // Create new conversation through session creation
      console.log('🆕 ConversationManagement: Creating new session')
      const session = conversationStore.createSession()
      console.log('🆕 ConversationManagement: Created new session:', session.id)
      
      // Wait for the save to complete before loading conversations
      await conversationStore.waitForSaveCompletion()
      console.log('🆕 ConversationManagement: Session save completed')
      
      await loadConversations()
      console.log('🆕 ConversationManagement: Conversations reloaded after new session creation')
    } catch (error) {
      console.error('Failed to create new conversation:', error)
    }
  }
  
  const resumeConversation = async (conversationId: string, forEditing = false) => {
    try {
      if (forEditing) {
        // Resume session for editing/adding new messages
        console.log('▶️ ConversationManagement: Resuming session for editing:', conversationId)
        await conversationStore.resumeSession(conversationId)
        console.log('▶️ ConversationManagement: Session resumed for editing:', conversationId)
      } else {
        // Just switch to view the session
        console.log('🔄 ConversationManagement: Switching to session for viewing:', conversationId)
        conversationStore.switchToSession(conversationId)
        console.log('🔄 ConversationManagement: Switched to session:', conversationId)
      }
      
      // Wait for any pending saves to complete
      await conversationStore.waitForSaveCompletion()
      
      await loadConversations()
      console.log('🔄 ConversationManagement: Conversations reloaded after session operation')
    } catch (error) {
      console.error('Failed to resume conversation:', error)
    }
  }
  
  const deleteConversation = async (conversationId: string) => {
    try {
      // Delete the session from the store
      console.log('🗑️ ConversationManagement: Deleting session:', conversationId)
      await conversationStore.deleteSession(conversationId)
      console.log('🗑️ ConversationManagement: Deleted session:', conversationId)
      
      // Wait for any pending saves to complete
      await conversationStore.waitForSaveCompletion()
      
      await loadConversations()
      console.log('🗑️ ConversationManagement: Conversations reloaded after deletion')
    } catch (error) {
      console.error('Failed to delete conversation:', error)
    }
  }
  
  return {
    allConversations,
    isLoadingConversations,
    loadConversations,
    createNewConversation,
    resumeConversation,
    deleteConversation
  }
}