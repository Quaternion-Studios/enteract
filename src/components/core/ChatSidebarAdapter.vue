<script setup lang="ts">
import { computed } from 'vue'
import { ChatBubbleLeftRightIcon } from '@heroicons/vue/24/outline'
import GenericSidebar from './GenericSidebar.vue'
import { transformChats } from '../../utils/sidebarTransformers'
import { useChatManagement } from '../../composables/useChatManagement'

interface Props {
  selectedModel: string | null
  isOpen: boolean
}

interface Emits {
  (e: 'close'): void
  (e: 'open-chat-window'): void
}

const props = defineProps<Props>()
const emit = defineEmits<Emits>()

// Dummy scroll function for chat management
const scrollChatToBottom = () => {}

// Get chat management functions
const {
  chatSessions,
  currentChatId,
  createNewChat,
  switchChat,
  deleteChat,
  renameChat,
  clearChat
} = useChatManagement(props.selectedModel, scrollChatToBottom)

// Transform chats to generic sidebar items
const sidebarItems = computed(() => transformChats(chatSessions.value))

// Event handlers
const handleNewItem = () => {
  createNewChat()
  emit('open-chat-window')
  emit('close')
}

const handleSelectItem = (chatId: string) => {
  switchChat(chatId)
  emit('close')
}

const handleDeleteItem = (chatId: string) => {
  const wasCurrentChat = chatId === currentChatId.value
  deleteChat(chatId)
  
  if (wasCurrentChat) {
    emit('open-chat-window')
  }
}

const handleRenameItem = (chatId: string, newTitle: string) => {
  renameChat(chatId, newTitle)
}

const handleClearAll = () => {
  // This would clear all chats, but the original doesn't have this feature
  // It only clears the current chat
  clearChat()
}
</script>

<template>
  <GenericSidebar
    v-if="isOpen"
    :items="sidebarItems"
    :current-item-id="currentChatId"
    :is-loading="false"
    title="Chat Sessions"
    :icon="ChatBubbleLeftRightIcon"
    empty-message="No chat sessions yet"
    empty-sub-message="Create your first chat to get started"
    theme="blue"
    display-mode="overlay"
    :show-new-button="true"
    :show-delete-button="true"
    :show-rename-button="true"
    :show-clear-all-button="false"
    :show-timestamps="true"
    :show-metadata="true"
    @close="emit('close')"
    @new-item="handleNewItem"
    @select-item="handleSelectItem"
    @delete-item="handleDeleteItem"
    @rename-item="handleRenameItem"
    @clear-all="handleClearAll"
  />
</template>