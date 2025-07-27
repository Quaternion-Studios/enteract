<script setup lang="ts">
import { ref, nextTick, watch } from 'vue'
import { ChatBubbleLeftRightIcon, ChevronDownIcon } from '@heroicons/vue/24/outline'
import MessageItem from './MessageItem.vue'

interface Message {
  id: string
  type: 'user' | 'system'
  source: 'microphone' | 'loopback'
  content: string
  confidence?: number
  timestamp: number
  isPreview?: boolean
  isTyping?: boolean
}

interface Props {
  messages: Message[]
  showExportControls?: boolean
  selectedMessages: Set<string>
}

interface Emits {
  (e: 'toggle-message-selection', id: string): void
}

const props = defineProps<Props>()
const emit = defineEmits<Emits>()

const scrollContainer = ref<HTMLElement>()
const isUserScrolled = ref(false)

const scrollToBottom = async () => {
  await nextTick()
  if (scrollContainer.value) {
    scrollContainer.value.scrollTop = scrollContainer.value.scrollHeight
    isUserScrolled.value = false
  }
}

const checkScrollPosition = () => {
  if (scrollContainer.value) {
    const { scrollTop, scrollHeight, clientHeight } = scrollContainer.value
    const isAtBottom = Math.abs(scrollHeight - scrollTop - clientHeight) < 10
    isUserScrolled.value = !isAtBottom
  }
}

const handleToggleSelection = (id: string) => {
  emit('toggle-message-selection', id)
}

// Auto-scroll when new messages arrive, but only if user hasn't scrolled up
watch(() => props.messages.length, () => {
  if (!isUserScrolled.value) {
    scrollToBottom()
  }
})

// Also watch for content changes (like typing indicators)
watch(() => props.messages, () => {
  if (!isUserScrolled.value) {
    scrollToBottom()
  }
}, { deep: true })

defineExpose({
  scrollToBottom
})
</script>

<template>
  <div ref="scrollContainer" class="messages-container" @scroll="checkScrollPosition">
    <div v-if="messages.length === 0" class="empty-state">
      <div class="empty-icon-wrapper">
        <ChatBubbleLeftRightIcon class="w-16 h-16 text-white/20" />
      </div>
      <h3 class="text-white/60 text-lg font-medium">Start a conversation</h3>
      <p class="text-white/40 text-sm mt-2">
        Click the microphone to speak or enable audio loopback to capture system audio
      </p>
      <p class="text-white/40 text-xs mt-1">
        Your voice will appear on the right, system audio on the left
      </p>
    </div>
    
    <div v-else class="messages-list">
      <MessageItem
        v-for="message in messages"
        :key="message.id"
        :message="message"
        :show-export-controls="showExportControls"
        :is-selected="selectedMessages.has(message.id)"
        @toggle-selection="handleToggleSelection"
      />
    </div>
    
    <!-- Scroll to bottom button -->
    <button 
      v-if="isUserScrolled" 
      @click="scrollToBottom"
      class="scroll-to-bottom-btn"
      title="Scroll to bottom"
    >
      <ChevronDownIcon class="w-4 h-4" />
    </button>
  </div>
</template>

<style scoped>
.messages-container {
  @apply flex-1 overflow-y-auto overflow-x-hidden relative;
  scroll-behavior: smooth;
}

.messages-list {
  @apply py-4;
}

.empty-state {
  @apply flex flex-col items-center justify-center h-full px-8 text-center;
}

.empty-icon-wrapper {
  @apply mb-4 p-6 rounded-full bg-gradient-to-br from-white/5 to-white/10;
}

/* Custom scrollbar */
.messages-container::-webkit-scrollbar {
  width: 6px;
}

.messages-container::-webkit-scrollbar-track {
  background: rgba(255, 255, 255, 0.05);
  border-radius: 3px;
}

.messages-container::-webkit-scrollbar-thumb {
  background: rgba(255, 255, 255, 0.2);
  border-radius: 3px;
}

.messages-container::-webkit-scrollbar-thumb:hover {
  background: rgba(255, 255, 255, 0.3);
}

/* Scroll to bottom button */
.scroll-to-bottom-btn {
  @apply absolute bottom-4 right-4 z-10;
  @apply w-10 h-10 rounded-full;
  @apply bg-blue-500/80 hover:bg-blue-500;
  @apply text-white shadow-lg;
  @apply flex items-center justify-center;
  @apply transition-all duration-200;
  @apply hover:scale-105 active:scale-95;
  backdrop-filter: blur(8px);
}
</style>