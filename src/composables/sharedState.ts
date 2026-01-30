// sharedState.ts - Shared singleton state
import { ref } from 'vue'
import type { ChatSession } from '../types'

// Create shared singleton state that persists across all component instances
export const sharedChatState = {
  chatSessions: ref<ChatSession[]>([]),
  currentChatId: ref<string | null>(null),
  isInitialized: ref(false)
}