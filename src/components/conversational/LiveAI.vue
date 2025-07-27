<script setup lang="ts">
import { computed, ref, nextTick, watch } from 'vue'
import { 
  RocketLaunchIcon, 
  XMarkIcon, 
  PlayIcon, 
  StopIcon 
} from '@heroicons/vue/24/outline'

interface SuggestionItem {
  id: string
  text: string
  timestamp: number
  contextLength: number
}

interface Props {
  show: boolean
  isActive?: boolean
  processing?: boolean
  response?: string
  suggestions?: SuggestionItem[]
  error?: string | null
  sessionId?: string | null
  isModelPreloaded?: boolean
}

interface Emits {
  (e: 'close'): void
  (e: 'toggle-live'): void
}

const props = defineProps<Props>()
const emit = defineEmits<Emits>()

const handleClose = () => emit('close')
const handleToggleLive = () => emit('toggle-live')

const statusMessage = computed(() => {
  if (props.isActive && props.processing) {
    return 'Processing conversation in real-time...'
  }
  if (props.isActive) {
    return 'Listening to conversation...'
  }
  if (props.isModelPreloaded) {
    return 'Model pre-loaded! Click Start to enable live AI responses'
  }
  return 'Click Start to enable live AI responses'
})

const copyToClipboard = async (text: string) => {
  if (!text) return
  
  try {
    await navigator.clipboard.writeText(text)
    console.log('✅ Response suggestion copied to clipboard')
  } catch (error) {
    console.error('❌ Failed to copy to clipboard:', error)
  }
}

const formatTimestamp = (timestamp: number) => {
  const date = new Date(timestamp)
  return date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })
}

// Ref for suggestions container to enable auto-scroll
const suggestionsContainer = ref<HTMLElement>()

// Auto-scroll to newest suggestion when suggestions update
watch(() => props.suggestions, async (newSuggestions) => {
  if (newSuggestions && newSuggestions.length > 0) {
    await nextTick()
    if (suggestionsContainer.value) {
      suggestionsContainer.value.scrollTop = 0 // Scroll to top since newest is first
    }
  }
}, { deep: true })
</script>

<template>
  <div v-if="show" class="live-ai-drawer">
    <div class="live-ai-header">
      <div class="flex items-center gap-2">
        <RocketLaunchIcon class="w-4 h-4 text-orange-400" />
        <h3 class="text-sm font-medium text-white">Live Response Assistant</h3>
        <div v-if="isActive" class="live-indicator">
          <div class="live-dot"></div>
          <span class="text-xs text-orange-400">LIVE</span>
        </div>
      </div>
      <button @click="handleClose" class="close-drawer-btn">
        <XMarkIcon class="w-4 h-4" />
      </button>
    </div>
    
    <div class="live-ai-content">
      <!-- Status Section -->
      <div class="live-status-section">
        <div v-if="!isActive" class="start-section">
          <button 
            @click="handleToggleLive"
            class="live-start-btn"
          >
            <PlayIcon class="w-4 h-4" />
            Start Live AI
          </button>
          <p class="text-xs text-white/60 mt-2 text-center">{{ statusMessage }}</p>
        </div>
        <div v-else class="active-status">
          <div class="flex items-center gap-2 justify-center">
            <div class="status-dot active"></div>
            <span class="text-sm text-green-400 font-medium">Live Response Assistant Active</span>
          </div>
          <p class="text-xs text-white/60 mt-1 text-center">{{ statusMessage }}</p>
        </div>
      </div>
      
      <!-- Response Area -->
      <div class="live-response-section">
        <div v-if="error" class="live-error">
          <div class="error-header">
            <XMarkIcon class="w-4 h-4 text-red-400" />
            <span class="text-sm text-red-400">Error</span>
          </div>
          <p class="text-xs text-red-300 mt-1">{{ error }}</p>
        </div>
        
        <div v-else-if="processing" class="live-processing">
          <div class="processing-indicator">
            <div class="processing-dots">
              <div class="dot"></div>
              <div class="dot"></div>
              <div class="dot"></div>
            </div>
            <span class="text-sm text-orange-400">AI is analyzing conversation...</span>
          </div>
        </div>
        
        <div v-else-if="suggestions && suggestions.length > 0" class="suggestions-list">
          <div class="suggestions-header">
            <RocketLaunchIcon class="w-4 h-4 text-orange-400" />
            <span class="text-sm text-orange-400">Response Suggestions</span>
            <span class="text-xs text-white/40">({{ suggestions.length }})</span>
          </div>
          
          <div ref="suggestionsContainer" class="suggestions-content">
            <div 
              v-for="suggestion in suggestions" 
              :key="suggestion.id"
              class="suggestion-item"
            >
              <div class="suggestion-meta">
                <span class="suggestion-time">{{ formatTimestamp(suggestion.timestamp) }}</span>
                <span v-if="suggestion.contextLength > 0" class="suggestion-context">
                  {{ suggestion.contextLength }} messages
                </span>
              </div>
              <div class="suggestion-text">
                <p class="text-sm text-white/90 leading-relaxed">{{ suggestion.text }}</p>
              </div>
              <div class="suggestion-actions">
                <button 
                  class="action-btn copy-btn"
                  @click="copyToClipboard(suggestion.text)"
                  title="Copy this suggestion"
                >
                  <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z" />
                  </svg>
                  Copy
                </button>
              </div>
            </div>
          </div>
        </div>
        
        <div v-else class="live-empty-state">
          <RocketLaunchIcon class="w-8 h-8 text-white/20 mx-auto mb-2" />
          <p class="text-white/60 text-xs text-center">Live Response Assistant</p>
          <p class="text-white/40 text-xs text-center mt-1">
            Start recording to enable AI-powered response suggestions
          </p>
        </div>
      </div>
      
      <!-- Floating Stop Button (only when active) -->
      <div v-if="isActive" class="floating-stop-btn">
        <button 
          @click="handleToggleLive"
          class="stop-btn"
          title="Stop Live AI"
        >
          <StopIcon class="w-3 h-3" />
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.live-ai-drawer {
  @apply w-4/5 border-l border-white/10 bg-white/5 backdrop-blur-sm flex flex-col;
  min-width: 600px;
  max-width: 85vw;
  background: linear-gradient(135deg, 
    rgba(20, 15, 15, 0.95) 0%,
    rgba(25, 15, 10, 0.95) 50%,
    rgba(20, 15, 15, 0.95) 100%
  );
}

.live-ai-header {
  @apply flex items-center justify-between px-4 py-3 border-b border-white/10;
  background: rgba(251, 146, 60, 0.1);
}

.live-indicator {
  @apply flex items-center gap-1 px-2 py-0.5 rounded-full bg-orange-500/20;
}

.live-dot {
  @apply w-1.5 h-1.5 bg-orange-400 rounded-full;
  animation: pulse 2s infinite;
}

@keyframes pulse {
  0%, 100% {
    opacity: 1;
    transform: scale(1);
  }
  50% {
    opacity: 0.6;
    transform: scale(0.8);
  }
}

.close-drawer-btn {
  @apply rounded-full p-1 hover:bg-white/10 transition-colors text-white/70 hover:text-white;
}

.live-ai-content {
  @apply flex-1 overflow-hidden flex flex-col relative;
}

.live-status-section {
  @apply p-4 border-b border-white/10;
}

.start-section {
  @apply flex flex-col items-center;
}

.live-start-btn {
  @apply flex items-center gap-2 px-4 py-2 rounded-lg font-medium text-sm transition-all duration-200;
  background: linear-gradient(135deg, rgba(251, 146, 60, 0.8), rgba(245, 124, 0, 0.8));
  border: 1px solid rgba(251, 146, 60, 0.4);
  color: white;
}

.live-start-btn:hover {
  background: linear-gradient(135deg, rgba(251, 146, 60, 0.9), rgba(245, 124, 0, 0.9));
  border-color: rgba(251, 146, 60, 0.6);
  transform: translateY(-1px);
  box-shadow: 0 4px 12px rgba(251, 146, 60, 0.3);
}

.active-status {
  @apply text-center;
}

.status-dot {
  @apply w-2 h-2 rounded-full bg-white/30 transition-all duration-200;
}

.status-dot.active {
  @apply bg-green-400 animate-pulse;
}

.live-response-section {
  @apply flex-1 overflow-y-auto p-4;
  min-height: 200px;
}

.live-error {
  @apply p-3 rounded-lg bg-red-500/10 border border-red-500/30;
}

.error-header {
  @apply flex items-center gap-2;
}

.live-processing {
  @apply flex items-center justify-center p-8;
}

.processing-indicator {
  @apply flex flex-col items-center gap-3;
}

.processing-dots {
  @apply flex gap-1;
}

.processing-dots .dot {
  @apply w-2 h-2 bg-orange-400 rounded-full;
  animation: bounce 1.4s infinite ease-in-out;
}

.processing-dots .dot:nth-child(2) {
  animation-delay: 0.16s;
}

.processing-dots .dot:nth-child(3) {
  animation-delay: 0.32s;
}

@keyframes bounce {
  0%, 80%, 100% {
    transform: scale(0);
    opacity: 0.5;
  }
  40% {
    transform: scale(1);
    opacity: 1;
  }
}

.suggestions-list {
  @apply space-y-3;
}

.suggestions-header {
  @apply flex items-center gap-2 p-3 rounded-t-lg bg-orange-500/10 border border-orange-500/30;
}

.suggestions-content {
  @apply space-y-2 max-h-96 overflow-y-auto;
}

.suggestion-item {
  @apply p-3 rounded-lg bg-white/5 border border-white/10 space-y-2;
  @apply hover:bg-white/10 transition-colors duration-200;
}

.suggestion-meta {
  @apply flex items-center justify-between text-xs text-white/50;
}

.suggestion-time {
  @apply font-mono;
}

.suggestion-context {
  @apply px-1.5 py-0.5 rounded bg-white/10;
}

.suggestion-text {
  @apply py-1;
}

.suggestion-actions {
  @apply flex justify-end;
}

.action-btn {
  @apply flex items-center gap-1.5 px-2 py-1 rounded text-xs font-medium transition-all duration-200;
  @apply hover:transform hover:scale-105;
}

.copy-btn {
  @apply bg-orange-500/20 text-orange-400 border border-orange-500/30;
  @apply hover:bg-orange-500/30 hover:border-orange-500/50;
}

.live-empty-state {
  @apply flex flex-col items-center justify-center p-8 text-center;
}

.floating-stop-btn {
  @apply absolute bottom-4 right-4;
}

.stop-btn {
  @apply p-2 rounded-full transition-all duration-200;
  background: linear-gradient(135deg, rgba(239, 68, 68, 0.8), rgba(220, 38, 38, 0.8));
  border: 1px solid rgba(239, 68, 68, 0.4);
  color: white;
  box-shadow: 0 2px 8px rgba(239, 68, 68, 0.3);
}

.stop-btn:hover {
  background: linear-gradient(135deg, rgba(239, 68, 68, 0.9), rgba(220, 38, 38, 0.9));
  border-color: rgba(239, 68, 68, 0.6);
  transform: translateY(-1px) scale(1.05);
  box-shadow: 0 4px 12px rgba(239, 68, 68, 0.4);
}
</style>