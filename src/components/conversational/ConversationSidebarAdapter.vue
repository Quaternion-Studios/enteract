<script setup lang="ts">
import { computed } from 'vue'
import { QueueListIcon } from '@heroicons/vue/24/outline'
import GenericSidebar from '../core/GenericSidebar.vue'
import { transformConversations } from '../../utils/sidebarTransformers'
import type { Conversation } from '../../utils/sidebarTransformers'

interface Props {
  show: boolean
  conversations: Conversation[]
  isLoading?: boolean
}

interface Emits {
  (e: 'close'): void
  (e: 'new-conversation'): void
  (e: 'resume-conversation', id: string): void
  (e: 'delete-conversation', id: string): void
}

const props = defineProps<Props>()
const emit = defineEmits<Emits>()

// Transform conversations to generic sidebar items
const sidebarItems = computed(() => transformConversations(props.conversations))

// Find currently active conversation
const currentConversationId = computed(() => {
  const activeConversation = props.conversations.find(c => c.isActive)
  return activeConversation?.id || null
})

// Event handlers that map generic events to conversation-specific events
const handleNewItem = () => emit('new-conversation')
const handleSelectItem = (id: string) => emit('resume-conversation', id)
const handleDeleteItem = (id: string) => emit('delete-conversation', id)
const handleRenameItem = (id: string, newTitle: string) => {
  // Rename functionality could be added here if needed
  console.log('Rename conversation:', id, newTitle)
}
</script>

<template>
  <GenericSidebar
    v-if="show"
    :items="sidebarItems"
    :current-item-id="currentConversationId"
    :is-loading="isLoading"
    title="Conversations"
    :icon="QueueListIcon"
    empty-message="No conversations yet"
    theme="purple"
    display-mode="inline"
    :show-new-button="true"
    :show-delete-button="true"
    :show-rename-button="false"
    :show-clear-all-button="false"
    :show-timestamps="true"
    :show-metadata="true"
    @close="emit('close')"
    @new-item="handleNewItem"
    @select-item="handleSelectItem"
    @delete-item="handleDeleteItem"
    @rename-item="handleRenameItem"
  />
</template>