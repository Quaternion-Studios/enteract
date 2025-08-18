<script setup lang="ts">
import { type PropType, ref, computed } from 'vue'
import { CloudArrowUpIcon, FolderIcon, ArrowsPointingOutIcon, TrashIcon, DocumentTextIcon, ChartBarIcon } from '@heroicons/vue/24/outline'
import { formatFileSize as formatBytes } from '@/utils/formatters'

interface RagDoc {
  id: string
  file_name: string
  file_size: number
  file_type: string
  created_at: string | number
  access_count: number
  is_cached: boolean
  last_accessed?: string | number | null
}

const props = defineProps({
  documents: { type: Array as PropType<RagDoc[]>, required: true },
  cachedDocuments: { type: Array as PropType<RagDoc[]>, required: true },
  selectedIds: { type: Object as PropType<Set<string>>, required: true },
  totalStorageSizeMB: { type: Number, required: true },
  settingsMaxDocSizeMb: { type: Number, required: true },
  maxCachedDocuments: { type: Number, required: true },
  isUploading: { type: Boolean, required: true },
  isDragOver: { type: Boolean, required: true },
  handleFileUpload: { type: Function as PropType<(e: Event) => Promise<void> | void>, required: true },
  handleDragOver: { type: Function as PropType<(e: DragEvent) => void>, required: true },
  handleDragLeave: { type: Function as PropType<() => void>, required: true },
  handleDrop: { type: Function as PropType<(e: DragEvent) => Promise<void> | void>, required: true },
  clearAllSelections: { type: Function as PropType<() => void>, required: true },
  selectAllDocuments: { type: Function as PropType<() => void>, required: true },
  toggleDocumentSelection: { type: Function as PropType<(id: string) => void>, required: true },
  deleteDocument: { type: Function as PropType<(id: string) => Promise<void> | void>, required: true },
  generateEmbeddings: { type: Function as PropType<(id: string) => Promise<void> | void>, required: true },
  getStorageStats: { type: Function as PropType<() => Promise<any> | void>, required: true },
  getDocumentIcon: { type: Function as PropType<(type: string) => string>, required: true },
  formatFileSize: { type: Function as PropType<(bytes: number) => string>, required: true }
})

const emit = defineEmits<{ (e: 'clearCache'): void }>()
const fileInputRef = ref<HTMLInputElement>()

const formattedStorageSize = computed(() => {
  const totalBytes = props.totalStorageSizeMB * 1024 * 1024
  return formatBytes(totalBytes)
})
</script>

<template>
  <div class="settings-section">
    <div class="section-header">
      <h2 class="section-title">Document Management</h2>
      <p class="section-description">
        Manage your RAG knowledge base. Upload documents to enhance AI responses with relevant context from your files.
      </p>
    </div>

    <input
      ref="fileInputRef"
      type="file"
      multiple
      accept=".pdf,.txt,.md,.doc,.docx,.rtf"
      @change="handleFileUpload"
      class="hidden"
    />

    <div class="documents-upload-section">
      <div class="upload-header">
        <h3 class="text-white/90 font-medium">Upload Documents</h3>
        <div class="upload-actions">
          <button 
            @click="() => fileInputRef?.click()" 
            :disabled="isUploading"
            class="upload-btn"
            title="Select Files"
          >
            <CloudArrowUpIcon class="w-4 h-4" />
            {{ isUploading ? 'Uploading...' : 'Upload Files' }}
          </button>
        </div>
      </div>

      <div
        class="upload-dropzone"
        :class="{ 'drag-over': isDragOver, 'uploading': isUploading }"
        @dragover="handleDragOver"
        @dragleave="handleDragLeave"
        @drop="handleDrop"
        @click="() => fileInputRef?.click()"
      >
        <div class="dropzone-content">
          <FolderIcon class="w-12 h-12 text-white/40 mb-4" />
          <p class="text-white/80 font-medium mb-2">
            {{ isUploading ? 'Processing files...' : 'Drop files here or click to upload' }}
          </p>
          <p class="text-white/50 text-sm">
            Supports PDF, TXT, MD, DOC, DOCX files (up to {{ settingsMaxDocSizeMb }}MB each)
          </p>
        </div>
      </div>
    </div>

    <div class="storage-stats-section">
      <div class="stats-header">
        <h4 class="text-white/80 font-medium flex items-center gap-2">
          <ChartBarIcon class="w-4 h-4" />
          Storage Statistics
        </h4>
        <button 
          @click="getStorageStats"
          class="refresh-btn"
          title="Refresh Stats"
        >
          <ArrowsPointingOutIcon class="w-4 h-4" />
        </button>
      </div>

      <div class="stats-grid">
        <div class="stat-item">
          <div class="stat-value">{{ documents.length }}</div>
          <div class="stat-label">Total Documents</div>
        </div>
        <div class="stat-item">
          <div class="stat-value">{{ selectedIds.size }}</div>
          <div class="stat-label">Active Context</div>
        </div>
        <div class="stat-item">
          <div class="stat-value">{{ cachedDocuments.length }}</div>
          <div class="stat-label">Cached</div>
        </div>
        <div class="stat-item">
          <div class="stat-value" :title="`${totalStorageSizeMB.toFixed(2)} MB`">{{ formattedStorageSize }}</div>
          <div class="stat-label">Storage Used</div>
        </div>
      </div>
    </div>

    <div class="documents-library-section">
      <div class="library-header">
        <h4 class="text-white/80 font-medium">Document Library</h4>
        <div class="library-actions">
          <button 
            @click="clearAllSelections"
            :disabled="selectedIds.size === 0"
            class="action-btn secondary"
            title="Clear Selection"
          >
            Clear Selection
          </button>
          <button 
            @click="selectAllDocuments"
            :disabled="documents.length === 0"
            class="action-btn secondary"
            title="Select All"
          >
            Select All
          </button>
        </div>
      </div>

      <div v-if="documents.length > 0" class="documents-list">
        <div 
          v-for="doc in documents"
          :key="doc.id"
          class="document-item"
          :class="{ 
            'selected': selectedIds.has(doc.id),
            'cached': doc.is_cached
          }"
        >
          <div class="document-checkbox">
            <input
              type="checkbox"
              :checked="selectedIds.has(doc.id)"
              @change="() => toggleDocumentSelection(doc.id)"
              class="setting-checkbox"
            />
          </div>

          <div class="document-icon">
            {{ getDocumentIcon(doc.file_type) }}
          </div>

          <div class="document-info">
            <div class="document-name">{{ doc.file_name }}</div>
            <div class="document-meta">
              <span>{{ formatFileSize(doc.file_size) }}</span>
              <span class="separator">•</span>
              <span>{{ new Date(doc.created_at).toLocaleDateString() }}</span>
              <span v-if="doc.access_count > 0" class="separator">•</span>
              <span v-if="doc.access_count > 0">Used {{ doc.access_count }}x</span>
            </div>
          </div>

          <div class="document-status">
            <div v-if="doc.is_cached" class="cache-badge active">
              ⚡ Cached
            </div>
            <div v-else class="cache-badge">
              💤 Not Cached
            </div>
          </div>

          <div class="document-actions">
            <button
              @click="() => generateEmbeddings(doc.id)"
              :disabled="doc.is_cached"
              class="action-btn small"
              title="Generate Embeddings"
            >
              🧠
            </button>
            <button
              @click="() => deleteDocument(doc.id)"
              class="action-btn small danger"
              title="Delete Document"
            >
              <TrashIcon class="w-3 h-3" />
            </button>
          </div>
        </div>
      </div>

      <div v-else class="empty-documents">
        <DocumentTextIcon class="w-16 h-16 text-white/20 mb-4" />
        <p class="text-white/60 font-medium mb-2">No documents uploaded</p>
        <p class="text-white/40 text-sm mb-4">
          Upload documents to create a knowledge base for your AI conversations
        </p>
        <button 
          @click="() => fileInputRef?.click()"
          class="upload-btn"
        >
          <CloudArrowUpIcon class="w-4 h-4" />
          Upload Your First Document
        </button>
      </div>
    </div>

    <div v-if="cachedDocuments.length > 0" class="cache-management-section">
      <div class="cache-header">
        <h4 class="text-white/80 font-medium">Cache Management</h4>
        <div class="cache-info">
          <span class="text-white/60 text-sm">
            {{ cachedDocuments.length }} / {{ maxCachedDocuments }} documents cached
          </span>
        </div>
      </div>

      <div class="cache-actions">
        <button 
          @click="$emit('clearCache')"
          class="action-btn danger"
          title="Clear All Cache"
        >
          Clear Embedding Cache
        </button>
      </div>

      <div class="cache-documents">
        <div 
          v-for="doc in cachedDocuments"
          :key="doc.id"
          class="cache-document-item"
        >
          <div class="cache-doc-icon">{{ getDocumentIcon(doc.file_type) }}</div>
          <div class="cache-doc-info">
            <div class="cache-doc-name">{{ doc.file_name }}</div>
            <div class="cache-doc-meta">
              Last accessed: {{ doc.last_accessed ? new Date(doc.last_accessed).toLocaleString() : 'Never' }}
            </div>
          </div>
          <div class="cache-indicator">⚡</div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.storage-stats {
  display: flex;
  gap: 1.5rem;
  margin-top: 1rem;
}

.stat-item {
  display: flex;
  flex-direction: column;
  align-items: center;
  min-width: 0;
  flex: 0 1 auto;
}

.stat-value {
  font-size: 1.25rem;
  font-weight: 600;
  color: rgba(255, 255, 255, 0.9);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  max-width: 100px;
  cursor: default;
}

.stat-value:hover {
  overflow: visible;
  position: relative;
  z-index: 10;
}

.stat-label {
  font-size: 0.75rem;
  color: rgba(255, 255, 255, 0.6);
  margin-top: 0.25rem;
  white-space: nowrap;
}

@media (max-width: 640px) {
  .storage-stats {
    gap: 1rem;
  }
  
  .stat-value {
    font-size: 1rem;
    max-width: 80px;
  }
  
  .stat-label {
    font-size: 0.7rem;
  }
}
</style>


