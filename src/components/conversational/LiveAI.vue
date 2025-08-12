<script setup lang="ts">
import { computed, ref } from 'vue'
import { 
  XMarkIcon, 
  PlayIcon, 
  StopIcon,
  ClipboardDocumentIcon,
  CheckIcon,
  BoltIcon,
  SparklesIcon
} from '@heroicons/vue/24/outline'

interface SuggestionItem {
  id: string
  text: string
  timestamp: number
  contextLength: number
  responseType?: string
  priority?: 'immediate' | 'soon' | 'normal' | 'low'
  confidence?: number
}


interface Props {
  show: boolean
  isActive?: boolean
  processing?: boolean
  response?: string
  suggestions?: SuggestionItem[]
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
}

const props = defineProps<Props>()
const emit = defineEmits<Emits>()

// UI State
const copiedStates = ref<Record<string, boolean>>({})



const handleClose = () => emit('close')
const handleToggleLive = () => emit('toggle-live')


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

const getConversationIcon = computed(() => {
  return SparklesIcon
})

const statusText = computed(() => {
  if (!props.isActive) return 'Start AI Insights'
  if (props.processing) return 'Listening...'
  return 'AI Insights Active'
})

</script>

<template>
  <div v-if="show" :class="['live-assistant', 'live-ai-drawer', { 'fullscreen': props.fullScreen || props.isActive }]">
    <!-- Minimal Header -->
    <div class="assistant-header">
      <div class="header-left">
        <component :is="getConversationIcon" class="w-4 h-4 text-blue-500" />
        <span class="header-title">{{ statusText }}</span>
        <div v-if="isActive" class="live-dot"></div>
      </div>
      <div class="header-right">
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
          <span>{{ isActive ? 'Stop' : 'Start' }}</span>
        </button>
      </div>
      
      <!-- Large Recommendations Area -->
      <div class="recommendations-area">
        <div v-if="processing || aiProcessing" class="processing">
          <BoltIcon class="w-5 h-5 animate-pulse text-blue-500" />
          <span>Thinking...</span>
        </div>
        
        <div v-else-if="suggestions && suggestions.length > 0" class="suggestions">
          <div class="suggestions-header">
            <SparklesIcon class="w-4 h-4 text-blue-500" />
            <span class="suggestions-title">AI Insights</span>
          </div>
          <!-- Display primary suggestion prominently -->
          <div class="primary-suggestion">
            <div
              v-for="suggestion in suggestions"
              :key="suggestion.id"
              @click="copyToClipboard(suggestion.text, suggestion.id)"
              class="suggestion-card"
              :class="{ 
                'urgent': suggestion.priority === 'immediate',
                'copied': copiedStates[suggestion.id]
              }"
            >
              <div class="suggestion-text">{{ suggestion.text }}</div>
              <div class="suggestion-actions">
                <button class="copy-btn" :class="{ 'copied': copiedStates[suggestion.id] }">
                  <ClipboardDocumentIcon v-if="!copiedStates[suggestion.id]" class="w-4 h-4" />
                  <CheckIcon v-else class="w-4 h-4 text-green-500" />
                  <span>{{ copiedStates[suggestion.id] ? 'Copied!' : 'Copy' }}</span>
                </button>
              </div>
            </div>
          </div>
        </div>
        
        <div v-else-if="aiResponse" class="ai-response">
          <div class="response-header">
            <SparklesIcon class="w-4 h-4 text-blue-500" />
            <span class="response-title">AI Analysis</span>
            <button 
              @click="copyToClipboard(aiResponse, 'ai-response')"
              class="copy-btn"
              :class="{ 'copied': copiedStates['ai-response'] }"
            >
              <component 
                :is="copiedStates['ai-response'] ? CheckIcon : ClipboardDocumentIcon" 
                class="w-3 h-3" 
              />
            </button>
          </div>
          <p class="response-text">{{ aiResponse }}</p>
        </div>
        
        <div v-else class="empty-state">
          <div class="empty-icon">
            <component :is="isActive ? BoltIcon : SparklesIcon" 
              class="w-8 h-8 text-gray-400" 
            />
          </div>
          <p class="empty-title">
            {{ isActive ? 'Listening for insights...' : 'AI Insights Ready' }}
          </p>
          <p class="empty-subtitle">
            {{ isActive ? 'Conversation summaries and suggestions will appear here' : 'Start to enable conversation insights' }}
          </p>
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
  @apply flex flex-col h-full;
  width: 340px;
  min-width: 340px;
  max-width: 400px;
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

.assistant-header {
  @apply flex items-center justify-between px-4 py-3 border-b border-white/10;
  @apply bg-gradient-to-r from-blue-500/5 to-transparent;
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
  @apply bg-blue-500/20 hover:bg-blue-500/30 text-blue-400 hover:text-blue-300;
  @apply border border-blue-500/30 hover:border-blue-500/50;
  @apply transition-all duration-200;
}

.toggle-button.active {
  @apply bg-green-500/20 hover:bg-green-500/30 text-green-400 hover:text-green-300;
  @apply border-green-500/30 hover:border-green-500/50;
}

.recommendations-area {
  @apply flex-1 flex flex-col min-h-[200px] bg-white/[0.02] rounded-xl p-3;
  @apply border border-white/10;
}

.primary-suggestion {
  @apply space-y-3;
}

.suggestion-card {
  @apply p-4 rounded-xl bg-white/[0.05] hover:bg-white/[0.08];
  @apply border border-white/10 hover:border-white/20;
  @apply cursor-pointer transition-all duration-200;
}

.suggestion-card.urgent {
  @apply border-orange-500/40 bg-orange-500/10;
  animation: gentle-pulse 2s infinite;
}

.suggestion-card.copied {
  @apply bg-green-500/20 border-green-500/40;
}

.processing {
  @apply flex items-center justify-center gap-2 py-8 text-blue-400;
}

.suggestions-header {
  @apply flex items-center gap-2 mb-3 pb-2 border-b border-white/10;
}

.suggestions-title {
  @apply text-sm font-semibold text-white/90;
}

.suggestion-text {
  @apply text-sm text-white/90 leading-relaxed mb-3;
  font-size: 14px;
  line-height: 1.6;
}

.suggestion-actions {
  @apply flex items-center justify-between;
}


.copy-btn {
  @apply flex items-center gap-1 px-3 py-1.5 rounded-lg;
  @apply bg-white/10 hover:bg-white/20 text-white/70 hover:text-white/90;
  @apply transition-all duration-200 text-xs font-medium;
}

.copy-btn.copied {
  @apply bg-green-500/20 text-green-400;
}

@keyframes gentle-pulse {
  0%, 100% { border-color: rgba(251, 146, 60, 0.4); }
  50% { border-color: rgba(251, 146, 60, 0.6); }
}

.ai-response {
  @apply p-3 rounded-lg bg-blue-500/10 border border-blue-500/30;
}

.response-header {
  @apply flex items-center gap-2 mb-2;
}

.response-title {
  @apply text-sm font-medium text-blue-400 flex-1;
}

.copy-btn {
  @apply p-1 rounded hover:bg-white/10 text-white/40 hover:text-white/80;
  @apply transition-colors;
}

.copy-btn.copied {
  @apply text-green-400;
}

.response-text {
  @apply text-xs text-white/80 leading-relaxed whitespace-pre-wrap;
  font-size: 12px;
  line-height: 1.5;
}

.empty-state {
  @apply flex-1 flex flex-col items-center justify-center py-8 text-center;
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
  .recommendations-area {
    @apply min-h-[150px];
  }
}
</style>