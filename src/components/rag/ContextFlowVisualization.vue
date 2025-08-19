<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { ArrowRightIcon, DocumentTextIcon, SparklesIcon } from '@heroicons/vue/24/outline'

interface ContextFlow {
  id: string
  type: 'query' | 'context' | 'result'
  content: string
  timestamp: Date
  relevance?: number
  documents?: string[]
}

interface Props {
  flows: ContextFlow[]
  showTimestamp?: boolean
  maxVisible?: number
}

const props = withDefaults(defineProps<Props>(), {
  showTimestamp: true,
  maxVisible: 10
})

// Reactive state
const containerRef = ref<HTMLElement>()
const autoScroll = ref(true)
const visibleFlows = computed(() => {
  return props.flows.slice(-props.maxVisible)
})

// Auto-scroll to bottom when new flows arrive
onMounted(() => {
  scrollToBottom()
})

const scrollToBottom = () => {
  if (containerRef.value && autoScroll.value) {
    containerRef.value.scrollTop = containerRef.value.scrollHeight
  }
}

// Handle scroll to detect manual scrolling
const handleScroll = () => {
  if (containerRef.value) {
    const { scrollTop, scrollHeight, clientHeight } = containerRef.value
    autoScroll.value = scrollTop + clientHeight >= scrollHeight - 10
  }
}

// Watch for new flows and auto-scroll
const observer = new MutationObserver(() => {
  if (autoScroll.value) {
    scrollToBottom()
  }
})

onMounted(() => {
  if (containerRef.value) {
    observer.observe(containerRef.value, { 
      childList: true, 
      subtree: true 
    })
  }
})

onUnmounted(() => {
  observer.disconnect()
})

// Format functions
const formatTime = (date: Date): string => {
  return date.toLocaleTimeString('en-US', { 
    hour12: false, 
    hour: '2-digit', 
    minute: '2-digit', 
    second: '2-digit' 
  })
}

const getFlowIcon = (type: string) => {
  switch (type) {
    case 'query': return '❓'
    case 'context': return '📚'
    case 'result': return '✨'
    default: return '•'
  }
}

const getFlowColor = (type: string) => {
  switch (type) {
    case 'query': return 'text-blue-400 bg-blue-400/10 border-blue-400/20'
    case 'context': return 'text-emerald-400 bg-emerald-400/10 border-emerald-400/20'
    case 'result': return 'text-orange-400 bg-orange-400/10 border-orange-400/20'
    default: return 'text-gray-400 bg-gray-400/10 border-gray-400/20'
  }
}
</script>

<template>
  <div class="context-flow-visualization">
    <!-- Header -->
    <div class="flow-header">
      <div class="flex items-center gap-2">
        <SparklesIcon class="w-4 h-4 text-emerald-400" />
        <span class="font-medium text-white/90">Context Flow</span>
      </div>
      
      <div class="flow-controls">
        <button 
          @click="scrollToBottom"
          :class="['scroll-button', { active: !autoScroll }]"
          title="Scroll to bottom"
        >
          ↓
        </button>
      </div>
    </div>

    <!-- Flow Container -->
    <div 
      ref="containerRef"
      class="flow-container"
      @scroll="handleScroll"
    >
      <div v-if="visibleFlows.length === 0" class="empty-state">
        <div class="empty-icon">
          <SparklesIcon class="w-8 h-8 text-white/20" />
        </div>
        <p class="text-white/40 text-sm">Context flows will appear here</p>
      </div>

      <div class="flow-timeline">
        <div 
          v-for="(flow, index) in visibleFlows" 
          :key="flow.id"
          class="flow-item"
          :class="getFlowColor(flow.type)"
        >
          <!-- Flow Icon -->
          <div class="flow-icon">
            <span class="flow-emoji">{{ getFlowIcon(flow.type) }}</span>
          </div>

          <!-- Flow Content -->
          <div class="flow-content">
            <!-- Header with type and timestamp -->
            <div class="flow-meta">
              <span class="flow-type">{{ flow.type.toUpperCase() }}</span>
              <span v-if="showTimestamp" class="flow-timestamp">
                {{ formatTime(flow.timestamp) }}
              </span>
            </div>

            <!-- Main content -->
            <div class="flow-text">{{ flow.content }}</div>

            <!-- Documents involved -->
            <div v-if="flow.documents && flow.documents.length > 0" class="flow-documents">
              <DocumentTextIcon class="w-3 h-3" />
              <span class="text-xs">{{ flow.documents.length }} document{{ flow.documents.length > 1 ? 's' : '' }}</span>
            </div>

            <!-- Relevance score -->
            <div v-if="flow.relevance !== undefined" class="flow-relevance">
              <div class="relevance-bar">
                <div 
                  class="relevance-fill" 
                  :style="{ width: `${flow.relevance * 100}%` }"
                ></div>
              </div>
              <span class="relevance-text">{{ Math.round(flow.relevance * 100) }}%</span>
            </div>
          </div>

          <!-- Connection arrow (except for last item) -->
          <div v-if="index < visibleFlows.length - 1" class="flow-arrow">
            <ArrowRightIcon class="w-3 h-3 text-white/30 transform rotate-90" />
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.context-flow-visualization {
  @apply flex flex-col h-full bg-gray-900/50 rounded-lg border border-white/10;
}

.flow-header {
  @apply flex items-center justify-between p-3 border-b border-white/10;
  @apply bg-gradient-to-r from-gray-900/80 to-gray-800/80;
}

.flow-controls {
  @apply flex items-center gap-2;
}

.scroll-button {
  @apply w-6 h-6 rounded text-xs font-mono text-white/60 hover:text-white;
  @apply bg-white/5 hover:bg-white/10 border border-white/10;
  @apply transition-all duration-200;
}

.scroll-button.active {
  @apply text-emerald-400 border-emerald-400/30 bg-emerald-400/5;
}

.flow-container {
  @apply flex-1 overflow-y-auto p-3;
  scrollbar-width: thin;
  scrollbar-color: rgba(255, 255, 255, 0.2) transparent;
}

.flow-container::-webkit-scrollbar {
  @apply w-1;
}

.flow-container::-webkit-scrollbar-track {
  @apply bg-transparent;
}

.flow-container::-webkit-scrollbar-thumb {
  @apply bg-white/20 rounded-full;
}

.empty-state {
  @apply flex flex-col items-center justify-center h-32;
}

.empty-icon {
  @apply mb-2;
}

.flow-timeline {
  @apply space-y-3 relative;
}

.flow-timeline::before {
  content: '';
  @apply absolute left-6 top-0 bottom-0 w-px bg-gradient-to-b from-white/20 to-transparent;
}

.flow-item {
  @apply relative flex items-start gap-3 p-3 rounded-lg border;
  @apply transition-all duration-200 hover:scale-[1.02] hover:shadow-lg;
  animation: flowItemAppear 0.3s ease-out;
}

.flow-icon {
  @apply flex-shrink-0 w-8 h-8 rounded-full flex items-center justify-center;
  @apply bg-white/10 border border-current relative z-10;
}

.flow-emoji {
  @apply text-sm;
}

.flow-content {
  @apply flex-1 min-w-0 space-y-2;
}

.flow-meta {
  @apply flex items-center justify-between;
}

.flow-type {
  @apply text-xs font-mono font-semibold tracking-wider;
}

.flow-timestamp {
  @apply text-xs text-white/50 font-mono;
}

.flow-text {
  @apply text-sm text-white/80 leading-relaxed;
}

.flow-documents {
  @apply flex items-center gap-1 text-white/60;
}

.flow-relevance {
  @apply flex items-center gap-2;
}

.relevance-bar {
  @apply w-16 h-1 bg-white/10 rounded-full overflow-hidden;
}

.relevance-fill {
  @apply h-full bg-current rounded-full transition-all duration-500;
}

.relevance-text {
  @apply text-xs font-medium;
}

.flow-arrow {
  @apply absolute -bottom-3 left-6 z-10;
  @apply w-6 h-6 bg-gray-900 rounded-full border border-white/10;
  @apply flex items-center justify-center;
}

/* Animations */
@keyframes flowItemAppear {
  from {
    opacity: 0;
    transform: translateX(-20px);
  }
  to {
    opacity: 1;
    transform: translateX(0);
  }
}

/* Responsive design */
@media (max-width: 640px) {
  .flow-item {
    @apply p-2 gap-2;
  }
  
  .flow-icon {
    @apply w-6 h-6;
  }
  
  .flow-emoji {
    @apply text-xs;
  }
  
  .flow-text {
    @apply text-xs;
  }
}

/* Dark mode enhancements */
.flow-item:hover {
  box-shadow: 0 4px 20px rgba(0, 0, 0, 0.3);
}

/* Pulse animation for active flows */
.flow-item:last-child .flow-icon {
  animation: pulse 2s infinite;
}

@keyframes pulse {
  0%, 100% {
    box-shadow: 0 0 0 0 rgba(current-color, 0.4);
  }
  50% {
    box-shadow: 0 0 0 8px rgba(current-color, 0);
  }
}
</style>