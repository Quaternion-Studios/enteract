<script setup lang="ts">
import { MicrophoneIcon, SpeakerWaveIcon, CheckIcon } from '@heroicons/vue/24/outline'
import MessageSaveIndicator from '../MessageSaveIndicator.vue'

interface Message {
  id: string
  type: 'user' | 'system'
  source: 'microphone' | 'loopback'
  content: string
  confidence?: number
  timestamp: number
  isPreview?: boolean
  isTyping?: boolean
  persistenceState?: 'pending' | 'saving' | 'saved' | 'failed'
  retryCount?: number
  lastSaveAttempt?: number
  saveError?: string
}

interface Props {
  message: Message
  showExportControls?: boolean
  isSelected?: boolean
}

interface Emits {
  (e: 'toggle-selection', id: string): void
}

defineProps<Props>()
const emit = defineEmits<Emits>()

const formatTime = (timestamp: number): string => {
  const date = new Date(timestamp)
  return date.toLocaleTimeString('en-US', { hour: '2-digit', minute: '2-digit' })
}

const handleClick = (messageId: string) => {
  emit('toggle-selection', messageId)
}
</script>

<template>
  <div 
    class="message-wrapper"
    :class="{
      'user-message': message.type === 'user',
      'system-message': message.type === 'system',
      'selectable': showExportControls,
      'selected': isSelected,
      'typing-preview': message.isPreview && message.isTyping
    }"
    @click="showExportControls ? handleClick(message.id) : null"
  >
    <div class="message-bubble" :class="{
      'user-bubble': message.type === 'user',
      'system-bubble': message.type === 'system',
      'typing-bubble': message.isPreview && message.isTyping
    }">
      <div class="message-header">
        <div class="message-source">
          <MicrophoneIcon 
            v-if="message.source === 'microphone'" 
            class="w-3 h-3"
          />
          <SpeakerWaveIcon 
            v-else-if="message.source === 'loopback'" 
            class="w-3 h-3"
          />
          <span class="source-label">
            {{ message.source === 'microphone' ? 'You' : 'System' }}
          </span>
          <div v-if="showExportControls" class="selection-indicator">
            <div class="selection-checkbox" :class="{ 'checked': isSelected }">
              <CheckIcon v-if="isSelected" class="w-2 h-2" />
            </div>
          </div>
        </div>
        <div class="message-meta">
          <MessageSaveIndicator :message="message" />
          <span class="message-time">{{ formatTime(message.timestamp) }}</span>
        </div>
      </div>
      <div class="message-content">
        {{ message.content }}
        <span v-if="message.isPreview && message.isTyping" class="typing-indicator">
          <span class="typing-dot"></span>
          <span class="typing-dot"></span>
          <span class="typing-dot"></span>
        </span>
      </div>
      <div v-if="message.confidence && !message.isPreview" class="message-confidence">
        Confidence: {{ Math.round(message.confidence * 100) }}%
      </div>
    </div>
  </div>
</template>

<style scoped>
.message-wrapper {
  @apply px-4 py-2 transition-all duration-200;
}

.message-wrapper.selectable {
  @apply cursor-pointer hover:bg-white/5;
}

.message-wrapper.selected {
  @apply bg-blue-500/10;
}

.user-message {
  @apply flex justify-end;
}

.system-message {
  @apply flex justify-start;
}

.message-bubble {
  @apply rounded-2xl px-4 py-3 w-full shadow-lg backdrop-blur-sm;
  animation: messageSlideIn 0.3s ease-out;
}

.user-bubble {
  @apply bg-gradient-to-br from-blue-500/20 to-blue-600/20 border border-blue-500/30;
}

.system-bubble {
  @apply bg-gradient-to-br from-purple-500/20 to-purple-600/20 border border-purple-500/30;
}

.typing-bubble {
  @apply opacity-70;
}

.message-header {
  @apply flex items-center justify-between text-xs mb-1;
}

.message-source {
  @apply flex items-center gap-1 text-white/50;
}

.source-label {
  @apply font-medium;
}

.message-meta {
  @apply flex items-center gap-2;
}

.message-time {
  @apply text-white/30;
}

.message-content {
  @apply text-sm text-white/90 leading-relaxed whitespace-pre-wrap;
}

.message-confidence {
  @apply text-xs text-white/40 mt-1;
}

.selection-indicator {
  @apply ml-2;
}

.selection-checkbox {
  @apply w-4 h-4 rounded border border-white/30 flex items-center justify-center transition-all;
}

.selection-checkbox.checked {
  @apply bg-blue-500 border-blue-500;
}

.typing-indicator {
  @apply inline-flex items-center gap-1 ml-1;
}

.typing-dot {
  @apply w-1 h-1 bg-white/50 rounded-full;
  animation: typingDot 1.4s infinite ease-in-out;
}

.typing-dot:nth-child(2) {
  animation-delay: 0.2s;
}

.typing-dot:nth-child(3) {
  animation-delay: 0.4s;
}

@keyframes messageSlideIn {
  from {
    opacity: 0;
    transform: translateY(10px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

@keyframes typingDot {
  0%, 60%, 100% {
    opacity: 0.3;
    transform: scale(1);
  }
  30% {
    opacity: 1;
    transform: scale(1.2);
  }
}
</style>