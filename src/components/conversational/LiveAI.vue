<script setup lang="ts">
import { computed, ref, watch, nextTick } from 'vue'
import { 
  XMarkIcon, 
  PlayIcon, 
  StopIcon,
  ClipboardDocumentIcon,
  CheckIcon,
  BoltIcon,
  SparklesIcon,
  ArrowsPointingInIcon,
  ArrowsPointingOutIcon
} from '@heroicons/vue/24/outline'

interface InsightItem {
  id: string
  text: string
  timestamp: number
  contextLength: number
  type: 'insight' | 'welcome' | 'question' | 'answer'
}


interface Props {
  show: boolean
  isActive?: boolean
  processing?: boolean
  response?: string
  insights?: InsightItem[]
  error?: string | null
  sessionId?: string | null
  // AI Assistant props (simplified)
  aiProcessing?: boolean
  aiResponse?: string
  aiError?: string | null
  messageCount?: number
  // When true, expand to overlay fullscreen
  fullScreen?: boolean
}

interface Emits {
  (e: 'close'): void
  (e: 'toggle-live'): void
  (e: 'ai-query', query: string): void
  (e: 'toggle-compact'): void
}

const props = defineProps<Props>()
const emit = defineEmits<Emits>()

// UI State
const copiedStates = ref<Record<string, boolean>>({})
const insightsContainer = ref<HTMLElement | null>(null)
const isCompactMode = ref(false)
const questionInput = ref('')
const isAskingQuestion = ref(false)



const handleClose = () => emit('close')
const handleToggleLive = () => emit('toggle-live')
const handleToggleCompact = () => {
  isCompactMode.value = !isCompactMode.value
  emit('toggle-compact')
}

const handleAskQuestion = () => {
  if (!questionInput.value.trim() || isAskingQuestion.value) return
  
  isAskingQuestion.value = true
  emit('ai-query', questionInput.value.trim())
  
  // Clear input after sending
  setTimeout(() => {
    questionInput.value = ''
    isAskingQuestion.value = false
  }, 500)
}


const copyToClipboard = async (text: string, id?: string) => {
  if (!text) return
  
  try {
    await navigator.clipboard.writeText(text)
    
    if (id) {
      copiedStates.value[id] = true
      setTimeout(() => {
        copiedStates.value[id] = false
      }, 1500)
    }
  } catch (error) {
    console.error('Failed to copy:', error)
  }
}

const scrollToBottom = () => {
  nextTick(() => {
    if (insightsContainer.value) {
      insightsContainer.value.scrollTop = insightsContainer.value.scrollHeight
    }
  })
}

const formatTimestamp = (timestamp: number) => {
  return new Date(timestamp).toLocaleTimeString('en-US', {
    hour12: false,
    hour: '2-digit',
    minute: '2-digit'
  })
}

// Auto-scroll when new insights are added
watch(() => props.insights?.length, () => {
  scrollToBottom()
}, { flush: 'post' })

const getConversationIcon = computed(() => {
  return SparklesIcon
})

const statusText = computed(() => {
  if (!props.isActive) return 'Start Conversation Coach'
  if (props.processing) return 'Analyzing...'
  return 'Coach Active'
})

</script>

<template>
  <div v-if="show" :class="['live-assistant', 'live-ai-drawer', { 'fullscreen': props.fullScreen, 'compact-mode': isCompactMode }]">
    <!-- Minimal Header -->
    <div class="assistant-header">
      <div class="header-left">
        <component :is="getConversationIcon" class="w-4 h-4 text-orange-500" />
        <span class="header-title">{{ statusText }}</span>
        <div v-if="isActive" class="live-dot"></div>
      </div>
      <div class="header-right">
        <button 
          @click="handleToggleCompact"
          class="compact-button"
          :class="{ 'active': isCompactMode }"
          :title="isCompactMode ? 'Expand drawer to normal width' : 'Compact mode (25% width)'"
        >
          <component :is="isCompactMode ? ArrowsPointingOutIcon : ArrowsPointingInIcon" class="w-4 h-4" />
        </button>
        <button @click="handleClose" class="close-button">
          <XMarkIcon class="w-4 h-4" />
        </button>
      </div>
    </div>
    
    
    <!-- Main Content -->
    <div class="assistant-content">
      <!-- Start/Stop Toggle -->
      <div class="toggle-section">
        <button 
          @click="handleToggleLive" 
          class="toggle-button"
          :class="{ 'active': isActive }"
        >
          <component :is="isActive ? StopIcon : PlayIcon" class="w-4 h-4" />
          <span>{{ isActive ? 'Stop Coach' : 'Start Coach' }}</span>
        </button>
      </div>
      
      <!-- Question Input Bar -->
      <div v-if="isActive" class="question-bar">
        <input
          v-model="questionInput"
          @keyup.enter="handleAskQuestion"
          type="text"
          placeholder="Ask a specific question about this conversation..."
          class="question-input"
          :disabled="isAskingQuestion"
        />
        <button 
          @click="handleAskQuestion"
          class="ask-button"
          :disabled="!questionInput.trim() || isAskingQuestion"
        >
          <BoltIcon class="w-4 h-4" />
        </button>
      </div>
      
      <!-- Chat-like Insights Feed -->
      <div class="insights-feed">
        <div ref="insightsContainer" class="insights-container">
          <!-- Show insights as chat messages -->
          <div v-if="insights && insights.length > 0" class="insights-list">
            <div 
              v-for="insight in insights" 
              :key="insight.id"
              class="insight-message"
              :class="{ 
                'welcome-message': insight.type === 'welcome',
                'question-message': insight.type === 'question',
                'answer-message': insight.type === 'answer'
              }"
            >
              <div class="message-header">
                <div class="message-icon">
                  <SparklesIcon v-if="insight.type === 'insight' || insight.type === 'welcome'" class="w-3 h-3 text-orange-400" />
                  <BoltIcon v-else-if="insight.type === 'answer'" class="w-3 h-3 text-blue-400" />
                  <component v-else :is="insight.type === 'question' ? 'span' : SparklesIcon" class="w-3 h-3 text-gray-400">
                    {{ insight.type === 'question' ? 'Q:' : '' }}
                  </component>
                </div>
                <span class="message-time">{{ formatTimestamp(insight.timestamp) }}</span>
                <button 
                  @click="copyToClipboard(insight.text, insight.id)"
                  class="copy-btn"
                  :class="{ 'copied': copiedStates[insight.id] }"
                  title="Copy insight"
                >
                  <ClipboardDocumentIcon v-if="!copiedStates[insight.id]" class="w-3 h-3" />
                  <CheckIcon v-else class="w-3 h-3 text-green-400" />
                </button>
              </div>
              <div class="message-content">
                <div class="insight-text" v-html="insight.text.replace(/\*\*(.*?)\*\*/g, '<strong>$1</strong>')" />
              </div>
            </div>
          </div>
          
          <!-- Processing indicator -->
          <div v-if="processing || aiProcessing" class="processing-message">
            <div class="message-header">
              <div class="message-icon">
                <BoltIcon class="w-3 h-3 animate-pulse text-orange-400" />
              </div>
              <span class="message-time">{{ formatTimestamp(Date.now()) }}</span>
            </div>
            <div class="message-content">
              <div class="processing-text">Analyzing conversation...</div>
            </div>
          </div>
          
          <!-- Empty state -->
          <div v-if="(!insights || insights.length === 0) && !processing && !aiProcessing" class="empty-state">
            <div class="empty-icon">
              <SparklesIcon class="w-8 h-8 text-gray-400" />
            </div>
            <p class="empty-title">
              {{ isActive ? 'Listening for insights...' : 'Conversation Coach Ready' }}
            </p>
            <p class="empty-subtitle">
              {{ isActive ? 'Real-time insights will appear here as the conversation progresses' : 'Start to enable conversation coaching' }}
            </p>
          </div>
        </div>
      </div>
      
      <!-- Error Display -->
      <div v-if="error || aiError" class="error-display">
        <XMarkIcon class="w-4 h-4 text-red-400" />
        <span>{{ error || aiError }}</span>
      </div>
    </div>
  </div>
</template>

<style scoped>
.live-assistant {
  @apply bg-white/[0.02] backdrop-blur-xl border-l border-white/10;
  @apply flex flex-col h-full transition-all duration-300 ease-in-out;
  width: 340px;
  min-width: 340px;
  max-width: 400px;
}

.live-assistant.compact-mode {
  width: 25vw;
  min-width: 280px;
  max-width: 25vw;
}

.live-assistant.fullscreen {
  position: absolute;
  inset: 0;
  width: 100%;
  min-width: 0;
  max-width: none;
  border-left-width: 0;
  z-index: 40;
  @apply border border-white/10 rounded-none;
}

.live-assistant.fullscreen.compact-mode {
  width: 100%;
  max-width: none;
}

.assistant-header {
  @apply flex items-center justify-between px-4 py-3 border-b border-white/10;
  @apply bg-gradient-to-r from-orange-500/5 to-transparent;
}

.header-left {
  @apply flex items-center gap-2;
}

.header-right {
  @apply flex items-center gap-1;
}

.header-title {
  @apply text-sm font-medium text-white/90;
}

.live-dot {
  @apply w-2 h-2 bg-green-400 rounded-full animate-pulse;
}


.compact-button {
  @apply p-1.5 rounded-lg hover:bg-white/10 transition-colors;
  @apply text-white/40 hover:text-white/70;
}

.compact-button.active {
  @apply bg-orange-500/20 text-orange-400;
}

.close-button {
  @apply p-1.5 rounded-lg hover:bg-white/10 transition-colors;
  @apply text-white/60 hover:text-white/90;
}


.assistant-content {
  @apply flex-1 flex flex-col p-4 gap-4 overflow-y-auto;
}

.toggle-section {
  @apply flex justify-center;
}

.toggle-button {
  @apply flex items-center gap-2 px-4 py-2 rounded-xl font-medium;
  @apply bg-orange-500/20 hover:bg-orange-500/30 text-orange-400 hover:text-orange-300;
  @apply border border-orange-500/30 hover:border-orange-500/50;
  @apply transition-all duration-200;
}

.toggle-button.active {
  @apply bg-green-500/20 hover:bg-green-500/30 text-green-400 hover:text-green-300;
  @apply border-green-500/30 hover:border-green-500/50;
}

.insights-feed {
  @apply flex-1 flex flex-col min-h-[200px];
}

.insights-container {
  @apply flex-1 overflow-y-auto bg-white/[0.02] rounded-xl border border-white/10;
  @apply p-3 space-y-3 scroll-smooth;
  max-height: calc(100vh - 200px);
}

.compact-mode .insights-container {
  @apply p-2 space-y-2;
}

.compact-mode .insight-text {
  font-size: 12px;
  line-height: 1.4;
}

.compact-mode .message-header {
  @apply px-2 py-1;
}

.compact-mode .message-content {
  @apply p-2;
}

.insights-list {
  @apply space-y-3;
}

.insight-message {
  @apply bg-white/[0.03] rounded-lg border border-white/10;
  @apply transition-all duration-200;
}

.insight-message.welcome-message {
  @apply bg-orange-500/10 border-orange-500/30;
}

.insight-message.question-message {
  @apply bg-gray-500/10 border-gray-500/30;
  @apply ml-4;
}

.insight-message.answer-message {
  @apply bg-blue-500/10 border-blue-500/30;
}

/* Question Input Bar Styles */
.question-bar {
  @apply flex gap-2 px-4 py-3 border-b border-white/10;
  @apply bg-white/[0.02];
}

.question-input {
  @apply flex-1 px-3 py-2 text-sm rounded-lg;
  @apply bg-white/[0.05] border border-white/10;
  @apply text-white/90 placeholder-white/30;
  @apply focus:outline-none focus:border-orange-500/50 focus:bg-white/[0.08];
  @apply transition-all duration-200;
}

.question-input:disabled {
  @apply opacity-50 cursor-not-allowed;
}

.ask-button {
  @apply px-3 py-2 rounded-lg;
  @apply bg-orange-500/20 hover:bg-orange-500/30;
  @apply text-orange-400 hover:text-orange-300;
  @apply border border-orange-500/30 hover:border-orange-500/50;
  @apply transition-all duration-200;
  @apply disabled:opacity-50 disabled:cursor-not-allowed;
}

.message-header {
  @apply flex items-center justify-between px-3 py-2 border-b border-white/10;
  @apply bg-white/[0.02] rounded-t-lg;
}

.message-icon {
  @apply flex items-center gap-2;
}

.message-time {
  @apply text-xs text-white/40 font-mono;
}

.message-content {
  @apply p-3;
}

.insight-text {
  @apply text-sm text-white/90 leading-relaxed whitespace-pre-wrap;
  font-size: 13px;
  line-height: 1.5;
}

.processing-message {
  @apply bg-orange-500/10 rounded-lg border border-orange-500/30;
}

.processing-text {
  @apply text-sm text-orange-400 italic;
}



.copy-btn {
  @apply p-1 rounded hover:bg-white/10 text-white/40 hover:text-white/80;
  @apply transition-colors duration-200;
}

.copy-btn.copied {
  @apply text-green-400;
}


.empty-state {
  @apply flex-1 flex flex-col items-center justify-center py-12 text-center;
}

.empty-icon {
  @apply mb-3;
}

.empty-title {
  @apply text-sm font-medium text-white/70 mb-1;
}

.empty-subtitle {
  @apply text-xs text-white/40;
}

.error-display {
  @apply flex items-center gap-2 p-3 rounded-lg;
  @apply bg-red-500/10 border border-red-500/30 text-red-400;
}

/* Responsive adjustments */
@media (max-height: 700px) {
  .insights-container {
    max-height: calc(100vh - 180px);
  }
}

/* Scrollbar styling */
.insights-container::-webkit-scrollbar {
  width: 4px;
}

.insights-container::-webkit-scrollbar-track {
  background: transparent;
}

.insights-container::-webkit-scrollbar-thumb {
  background: rgba(255, 255, 255, 0.2);
  border-radius: 2px;
}

.insights-container::-webkit-scrollbar-thumb:hover {
  background: rgba(255, 255, 255, 0.3);
}
</style>