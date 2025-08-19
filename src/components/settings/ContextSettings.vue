<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { SparklesIcon, CogIcon, DocumentTextIcon, ClockIcon } from '@heroicons/vue/24/outline'
import { useContextIntelligence } from '../../composables/useContextIntelligence'
import { useDocumentPriority } from '../../composables/useDocumentPriority'

interface ContextSettings {
  maxCachedDocuments: number
  minRelevanceScore: number
  autoContextMode: boolean
  enableSmartSuggestions: boolean
  backgroundProcessing: boolean
  preloadSimilarDocs: boolean
  contextRefreshInterval: number
  showContextVisualization: boolean
  persistContextSessions: boolean
}

// Composables
const contextIntelligence = useContextIntelligence()
const documentPriority = useDocumentPriority()

// Settings state
const settings = reactive<ContextSettings>({
  maxCachedDocuments: 10,
  minRelevanceScore: 0.7,
  autoContextMode: true,
  enableSmartSuggestions: true,
  backgroundProcessing: true,
  preloadSimilarDocs: true,
  contextRefreshInterval: 5000,
  showContextVisualization: true,
  persistContextSessions: true,
})

const isLoading = ref(false)
const isSaving = ref(false)
const lastSaved = ref<Date | null>(null)

// Load settings on mount
onMounted(async () => {
  await loadSettings()
})

const loadSettings = async () => {
  isLoading.value = true
  try {
    // Load from backend or localStorage
    const savedSettings = localStorage.getItem('context-settings')
    if (savedSettings) {
      const parsed = JSON.parse(savedSettings)
      Object.assign(settings, parsed)
    }
  } catch (error) {
    console.error('Failed to load context settings:', error)
  } finally {
    isLoading.value = false
  }
}

const saveSettings = async () => {
  isSaving.value = true
  try {
    // Save to localStorage
    localStorage.setItem('context-settings', JSON.stringify(settings))
    
    // Update cache strategy
    await documentPriority.updateCacheStrategy({
      maxCachedDocuments: settings.maxCachedDocuments,
      priorityThreshold: settings.minRelevanceScore,
      backgroundProcessing: settings.backgroundProcessing,
      preloadSimilarDocuments: settings.preloadSimilarDocs,
    })
    
    lastSaved.value = new Date()
    
    // Show success feedback
    setTimeout(() => {
      lastSaved.value = null
    }, 3000)
  } catch (error) {
    console.error('Failed to save context settings:', error)
  } finally {
    isSaving.value = false
  }
}

const resetToDefaults = () => {
  settings.maxCachedDocuments = 10
  settings.minRelevanceScore = 0.7
  settings.autoContextMode = true
  settings.enableSmartSuggestions = true
  settings.backgroundProcessing = true
  settings.preloadSimilarDocs = true
  settings.contextRefreshInterval = 5000
  settings.showContextVisualization = true
  settings.persistContextSessions = true
}

const formatInterval = (ms: number): string => {
  const seconds = ms / 1000
  if (seconds < 60) return `${seconds}s`
  return `${Math.round(seconds / 60)}m`
}
</script>

<template>
  <div class="context-settings">
    <!-- Header -->
    <div class="settings-header">
      <div class="flex items-center gap-3">
        <SparklesIcon class="w-5 h-5 text-emerald-400" />
        <h2 class="text-lg font-semibold text-white">Context Intelligence Settings</h2>
      </div>
      
      <div class="flex items-center gap-2">
        <button
          @click="resetToDefaults"
          class="reset-button"
          :disabled="isSaving"
        >
          Reset
        </button>
        <button
          @click="saveSettings"
          class="save-button"
          :disabled="isSaving"
        >
          <CogIcon v-if="isSaving" class="w-4 h-4 animate-spin" />
          <span>{{ isSaving ? 'Saving...' : 'Save' }}</span>
        </button>
      </div>
    </div>

    <div v-if="isLoading" class="loading-state">
      <div class="loading-spinner"></div>
      <p>Loading settings...</p>
    </div>

    <div v-else class="settings-content">
      <!-- Success Message -->
      <div v-if="lastSaved" class="success-message">
        <span class="success-icon">✓</span>
        Settings saved at {{ lastSaved.toLocaleTimeString() }}
      </div>

      <!-- Cache Settings -->
      <div class="settings-section">
        <h3 class="section-title">
          <DocumentTextIcon class="w-4 h-4" />
          Document Caching
        </h3>
        
        <div class="setting-item">
          <label for="maxCached" class="setting-label">
            Maximum Cached Documents
            <span class="setting-description">Number of documents to keep in memory</span>
          </label>
          <input
            id="maxCached"
            v-model.number="settings.maxCachedDocuments"
            type="range"
            min="5"
            max="25"
            step="1"
            class="setting-slider"
          />
          <span class="setting-value">{{ settings.maxCachedDocuments }}</span>
        </div>

        <div class="setting-item">
          <label for="relevanceThreshold" class="setting-label">
            Minimum Relevance Score
            <span class="setting-description">Threshold for caching documents (0-1)</span>
          </label>
          <input
            id="relevanceThreshold"
            v-model.number="settings.minRelevanceScore"
            type="range"
            min="0.1"
            max="1.0"
            step="0.1"
            class="setting-slider"
          />
          <span class="setting-value">{{ settings.minRelevanceScore.toFixed(1) }}</span>
        </div>

        <div class="setting-item">
          <label class="setting-checkbox">
            <input
              v-model="settings.backgroundProcessing"
              type="checkbox"
            />
            <span class="checkmark"></span>
            <span class="checkbox-label">
              Background Processing
              <span class="setting-description">Process embeddings in background</span>
            </span>
          </label>
        </div>

        <div class="setting-item">
          <label class="setting-checkbox">
            <input
              v-model="settings.preloadSimilarDocs"
              type="checkbox"
            />
            <span class="checkmark"></span>
            <span class="checkbox-label">
              Preload Similar Documents
              <span class="setting-description">Automatically cache related documents</span>
            </span>
          </label>
        </div>
      </div>

      <!-- Context Behavior -->
      <div class="settings-section">
        <h3 class="section-title">
          <SparklesIcon class="w-4 h-4" />
          Context Behavior
        </h3>

        <div class="setting-item">
          <label class="setting-checkbox">
            <input
              v-model="settings.autoContextMode"
              type="checkbox"
            />
            <span class="checkmark"></span>
            <span class="checkbox-label">
              Auto Context Mode
              <span class="setting-description">Automatically enable @context for relevant queries</span>
            </span>
          </label>
        </div>

        <div class="setting-item">
          <label class="setting-checkbox">
            <input
              v-model="settings.enableSmartSuggestions"
              type="checkbox"
            />
            <span class="checkmark"></span>
            <span class="checkbox-label">
              Smart Document Suggestions
              <span class="setting-description">AI-powered document recommendations</span>
            </span>
          </label>
        </div>

        <div class="setting-item">
          <label for="refreshInterval" class="setting-label">
            Context Refresh Interval
            <span class="setting-description">How often to update context suggestions</span>
          </label>
          <input
            id="refreshInterval"
            v-model.number="settings.contextRefreshInterval"
            type="range"
            min="1000"
            max="30000"
            step="1000"
            class="setting-slider"
          />
          <span class="setting-value">{{ formatInterval(settings.contextRefreshInterval) }}</span>
        </div>
      </div>

      <!-- User Interface -->
      <div class="settings-section">
        <h3 class="section-title">
          <ClockIcon class="w-4 h-4" />
          User Interface
        </h3>

        <div class="setting-item">
          <label class="setting-checkbox">
            <input
              v-model="settings.showContextVisualization"
              type="checkbox"
            />
            <span class="checkmark"></span>
            <span class="checkbox-label">
              Show Context Visualization
              <span class="setting-description">Display context flow and indicators</span>
            </span>
          </label>
        </div>

        <div class="setting-item">
          <label class="setting-checkbox">
            <input
              v-model="settings.persistContextSessions"
              type="checkbox"
            />
            <span class="checkmark"></span>
            <span class="checkbox-label">
              Persist Context Sessions
              <span class="setting-description">Remember context between chat sessions</span>
            </span>
          </label>
        </div>
      </div>

      <!-- Stats -->
      <div class="settings-section">
        <h3 class="section-title">Statistics</h3>
        <div class="stats-grid">
          <div class="stat-item">
            <span class="stat-label">Total Documents</span>
            <span class="stat-value">{{ documentPriority.priorityCount.value }}</span>
          </div>
          <div class="stat-item">
            <span class="stat-label">High Priority</span>
            <span class="stat-value">{{ documentPriority.highPriorityCount.value }}</span>
          </div>
          <div class="stat-item">
            <span class="stat-label">Processing</span>
            <span class="stat-value">{{ documentPriority.isProcessingPriorities.value ? 'Yes' : 'No' }}</span>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.context-settings {
  @apply p-6 bg-gray-900 text-white min-h-full;
}

.settings-header {
  @apply flex items-center justify-between mb-6 pb-4 border-b border-white/10;
}

.reset-button {
  @apply px-3 py-1.5 text-sm text-white/70 hover:text-white;
  @apply bg-white/5 hover:bg-white/10 rounded-md border border-white/10;
  @apply transition-all duration-200;
}

.save-button {
  @apply flex items-center gap-2 px-4 py-2 text-sm font-medium;
  @apply bg-emerald-600 hover:bg-emerald-500 text-white rounded-md;
  @apply transition-all duration-200 disabled:opacity-50;
}

.loading-state {
  @apply flex flex-col items-center justify-center py-12 text-white/60;
}

.loading-spinner {
  @apply w-8 h-8 border-2 border-white/20 border-t-emerald-400 rounded-full animate-spin mb-3;
}

.success-message {
  @apply flex items-center gap-2 p-3 mb-4 text-sm bg-emerald-400/10 border border-emerald-400/20 rounded-lg;
}

.success-icon {
  @apply text-emerald-400 font-bold;
}

.settings-content {
  @apply space-y-6;
}

.settings-section {
  @apply bg-white/5 rounded-xl p-4 border border-white/10;
}

.section-title {
  @apply flex items-center gap-2 text-lg font-medium text-white mb-4;
}

.setting-item {
  @apply mb-4 last:mb-0;
}

.setting-label {
  @apply block text-sm font-medium text-white/90 mb-2;
}

.setting-description {
  @apply block text-xs text-white/60 font-normal mt-1;
}

.setting-slider {
  @apply w-full h-2 bg-white/20 rounded-lg appearance-none cursor-pointer;
  @apply focus:outline-none focus:ring-2 focus:ring-emerald-400;
}

.setting-slider::-webkit-slider-thumb {
  @apply appearance-none w-4 h-4 bg-emerald-400 rounded-full cursor-pointer;
  @apply hover:bg-emerald-300 transition-colors duration-200;
}

.setting-slider::-moz-range-thumb {
  @apply w-4 h-4 bg-emerald-400 rounded-full cursor-pointer border-none;
  @apply hover:bg-emerald-300 transition-colors duration-200;
}

.setting-value {
  @apply inline-block min-w-[3rem] text-sm font-medium text-emerald-400 ml-3;
}

.setting-checkbox {
  @apply flex items-start gap-3 cursor-pointer text-sm text-white/90;
}

.setting-checkbox input[type="checkbox"] {
  @apply sr-only;
}

.checkmark {
  @apply flex-shrink-0 w-4 h-4 mt-0.5 bg-white/10 border border-white/30 rounded;
  @apply transition-all duration-200;
}

.setting-checkbox input[type="checkbox"]:checked + .checkmark {
  @apply bg-emerald-600 border-emerald-600;
}

.setting-checkbox input[type="checkbox"]:checked + .checkmark::after {
  content: '✓';
  @apply text-white text-xs flex items-center justify-center;
}

.checkbox-label {
  @apply flex-1;
}

.stats-grid {
  @apply grid grid-cols-3 gap-4;
}

.stat-item {
  @apply bg-white/5 rounded-lg p-3 text-center;
}

.stat-label {
  @apply block text-xs text-white/60 mb-1;
}

.stat-value {
  @apply text-lg font-semibold text-emerald-400;
}

/* Responsive design */
@media (max-width: 640px) {
  .context-settings {
    @apply p-4;
  }
  
  .settings-header {
    @apply flex-col gap-3 items-stretch;
  }
  
  .stats-grid {
    @apply grid-cols-1;
  }
}
</style>