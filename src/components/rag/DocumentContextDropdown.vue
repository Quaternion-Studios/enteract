<script setup lang="ts">
import { ref, computed, watch, onMounted } from 'vue'
import { DocumentTextIcon, FolderIcon, MagnifyingGlassIcon, XMarkIcon, CloudArrowUpIcon, SparklesIcon, LightBulbIcon, ClockIcon } from '@heroicons/vue/24/outline'
import { CheckCircleIcon } from '@heroicons/vue/24/solid'
import type { EnhancedDocument as Document } from '../../services/rag'
import { useContextIntelligence } from '../../composables/useContextIntelligence'

interface Props {
  documents: Document[]
  selectedDocumentIds: Set<string>
  maxSelections?: number
  show: boolean
  position?: { x: number; y: number }
  searchQuery?: string
  enableSmartSuggestions?: boolean
}

interface Emits {
  (e: 'select', documentId: string): void
  (e: 'deselect', documentId: string): void
  (e: 'close'): void
  (e: 'insertReference', fileName: string): void
  (e: 'uploadDocuments', files: FileList): void
}

const props = withDefaults(defineProps<Props>(), {
  maxSelections: 5,
  searchQuery: '',
  enableSmartSuggestions: true
})

const emit = defineEmits<Emits>()

// State
const searchInput = ref('')
const hoveredDocumentId = ref<string | null>(null)
const dropdownRef = ref<HTMLElement>()
const fileInputRef = ref<HTMLInputElement>()
const activeTab = ref<'suggested' | 'recent' | 'all'>('suggested')

// Context Intelligence
const contextIntelligence = useContextIntelligence()
const suggestedDocuments = computed(() => contextIntelligence.getSuggestedDocuments())
const activeContextDocuments = computed(() => contextIntelligence.getActiveDocuments())

// Computed
const filteredDocuments = computed(() => {
  if (!searchInput.value.trim()) {
    return props.documents
  }
  
  const query = searchInput.value.toLowerCase()
  return props.documents.filter(doc => 
    doc.file_name.toLowerCase().includes(query) ||
    doc.file_type.toLowerCase().includes(query)
  )
})

// Smart document categorization
const smartFilteredDocuments = computed(() => {
  const docs = filteredDocuments.value
  
  if (activeTab.value === 'suggested' && props.enableSmartSuggestions) {
    // Show AI-suggested documents based on conversation context
    const suggestedIds = new Set(suggestedDocuments.value.map(d => d.id))
    return docs.filter(doc => suggestedIds.has(doc.id))
  } else if (activeTab.value === 'recent') {
    // Show recently accessed documents
    const activeIds = new Set(activeContextDocuments.value.map(d => d.id))
    return docs.filter(doc => activeIds.has(doc.id) || doc.is_cached)
      .sort((a, b) => {
        // Sort by last accessed time if available
        const aTime = activeContextDocuments.value.find(d => d.id === a.id)?.last_accessed
        const bTime = activeContextDocuments.value.find(d => d.id === b.id)?.last_accessed
        if (aTime && bTime) {
          return new Date(bTime).getTime() - new Date(aTime).getTime()
        }
        return 0
      })
  } else {
    // Show all documents
    return docs
  }
})

const cachedDocuments = computed(() => {
  return smartFilteredDocuments.value.filter(doc => doc.is_cached)
})

const uncachedDocuments = computed(() => {
  return smartFilteredDocuments.value.filter(doc => !doc.is_cached)
})

// Check if document is suggested by AI
const isDocumentSuggested = (docId: string): boolean => {
  return suggestedDocuments.value.some(d => d.id === docId)
}

// Get suggestion reason for a document
const getSuggestionReason = (docId: string): string | null => {
  const contextDoc = suggestedDocuments.value.find(d => d.id === docId)
  if (contextDoc && contextDoc.metadata?.suggestion_reason) {
    return contextDoc.metadata.suggestion_reason
  }
  return null
}

const canSelectMore = computed(() => {
  return props.selectedDocumentIds.size < props.maxSelections
})

// Watch for search query from parent
watch(() => props.searchQuery, (newQuery) => {
  if (newQuery && (newQuery.startsWith('/') || newQuery.startsWith('@'))) {
    searchInput.value = newQuery.substring(1)
  }
})

// Methods
const toggleDocument = (document: Document) => {
  if (props.selectedDocumentIds.has(document.id)) {
    emit('deselect', document.id)
  } else if (canSelectMore.value) {
    emit('select', document.id)
  }
}

const insertReference = (document: Document) => {
  emit('insertReference', document.file_name)
  emit('close')
}

const formatFileSize = (bytes: number): string => {
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`
}

const formatDate = (dateString: string): string => {
  const date = new Date(dateString)
  const now = new Date()
  const diffMs = now.getTime() - date.getTime()
  const diffHours = Math.floor(diffMs / (1000 * 60 * 60))
  
  if (diffHours < 1) return 'Just now'
  if (diffHours < 24) return `${diffHours}h ago`
  if (diffHours < 48) return 'Yesterday'
  
  const diffDays = Math.floor(diffHours / 24)
  if (diffDays < 7) return `${diffDays}d ago`
  
  return date.toLocaleDateString()
}

const getFileIcon = (fileType: string) => {
  if (fileType.includes('pdf')) return '📄'
  if (fileType.includes('image')) return '🖼️'
  if (fileType.includes('text')) return '📝'
  if (fileType.includes('doc')) return '📃'
  return '📎'
}

const triggerFileUpload = () => {
  fileInputRef.value?.click()
}

const handleFileUpload = (event: Event) => {
  const target = event.target as HTMLInputElement
  if (target.files && target.files.length > 0) {
    emit('uploadDocuments', target.files)
    // Reset the input so the same file can be uploaded again
    target.value = ''
  }
}

// Position dropdown
onMounted(() => {
  if (props.position && dropdownRef.value) {
    dropdownRef.value.style.left = `${props.position.x}px`
    dropdownRef.value.style.top = `${props.position.y}px`
  }
})

// Handle clicks outside
const handleClickOutside = (event: MouseEvent) => {
  if (dropdownRef.value && !dropdownRef.value.contains(event.target as Node)) {
    emit('close')
  }
}

onMounted(() => {
  document.addEventListener('click', handleClickOutside)
})
</script>

<template>
  <Transition name="dropdown">
    <div 
      v-if="show" 
      ref="dropdownRef"
      class="document-context-dropdown"
    >
      <!-- Header -->
      <div class="dropdown-header">
        <div class="header-title">
          <FolderIcon class="w-4 h-4" />
          <span>Document Context</span>
          <span class="selection-count">
            {{ selectedDocumentIds.size }}/{{ maxSelections }}
          </span>
        </div>
        <button @click="$emit('close')" class="close-btn">
          <XMarkIcon class="w-4 h-4" />
        </button>
      </div>
      
      <!-- Search -->
      <div class="search-container">
        <MagnifyingGlassIcon class="w-4 h-4 search-icon" />
        <input
          v-model="searchInput"
          type="text"
          placeholder="Search documents..."
          class="search-input"
          @keydown.escape="$emit('close')"
        />
      </div>
      
      <!-- Tab Navigation for Smart Suggestions -->
      <div v-if="enableSmartSuggestions" class="tab-navigation">
        <button 
          @click="activeTab = 'suggested'"
          :class="['tab-button', { active: activeTab === 'suggested' }]"
        >
          <SparklesIcon class="w-3.5 h-3.5" />
          <span>Suggested</span>
        </button>
        <button 
          @click="activeTab = 'recent'"
          :class="['tab-button', { active: activeTab === 'recent' }]"
        >
          <ClockIcon class="w-3.5 h-3.5" />
          <span>Recent</span>
        </button>
        <button 
          @click="activeTab = 'all'"
          :class="['tab-button', { active: activeTab === 'all' }]"
        >
          <FolderIcon class="w-3.5 h-3.5" />
          <span>All</span>
        </button>
      </div>
      
      <!-- Document List -->
      <div class="document-list">
        <!-- AI Suggested Context (when in suggested tab) -->
        <div v-if="activeTab === 'suggested' && suggestedDocuments.length > 0" class="document-section suggested-section">
          <div class="section-header">
            <span class="section-title">
              <LightBulbIcon class="w-4 h-4 inline-block" />
              AI Recommended
            </span>
            <span class="suggestion-badge">Based on conversation</span>
          </div>
          <div 
            v-for="doc in suggestedDocuments"
            :key="doc.id"
            class="document-item suggested-item"
            :class="{ 
              selected: selectedDocumentIds.has(doc.id),
              disabled: !canSelectMore && !selectedDocumentIds.has(doc.id)
            }"
            @click="toggleDocument(doc)"
            @mouseenter="hoveredDocumentId = doc.id"
            @mouseleave="hoveredDocumentId = null"
          >
            <div class="document-checkbox">
              <CheckCircleIcon v-if="selectedDocumentIds.has(doc.id)" class="w-4 h-4 text-emerald-400" />
              <div v-else class="checkbox-empty" />
            </div>
            <div class="document-info">
              <div class="document-name">
                <span class="file-icon">{{ getFileIcon(doc.file_type || '') }}</span>
                <span class="file-name">{{ doc.filename }}</span>
                <SparklesIcon class="w-3 h-3 text-yellow-400 ml-1" />
              </div>
              <div class="document-meta">
                <span class="relevance-score" v-if="doc.relevance_score">
                  {{ Math.round(doc.relevance_score * 100) }}% relevant
                </span>
                <span v-if="getSuggestionReason(doc.id)" class="suggestion-reason">
                  • {{ getSuggestionReason(doc.id) }}
                </span>
              </div>
            </div>
          </div>
        </div>
        
        <!-- Cached Documents Section -->
        <div v-if="cachedDocuments.length > 0" class="document-section">
          <div class="section-header">
            <span class="section-title">{{ activeTab === 'suggested' ? 'Other' : '' }} Cached Documents</span>
            <span class="cache-indicator active">⚡</span>
          </div>
          <div 
            v-for="doc in cachedDocuments"
            :key="doc.id"
            class="document-item cached"
            :class="{ 
              selected: selectedDocumentIds.has(doc.id),
              hovered: hoveredDocumentId === doc.id
            }"
            @mouseenter="hoveredDocumentId = doc.id"
            @mouseleave="hoveredDocumentId = null"
            @click="toggleDocument(doc)"
          >
            <div class="document-checkbox">
              <CheckCircleIcon 
                v-if="selectedDocumentIds.has(doc.id)" 
                class="w-4 h-4 text-blue-400" 
              />
              <div v-else class="checkbox-empty"></div>
            </div>
            
            <div class="document-icon">
              {{ getFileIcon(doc.file_type) }}
            </div>
            
            <div class="document-info">
              <div class="document-name">{{ doc.file_name }}</div>
              <div class="document-meta">
                <span>{{ formatFileSize(doc.file_size) }}</span>
                <span class="separator">•</span>
                <span>{{ formatDate(doc.created_at) }}</span>
                <span v-if="doc.access_count > 0" class="separator">•</span>
                <span v-if="doc.access_count > 0">Used {{ doc.access_count }}x</span>
              </div>
            </div>
            
            <button 
              class="insert-btn"
              @click.stop="insertReference(doc)"
              title="Insert /reference"
            >
              /
            </button>
          </div>
        </div>
        
        <!-- Uncached Documents Section -->
        <div v-if="uncachedDocuments.length > 0" class="document-section">
          <div class="section-header">
            <span class="section-title">Available Documents</span>
          </div>
          <div 
            v-for="doc in uncachedDocuments"
            :key="doc.id"
            class="document-item"
            :class="{ 
              selected: selectedDocumentIds.has(doc.id),
              hovered: hoveredDocumentId === doc.id,
              disabled: !canSelectMore && !selectedDocumentIds.has(doc.id)
            }"
            @mouseenter="hoveredDocumentId = doc.id"
            @mouseleave="hoveredDocumentId = null"
            @click="toggleDocument(doc)"
          >
            <div class="document-checkbox">
              <CheckCircleIcon 
                v-if="selectedDocumentIds.has(doc.id)" 
                class="w-4 h-4 text-blue-400" 
              />
              <div v-else class="checkbox-empty"></div>
            </div>
            
            <div class="document-icon">
              {{ getFileIcon(doc.file_type) }}
            </div>
            
            <div class="document-info">
              <div class="document-name">{{ doc.file_name }}</div>
              <div class="document-meta">
                <span>{{ formatFileSize(doc.file_size) }}</span>
                <span class="separator">•</span>
                <span>{{ formatDate(doc.created_at) }}</span>
              </div>
            </div>
            
            <button 
              class="insert-btn"
              @click.stop="insertReference(doc)"
              title="Insert /reference"
            >
              /
            </button>
          </div>
        </div>
        
        <!-- Empty State -->
        <div v-if="filteredDocuments.length === 0" class="empty-state">
          <DocumentTextIcon class="w-8 h-8 text-white/30" />
          <p>No documents found</p>
          <p class="empty-hint">Upload documents to enable RAG</p>
        </div>
      </div>
      
      <!-- Hidden file input -->
      <input
        ref="fileInputRef"
        type="file"
        multiple
        accept=".pdf,.txt,.md,.doc,.docx,.rtf"
        @change="handleFileUpload"
        class="hidden"
      />
      
      <!-- Footer -->
      <div class="dropdown-footer">
        <div class="footer-left">
          <div class="footer-info">
            <span v-if="selectedDocumentIds.size > 0">
              {{ selectedDocumentIds.size }} document{{ selectedDocumentIds.size !== 1 ? 's' : '' }} selected
            </span>
            <span v-else>
              Select documents for context
            </span>
          </div>
          <div class="footer-hint">
            Type / to reference documents
          </div>
        </div>
        <button 
          @click="triggerFileUpload"
          class="upload-btn"
          title="Upload new documents"
        >
          <CloudArrowUpIcon class="w-4 h-4" />
          Upload
        </button>
      </div>
    </div>
  </Transition>
</template>

<style scoped>
.document-context-dropdown {
  @apply absolute z-50 w-96 max-h-[500px] rounded-xl overflow-hidden;
  background: linear-gradient(to bottom, 
    rgba(20, 20, 25, 0.98) 0%, 
    rgba(15, 15, 20, 0.98) 100%
  );
  border: 1px solid rgba(255, 255, 255, 0.1);
  box-shadow: 
    0 0 0 1px rgba(0, 0, 0, 0.2),
    0 10px 40px rgba(0, 0, 0, 0.5),
    0 0 60px rgba(59, 130, 246, 0.1);
  backdrop-filter: blur(20px);
  display: flex;
  flex-direction: column;
}

.dropdown-header {
  @apply flex items-center justify-between px-4 py-3 border-b border-white/10;
  background: rgba(0, 0, 0, 0.2);
}

/* Tab Navigation Styles */
.tab-navigation {
  @apply flex items-center gap-1 px-3 py-2 border-b border-white/10;
  background: rgba(0, 0, 0, 0.1);
}

.tab-button {
  @apply flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-medium;
  @apply text-white/60 hover:text-white/80 transition-all duration-200;
  background: transparent;
  border: none;
  cursor: pointer;
}

.tab-button.active {
  @apply text-white bg-white/10;
}

.tab-button svg {
  @apply opacity-70;
}

.tab-button.active svg {
  @apply opacity-100;
}

/* Suggested Section Styles */
.suggested-section {
  @apply border-l-2 border-yellow-400/30;
}

.suggested-item {
  @apply relative;
}

.suggested-item::before {
  content: '';
  @apply absolute inset-0 bg-gradient-to-r from-yellow-400/5 to-transparent pointer-events-none;
}

.suggestion-badge {
  @apply text-[10px] px-2 py-0.5 rounded-full bg-yellow-400/10 text-yellow-400/80;
}

.relevance-score {
  @apply text-[10px] font-medium text-emerald-400/80;
}

.suggestion-reason {
  @apply text-[10px] text-white/40 italic;
}

.header-title {
  @apply flex items-center gap-2 text-sm font-medium text-white/90;
}

.selection-count {
  @apply px-2 py-0.5 rounded-md text-xs;
  background: rgba(59, 130, 246, 0.2);
  color: #60a5fa;
}

.close-btn {
  @apply p-1 rounded-md hover:bg-white/10 transition-colors;
  color: rgba(255, 255, 255, 0.5);
}

.search-container {
  @apply relative px-4 py-3 border-b border-white/10;
}

.search-icon {
  @apply absolute left-6 top-1/2 transform -translate-y-1/2 text-white/40;
}

.search-input {
  @apply w-full pl-8 pr-3 py-2 rounded-lg text-sm;
  background: rgba(255, 255, 255, 0.05);
  border: 1px solid rgba(255, 255, 255, 0.1);
  color: white;
  outline: none;
  transition: all 0.2s;
}

.search-input:focus {
  background: rgba(255, 255, 255, 0.08);
  border-color: rgba(59, 130, 246, 0.4);
}

.document-list {
  @apply flex-1 overflow-y-auto;
  min-height: 200px;
}

.document-section {
  @apply py-2;
}

.section-header {
  @apply flex items-center justify-between px-4 py-2 text-xs font-medium text-white/50;
}

.cache-indicator {
  @apply px-1.5 py-0.5 rounded text-xs;
}

.cache-indicator.active {
  background: rgba(251, 191, 36, 0.2);
  color: #fbbf24;
}

.document-item {
  @apply flex items-center gap-3 px-4 py-2.5 cursor-pointer transition-all duration-200;
}

.document-item:hover {
  background: rgba(255, 255, 255, 0.05);
}

.document-item.selected {
  background: rgba(59, 130, 246, 0.1);
}

.document-item.cached {
  border-left: 2px solid #fbbf24;
}

.document-item.disabled {
  @apply opacity-50 cursor-not-allowed;
}

.document-checkbox {
  @apply flex-shrink-0;
}

.checkbox-empty {
  @apply w-4 h-4 rounded-full border border-white/30;
}

.document-icon {
  @apply text-lg flex-shrink-0;
}

.document-info {
  @apply flex-1 min-w-0;
}

.document-name {
  @apply text-sm font-medium text-white/90 truncate;
}

.document-meta {
  @apply flex items-center gap-1 text-xs text-white/50 mt-0.5;
}

.separator {
  @apply text-white/20;
}

.insert-btn {
  @apply px-2 py-1 rounded-md text-xs font-bold transition-all duration-200;
  background: rgba(139, 92, 246, 0.2);
  color: #a78bfa;
  border: 1px solid rgba(139, 92, 246, 0.3);
}

.insert-btn:hover {
  background: rgba(139, 92, 246, 0.3);
  transform: scale(1.05);
}

.empty-state {
  @apply flex flex-col items-center justify-center py-12 text-center;
}

.empty-state p {
  @apply text-sm text-white/50 mt-2;
}

.empty-hint {
  @apply text-xs text-white/30 mt-1;
}

.dropdown-footer {
  @apply flex items-center justify-between px-4 py-3 border-t border-white/10;
  background: rgba(0, 0, 0, 0.2);
}

.footer-left {
  @apply flex-1;
}

.footer-info {
  @apply text-xs text-white/60;
}

.footer-hint {
  @apply text-xs text-white/40;
}

.upload-btn {
  @apply flex items-center gap-2 px-3 py-2 rounded-lg text-xs font-medium transition-all duration-200;
  background: rgba(34, 197, 94, 0.15);
  border: 1px solid rgba(34, 197, 94, 0.3);
  color: rgb(134, 239, 172);
}

.upload-btn:hover {
  background: rgba(34, 197, 94, 0.25);
  border-color: rgba(34, 197, 94, 0.5);
  color: rgb(187, 247, 208);
  transform: translateY(-1px);
}

.hidden {
  display: none !important;
}

/* Scrollbar */
.document-list::-webkit-scrollbar {
  width: 4px;
}

.document-list::-webkit-scrollbar-track {
  background: transparent;
}

.document-list::-webkit-scrollbar-thumb {
  background: rgba(255, 255, 255, 0.2);
  border-radius: 2px;
}

/* Transitions */
.dropdown-enter-active,
.dropdown-leave-active {
  transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
}

.dropdown-enter-from {
  opacity: 0;
  transform: translateY(-10px) scale(0.95);
}

.dropdown-leave-to {
  opacity: 0;
  transform: translateY(-10px) scale(0.95);
}
</style>