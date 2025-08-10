<script setup lang="ts">
import { computed } from 'vue'
import { QueueListIcon } from '@heroicons/vue/24/outline'
import GenericSidebar from './GenericSidebar.vue'
import { transformChats } from '../../utils/sidebarTransformers'
import { useChatManagement } from '../../composables/useChatManagement'

interface Props {
  show: boolean
  selectedModel: string | null
}

interface Emits {
  (e: 'close'): void
  (e: 'new-chat'): void
  (e: 'switch-chat', chatId: string): void
  (e: 'delete-chat', chatId: string): void
  (e: 'clear-chat'): void
}

const props = defineProps<Props>()
const emit = defineEmits<Emits>()

// Dummy scroll function for chat management
const scrollChatToBottom = () => {}

// Get chat management functions
const {
  chatSessions,
  currentChatId,
  renameChat
} = useChatManagement(props.selectedModel, scrollChatToBottom)

// Transform chats to generic sidebar items
const sidebarItems = computed(() => transformChats(chatSessions.value))

// Event handlers that map generic events to chat-specific events
const handleNewItem = () => emit('new-chat')
const handleSelectItem = (chatId: string) => emit('switch-chat', chatId)
const handleDeleteItem = (chatId: string) => emit('delete-chat', chatId)
const handleRenameItem = (chatId: string, newTitle: string) => renameChat(chatId, newTitle)
const handleClearAll = () => emit('clear-chat')
</script>

<template>
  <GenericSidebar
    v-if="show"
    :items="sidebarItems"
    :current-item-id="currentChatId"
    :is-loading="false"
    title="Chat History"
    :icon="QueueListIcon"
    empty-message="No chat history"
    theme="blue"
    display-mode="inline"
    :show-new-button="true"
    :show-delete-button="true"
    :show-rename-button="true"
    :show-clear-all-button="true"
    :show-timestamps="true"
    :show-metadata="false"
    @close="emit('close')"
    @new-item="handleNewItem"
    @select-item="handleSelectItem"
    @delete-item="handleDeleteItem"
    @rename-item="handleRenameItem"
    @clear-all="handleClearAll"
  />
</template>