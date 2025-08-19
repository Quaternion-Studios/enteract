<script setup lang="ts">
import { computed } from 'vue'
import { SparklesIcon, DocumentTextIcon, ClockIcon, XMarkIcon } from '@heroicons/vue/24/outline'
import { CheckCircleIcon } from '@heroicons/vue/24/solid'
import { useContextIntelligence } from '../../composables/useContextIntelligence'

interface Props {
  showContextDetails?: boolean
  compactMode?: boolean
}

interface Emits {
  (e: 'toggleDetails'): void
  (e: 'clearContext'): void
  (e: 'removeDocument', docId: string): void
}

const props = withDefaults(defineProps<Props>(), {
  showContextDetails: false,
  compactMode: false
})

const emit = defineEmits<Emits>()

// Context Intelligence
const contextIntelligence = useContextIntelligence()
const activeDocuments = computed(() => contextIntelligence.getActiveDocuments())
const suggestedDocuments = computed(() => contextIntelligence.getSuggestedDocuments())
const contextMode = computed(() => contextIntelligence.getContextMode())

// Computed properties
const hasActiveContext = computed(() => activeDocuments.value.length > 0)
const contextStrength = computed(() => {
  const count = activeDocuments.value.length
  if (count === 0) return 'none'
  if (count <= 2) return 'light'
  if (count <= 4) return 'medium'
  return 'strong'
})

const contextModeLabel = computed(() => {
  switch (contextMode.value) {
    case 'auto': return 'Auto'
    case 'manual': return 'Manual'
    case 'search': return 'Search'
    case 'all': return 'All'
    case 'none': return 'None'
    default: return 'Unknown'
  }
})

const contextColor = computed(() => {
  switch (contextStrength.value) {
    case 'light': return 'text-blue-400 bg-blue-400/10 border-blue-400/20'
    case 'medium': return 'text-emerald-400 bg-emerald-400/10 border-emerald-400/20'
    case 'strong': return 'text-orange-400 bg-orange-400/10 border-orange-400/20'
    default: return 'text-gray-400 bg-gray-400/10 border-gray-400/20'
  }
})

// Format file size
const formatFileSize = (bytes: number): string => {
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`
}

// Format relevance score
const formatRelevance = (score: number): string => {
  return `${Math.round(score * 100)}%`
}

// Handle actions
const handleToggleDetails = () => {
  emit('toggleDetails')
}

const handleClearContext = async () => {
  await contextIntelligence.clearContext()
  emit('clearContext')
}

const handleRemoveDocument = async (docId: string) => {
  await contextIntelligence.removeDocumentFromContext(docId)
  emit('removeDocument', docId)
}
</script>

<template>
  <div v-if="hasActiveContext || showContextDetails" class="context-indicator">
    <!-- Compact Mode -->
    <div v-if="compactMode" class="context-pill" :class="contextColor">
      <SparklesIcon class="w-3 h-3" />
      <span class="text-xs font-medium">{{ activeDocuments.length }} docs</span>
      <button 
        @click="handleToggleDetails" 
        class="ml-1 hover:bg-white/10 rounded p-0.5 transition-colors"
      >
        <span class="sr-only">Toggle context details</span>
        <svg class="w-3 h-3" :class="{ 'rotate-180': showContextDetails }" 
             fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
        </svg>
      </button>
    </div>

    <!-- Full Mode -->
    <div v-else class="context-panel">
      <!-- Header -->
      <div class="context-header">
        <div class="flex items-center gap-2">
          <SparklesIcon class="w-4 h-4 text-emerald-400" />
          <span class="font-semibold text-white">Active Context</span>
          <div class="context-mode-badge" :class="contextColor">
            {{ contextModeLabel }}
          </div>
        </div>
        
        <div class="flex items-center gap-1">
          <button 
            @click="handleToggleDetails" 
            class="icon-button"
            title="Toggle details"
          >
            <svg class="w-4 h-4" :class="{ 'rotate-180': showContextDetails }" 
                 fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
            </svg>
          </button>
          <button 
            @click="handleClearContext" 
            class="icon-button text-red-400 hover:text-red-300"
            title="Clear context"
          >
            <XMarkIcon class="w-4 h-4" />
          </button>
        </div>
      </div>

      <!-- Context Summary -->
      <div class="context-summary">
        <div class="summary-stat">
          <DocumentTextIcon class="w-4 h-4 text-blue-400" />
          <span>{{ activeDocuments.length }} documents</span>
        </div>
        
        <div v-if="contextStrength !== 'none'" class="context-strength">
          <div class="strength-indicator" :class="contextColor">
            <div class="strength-bar" :class="`strength-${contextStrength}`"></div>
          </div>
          <span class="text-xs text-white/60">{{ contextStrength }} context</span>
        </div>
      </div>

      <!-- Detailed View -->
      <div v-if="showContextDetails" class="context-details">
        <!-- Active Documents -->
        <div v-if="activeDocuments.length > 0" class="document-section">
          <h4 class="section-title">Active Documents</h4>
          <div class="document-list">
            <div 
              v-for="doc in activeDocuments"
              :key="doc.id"
              class="document-item active-document"
            >
              <div class="document-info">
                <div class="document-name">
                  <CheckCircleIcon class="w-3.5 h-3.5 text-emerald-400" />
                  <span>{{ doc.filename }}</span>
                  <div v-if="doc.relevance_score" class="relevance-badge">
                    {{ formatRelevance(doc.relevance_score) }}
                  </div>
                </div>
                <div class="document-meta">
                  <span class="access-count">{{ doc.access_count }} accesses</span>
                  <span class="separator">•</span>
                  <span class="last-accessed">
                    <ClockIcon class="w-3 h-3 inline" />
                    {{ new Date(doc.last_accessed).toLocaleDateString() }}
                  </span>
                </div>
              </div>
              
              <button 
                @click="handleRemoveDocument(doc.id)"
                class="remove-button"
                title="Remove from context"
              >
                <XMarkIcon class="w-3.5 h-3.5" />
              </button>
            </div>
          </div>
        </div>

        <!-- Suggested Documents -->
        <div v-if="suggestedDocuments.length > 0" class="document-section">
          <h4 class="section-title">
            <SparklesIcon class="w-4 h-4 text-yellow-400" />
            AI Suggestions
          </h4>
          <div class="document-list">
            <div 
              v-for="doc in suggestedDocuments.slice(0, 3)"
              :key="doc.id"
              class="document-item suggested-document"
            >
              <div class="document-info">
                <div class="document-name">
                  <span>{{ doc.filename }}</span>
                  <div v-if="doc.relevance_score" class="relevance-badge suggested">
                    {{ formatRelevance(doc.relevance_score) }}
                  </div>
                </div>
                <div v-if="doc.metadata?.suggestion_reason" class="suggestion-reason">
                  {{ doc.metadata.suggestion_reason }}
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.context-indicator {
  @apply relative;
}

.context-pill {
  @apply flex items-center gap-1.5 px-2 py-1 rounded-full border text-xs font-medium;
  @apply transition-all duration-200 hover:scale-105;
}

.context-panel {
  @apply bg-gray-900/95 border border-white/10 rounded-xl p-3;
  @apply backdrop-blur-sm shadow-lg;
  max-width: 400px;
}

.context-header {
  @apply flex items-center justify-between mb-3;
}

.context-mode-badge {
  @apply px-2 py-0.5 rounded-full text-[10px] font-medium border;
}

.icon-button {
  @apply p-1 rounded-md text-white/60 hover:text-white hover:bg-white/5;
  @apply transition-all duration-200;
}

.context-summary {
  @apply flex items-center justify-between mb-3 pb-3 border-b border-white/10;
}

.summary-stat {
  @apply flex items-center gap-2 text-sm text-white/80;
}

.context-strength {
  @apply flex items-center gap-2;
}

.strength-indicator {
  @apply w-16 h-1.5 rounded-full border overflow-hidden;
}

.strength-bar {
  @apply h-full rounded-full transition-all duration-300;
}

.strength-light .strength-bar {
  @apply w-1/3 bg-blue-400;
}

.strength-medium .strength-bar {
  @apply w-2/3 bg-emerald-400;
}

.strength-strong .strength-bar {
  @apply w-full bg-orange-400;
}

.context-details {
  @apply space-y-4;
}

.document-section {
  @apply space-y-2;
}

.section-title {
  @apply flex items-center gap-2 text-sm font-medium text-white/90 mb-2;
}

.document-list {
  @apply space-y-2;
}

.document-item {
  @apply flex items-center justify-between p-2 rounded-lg;
  @apply transition-colors duration-200;
}

.active-document {
  @apply bg-emerald-400/5 border border-emerald-400/20;
}

.suggested-document {
  @apply bg-yellow-400/5 border border-yellow-400/20 hover:bg-yellow-400/10;
}

.document-info {
  @apply flex-1 min-w-0;
}

.document-name {
  @apply flex items-center gap-2 text-sm font-medium text-white/90 mb-1;
}

.document-meta {
  @apply flex items-center gap-1 text-xs text-white/60;
}

.separator {
  @apply text-white/40;
}

.access-count {
  @apply text-emerald-400/80;
}

.last-accessed {
  @apply flex items-center gap-1;
}

.relevance-badge {
  @apply px-1.5 py-0.5 rounded text-[10px] font-medium bg-emerald-400/10 text-emerald-400;
}

.relevance-badge.suggested {
  @apply bg-yellow-400/10 text-yellow-400;
}

.suggestion-reason {
  @apply text-xs text-yellow-400/70 italic mt-1;
}

.remove-button {
  @apply p-1 rounded-md text-white/40 hover:text-red-400 hover:bg-red-400/10;
  @apply transition-all duration-200;
}

/* Animations */
.context-indicator {
  animation: contextFadeIn 0.3s ease-out;
}

@keyframes contextFadeIn {
  from {
    opacity: 0;
    transform: translateY(-10px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

.strength-bar {
  animation: strengthGrow 0.5s ease-out;
}

@keyframes strengthGrow {
  from {
    width: 0;
  }
}
</style>