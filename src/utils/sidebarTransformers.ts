import type { SidebarItem } from '../components/core/GenericSidebar.vue'

// Conversation data structure from ConversationSidebar
export interface Conversation {
  id: string
  name: string
  startTime: number
  endTime?: number
  messages: any[]
  isActive?: boolean
}

// Chat data structure from ChatSidebar/ChatWindowSidebar
export interface ChatSession {
  id: string
  title: string
  updatedAt: string
  history: any[]
}

// Transform conversation to generic sidebar item
export function conversationToSidebarItem(conversation: Conversation): SidebarItem {
  const messageCount = conversation.messages.length
  const userMessages = conversation.messages.filter(m => m.type === 'user').length
  const systemMessages = conversation.messages.filter(m => m.type === 'system').length
  
  // Get last message preview
  let preview = 'No messages'
  if (conversation.messages.length > 0) {
    const lastMessage = conversation.messages[conversation.messages.length - 1]
    preview = lastMessage.content.substring(0, 100)
    if (preview.length < lastMessage.content.length) {
      preview += '...'
    }
  }
  
  // Calculate duration
  let duration = 'Ongoing'
  if (conversation.endTime) {
    const durationMs = conversation.endTime - conversation.startTime
    const durationSecs = Math.floor(durationMs / 1000)
    const durationMins = Math.floor(durationSecs / 60)
    
    if (durationMins < 1) {
      duration = `${durationSecs}s`
    } else if (durationMins < 60) {
      duration = `${durationMins}m`
    } else {
      const hours = Math.floor(durationMins / 60)
      const mins = durationMins % 60
      duration = `${hours}h ${mins}m`
    }
  }
  
  return {
    id: conversation.id,
    title: conversation.name,
    subtitle: `${messageCount} messages (${userMessages} user, ${systemMessages} system)`,
    timestamp: conversation.startTime,
    metadata: {
      duration,
      preview,
      messageCount,
      userMessages,
      systemMessages
    },
    isActive: conversation.isActive
  }
}

// Transform chat session to generic sidebar item
export function chatToSidebarItem(chat: ChatSession): SidebarItem {
  return {
    id: chat.id,
    title: chat.title,
    subtitle: `${chat.history.length} messages`,
    timestamp: chat.updatedAt,
    metadata: {
      count: chat.history.length
    }
  }
}

// Batch transform functions
export function transformConversations(conversations: Conversation[]): SidebarItem[] {
  return conversations.map(conversationToSidebarItem)
}

export function transformChats(chats: ChatSession[]): SidebarItem[] {
  return chats.map(chatToSidebarItem)
}