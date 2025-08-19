<template>
  <div v-if="isOpen" class="fixed inset-0 z-50 overflow-y-auto" @keydown.esc="closeModal">
    <!-- Background overlay -->
    <div class="fixed inset-0 bg-black bg-opacity-50 transition-opacity" @click="closeModal"></div>
    
    <!-- Modal content -->
    <div class="relative min-h-screen flex items-center justify-center p-4">
      <div class="relative bg-white dark:bg-gray-800 rounded-lg shadow-xl max-w-4xl w-full max-h-[90vh] flex flex-col">
        
        <!-- Header -->
        <div class="flex items-center justify-between p-6 border-b border-gray-200 dark:border-gray-700">
          <div>
            <h2 class="text-xl font-semibold text-gray-900 dark:text-white">
              Context Search
            </h2>
            <p class="text-sm text-gray-500 dark:text-gray-400 mt-1">
              Search across all documents with intelligent ranking
            </p>
          </div>
          <button 
            @click="closeModal"
            class="text-gray-400 hover:text-gray-600 dark:hover:text-gray-300 transition-colors"
          >
            <XMarkIcon class="h-6 w-6" />
          </button>
        </div>

        <!-- Search Input -->
        <div class="p-6 border-b border-gray-200 dark:border-gray-700">
          <div class="relative">
            <MagnifyingGlassIcon class="absolute left-3 top-1/2 transform -translate-y-1/2 h-5 w-5 text-gray-400" />
            <input
              ref="searchInput"
              v-model="searchQuery"
              @input="handleSearchInput"
              @keydown.enter="performSearch"
              type="text"
              placeholder="Search documents... (e.g., authentication, database setup, API docs)"
              class="w-full pl-10 pr-4 py-3 border border-gray-300 dark:border-gray-600 rounded-lg 
                     bg-white dark:bg-gray-700 text-gray-900 dark:text-white
                     focus:ring-2 focus:ring-blue-500 focus:border-transparent
                     placeholder-gray-500 dark:placeholder-gray-400"
            />
            <div v-if="isLoading" class="absolute right-3 top-1/2 transform -translate-y-1/2">
              <div class="animate-spin rounded-full h-5 w-5 border-b-2 border-blue-500"></div>
            </div>
          </div>

          <!-- Search Suggestions -->
          <div v-if="searchSuggestions.length > 0 && !hasResults" class="mt-3">
            <div class="text-sm text-gray-600 dark:text-gray-400 mb-2">Suggestions:</div>
            <div class="flex flex-wrap gap-2">
              <button
                v-for="suggestion in searchSuggestions"
                :key="suggestion"
                @click="searchQuery = suggestion; performSearch()"
                class="px-3 py-1 text-sm bg-gray-100 dark:bg-gray-700 text-gray-700 dark:text-gray-300
                       rounded-full hover:bg-gray-200 dark:hover:bg-gray-600 transition-colors"
              >
                {{ suggestion }}
              </button>
            </div>
          </div>
        </div>

        <!-- Filters (Collapsible) -->
        <div class="px-6 py-4 border-b border-gray-200 dark:border-gray-700">
          <button
            @click="showFilters = !showFilters"
            class="flex items-center text-sm text-gray-600 dark:text-gray-400 hover:text-gray-800 dark:hover:text-gray-200"
          >
            <AdjustmentsHorizontalIcon class="h-4 w-4 mr-2" />
            Filters & Ranking
            <ChevronDownIcon 
              class="h-4 w-4 ml-2 transition-transform"
              :class="{ 'rotate-180': showFilters }"
            />
          </button>
          
          <div v-if="showFilters" class="mt-4 space-y-4">
            <!-- File Type Filters -->
            <div>
              <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                File Types
              </label>
              <div class="flex flex-wrap gap-2">
                <label
                  v-for="type in availableFileTypes"
                  :key="type"
                  class="flex items-center"
                >
                  <input
                    v-model="searchFilters.file_types"
                    :value="type"
                    type="checkbox"
                    class="rounded border-gray-300 text-blue-600 focus:ring-blue-500"
                  />
                  <span class="ml-2 text-sm text-gray-600 dark:text-gray-400">{{ type }}</span>
                </label>
              </div>
            </div>

            <!-- Ranking Options -->
            <div>
              <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                Ranking Weights
              </label>
              <div class="grid grid-cols-2 gap-4">
                <div>
                  <label class="block text-xs text-gray-500 dark:text-gray-400">
                    Semantic ({{ Math.round(rankingOptions.semantic_weight * 100) }}%)
                  </label>
                  <input
                    v-model.number="rankingOptions.semantic_weight"
                    type="range"
                    min="0"
                    max="1"
                    step="0.1"
                    class="w-full"
                  />
                </div>
                <div>
                  <label class="block text-xs text-gray-500 dark:text-gray-400">
                    Keyword ({{ Math.round(rankingOptions.keyword_weight * 100) }}%)
                  </label>
                  <input
                    v-model.number="rankingOptions.keyword_weight"
                    type="range"
                    min="0"
                    max="1"
                    step="0.1"
                    class="w-full"
                  />
                </div>
                <div>
                  <label class="block text-xs text-gray-500 dark:text-gray-400">
                    Recency ({{ Math.round(rankingOptions.recency_weight * 100) }}%)
                  </label>
                  <input
                    v-model.number="rankingOptions.recency_weight"
                    type="range"
                    min="0"
                    max="1"
                    step="0.1"
                    class="w-full"
                  />
                </div>
                <div>
                  <label class="block text-xs text-gray-500 dark:text-gray-400">
                    Usage ({{ Math.round(rankingOptions.usage_weight * 100) }}%)
                  </label>
                  <input
                    v-model.number="rankingOptions.usage_weight"
                    type="range"
                    min="0"
                    max="1"
                    step="0.1"
                    class="w-full"
                  />
                </div>
              </div>
            </div>
          </div>
        </div>

        <!-- Results -->
        <div class="flex-1 overflow-y-auto">
          <!-- No Results -->
          <div v-if="!hasResults && !isLoading && searchQuery" class="p-6 text-center">
            <DocumentMagnifyingGlassIcon class="h-12 w-12 text-gray-400 mx-auto mb-4" />
            <h3 class="text-lg font-medium text-gray-900 dark:text-white mb-2">
              No documents found
            </h3>
            <p class="text-gray-500 dark:text-gray-400">
              Try adjusting your search terms or filters
            </p>
          </div>

          <!-- Search Results -->
          <div v-else-if="hasResults" class="p-6">
            <div class="mb-4 flex items-center justify-between">
              <span class="text-sm text-gray-600 dark:text-gray-400">
                {{ totalDocuments }} document{{ totalDocuments !== 1 ? 's' : '' }} found
              </span>
              <button
                @click="selectAllVisible"
                class="text-sm text-blue-600 dark:text-blue-400 hover:text-blue-800 dark:hover:text-blue-300"
              >
                Select All Visible
              </button>
            </div>

            <div class="space-y-4">
              <div
                v-for="result in searchResults"
                :key="result.document.id"
                class="border border-gray-200 dark:border-gray-700 rounded-lg p-4 hover:bg-gray-50 dark:hover:bg-gray-700/50 transition-colors cursor-pointer"
                @click="toggleDocumentSelection(result.document)"
              >
                <div class="flex items-start space-x-3">
                  <!-- Checkbox -->
                  <input
                    :checked="selectedDocuments.has(result.document.id)"
                    type="checkbox"
                    class="mt-1 rounded border-gray-300 text-blue-600 focus:ring-blue-500"
                    @click.stop
                  />

                  <!-- Document Content -->
                  <div class="flex-1 min-w-0">
                    <div class="flex items-center space-x-2 mb-2">
                      <DocumentTextIcon class="h-5 w-5 text-gray-400" />
                      <h4 class="text-sm font-medium text-gray-900 dark:text-white truncate">
                        {{ result.document.file_name }}
                      </h4>
                      <span class="text-xs text-gray-500 dark:text-gray-400 bg-gray-100 dark:bg-gray-700 px-2 py-1 rounded">
                        {{ formatFileType(result.document.file_type) }}
                      </span>
                    </div>

                    <!-- Relevance Score -->
                    <div class="flex items-center space-x-4 mb-2">
                      <div class="flex items-center space-x-1">
                        <span class="text-xs text-gray-500 dark:text-gray-400">Relevance:</span>
                        <div class="w-16 bg-gray-200 dark:bg-gray-600 rounded-full h-2">
                          <div 
                            class="bg-blue-500 h-2 rounded-full"
                            :style="{ width: `${Math.round(result.relevance_score * 100)}%` }"
                          ></div>
                        </div>
                        <span class="text-xs text-gray-500 dark:text-gray-400">
                          {{ Math.round(result.relevance_score * 100) }}%
                        </span>
                      </div>
                      
                      <div v-if="result.document.access_count > 0" class="text-xs text-gray-500 dark:text-gray-400">
                        {{ result.document.access_count }} access{{ result.document.access_count !== 1 ? 'es' : '' }}
                      </div>
                    </div>

                    <!-- Preview -->
                    <p class="text-sm text-gray-600 dark:text-gray-300 line-clamp-2">
                      {{ result.document.content.substring(0, 200) }}...
                    </p>

                    <!-- Rank Factors (Detailed) -->
                    <div v-if="showDetailedScoring" class="mt-2 grid grid-cols-4 gap-2 text-xs">
                      <div class="text-center">
                        <div class="text-gray-500 dark:text-gray-400">Semantic</div>
                        <div class="font-medium">{{ Math.round((result.rank_factors.semantic_relevance || 0) * 100) }}%</div>
                      </div>
                      <div class="text-center">
                        <div class="text-gray-500 dark:text-gray-400">Keyword</div>
                        <div class="font-medium">{{ Math.round((result.rank_factors.keyword_score || 0) * 100) }}%</div>
                      </div>
                      <div class="text-center">
                        <div class="text-gray-500 dark:text-gray-400">Usage</div>
                        <div class="font-medium">{{ Math.round((result.rank_factors.usage_frequency || 0) * 100) }}%</div>
                      </div>
                      <div class="text-center">
                        <div class="text-gray-500 dark:text-gray-400">Recent</div>
                        <div class="font-medium">{{ Math.round((result.rank_factors.recency || 0) * 100) }}%</div>
                      </div>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </div>

          <!-- Initial State -->
          <div v-else class="p-6 text-center">
            <MagnifyingGlassIcon class="h-12 w-12 text-gray-400 mx-auto mb-4" />
            <h3 class="text-lg font-medium text-gray-900 dark:text-white mb-2">
              Intelligent Document Search
            </h3>
            <p class="text-gray-500 dark:text-gray-400 mb-4">
              Search across your entire document collection with AI-powered ranking
            </p>
            
            <!-- Context Suggestions -->
            <div v-if="topSuggestions.length > 0" class="mt-6">
              <h4 class="text-sm font-medium text-gray-700 dark:text-gray-300 mb-3">
                Suggested based on your conversation:
              </h4>
              <div class="space-y-2">
                <button
                  v-for="suggestion in topSuggestions"
                  :key="suggestion.document_id"
                  @click="selectSuggestion(suggestion)"
                  class="block w-full text-left p-3 border border-gray-200 dark:border-gray-700 rounded-lg
                         hover:bg-gray-50 dark:hover:bg-gray-700/50 transition-colors"
                >
                  <div class="font-medium text-sm text-gray-900 dark:text-white">
                    {{ suggestion.document_name }}
                  </div>
                  <div class="text-xs text-gray-500 dark:text-gray-400 mt-1">
                    {{ suggestion.reason }}
                  </div>
                </button>
              </div>
            </div>
          </div>
        </div>

        <!-- Footer -->
        <div class="p-6 border-t border-gray-200 dark:border-gray-700 flex items-center justify-between">
          <div class="flex items-center space-x-4">
            <button
              @click="showDetailedScoring = !showDetailedScoring"
              class="text-sm text-gray-600 dark:text-gray-400 hover:text-gray-800 dark:hover:text-gray-200"
            >
              {{ showDetailedScoring ? 'Hide' : 'Show' }} Detailed Scoring
            </button>
            
            <span class="text-sm text-gray-500 dark:text-gray-400">
              {{ selectedDocuments.size }} selected
            </span>
          </div>
          
          <div class="flex items-center space-x-3">
            <button
              @click="clearSelection"
              class="px-4 py-2 text-sm text-gray-600 dark:text-gray-400 hover:text-gray-800 dark:hover:text-gray-200"
            >
              Clear Selection
            </button>
            <button
              @click="applySelection"
              :disabled="selectedDocuments.size === 0"
              class="px-4 py-2 bg-blue-600 text-white text-sm rounded-lg
                     hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed
                     transition-colors"
            >
              Apply Selection ({{ selectedDocuments.size }})
            </button>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, nextTick, watch } from 'vue'
import {
  XMarkIcon,
  MagnifyingGlassIcon,
  DocumentTextIcon,
  DocumentMagnifyingGlassIcon,
  AdjustmentsHorizontalIcon,
  ChevronDownIcon
} from '@heroicons/vue/24/outline'

import { useContextIntelligence } from '../../composables/rag/useContextIntelligence'

interface Props {
  isOpen: boolean
  selectedDocumentIds?: Set<string>
  conversationHistory?: string[]
}

interface Emits {
  (e: 'close'): void
  (e: 'documents-selected', documentIds: string[]): void
}

const props = withDefaults(defineProps<Props>(), {
  selectedDocumentIds: () => new Set(),
  conversationHistory: () => []
})

const emit = defineEmits<Emits>()

// Context intelligence composable
const {
  isLoading,
  error,
  searchResults,
  topSuggestions,
  hasResults,
  totalDocuments,
  searchFilters,
  rankingOptions,
  searchContextDocuments,
  getContextSuggestions,
  getSearchSuggestions,
  clearContext
} = useContextIntelligence()

// Local state
const searchInput = ref<HTMLInputElement>()
const searchQuery = ref('')
const selectedDocuments = ref(new Set<string>(props.selectedDocumentIds))
const showFilters = ref(false)
const showDetailedScoring = ref(false)
const searchSuggestions = ref<string[]>([])

// Available file types for filtering
const availableFileTypes = [
  'text/plain',
  'text/markdown', 
  'application/pdf',
  'application/msword',
  'application/vnd.openxmlformats-officedocument.wordprocessingml.document'
]

// Watch for prop changes
watch(() => props.selectedDocumentIds, (newIds) => {
  selectedDocuments.value = new Set(newIds)
})

// Handle search input with debouncing
let searchTimeout: NodeJS.Timeout
const handleSearchInput = async () => {
  clearTimeout(searchTimeout)
  
  // Get search suggestions
  if (searchQuery.value.length >= 2) {
    searchSuggestions.value = await getSearchSuggestions(searchQuery.value)
  } else {
    searchSuggestions.value = []
  }
  
  // Debounce actual search
  searchTimeout = setTimeout(() => {
    if (searchQuery.value.trim()) {
      performSearch()
    }
  }, 300)
}

const performSearch = async () => {
  if (!searchQuery.value.trim()) return
  await searchContextDocuments(searchQuery.value)
}

const toggleDocumentSelection = (document: any) => {
  if (selectedDocuments.value.has(document.id)) {
    selectedDocuments.value.delete(document.id)
  } else {
    selectedDocuments.value.add(document.id)
  }
}

const selectAllVisible = () => {
  searchResults.value.forEach(result => {
    selectedDocuments.value.add(result.document.id)
  })
}

const clearSelection = () => {
  selectedDocuments.value.clear()
}

const applySelection = () => {
  emit('documents-selected', Array.from(selectedDocuments.value))
  closeModal()
}

const selectSuggestion = (suggestion: any) => {
  selectedDocuments.value.add(suggestion.document_id)
  emit('documents-selected', Array.from(selectedDocuments.value))
  closeModal()
}

const closeModal = () => {
  clearContext()
  searchQuery.value = ''
  searchSuggestions.value = []
  emit('close')
}

const formatFileType = (fileType: string) => {
  const typeMap: Record<string, string> = {
    'text/plain': 'TXT',
    'text/markdown': 'MD',
    'application/pdf': 'PDF',
    'application/msword': 'DOC',
    'application/vnd.openxmlformats-officedocument.wordprocessingml.document': 'DOCX'
  }
  return typeMap[fileType] || 'FILE'
}

// Initialize component
onMounted(async () => {
  if (props.isOpen) {
    await nextTick()
    searchInput.value?.focus()
    
    // Get context suggestions if we have conversation history
    if (props.conversationHistory.length > 0) {
      await getContextSuggestions(props.conversationHistory)
    }
  }
})

// Watch for modal opening
watch(() => props.isOpen, async (isOpen) => {
  if (isOpen) {
    await nextTick()
    searchInput.value?.focus()
    
    if (props.conversationHistory.length > 0) {
      await getContextSuggestions(props.conversationHistory)
    }
  }
})
</script>

<style scoped>
.line-clamp-2 {
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}
</style>